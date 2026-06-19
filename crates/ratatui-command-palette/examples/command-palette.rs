use std::fmt;
use std::io::{self, Stdout};
use std::time::Duration;

use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use ratatui_action::id::InputId;
use ratatui_action::input::{ActionChoice, ActionInput};
use ratatui_action::invocation::{ActionArgs, ActionInvocation};
use ratatui_action::spec::{ActionSpec, Availability};
use ratatui_command_palette::event::{MoveSelection, PaletteEvent, PaletteMode};
use ratatui_command_palette::key::PaletteKey;
use ratatui_command_palette::render::{
    FlatOverlayRenderer, FullscreenRenderer, InlineDropdownRenderer, ModalRenderer,
    PaletteRenderer, SplitPreviewRenderer,
};
use ratatui_command_palette::state::PaletteState;

fn main() -> io::Result<()> {
    let args = Args::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let result = run(Terminal::new(CrosstermBackend::new(stdout))?, args);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>, args: Args) -> io::Result<()> {
    let actions = demo_actions();
    let mut app = App::new(args.renderer);
    app.palette.open(&actions);

    loop {
        terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut(), &actions))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        match PaletteKey::from_crossterm(key) {
            PaletteKey::Cancel => {
                if matches!(app.palette.mode(), PaletteMode::CollectingInput { .. }) {
                    app.cancel();
                } else {
                    break;
                }
            }
            PaletteKey::Insert(character) => {
                app.palette.push_query_char(character, &actions);
                app.status = match app.palette.mode() {
                    PaletteMode::Searching => format!("filter: {}", app.palette.query()),
                    PaletteMode::CollectingInput { .. } => {
                        format!("input: {}", app.palette.query())
                    }
                };
            }
            PaletteKey::Backspace => {
                app.palette.pop_query_char(&actions);
                app.status = format!("filter: {}", app.palette.query());
            }
            PaletteKey::Move(movement) => app.move_selection(movement, &actions),
            PaletteKey::Accept => {
                app.accept(&actions);
                if app.should_quit {
                    break;
                }
            }
            PaletteKey::Ignore => {}
        }
    }

    Ok(())
}

struct App {
    palette: PaletteState,
    renderer: RendererKind,
    status: String,
    theme: String,
    preview_theme: Option<String>,
    should_quit: bool,
}

impl App {
    fn new(renderer: RendererKind) -> Self {
        Self {
            palette: PaletteState::new(),
            renderer,
            status: "ready".into(),
            theme: "catppuccin".into(),
            preview_theme: None,
            should_quit: false,
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer, actions: &[ActionSpec]) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(area);

        let theme = self.preview_theme.as_deref().unwrap_or(&self.theme);
        let header = format!("theme: {theme}  renderer: {}", self.renderer.label());
        let header_area = inset_x(layout[0], 1);
        let header_style = Style::new().fg(Color::Cyan).bg(Color::Black);
        buf.set_style(layout[0], header_style);
        Paragraph::new(Line::from(header))
            .style(header_style)
            .render(header_area, buf);

        self.renderer
            .render(layout[1], buf, &self.palette.view(actions));

        let status_area = inset_x(layout[2], 1);
        let status_style = Style::new().fg(Color::Gray).bg(Color::Black);
        buf.set_style(layout[2], status_style);
        Paragraph::new(Line::from(format!("status: {}", self.status)))
            .style(status_style)
            .render(status_area, buf);
    }

    fn accept(&mut self, actions: &[ActionSpec]) {
        let event = self.palette.accept(actions);
        if event.is_none() && matches!(self.palette.mode(), PaletteMode::CollectingInput { .. }) {
            self.status = "collecting input".into();
            return;
        }
        self.handle_event(event);
    }

    fn move_selection(&mut self, movement: MoveSelection, actions: &[ActionSpec]) {
        let event = self.palette.move_selection(movement, actions);
        self.handle_event(event);
    }

    fn cancel(&mut self) {
        let events = self.palette.cancel_events();
        for event in events {
            self.handle_event(Some(event));
        }
    }

    fn handle_event(&mut self, event: Option<PaletteEvent>) {
        match event {
            Some(PaletteEvent::Invoke(invocation)) => {
                self.apply_invocation(&invocation);
                self.status = format!(
                    "invoked {} {}",
                    invocation.id().as_str(),
                    format_args(invocation.args())
                );
            }
            Some(PaletteEvent::PreviewChanged(Some(invocation))) => {
                self.apply_preview(&invocation);
                self.status = format!(
                    "preview {} {}",
                    invocation.id().as_str(),
                    format_args(invocation.args())
                );
            }
            Some(PaletteEvent::PreviewChanged(None)) => {
                self.preview_theme = None;
                self.status = "preview cleared".into();
            }
            Some(PaletteEvent::Cancelled) => {
                self.status = "cancelled".into();
            }
            Some(event) => {
                self.status = format!("event {event:?}");
            }
            None => {
                self.status = "no invocation".into();
            }
        }
    }

    fn apply_preview(&mut self, invocation: &ActionInvocation) {
        if invocation.id().as_str() == "theme.switch"
            && let Some(theme) = invocation.args().get(&InputId::new("theme"))
        {
            self.preview_theme = Some(theme.into());
        }
    }

    fn apply_invocation(&mut self, invocation: &ActionInvocation) {
        match invocation.id().as_str() {
            "theme.switch" => {
                if let Some(theme) = invocation.args().get(&InputId::new("theme")) {
                    self.theme = theme.into();
                    self.preview_theme = None;
                }
            }
            "renderer.switch" => {
                if let Some(renderer) = invocation.args().get(&InputId::new("renderer"))
                    && let Some(renderer) = RendererKind::from_value(renderer)
                {
                    self.renderer = renderer;
                }
            }
            "app.quit" => {
                self.should_quit = true;
            }
            _ => {
                self.preview_theme = None;
            }
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "command-palette",
    about = "Run the Ratatui command palette example",
    version
)]
struct Args {
    /// Renderer to use when the example starts.
    #[arg(long, value_enum, default_value_t = RendererKind::Modal)]
    renderer: RendererKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum RendererKind {
    Modal,
    Flat,
    Split,
    Fullscreen,
    Inline,
}

impl RendererKind {
    fn label(self) -> &'static str {
        match self {
            Self::Modal => "modal",
            Self::Flat => "flat",
            Self::Split => "split",
            Self::Fullscreen => "fullscreen",
            Self::Inline => "inline",
        }
    }

    fn from_value(value: &str) -> Option<Self> {
        match value {
            "modal" => Some(Self::Modal),
            "flat" => Some(Self::Flat),
            "split" => Some(Self::Split),
            "fullscreen" => Some(Self::Fullscreen),
            "inline" => Some(Self::Inline),
            _ => None,
        }
    }

    fn render(
        self,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        view: &ratatui_command_palette::view::PaletteView,
    ) {
        match self {
            Self::Modal => ModalRenderer::new().title("Command Palette").render(
                centered_rect(area, 72, 14),
                buf,
                view,
            ),
            Self::Flat => FlatOverlayRenderer::new().title("Commands").render(
                centered_rect(area, 72, 14),
                buf,
                view,
            ),
            Self::Split => SplitPreviewRenderer::new()
                .title("Commands")
                .preview_title("Details")
                .render(centered_rect(area, 90, 16), buf, view),
            Self::Fullscreen => FullscreenRenderer::new()
                .title("Command Palette")
                .render(area, buf, view),
            Self::Inline => InlineDropdownRenderer::new().title("Commands").render(
                inline_rect(area, 72, 10),
                buf,
                view,
            ),
        }
    }
}

impl fmt::Display for RendererKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn format_args(args: &ActionArgs) -> String {
    let args = args
        .iter()
        .map(|(input, value)| format!("{}={value}", input.as_str()))
        .collect::<Vec<_>>();

    if args.is_empty() {
        String::new()
    } else {
        format!("({})", args.join(", "))
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}

fn inset_x(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x + margin.min(area.width),
        y: area.y,
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        height: area.height,
    }
}

fn inline_rect(area: Rect, width: u16, height: u16) -> Rect {
    Rect {
        x: area.x + 2.min(area.width),
        y: area.y,
        width: width.min(area.width.saturating_sub(4)),
        height: height.min(area.height),
    }
}

fn demo_actions() -> Vec<ActionSpec> {
    let open = ActionSpec::new("document.open", "Open document")
        .with_description("Open the selected document")
        .with_category("Navigation")
        .with_keywords(["jump", "select"]);

    let expand = ActionSpec::new("row.expand", "Expand row")
        .with_description("Show commit details for the selected row")
        .with_category("Rows")
        .with_keywords(["details", "collapse"]);

    let theme = ActionSpec::new("theme.switch", "Switch theme")
        .with_description("Preview and apply a terminal color theme")
        .with_category("Appearance")
        .with_keywords(["color", "style"])
        .with_input(ActionInput::Choice {
            id: InputId::new("theme"),
            label: "Theme".into(),
            choices: vec![
                ActionChoice::new("catppuccin", "Catppuccin"),
                ActionChoice::new("github-dark", "GitHub Dark"),
            ],
        });

    let search = ActionSpec::new("search.open", "Open search")
        .with_description("Collect a search query inline")
        .with_category("Navigation")
        .with_keywords(["find", "filter"])
        .with_input(ActionInput::Text {
            id: InputId::new("query"),
            label: "Query".into(),
            placeholder: Some("Search text".into()),
        });

    let renderer = ActionSpec::new("renderer.switch", "Switch renderer")
        .with_description("Choose a command palette renderer")
        .with_category("Appearance")
        .with_keywords(["layout", "presentation", "view"])
        .with_input(ActionInput::Choice {
            id: InputId::new("renderer"),
            label: "Renderer".into(),
            choices: vec![
                ActionChoice::new("modal", "Modal"),
                ActionChoice::new("flat", "Flat overlay"),
                ActionChoice::new("split", "Split preview"),
                ActionChoice::new("fullscreen", "Fullscreen"),
                ActionChoice::new("inline", "Inline dropdown"),
            ],
        });

    let debug = ActionSpec::new("debug.toggle", "Toggle debug mode")
        .with_description("Collect a boolean value inline")
        .with_category("Debug")
        .with_keywords(["trace", "diagnostics"])
        .with_input(ActionInput::Bool {
            id: InputId::new("enabled"),
            label: "Enabled".into(),
        });

    let disabled = ActionSpec::new("workspace.close", "Close workspace")
        .with_description("Disabled until a workspace is open")
        .with_category("Workspace")
        .with_availability(Availability::Disabled {
            reason: "no workspace".into(),
        });

    let help = ActionSpec::new("help.open", "Open help")
        .with_description("Show application help")
        .with_category("Help")
        .with_keywords(["docs", "manual"]);

    let layout = ActionSpec::new("layout.reset", "Reset layout")
        .with_description("Restore the default pane layout")
        .with_category("Layout")
        .with_keywords(["panes", "default"]);

    let save = ActionSpec::new("document.save", "Save document")
        .with_description("Persist the active document")
        .with_category("Document")
        .with_keywords(["write", "persist"]);

    let zoom = ActionSpec::new("terminal.zoom", "Zoom terminal")
        .with_description("Increase terminal zoom level")
        .with_category("View")
        .with_keywords(["font", "size"]);

    let split = ActionSpec::new("pane.split", "Split pane")
        .with_description("Create a second editing pane")
        .with_category("Layout")
        .with_keywords(["window", "tile"]);

    vec![
        open,
        expand,
        help,
        layout,
        save,
        search,
        theme,
        renderer,
        debug,
        zoom,
        split,
        disabled,
        ActionSpec::new("app.quit", "Quit"),
    ]
}

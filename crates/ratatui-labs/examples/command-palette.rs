use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui_labs::{
    ActionChoice, ActionInput, ActionSpec, Availability, InputId, ModalRenderer, MoveSelection,
    PaletteEvent, PaletteRenderer, PaletteState,
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let result = run(Terminal::new(CrosstermBackend::new(stdout))?);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let actions = demo_actions();
    let mut app = App::new();
    app.palette.open(&actions);

    loop {
        terminal.draw(|frame| app.render(frame.area(), frame.buffer_mut(), &actions))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => break,
            KeyCode::Char(character) => {
                app.palette.push_query_char(character, &actions);
                app.status = format!("filter: {}", app.palette.query());
            }
            KeyCode::Backspace => {
                app.palette.pop_query_char(&actions);
                app.status = format!("filter: {}", app.palette.query());
            }
            KeyCode::Down => app.palette.move_selection(MoveSelection::Next, &actions),
            KeyCode::Up => app
                .palette
                .move_selection(MoveSelection::Previous, &actions),
            KeyCode::PageDown => app
                .palette
                .move_selection(MoveSelection::PageDown(5), &actions),
            KeyCode::PageUp => app
                .palette
                .move_selection(MoveSelection::PageUp(5), &actions),
            KeyCode::Home => app.palette.move_selection(MoveSelection::First, &actions),
            KeyCode::End => app.palette.move_selection(MoveSelection::Last, &actions),
            KeyCode::Enter => app.accept(&actions),
            _ => {}
        }
    }

    Ok(())
}

struct App {
    palette: PaletteState,
    renderer: ModalRenderer,
    status: String,
}

impl App {
    fn new() -> Self {
        Self {
            palette: PaletteState::new(),
            renderer: ModalRenderer::new().title("jk log actions"),
            status: "ready".into(),
        }
    }

    fn render(&self, area: Rect, buf: &mut ratatui::buffer::Buffer, actions: &[ActionSpec]) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        Paragraph::new(Line::from("jk log"))
            .block(Block::new().borders(Borders::ALL).title("Title"))
            .style(Style::new().fg(Color::Cyan))
            .render(layout[0], buf);

        self.renderer.render(
            centered_rect(layout[1], 72, 14),
            buf,
            &self.palette.view(actions),
        );

        Paragraph::new(Line::from(self.status.as_str()))
            .block(Block::new().borders(Borders::ALL).title("Status"))
            .render(layout[2], buf);
    }

    fn accept(&mut self, actions: &[ActionSpec]) {
        match self.palette.accept(actions) {
            Some(PaletteEvent::Invoke(invocation)) => {
                self.status = format!("invoked {}", invocation.id.as_str());
            }
            Some(event) => {
                self.status = format!("event {event:?}");
            }
            None => {
                self.status = "no invocation".into();
            }
        }
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

fn demo_actions() -> Vec<ActionSpec> {
    let mut open = ActionSpec::new("log.open", "Open log");
    open.description = Some("Open the selected jj log entry".into());
    open.category = Some("Navigation".into());
    open.keywords = vec!["jump".into(), "select".into()];

    let mut expand = ActionSpec::new("row.expand", "Expand row");
    expand.description = Some("Show commit details for the selected row".into());
    expand.category = Some("Rows".into());
    expand.keywords = vec!["details".into(), "collapse".into()];

    let mut theme = ActionSpec::new("theme.switch", "Switch theme");
    theme.description = Some("Preview and apply a terminal color theme".into());
    theme.category = Some("Appearance".into());
    theme.keywords = vec!["color".into(), "style".into()];
    theme.inputs.push(ActionInput::Choice {
        id: InputId::new("theme"),
        label: "Theme".into(),
        choices: vec![
            ActionChoice::new("catppuccin", "Catppuccin"),
            ActionChoice::new("github-dark", "GitHub Dark"),
        ],
    });

    let mut disabled = ActionSpec::new("workspace.close", "Close workspace");
    disabled.description = Some("Disabled until a workspace is open".into());
    disabled.category = Some("Workspace".into());
    disabled.availability = Availability::Disabled {
        reason: "no workspace".into(),
    };

    vec![
        open,
        expand,
        theme,
        disabled,
        ActionSpec::new("app.quit", "Quit"),
    ]
}

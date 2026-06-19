use std::io::{self, Stdout};
use std::time::Duration;

use clap::Parser;
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
use ratatui::widgets::{Paragraph, Widget};
use ratatui_action::spec::{ActionSpec, Availability};
use ratatui_command_palette::render::{
    FlatOverlayRenderer, FullscreenRenderer, InlineDropdownRenderer, ModalRenderer,
    PaletteRenderer, SplitPreviewRenderer,
};
use ratatui_command_palette::state::PaletteState;

fn main() -> io::Result<()> {
    Args::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let result = run(Terminal::new(CrosstermBackend::new(stdout))?);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

#[derive(Debug, Parser)]
#[command(
    name = "renderer-gallery",
    about = "Show all built-in command palette renderers in one terminal frame",
    version
)]
struct Args;

fn run(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let actions = demo_actions();
    let mut palette = PaletteState::new();
    palette.open(&actions);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let view = palette.view(&actions);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(area);

            let header_style = Style::new().fg(Color::Cyan).bg(Color::Black);
            frame.buffer_mut().set_style(layout[0], header_style);
            Paragraph::new(Line::from("Renderer gallery"))
                .style(header_style)
                .render(inset_x(layout[0], 1), frame.buffer_mut());

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[1]);
            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[0]);
            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .split(rows[1]);

            ModalRenderer::new()
                .title("Modal")
                .render(inset(top[0], 1), frame.buffer_mut(), &view);
            SplitPreviewRenderer::new()
                .title("Split")
                .preview_title("Details")
                .render(inset(top[1], 1), frame.buffer_mut(), &view);
            FlatOverlayRenderer::new().title("Flat overlay").render(
                inset(bottom[0], 1),
                frame.buffer_mut(),
                &view,
            );
            InlineDropdownRenderer::new().title("Inline").render(
                inset(bottom[1], 1),
                frame.buffer_mut(),
                &view,
            );
            FullscreenRenderer::new().title("Fullscreen").render(
                inset(bottom[2], 1),
                frame.buffer_mut(),
                &view,
            );
        })?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind == KeyEventKind::Press && matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            break;
        }
    }

    Ok(())
}

fn inset(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x + margin.min(area.width),
        y: area.y + margin.min(area.height),
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        height: area.height.saturating_sub(margin.saturating_mul(2)),
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

fn demo_actions() -> Vec<ActionSpec> {
    vec![
        ActionSpec::new("document.open", "Open document")
            .with_description("Open the selected document")
            .with_category("Navigation"),
        ActionSpec::new("search.open", "Open search")
            .with_description("Collect a search query inline")
            .with_category("Navigation"),
        ActionSpec::new("theme.switch", "Switch theme")
            .with_description("Preview and apply a terminal color theme")
            .with_category("Appearance"),
        ActionSpec::new("workspace.close", "Close workspace")
            .with_description("Disabled until a workspace is open")
            .with_category("Workspace")
            .with_availability(Availability::Disabled {
                reason: "no workspace".into(),
            }),
        ActionSpec::new("app.quit", "Quit"),
    ]
}

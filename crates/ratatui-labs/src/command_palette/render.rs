//! Ratatui renderers for command palette views.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};

use crate::action::Availability;
use crate::command_palette::{PaletteMode, PaletteView};

/// Renders a command palette view into a Ratatui buffer.
pub trait PaletteRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView);
}

/// Modal command palette renderer.
#[derive(Clone, Debug)]
pub struct ModalRenderer {
    title: String,
}

impl Default for ModalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ModalRenderer {
    /// Creates a modal renderer with the default title.
    pub fn new() -> Self {
        Self {
            title: "Command Palette".into(),
        }
    }

    /// Sets the modal title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl PaletteRenderer for ModalRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
        Clear.render(area, buf);

        let block = Block::new()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(Color::Cyan));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        render_query(sections[0], buf, view);
        render_rows(sections[1], buf, view);
        render_footer(sections[2], buf, view);
    }
}

fn render_query(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    let query = if view.query.is_empty() {
        Span::styled("type to filter", Style::new().fg(Color::DarkGray))
    } else {
        Span::raw(view.query.as_str())
    };

    let line = Line::from(vec![
        Span::styled(
            "> ",
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        query,
    ]);

    Paragraph::new(line).render(area, buf);
}

fn render_rows(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    if view.rows.is_empty() {
        Paragraph::new(Line::from(Span::styled(
            "No actions match",
            Style::new().fg(Color::DarkGray),
        )))
        .render(area, buf);
        return;
    }

    for (index, row) in view.rows.iter().take(area.height as usize).enumerate() {
        let row_area = Rect {
            x: area.x,
            y: area.y + index as u16,
            width: area.width,
            height: 1,
        };
        let is_selected = view.selected == Some(index);
        let style = row_style(&row.availability, is_selected);

        buf.set_style(row_area, style);

        let text = row_text(row);
        let text = truncate_to_width(&text, row_area.width as usize);

        Paragraph::new(Line::from(Span::styled(text, style))).render(row_area, buf);
    }
}

fn row_text(row: &crate::command_palette::PaletteRow) -> String {
    let detail = match &row.availability {
        Availability::Disabled { reason } => Some(reason.as_str()),
        Availability::Enabled | Availability::Hidden => row.subtitle.as_deref(),
    };

    match (&row.category, detail) {
        (Some(category), Some(detail)) => format!("{}  {category}  {detail}", row.title),
        (Some(category), None) => format!("{}  {category}", row.title),
        (None, Some(detail)) => format!("{}  {detail}", row.title),
        (None, None) => row.title.clone(),
    }
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }

    match width {
        0 => String::new(),
        1..=3 => ".".repeat(width),
        width => {
            let mut truncated = text.chars().take(width - 3).collect::<String>();
            truncated.push_str("...");
            truncated
        }
    }
}

fn row_style(availability: &Availability, selected: bool) -> Style {
    match (availability, selected) {
        (Availability::Enabled, true) => Style::new()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        (Availability::Enabled, false) => Style::new().fg(Color::White),
        (Availability::Disabled { .. }, true) => Style::new().fg(Color::Gray).bg(Color::DarkGray),
        (Availability::Disabled { .. }, false) => Style::new().fg(Color::DarkGray),
        (Availability::Hidden, _) => Style::new().fg(Color::DarkGray),
    }
}

fn render_footer(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    let label = match &view.mode {
        PaletteMode::Searching => "Searching",
        PaletteMode::CollectingInput { .. } => "Collecting input",
    };

    let count = match view.rows.len() {
        1 => "1 action".to_string(),
        count => format!("{count} actions"),
    };

    let line = Line::from(vec![
        Span::styled(label, Style::new().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(count, Style::new().fg(Color::DarkGray)),
    ]);

    Paragraph::new(line).render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionSpec, PaletteState};

    #[test]
    fn selected_row_gets_highlight_background() {
        let actions = vec![
            ActionSpec::new("app.quit", "Quit"),
            ActionSpec::new("theme.switch", "Switch Theme"),
        ];
        let mut state = PaletteState::new();
        state.open(&actions);
        let view = state.view(&actions);
        let area = Rect::new(0, 0, 40, 8);
        let mut buffer = Buffer::empty(area);

        ModalRenderer::new().render(area, &mut buffer, &view);

        assert_eq!(buffer[(1, 2)].style().bg, Some(Color::Cyan));
    }

    #[test]
    fn long_row_text_is_truncated_with_marker() {
        let text = truncate_to_width("Open the selected jj log entry", 12);

        assert_eq!(text, "Open the ...");
    }
}

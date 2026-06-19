use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Widget};

use super::PaletteRenderer;
use super::parts::{render_footer, render_query, render_rows};
use crate::view::PaletteView;

/// Fullscreen command palette renderer.
///
/// The fullscreen renderer clears the whole supplied area and uses a header,
/// query row, result list, and footer. Applications can pass the entire frame
/// when command search should become the primary screen.
#[derive(Clone, Debug)]
pub struct FullscreenRenderer {
    title: String,
}

impl Default for FullscreenRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl FullscreenRenderer {
    /// Creates a fullscreen renderer with the default title.
    pub fn new() -> Self {
        Self {
            title: "Command Palette".into(),
        }
    }

    /// Sets the title rendered in the fullscreen header.
    ///
    /// ```
    /// use ratatui_command_palette::render::FullscreenRenderer;
    ///
    /// let renderer = FullscreenRenderer::new().title("All commands");
    ///
    /// assert!(format!("{renderer:?}").contains("All commands"));
    /// ```
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl PaletteRenderer for FullscreenRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
        Clear.render(area, buf);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let title = Line::from(vec![
            Span::styled(
                self.title.as_str(),
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("fullscreen", Style::new().fg(Color::DarkGray)),
        ]);
        Paragraph::new(title).render(sections[0], buf);
        render_query(sections[1], buf, view);
        render_rows(sections[2], buf, view);
        render_footer(sections[3], buf, view);
    }
}

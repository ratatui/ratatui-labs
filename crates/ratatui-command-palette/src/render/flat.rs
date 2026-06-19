use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph, Widget};

use super::PaletteRenderer;
use super::parts::{render_footer, render_query, render_rows};
use crate::view::PaletteView;

/// Borderless overlay renderer.
///
/// This renderer clears its area but does not draw a surrounding border. It is
/// useful for command surfaces that should feel like a flat sheet over existing
/// content rather than a dialog. Use it through the
/// [`PaletteRenderer`](super::PaletteRenderer) trait with a
/// [`PaletteView`](crate::view::PaletteView) produced by
/// [`PaletteState::view`](crate::state::PaletteState::view).
#[derive(Clone, Debug)]
pub struct FlatOverlayRenderer {
    title: String,
}

impl Default for FlatOverlayRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatOverlayRenderer {
    /// Creates a [`FlatOverlayRenderer`] with the default title.
    pub fn new() -> Self {
        Self {
            title: "Actions".into(),
        }
    }

    /// Sets the heading rendered above the query row.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_command_palette::render::FlatOverlayRenderer;
    ///
    /// let renderer = FlatOverlayRenderer::new().title("Commands");
    ///
    /// assert!(format!("{renderer:?}").contains("Commands"));
    /// ```
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl PaletteRenderer for FlatOverlayRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
        Clear.render(area, buf);

        if area.height == 0 || area.width == 0 {
            return;
        }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        Paragraph::new(Line::from(self.title.as_str()))
            .style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .render(sections[0], buf);
        render_query(sections[1], buf, view);
        render_rows(sections[2], buf, view);
        render_footer(sections[3], buf, view);
    }
}

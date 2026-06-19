use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Widget};

use super::PaletteRenderer;
use super::parts::{render_query, render_rows};
use crate::view::PaletteView;

/// Inline dropdown command palette renderer.
///
/// This renderer draws a compact bordered dropdown inside the supplied area and
/// does not clear content outside that area. It is intended for embedding below
/// another input or toolbar. See the [`render`](super) module for the renderer
/// contract shared by all built-in renderers.
#[derive(Clone, Debug)]
pub struct InlineDropdownRenderer {
    title: String,
}

impl Default for InlineDropdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineDropdownRenderer {
    /// Creates an [`InlineDropdownRenderer`] with the default title.
    pub fn new() -> Self {
        Self {
            title: "Commands".into(),
        }
    }

    /// Sets the dropdown border title.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_command_palette::render::InlineDropdownRenderer;
    ///
    /// let renderer = InlineDropdownRenderer::new().title("Palette");
    ///
    /// assert!(format!("{renderer:?}").contains("Palette"));
    /// ```
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl PaletteRenderer for InlineDropdownRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
        let block = Block::new()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::DarkGray));
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(inner);

        render_query(sections[0], buf, view);
        render_rows(sections[1], buf, view);
    }
}

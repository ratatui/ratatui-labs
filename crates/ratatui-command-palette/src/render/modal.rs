use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::PaletteRenderer;
use super::parts::{render_footer, render_query, render_rows};
use crate::view::PaletteView;

/// Modal command palette renderer.
///
/// The modal renderer clears its area, draws a rounded border, then renders the
/// prompt, rows, and footer. It is useful when the palette should interrupt the
/// current surface. See the [`render`](super) module for the renderer contract
/// shared by all built-in renderers.
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
    /// Creates a [`ModalRenderer`] with the default title.
    pub fn new() -> Self {
        Self {
            title: "Command Palette".into(),
        }
    }

    /// Sets the modal title.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_command_palette::render::ModalRenderer;
    ///
    /// let renderer = ModalRenderer::new().title("Actions");
    ///
    /// assert_eq!(
    ///     format!("{renderer:?}"),
    ///     "ModalRenderer { title: \"Actions\" }"
    /// );
    /// ```
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

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::PaletteRenderer;
use super::parts::{render_footer, render_query, render_rows, render_selected_row_details};
use crate::view::PaletteView;

/// Split command palette renderer with a preview pane.
///
/// The split renderer renders search results on the left and selected-row
/// details on the right. It uses only [`PaletteView`] data, so applications can
/// replace this preview with richer domain-specific rendering later. See the
/// [`render`](super) module for the renderer contract shared by all built-in
/// renderers.
#[derive(Clone, Debug)]
pub struct SplitPreviewRenderer {
    title: String,
    preview_title: String,
}

impl Default for SplitPreviewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SplitPreviewRenderer {
    /// Creates a split preview renderer with default titles.
    pub fn new() -> Self {
        Self {
            title: "Command Palette".into(),
            preview_title: "Preview".into(),
        }
    }

    /// Sets the palette pane title.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_command_palette::render::SplitPreviewRenderer;
    ///
    /// let renderer = SplitPreviewRenderer::new().title("Commands");
    ///
    /// assert!(format!("{renderer:?}").contains("Commands"));
    /// ```
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the preview pane title.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_command_palette::render::SplitPreviewRenderer;
    ///
    /// let renderer = SplitPreviewRenderer::new().preview_title("Details");
    ///
    /// assert!(format!("{renderer:?}").contains("Details"));
    /// ```
    pub fn preview_title(mut self, title: impl Into<String>) -> Self {
        self.preview_title = title.into();
        self
    }
}

impl PaletteRenderer for SplitPreviewRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
        Clear.render(area, buf);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);

        render_palette_panel(columns[0], buf, view, &self.title);
        render_preview_panel(columns[1], buf, view, &self.preview_title);
    }
}

fn render_palette_panel(area: Rect, buf: &mut Buffer, view: &PaletteView, title: &str) {
    let block = Block::new()
        .title(title)
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

fn render_preview_panel(area: Rect, buf: &mut Buffer, view: &PaletteView, title: &str) {
    let block = Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    render_selected_row_details(inner, buf, view);
}

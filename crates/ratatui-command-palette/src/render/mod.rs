#![warn(missing_docs)]

//! Ratatui renderers for command palette views.
//!
//! Renderers draw a [`PaletteView`] prepared by
//! [`PaletteState::view`](crate::state::PaletteState::view). They do not filter
//! actions, move selection, collect input, or dispatch invocations.
//!
//! Renderer choice:
//!
//! - [`ModalRenderer`] draws an interrupting bordered dialog.
//! - [`FlatOverlayRenderer`] draws a borderless sheet over existing content.
//! - [`SplitPreviewRenderer`] draws results and selected-row details side by side.
//! - [`FullscreenRenderer`] uses the whole supplied area as the command surface.
//! - [`InlineDropdownRenderer`] draws a compact embedded dropdown.
//! - [`PaletteRenderer`] is the trait for custom renderers.
//!
//! # Examples
//!
//! ```
//! use ratatui::buffer::Buffer;
//! use ratatui::layout::Rect;
//! use ratatui_action::spec::ActionSpec;
//! use ratatui_command_palette::render::{ModalRenderer, PaletteRenderer};
//! use ratatui_command_palette::state::PaletteState;
//!
//! let actions = vec![ActionSpec::new("document.open", "Open document")];
//! let mut palette = PaletteState::new();
//! palette.open(&actions);
//!
//! let view = palette.view(&actions);
//! let area = Rect::new(0, 0, 40, 8);
//! let mut buffer = Buffer::empty(area);
//!
//! ModalRenderer::new().render(area, &mut buffer, &view);
//! ```

mod flat;
mod fullscreen;
mod inline;
mod modal;
mod parts;
mod split;

pub use flat::FlatOverlayRenderer;
pub use fullscreen::FullscreenRenderer;
pub use inline::InlineDropdownRenderer;
pub use modal::ModalRenderer;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
pub use split::SplitPreviewRenderer;

use crate::view::PaletteView;

/// Renders a command palette view into a Ratatui buffer.
///
/// Implement this trait to provide an alternate presentation for the same
/// [`PaletteView`] model. Renderers should be pure drawing code: input handling
/// and event dispatch stay with [`PaletteState`](crate::state::PaletteState)
/// and the application.
///
/// Use [`PaletteRenderer::render`] when an application has already produced a
/// [`PaletteView`] and only needs to draw it into a Ratatui buffer. Use a custom
/// implementation when the built-in renderer layout is not a good fit.
///
/// # Examples
///
/// ```
/// use ratatui::buffer::Buffer;
/// use ratatui::layout::Rect;
/// use ratatui::text::Line;
/// use ratatui::widgets::{Paragraph, Widget};
/// use ratatui_action::spec::ActionSpec;
/// use ratatui_command_palette::render::PaletteRenderer;
/// use ratatui_command_palette::state::PaletteState;
/// use ratatui_command_palette::view::PaletteView;
///
/// struct CountRenderer;
///
/// impl PaletteRenderer for CountRenderer {
///     fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView) {
///         let line = Line::from(format!("{} rows", view.rows().len()));
///         Paragraph::new(line).render(area, buf);
///     }
/// }
///
/// let actions = vec![ActionSpec::new("document.open", "Open document")];
/// let mut palette = PaletteState::new();
/// palette.open(&actions);
///
/// let area = Rect::new(0, 0, 12, 1);
/// let mut buffer = Buffer::empty(area);
/// CountRenderer.render(area, &mut buffer, &palette.view(&actions));
///
/// assert_eq!(buffer[(0, 0)].symbol(), "1");
/// ```
pub trait PaletteRenderer {
    /// Draws `view` into `buf` within `area`.
    fn render(&self, area: Rect, buf: &mut Buffer, view: &PaletteView);
}

#[cfg(test)]
mod tests {
    use ratatui::buffer::{Buffer, Cell};
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use ratatui_action::spec::ActionSpec;

    use super::*;
    use crate::event::MoveSelection;
    use crate::state::PaletteState;

    fn view_for(actions: &[ActionSpec]) -> crate::view::PaletteView {
        let mut state = PaletteState::new();
        state.open(actions);
        state.view(actions)
    }

    #[test]
    fn modal_selected_row_gets_highlight_background() {
        let actions = vec![
            ActionSpec::new("app.quit", "Quit"),
            ActionSpec::new("theme.switch", "Switch Theme"),
        ];
        let view = view_for(&actions);
        let area = Rect::new(0, 0, 40, 8);
        let mut buffer = Buffer::empty(area);

        ModalRenderer::new().render(area, &mut buffer, &view);

        assert_eq!(buffer[(1, 2)].style().bg, Some(Color::Cyan));
    }

    #[test]
    fn long_row_text_is_truncated_with_marker() {
        let text = parts::truncate_to_width("Open the selected jj log entry", 12);

        assert_eq!(text, "Open the ...");
    }

    #[test]
    fn selected_row_is_scrolled_into_view() {
        let actions = (0..8)
            .map(|index| ActionSpec::new(format!("action.{index}"), format!("Action {index}")))
            .collect::<Vec<_>>();
        let mut state = PaletteState::new();
        state.open(&actions);
        state.move_selection(MoveSelection::Last, &actions);
        let view = state.view(&actions);
        let area = Rect::new(0, 0, 24, 9);
        let mut buffer = Buffer::empty(area);

        ModalRenderer::new().render(area, &mut buffer, &view);

        assert_eq!(buffer[(1, 6)].style().bg, Some(Color::Cyan));
        assert!(buffer.content().iter().any(|cell| cell.symbol() == "7"));
    }

    #[test]
    fn flat_overlay_does_not_draw_modal_border() {
        let actions = vec![ActionSpec::new("document.open", "Open document")];
        let view = view_for(&actions);
        let area = Rect::new(0, 0, 40, 8);
        let mut buffer = Buffer::empty(area);

        FlatOverlayRenderer::new().render(area, &mut buffer, &view);

        assert_eq!(buffer[(0, 0)].symbol(), "A");
        assert_eq!(buffer[(0, 2)].style().bg, Some(Color::Cyan));
    }

    #[test]
    fn split_preview_renders_selected_row_details() {
        let actions = vec![
            ActionSpec::new("document.open", "Open document")
                .with_description("Open the selected document")
                .with_category("Navigation"),
        ];
        let view = view_for(&actions);
        let area = Rect::new(0, 0, 80, 10);
        let mut buffer = Buffer::empty(area);

        SplitPreviewRenderer::new().render(area, &mut buffer, &view);

        assert!(buffer.content().iter().any(|cell| cell.symbol() == "N"));
    }

    #[test]
    fn selected_row_details_wrap_long_text() {
        let actions = vec![
            ActionSpec::new("row.expand", "Expand row")
                .with_description("alpha beta gamma")
                .with_category("Rows"),
        ];
        let view = view_for(&actions);
        let area = Rect::new(0, 0, 12, 8);
        let mut buffer = Buffer::empty(area);

        parts::render_selected_row_details(area, &mut buffer, &view);

        let rendered = buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("gamma"));
    }

    #[test]
    fn inline_dropdown_keeps_surrounding_buffer_contents() {
        let actions = vec![ActionSpec::new("document.open", "Open document")];
        let view = view_for(&actions);
        let area = Rect::new(2, 1, 40, 5);
        let mut buffer = Buffer::filled(Rect::new(0, 0, 50, 8), Cell::new("x"));

        InlineDropdownRenderer::new().render(area, &mut buffer, &view);

        assert_eq!(buffer[(0, 0)].symbol(), "x");
        assert_eq!(buffer[(3, 3)].style().bg, Some(Color::Cyan));
    }
}

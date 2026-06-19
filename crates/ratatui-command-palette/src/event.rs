//! Palette events and interaction mode.
//!
//! These types are emitted by [`PaletteState`](crate::state::PaletteState) and
//! interpreted by the application:
//!
//! - [`PaletteMode`] tells renderers and input handlers whether rows represent actions or inputs.
//! - [`PaletteEvent`] is the application-facing result of lifecycle, preview, and accept actions.
//! - [`MoveSelection`] describes relative and absolute selection movement.

use ratatui_action::id::ActionId;
use ratatui_action::invocation::ActionInvocation;

/// Interaction mode for the command palette.
///
/// The mode tells renderers whether rows are actions or input choices, and
/// tells input handlers how to interpret selection and query edits. Read the
/// mode from [`PaletteState::mode`](crate::state::PaletteState::mode) or from a
/// rendered [`PaletteView`](crate::view::PaletteView).
///
/// # Examples
///
/// ```
/// use ratatui_action::id::ActionId;
/// use ratatui_command_palette::event::PaletteMode;
///
/// let mode = PaletteMode::CollectingInput {
///     action: ActionId::new("theme.switch"),
///     input_index: 0,
/// };
///
/// assert!(matches!(
///     mode,
///     PaletteMode::CollectingInput { input_index: 0, .. }
/// ));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// The palette is filtering and selecting actions.
    Searching,
    /// The palette is collecting an input value for an action after
    /// [`PaletteState::accept`](crate::state::PaletteState::accept) selects an
    /// action with inputs.
    CollectingInput {
        /// Action whose input is being collected.
        action: ActionId,
        /// Zero-based index in [`ratatui_action::spec::ActionSpec::inputs`].
        input_index: usize,
    },
}

/// Event emitted by palette state transitions.
///
/// Applications should handle these events at their own boundary. The palette
/// never executes callbacks or mutates application state.
///
/// # Examples
///
/// ```
/// use ratatui_action::spec::ActionSpec;
/// use ratatui_command_palette::event::PaletteEvent;
/// use ratatui_command_palette::state::PaletteState;
///
/// let actions = vec![ActionSpec::new("document.open", "Open document")];
/// let mut palette = PaletteState::new();
/// palette.open(&actions);
///
/// match palette.accept(&actions) {
///     Some(PaletteEvent::Invoke(invocation)) => {
///         assert_eq!(invocation.id().as_str(), "document.open");
///     }
///     event => panic!("unexpected event: {event:?}"),
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteEvent {
    /// The selected action is ready to dispatch.
    ///
    /// Emitted by [`PaletteState::accept`](crate::state::PaletteState::accept)
    /// after an enabled action has all required inputs.
    Invoke(ActionInvocation),
    /// The currently selected choice changed and may be previewed.
    ///
    /// Emitted by [`PaletteState::move_selection`](crate::state::PaletteState::move_selection) or
    /// [`PaletteState::preview_event`](crate::state::PaletteState::preview_event) while collecting
    /// previewable input. `None` means any transient preview should be cleared.
    PreviewChanged(Option<ActionInvocation>),
    /// The palette was opened.
    ///
    /// Emitted by [`PaletteState::open`](crate::state::PaletteState::open).
    Opened,
    /// The palette was closed without implying cancellation.
    ///
    /// Emitted by [`PaletteState::close`](crate::state::PaletteState::close).
    Closed,
    /// The palette was cancelled and transient input state was cleared.
    ///
    /// Emitted by [`PaletteState::cancel`](crate::state::PaletteState::cancel)
    /// and [`PaletteState::cancel_events`](crate::state::PaletteState::cancel_events).
    Cancelled,
}

/// Direction or distance for selection movement.
///
/// Relative movement wraps at the first and last visible row.
/// [`MoveSelection::First`] and [`MoveSelection::Last`] jump to absolute
/// boundaries.
///
/// # Examples
///
/// ```
/// use ratatui_action::spec::ActionSpec;
/// use ratatui_command_palette::event::MoveSelection;
/// use ratatui_command_palette::state::PaletteState;
///
/// let actions = vec![
///     ActionSpec::new("first", "First"),
///     ActionSpec::new("second", "Second"),
/// ];
/// let mut palette = PaletteState::new();
/// palette.open(&actions);
///
/// palette.move_selection(MoveSelection::Previous, &actions);
///
/// assert_eq!(palette.selected(), Some(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveSelection {
    /// Move to the next row.
    Next,
    /// Move to the previous row.
    Previous,
    /// Move down by the given number of rows.
    PageDown(usize),
    /// Move up by the given number of rows.
    PageUp(usize),
    /// Move to the first row.
    First,
    /// Move to the last row.
    Last,
}

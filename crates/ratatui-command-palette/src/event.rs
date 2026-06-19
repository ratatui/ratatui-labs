//! Palette events and interaction mode.

use ratatui_action::id::ActionId;
use ratatui_action::invocation::ActionInvocation;

/// Interaction mode for the command palette.
///
/// The mode tells renderers whether rows are actions or input choices, and
/// tells input handlers how to interpret selection and query edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    /// The palette is filtering and selecting actions.
    Searching,
    /// The palette is collecting an input value for an action.
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteEvent {
    /// The selected action is ready to dispatch.
    Invoke(ActionInvocation),
    /// The currently selected choice changed and may be previewed.
    PreviewChanged(Option<ActionInvocation>),
    /// The palette was opened.
    Opened,
    /// The palette was closed without implying cancellation.
    Closed,
    /// The palette was cancelled and transient input state was cleared.
    Cancelled,
}

/// Direction or distance for selection movement.
///
/// Relative movement wraps at the first and last visible row.
/// [`MoveSelection::First`] and [`MoveSelection::Last`] jump to absolute
/// boundaries.
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

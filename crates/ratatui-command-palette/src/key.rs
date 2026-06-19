//! Key input commands for driving a command palette.
//!
//! This module keeps terminal-backend details at the edge of the API. Apps can
//! convert crossterm key events with [`PaletteKey::from_crossterm`] and then
//! handle palette concepts such as movement, acceptance, cancellation, and text
//! edits by calling methods on [`PaletteState`](crate::state::PaletteState).
//!
//! Use [`PaletteKey`] as the small command enum in your event loop. With the
//! `crossterm` feature enabled, use [`PaletteKey::from_crossterm`] as the adapter from backend
//! events to palette commands.

use crate::event::MoveSelection;

/// A normalized key command for palette interaction.
///
/// [`PaletteKey`] intentionally does not decide whether cancellation exits the
/// application, closes a palette, or only rolls back transient preview state.
/// Applications keep that policy at their boundary.
///
/// Variant map:
///
/// - [`Accept`](Self::Accept) calls [`PaletteState::accept`](crate::state::PaletteState::accept).
/// - [`Cancel`](Self::Cancel) calls [`PaletteState::cancel`](crate::state::PaletteState::cancel) or
///   [`PaletteState::cancel_events`](crate::state::PaletteState::cancel_events).
/// - [`Move`](Self::Move) calls
///   [`PaletteState::move_selection`](crate::state::PaletteState::move_selection).
/// - [`Insert`](Self::Insert) and [`Backspace`](Self::Backspace) edit the query or active text
///   input.
/// - [`Ignore`](Self::Ignore) leaves the palette unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteKey {
    /// Accept the selected row or collected input with
    /// [`PaletteState::accept`](crate::state::PaletteState::accept).
    Accept,
    /// Cancel the current palette interaction with
    /// [`PaletteState::cancel`](crate::state::PaletteState::cancel) or
    /// [`PaletteState::cancel_events`](crate::state::PaletteState::cancel_events).
    Cancel,
    /// Move selection within the current row set with
    /// [`PaletteState::move_selection`](crate::state::PaletteState::move_selection).
    Move(MoveSelection),
    /// Insert a character into the search query or active text input with
    /// [`PaletteState::push_query_char`](crate::state::PaletteState::push_query_char).
    Insert(char),
    /// Remove the previous query or text-input character with
    /// [`PaletteState::pop_query_char`](crate::state::PaletteState::pop_query_char).
    Backspace,
    /// Ignore this key event.
    Ignore,
}

#[cfg(feature = "crossterm")]
impl PaletteKey {
    /// Converts a crossterm key event into a palette key command.
    ///
    /// Non-press events are ignored so callers can pass crossterm events
    /// directly from an event loop without duplicating key-kind checks.
    ///
    /// ```
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    /// use ratatui_command_palette::key::PaletteKey;
    ///
    /// let event = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    ///
    /// assert_eq!(
    ///     PaletteKey::from_crossterm(event),
    ///     PaletteKey::Move(ratatui_command_palette::event::MoveSelection::Next)
    /// );
    /// ```
    pub fn from_crossterm(event: crossterm::event::KeyEvent) -> Self {
        use crossterm::event::{KeyCode, KeyEventKind};

        if event.kind != KeyEventKind::Press {
            return Self::Ignore;
        }

        match event.code {
            KeyCode::Esc => Self::Cancel,
            KeyCode::Enter => Self::Accept,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::Down => Self::Move(MoveSelection::Next),
            KeyCode::Up => Self::Move(MoveSelection::Previous),
            KeyCode::PageDown => Self::Move(MoveSelection::PageDown(5)),
            KeyCode::PageUp => Self::Move(MoveSelection::PageUp(5)),
            KeyCode::Home => Self::Move(MoveSelection::First),
            KeyCode::End => Self::Move(MoveSelection::Last),
            KeyCode::Char(character) => Self::Insert(character),
            _ => Self::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "crossterm")]
    #[test]
    fn converts_crossterm_key_presses() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        let event =
            KeyEvent::new_with_kind(KeyCode::Char('q'), KeyModifiers::NONE, KeyEventKind::Press);

        assert_eq!(PaletteKey::from_crossterm(event), PaletteKey::Insert('q'));
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn ignores_crossterm_key_releases() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        let event =
            KeyEvent::new_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release);

        assert_eq!(PaletteKey::from_crossterm(event), PaletteKey::Ignore);
    }
}

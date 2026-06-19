//! Presentation-only shortcut labels for palette rows.
//!
//! Shortcuts are intentionally separate from [`ActionSpec`](ratatui_action::spec::ActionSpec).
//! Applications often have context-specific keymaps, and the same action can
//! have different bindings in different screens. Keep those bindings in the
//! application or keymap layer, then pass display labels into the palette view.
//!
//! Use [`ShortcutLabels::insert`] to attach presentation-only labels and
//! [`PaletteState::view_with_shortcuts`](crate::state::PaletteState::view_with_shortcuts) to put
//! them in a renderable view.

use std::collections::BTreeMap;

use ratatui_action::id::ActionId;

/// Display labels for action shortcuts.
///
/// [`ShortcutLabels`] is a small presentation map. It does not parse key events,
/// own keybinding precedence, or decide whether a keybinding is active.
///
/// Use [`new`](Self::new) to create a map, [`insert`](Self::insert) to add or replace labels,
/// [`get`](Self::get) to read them, and [`is_empty`](Self::is_empty) for optional display logic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShortcutLabels {
    labels: BTreeMap<ActionId, String>,
}

impl ShortcutLabels {
    /// Creates an empty shortcut-label map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a display label for an action.
    ///
    /// ```
    /// use ratatui_command_palette::shortcut::ShortcutLabels;
    ///
    /// let mut shortcuts = ShortcutLabels::new();
    /// shortcuts.insert("document.open", "Ctrl-O");
    ///
    /// assert_eq!(shortcuts.get(&"document.open".into()), Some("Ctrl-O"));
    /// ```
    pub fn insert(
        &mut self,
        action: impl Into<ActionId>,
        label: impl Into<String>,
    ) -> Option<String> {
        self.labels.insert(action.into(), label.into())
    }

    /// Returns a shortcut display label for an action.
    pub fn get(&self, action: &ActionId) -> Option<&str> {
        self.labels.get(action).map(String::as_str)
    }

    /// Returns true when no shortcut labels have been registered.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

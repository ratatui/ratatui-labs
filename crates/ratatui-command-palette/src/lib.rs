#![warn(missing_docs)]

//! Command palette state, events, view data, and renderer integration.
//!
//! The palette owns interaction state such as the current query and selected
//! row. Applications own dispatch and side effects: accepting a row returns an
//! event that the application can handle.
//!
//! ```
//! use ratatui_action::spec::ActionSpec;
//! use ratatui_command_palette::event::PaletteEvent;
//! use ratatui_command_palette::state::PaletteState;
//!
//! let actions = vec![ActionSpec::new("document.open", "Open document")];
//! let mut palette = PaletteState::new();
//! palette.open(&actions);
//!
//! if let Some(PaletteEvent::Invoke(invocation)) = palette.accept(&actions) {
//!     assert_eq!(invocation.id().as_str(), "document.open");
//!     // The application decides how to handle the invocation.
//! }
//! ```
//!
//! # Choice Input And Preview Events
//!
//! Actions with a choice input move the palette into
//! [`PaletteMode::CollectingInput`](event::PaletteMode::CollectingInput).
//! Moving the selected choice emits
//! [`PaletteEvent::PreviewChanged`](event::PaletteEvent::PreviewChanged), and
//! accepting the choice emits [`PaletteEvent::Invoke`](event::PaletteEvent::Invoke)
//! with resolved arguments.
//!
//! ```
//! use ratatui_action::id::InputId;
//! use ratatui_action::input::{ActionChoice, ActionInput};
//! use ratatui_action::spec::ActionSpec;
//! use ratatui_command_palette::event::{MoveSelection, PaletteEvent};
//! use ratatui_command_palette::state::PaletteState;
//!
//! let actions =
//!     vec![
//!         ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
//!             id: InputId::new("theme"),
//!             label: "Theme".into(),
//!             choices: vec![
//!                 ActionChoice::new("catppuccin", "Catppuccin"),
//!                 ActionChoice::new("github-dark", "GitHub Dark"),
//!             ],
//!         }),
//!     ];
//!
//! let mut palette = PaletteState::new();
//! palette.open(&actions);
//! palette.accept(&actions);
//!
//! let Some(PaletteEvent::PreviewChanged(Some(preview))) =
//!     palette.move_selection(MoveSelection::Next, &actions)
//! else {
//!     panic!("expected preview event");
//! };
//!
//! assert_eq!(
//!     preview.args().get(&InputId::new("theme")),
//!     Some("github-dark")
//! );
//! ```
//!
//! # Rendering
//!
//! Renderers consume [`view::PaletteView`] values produced by
//! [`state::PaletteState::view`]. The built-in renderers include
//! [`render::ModalRenderer`], [`render::FlatOverlayRenderer`],
//! [`render::SplitPreviewRenderer`], [`render::FullscreenRenderer`], and
//! [`render::InlineDropdownRenderer`].

pub mod event;
pub mod key;
pub mod matching;
pub mod render;
mod selection;
pub mod shortcut;
pub mod state;
pub mod view;

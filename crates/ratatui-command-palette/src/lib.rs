#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

//! Command palette state, view data, events, and renderers for Ratatui apps.
//!
//! `ratatui-command-palette` is the UI-state half of the command palette
//! experiment in `ratatui-labs`. It consumes semantic action descriptions from
//! [`ratatui_action`], tracks palette interaction state, prepares renderable
//! view data, and emits events for the application to handle.
//!
//! The palette owns interaction state such as the current query, selected row,
//! argument collection mode, and collected argument values. Applications own the
//! action list, keybinding policy, dispatch, side effects, and any preview
//! rollback. Accepting a row returns a [`PaletteEvent`](event::PaletteEvent)
//! rather than executing application code directly.
//!
//! The API is experimental. Callers can rely on the documented behavior while
//! evaluating the crate, but names, module boundaries, renderer options, and
//! storage choices may change before any release with compatibility
//! commitments.
//!
//! # Crate Model
//!
//! The crate separates palette work into four boundaries:
//!
//! - [`state::PaletteState`] owns search text, selection, input collection, and event emission.
//! - [`view::PaletteView`] and [`view::PaletteRow`] are snapshots prepared for rendering.
//! - [`render`] contains built-in renderers and the [`render::PaletteRenderer`] trait for alternate
//!   layouts.
//! - [`key::PaletteKey`] is a small normalized input command type, including a crossterm adapter
//!   for applications that use crossterm events.
//!
//! Matching, selection, input collection, preview events, and rendering are
//! intentionally separate so applications can keep one action model while
//! changing presentation.
//!
//! # Lifecycle
//!
//! A typical application:
//!
//! 1. Builds or refreshes a list of [`ratatui_action::spec::ActionSpec`] values.
//! 1. Calls [`PaletteState::open`](state::PaletteState::open).
//! 1. Sends key-derived edits, movement, cancellation, and accept actions to the state machine.
//! 1. Renders [`PaletteState::view`](state::PaletteState::view) with a [`render::PaletteRenderer`].
//! 1. Handles emitted [`PaletteEvent`](event::PaletteEvent) values at the application's dispatch
//!    boundary.
//!
//! # Basic Invocation
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
//! Text inputs use the query line as the input field and invoke with the typed
//! value when accepted. Choice and boolean inputs emit preview events as
//! selection changes. When an application applies transient preview state, use
//! [`PaletteState::cancel_events`](state::PaletteState::cancel_events) to get an
//! explicit rollback event before closing the palette.
//!
//! # Keyboard Input
//!
//! The palette does not decide which application key opens or closes it. It only
//! provides [`key::PaletteKey`] as a small command type for operations the state
//! machine understands. Applications can convert crossterm key events with
//! [`PaletteKey::from_crossterm`](key::PaletteKey::from_crossterm) and keep
//! policy decisions, such as whether `Esc` cancels the palette or exits the
//! application, outside the crate.
//!
//! # Rendering
//!
//! Renderers consume [`view::PaletteView`] values produced by
//! [`state::PaletteState::view`]. The built-in renderers include
//! [`render::ModalRenderer`], [`render::FlatOverlayRenderer`],
//! [`render::SplitPreviewRenderer`], [`render::FullscreenRenderer`], and
//! [`render::InlineDropdownRenderer`].
//!
//! Renderers are pure drawing code. They should not filter actions, move
//! selection, collect input, dispatch invocations, or mutate application state.
//! See the `command-palette` and `renderer-gallery` examples for complete
//! terminal integrations and the repository Betamax tapes for rendered
//! validation.

pub mod event;
pub mod key;
pub mod matching;
pub mod render;
mod selection;
pub mod shortcut;
pub mod state;
pub mod view;

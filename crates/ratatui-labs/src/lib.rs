#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]

/// Semantic action model.
pub use ratatui_action as action;
/// Command palette state, view data, and rendering.
pub use ratatui_command_palette as command_palette;
/// Derive macros and code generation helpers.
pub use ratatui_derive as derive;
/// Frame-local UI coordination primitives.
pub use ratatui_layout as layout;
/// Owned styled-text wrapping.
pub use ratatui_textwrap as textwrap;

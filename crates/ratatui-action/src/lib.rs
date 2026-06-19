#![warn(missing_docs)]

//! Semantic actions and invocation requests for Ratatui applications.
//!
//! This crate describes application capabilities without binding them to a
//! specific UI surface. A command palette, keybinding table, menu, help screen,
//! or automation layer can all read the same [`spec::ActionSpec`] values and
//! return [`invocation::ActionInvocation`] requests for the application to
//! dispatch.
//!
//! The action model deliberately does not store callbacks. Applications keep
//! ownership of state mutation, side effects, permissions, and error handling.
//!
//! # Core Flow
//!
//! A typical integration builds a list of [`spec::ActionSpec`] values, passes
//! them to a UI surface, and dispatches the returned
//! [`invocation::ActionInvocation`].
//!
//! ```
//! use ratatui_action::invocation::{ActionInvocation, InvocationSource};
//! use ratatui_action::spec::ActionSpec;
//!
//! let action = ActionSpec::new("document.open", "Open document");
//! let invocation = ActionInvocation::new(action.id().clone(), InvocationSource::Palette);
//!
//! match invocation.id().as_str() {
//!     "document.open" => {
//!         // Open the document in application-owned state.
//!     }
//!     _ => {}
//! }
//! ```
//!
//! # Inputs And Arguments
//!
//! Actions can declare simple inputs. The UI surface collects values for those
//! inputs and returns them as [`invocation::ActionArgs`].
//!
//! ```
//! use ratatui_action::id::InputId;
//! use ratatui_action::input::{ActionChoice, ActionInput};
//! use ratatui_action::invocation::{ActionArgs, ActionInvocation, InvocationSource};
//! use ratatui_action::spec::ActionSpec;
//!
//! let action = ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
//!     id: InputId::new("theme"),
//!     label: "Theme".into(),
//!     choices: vec![ActionChoice::new("github-dark", "GitHub Dark")],
//! });
//!
//! let mut args = ActionArgs::new();
//! args.insert("theme", "github-dark");
//! let invocation =
//!     ActionInvocation::with_args(action.id().clone(), args, InvocationSource::Palette);
//!
//! assert_eq!(
//!     invocation.args().get(&InputId::new("theme")),
//!     Some("github-dark")
//! );
//! ```
//!
//! # Modules
//!
//! - [`id`] owns stable action and input identifiers.
//! - [`spec`] owns action metadata and availability.
//! - [`input`] owns input declarations and choice values.
//! - [`invocation`] owns resolved arguments and invocation requests.
//!
//! # Related Crates
//!
//! [`ratatui-command-palette`](https://docs.rs/ratatui-command-palette) consumes
//! these action specs and provides command palette state and rendering.

pub mod id;
pub mod input;
pub mod invocation;
pub mod spec;

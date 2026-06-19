#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

//! Semantic action descriptions for Ratatui applications.
//!
//! `ratatui-action` is the action-model half of the command palette experiment
//! in `ratatui-labs`. It gives an application a way to name the capabilities it
//! already has, attach user-facing metadata, describe simple required inputs,
//! and receive resolved invocation requests from UI surfaces.
//!
//! The crate is intentionally surface-agnostic. A command palette, keybinding
//! table, menu, help screen, toolbar, context menu, or automation layer can all
//! read the same [`ActionSpec`](spec::ActionSpec) values and return
//! [`ActionInvocation`](invocation::ActionInvocation) requests for the
//! application to dispatch.
//!
//! The action model deliberately does not store callbacks. Applications keep
//! ownership of state mutation, side effects, permissions, and error handling.
//! This keeps the API usable by applications with very different state models
//! and avoids turning the experiment into an application framework.
//!
//! The API is experimental. Callers can rely on the behavior documented here
//! while experimenting, but names, module boundaries, and storage choices may
//! change before any release with compatibility commitments.
//!
//! # Crate Model
//!
//! The primary types are:
//!
//! - [`ActionId`](id::ActionId) and [`InputId`](id::InputId) for stable identifiers.
//! - [`ActionSpec`](spec::ActionSpec) and [`Availability`](spec::Availability) for describing
//!   actions and whether they can be invoked.
//! - [`ActionInput`](input::ActionInput) and [`ActionChoice`](input::ActionChoice) for declaring
//!   values a UI surface must collect.
//! - [`ActionArgs`](invocation::ActionArgs), [`ActionInvocation`](invocation::ActionInvocation),
//!   and [`InvocationSource`](invocation::InvocationSource) for returning resolved work to the
//!   application.
//!
//! A UI surface should treat action specs as input data and invocations as
//! output data. The application remains the only place that knows how to perform
//! an action.
//!
//! Module map:
//!
//! - [`id`] defines stable [`ActionId`](id::ActionId) and [`InputId`](id::InputId) keys.
//! - [`spec`] defines [`ActionSpec`](spec::ActionSpec) and [`Availability`](spec::Availability).
//! - [`input`] defines [`ActionInput`](input::ActionInput) and
//!   [`ActionChoice`](input::ActionChoice).
//! - [`invocation`] defines [`ActionArgs`](invocation::ActionArgs),
//!   [`ActionInvocation`](invocation::ActionInvocation), and
//!   [`InvocationSource`](invocation::InvocationSource).
//!
//! # Basic Flow
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
//! # Compatibility Boundary
//!
//! Public structs use constructors, accessors, and builder-style modifiers
//! instead of public fields. That keeps the experimental API flexible enough to
//! add validation, change storage, or introduce borrowed view data before
//! publication. Prefer matching on stable identifiers at the application
//! dispatch boundary rather than depending on current struct layout.
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
//! `ratatui-command-palette` consumes these action specs and provides command
//! palette state, view data, and rendering. The `ratatui-labs` crate re-exports
//! this crate under `ratatui_labs::action` as a convenience namespace while the
//! experiment is being evaluated.

pub mod id;
pub mod input;
pub mod invocation;
pub mod spec;

//! Resolved arguments and invocation requests.

use std::collections::BTreeMap;

use crate::id::{ActionId, InputId};

/// Resolved arguments for an action invocation.
///
/// Arguments are keyed by [`InputId`] so dispatch code can read values without
/// knowing which UI surface collected them.
///
/// ```
/// use ratatui_action::id::InputId;
/// use ratatui_action::invocation::ActionArgs;
///
/// let mut args = ActionArgs::new();
/// args.insert("theme", "github-dark");
///
/// assert_eq!(args.get(&InputId::new("theme")), Some("github-dark"));
/// assert!(!args.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActionArgs {
    values: BTreeMap<InputId, String>,
}

impl ActionArgs {
    /// Creates an empty argument set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces an argument value.
    pub fn insert(&mut self, id: impl Into<InputId>, value: impl Into<String>) -> Option<String> {
        self.values.insert(id.into(), value.into())
    }

    /// Returns an argument value by input id.
    pub fn get(&self, id: &InputId) -> Option<&str> {
        self.values.get(id).map(String::as_str)
    }

    /// Iterates over resolved argument values in input-id order.
    pub fn iter(&self) -> impl Iterator<Item = (&InputId, &str)> {
        self.values
            .iter()
            .map(|(input, value)| (input, value.as_str()))
    }

    /// Returns true when no argument values have been resolved.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A request for the application to run an action.
///
/// Invocation is the handoff point between a UI surface and application-owned
/// dispatch. The action crate does not execute callbacks or mutate application
/// state.
///
/// ```
/// use ratatui_action::id::InputId;
/// use ratatui_action::invocation::{ActionArgs, ActionInvocation, InvocationSource};
///
/// let mut args = ActionArgs::new();
/// args.insert(InputId::new("theme"), "github-dark");
///
/// let invocation = ActionInvocation::with_args("theme.switch", args, InvocationSource::Palette);
///
/// assert_eq!(invocation.id().as_str(), "theme.switch");
/// assert_eq!(invocation.source(), InvocationSource::Palette);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionInvocation {
    id: ActionId,
    args: ActionArgs,
    source: InvocationSource,
}

impl ActionInvocation {
    /// Creates an invocation with no arguments.
    pub fn new(id: impl Into<ActionId>, source: InvocationSource) -> Self {
        Self {
            id: id.into(),
            args: ActionArgs::new(),
            source,
        }
    }

    /// Creates an invocation with resolved arguments.
    pub fn with_args(id: impl Into<ActionId>, args: ActionArgs, source: InvocationSource) -> Self {
        Self {
            id: id.into(),
            args,
            source,
        }
    }

    /// Returns the action identifier to invoke.
    pub fn id(&self) -> &ActionId {
        &self.id
    }

    /// Returns resolved invocation arguments.
    pub fn args(&self) -> &ActionArgs {
        &self.args
    }

    /// Returns the surface that requested this invocation.
    pub fn source(&self) -> InvocationSource {
        self.source
    }
}

/// UI or integration surface that requested an action invocation.
///
/// The source lets dispatch code distinguish user intent from a palette,
/// keybinding, menu, mouse action, or automation path when that distinction
/// affects behavior or telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationSource {
    /// Invocation came from a command palette.
    Palette,
    /// Invocation came from a keybinding.
    KeyBinding,
    /// Invocation came from a menu item.
    Menu,
    /// Invocation came from a mouse interaction.
    Mouse,
    /// Invocation came from automation or a non-interactive integration.
    Automation,
}

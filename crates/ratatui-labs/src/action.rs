//! Semantic actions and invocation requests.
//!
//! Actions describe what an application can do without binding that capability
//! to a particular widget, keybinding, menu, or dispatcher. UI surfaces such as
//! a command palette can present these specs and return invocation requests for
//! the application to handle.

use std::collections::BTreeMap;

/// Stable identifier for an application action.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(String);

impl ActionId {
    /// Creates an action identifier from application-owned text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ActionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ActionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable identifier for an action input.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputId(String);

impl InputId {
    /// Creates an input identifier from application-owned text.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for InputId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for InputId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Metadata describing a semantic application action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSpec {
    pub id: ActionId,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub keywords: Vec<String>,
    pub inputs: Vec<ActionInput>,
    pub availability: Availability,
}

impl ActionSpec {
    /// Creates an enabled action with no description, category, keywords, or
    /// inputs.
    pub fn new(id: impl Into<ActionId>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            category: None,
            keywords: Vec::new(),
            inputs: Vec::new(),
            availability: Availability::Enabled,
        }
    }

    /// Returns true when this action should be omitted from ordinary palette
    /// results.
    pub fn is_hidden(&self) -> bool {
        matches!(self.availability, Availability::Hidden)
    }

    /// Returns true when this action can be invoked immediately.
    pub fn is_enabled(&self) -> bool {
        matches!(self.availability, Availability::Enabled)
    }
}

/// Whether an action can be presented or invoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    Enabled,
    Disabled { reason: String },
    Hidden,
}

/// Input required before an action can be invoked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionInput {
    Text {
        id: InputId,
        label: String,
        placeholder: Option<String>,
    },
    Choice {
        id: InputId,
        label: String,
        choices: Vec<ActionChoice>,
    },
    Bool {
        id: InputId,
        label: String,
    },
}

/// A selectable value for a choice input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionChoice {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl ActionChoice {
    /// Creates a choice with no description.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
        }
    }
}

/// Resolved arguments for an action invocation.
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

    /// Returns true when no argument values have been resolved.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A request for the application to run an action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionInvocation {
    pub id: ActionId,
    pub args: ActionArgs,
    pub source: InvocationSource,
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
}

/// UI or integration surface that requested an action invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvocationSource {
    Palette,
    KeyBinding,
    Menu,
    Mouse,
    Automation,
}

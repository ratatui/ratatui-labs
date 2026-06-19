//! Input declarations and choice values for action invocation.

use crate::id::InputId;

/// Input required before an action can be invoked.
///
/// Inputs describe values the application needs, not how a particular UI should
/// collect them. A command palette might render choices as rows while another
/// surface might render the same input as a menu.
///
/// ```
/// use ratatui_action::id::InputId;
/// use ratatui_action::input::{ActionChoice, ActionInput};
///
/// let input = ActionInput::Choice {
///     id: InputId::new("theme"),
///     label: "Theme".into(),
///     choices: vec![ActionChoice::new("catppuccin", "Catppuccin")],
/// };
///
/// assert_eq!(input.id().as_str(), "theme");
/// assert_eq!(input.label(), "Theme");
/// assert_eq!(input.choices().unwrap()[0].value(), "catppuccin");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionInput {
    /// Free-form text input.
    Text {
        /// Stable key used when the value is inserted into
        /// [`ActionArgs`](crate::invocation::ActionArgs).
        id: InputId,
        /// User-facing label for the input.
        label: String,
        /// Optional placeholder text for empty text input controls.
        placeholder: Option<String>,
    },
    /// Selection from a finite set of choices.
    Choice {
        /// Stable key used when the selected value is inserted into
        /// [`ActionArgs`](crate::invocation::ActionArgs).
        id: InputId,
        /// User-facing label for the input.
        label: String,
        /// Available choices for this input.
        choices: Vec<ActionChoice>,
    },
    /// Boolean input.
    Bool {
        /// Stable key used when the value is inserted into
        /// [`ActionArgs`](crate::invocation::ActionArgs).
        id: InputId,
        /// User-facing label for the input.
        label: String,
    },
}

impl ActionInput {
    /// Returns the stable identifier for this input.
    pub fn id(&self) -> &InputId {
        match self {
            Self::Text { id, .. } | Self::Choice { id, .. } | Self::Bool { id, .. } => id,
        }
    }

    /// Returns the user-facing label for this input.
    pub fn label(&self) -> &str {
        match self {
            Self::Text { label, .. } | Self::Choice { label, .. } | Self::Bool { label, .. } => {
                label
            }
        }
    }

    /// Returns choice values when this input is a choice input.
    ///
    /// `None` means the input is not a choice input. `Some(&[])` means the
    /// input is a choice input with no currently available choices.
    pub fn choices(&self) -> Option<&[ActionChoice]> {
        match self {
            Self::Choice { choices, .. } => Some(choices),
            Self::Text { .. } | Self::Bool { .. } => None,
        }
    }

    /// Returns this input as a choice input when applicable.
    pub fn as_choice(&self) -> Option<&Self> {
        match self {
            Self::Choice { .. } => Some(self),
            Self::Text { .. } | Self::Bool { .. } => None,
        }
    }
}

/// A selectable value for a choice input.
///
/// The `value` is the stable value inserted into
/// [`ActionArgs`](crate::invocation::ActionArgs). The `label` and optional
/// description are presentation text.
///
/// ```
/// use ratatui_action::input::ActionChoice;
///
/// let choice = ActionChoice::new("github-dark", "GitHub Dark")
///     .with_description("Dark theme based on GitHub syntax colors");
///
/// assert_eq!(choice.value(), "github-dark");
/// assert_eq!(choice.label(), "GitHub Dark");
/// assert_eq!(
///     choice.description(),
///     Some("Dark theme based on GitHub syntax colors")
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionChoice {
    value: String,
    label: String,
    description: Option<String>,
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

    /// Sets the optional user-facing description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Returns the stable value inserted into
    /// [`ActionArgs`](crate::invocation::ActionArgs).
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the user-facing label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the optional user-facing description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

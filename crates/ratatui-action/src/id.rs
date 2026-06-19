//! Stable identifiers for actions and inputs.

/// Stable identifier for an application action.
///
/// Action identifiers are semantic names owned by the application, such as
/// `document.open` or `theme.switch`. They should not encode presentation
/// details such as row position, colors, or keybindings.
///
/// ```
/// use ratatui_action::id::ActionId;
///
/// let id = ActionId::new("document.open");
///
/// assert_eq!(id.as_str(), "document.open");
/// ```
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
///
/// Input identifiers become keys in [`ActionArgs`](crate::invocation::ActionArgs).
/// Keep them stable for a given action so dispatch code can read resolved
/// arguments without depending on the UI that collected them.
///
/// ```
/// use ratatui_action::id::InputId;
/// use ratatui_action::invocation::ActionArgs;
///
/// let mut args = ActionArgs::new();
/// args.insert(InputId::new("theme"), "github-dark");
///
/// assert_eq!(args.get(&InputId::new("theme")), Some("github-dark"));
/// ```
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

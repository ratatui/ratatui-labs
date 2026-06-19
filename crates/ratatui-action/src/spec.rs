//! Action metadata and availability.
//!
//! Use this module to describe what an application can do:
//!
//! - [`ActionSpec`] stores stable identity, user-facing metadata, inputs, and availability.
//! - [`Availability`] controls whether an action is shown, hidden, or disabled.

use crate::id::ActionId;
use crate::input::ActionInput;

/// Metadata describing a semantic application action.
///
/// [`ActionSpec`] is presentation-neutral. It names what the application can do
/// and supplies metadata that surfaces such as command palettes, menus, help
/// screens, or keybinding inspectors can reuse.
///
/// Method groups:
///
/// - **Construction:** [`new`](Self::new), [`with_description`](Self::with_description),
///   [`with_category`](Self::with_category), [`with_keywords`](Self::with_keywords),
///   [`with_input`](Self::with_input), and [`with_availability`](Self::with_availability).
/// - **Metadata:** [`id`](Self::id), [`title`](Self::title), [`description`](Self::description),
///   [`category`](Self::category), [`keywords`](Self::keywords), and [`inputs`](Self::inputs).
/// - **Availability:** [`availability`](Self::availability), [`is_hidden`](Self::is_hidden), and
///   [`is_enabled`](Self::is_enabled).
///
/// # Examples
///
/// ```
/// use ratatui_action::spec::{ActionSpec, Availability};
///
/// let action = ActionSpec::new("workspace.close", "Close workspace")
///     .with_category("Workspace")
///     .with_description("Close the active workspace")
///     .with_keywords(["project", "folder"])
///     .with_availability(Availability::Disabled {
///         reason: "No workspace is open".into(),
///     });
///
/// assert_eq!(action.id().as_str(), "workspace.close");
/// assert_eq!(action.category(), Some("Workspace"));
/// assert!(!action.is_enabled());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSpec {
    id: ActionId,
    title: String,
    description: Option<String>,
    category: Option<String>,
    keywords: Vec<String>,
    inputs: Vec<ActionInput>,
    availability: Availability,
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

    /// Returns this action's stable identifier.
    pub fn id(&self) -> &ActionId {
        &self.id
    }

    /// Returns this action's user-facing title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the optional user-facing description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the optional grouping category.
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }

    /// Iterates over additional searchable keywords.
    ///
    /// The iterator yields `&str` rather than exposing the backing `Vec<String>`
    /// so the storage can change without affecting callers.
    pub fn keywords(&self) -> impl Iterator<Item = &str> {
        self.keywords.iter().map(String::as_str)
    }

    /// Returns inputs required before invocation.
    pub fn inputs(&self) -> &[ActionInput] {
        &self.inputs
    }

    /// Returns this action's presentation and invocation availability.
    pub fn availability(&self) -> &Availability {
        &self.availability
    }

    /// Sets the optional user-facing description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the optional grouping category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Replaces the searchable keywords.
    pub fn with_keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Appends an input required before invocation.
    pub fn with_input(mut self, input: ActionInput) -> Self {
        self.inputs.push(input);
        self
    }

    /// Sets this action's presentation and invocation availability.
    pub fn with_availability(mut self, availability: Availability) -> Self {
        self.availability = availability;
        self
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
///
/// Availability is a UI-facing contract. A surface may show disabled actions
/// with a reason, omit hidden actions from normal results, and invoke only
/// enabled actions.
///
/// Variant meanings:
///
/// - [`Enabled`](Self::Enabled) can be shown and invoked.
/// - [`Disabled`](Self::Disabled) can be shown with a reason but not invoked.
/// - [`Hidden`](Self::Hidden) should be omitted from ordinary UI results.
///
/// # Examples
///
/// ```
/// use ratatui_action::spec::{ActionSpec, Availability};
///
/// let action = ActionSpec::new("workspace.close", "Close workspace").with_availability(
///     Availability::Disabled {
///         reason: "No workspace is open".into(),
///     },
/// );
///
/// assert!(matches!(
///     action.availability(),
///     Availability::Disabled { .. }
/// ));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    /// The action can be shown and invoked.
    Enabled,
    /// The action can be shown, but not invoked.
    Disabled {
        /// Human-readable reason to show near the disabled action.
        reason: String,
    },
    /// The action should be omitted from ordinary UI results.
    Hidden,
}

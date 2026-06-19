//! Command palette state and events.
//!
//! The palette owns interaction state such as the current query and selected
//! row. Applications own dispatch and side effects: accepting a row returns an
//! event that the application can handle.

use crate::action::{ActionId, ActionInvocation, ActionSpec, Availability, InvocationSource};

/// Interaction mode for the command palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteMode {
    Searching,
    CollectingInput {
        action: ActionId,
        input_index: usize,
    },
}

/// Event emitted by palette state transitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteEvent {
    Invoke(ActionInvocation),
    PreviewChanged(Option<ActionInvocation>),
    Opened,
    Closed,
    Cancelled,
}

/// Direction or distance for selection movement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveSelection {
    Next,
    Previous,
    PageDown(usize),
    PageUp(usize),
    First,
    Last,
}

/// A prepared row for command palette rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteRow {
    pub action_id: ActionId,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: Option<String>,
    pub shortcut: Option<String>,
    pub availability: Availability,
}

impl PaletteRow {
    fn from_action(action: &ActionSpec) -> Self {
        Self {
            action_id: action.id.clone(),
            title: action.title.clone(),
            subtitle: action.description.clone(),
            category: action.category.clone(),
            shortcut: None,
            availability: action.availability.clone(),
        }
    }
}

/// Prepared command palette view data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteView {
    pub query: String,
    pub rows: Vec<PaletteRow>,
    pub selected: Option<usize>,
    pub mode: PaletteMode,
}

/// Stateful command palette model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteState {
    query: String,
    selected: Option<usize>,
    mode: PaletteMode,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected: None,
            mode: PaletteMode::Searching,
        }
    }
}

impl PaletteState {
    /// Creates a palette in searching mode.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the selected visible row index.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Returns the current interaction mode.
    pub fn mode(&self) -> &PaletteMode {
        &self.mode
    }

    /// Opens the palette and selects the first visible row when available.
    pub fn open(&mut self, actions: &[ActionSpec]) -> PaletteEvent {
        self.mode = PaletteMode::Searching;
        self.clamp_selection(actions);
        PaletteEvent::Opened
    }

    /// Closes the palette without implying cancellation.
    pub fn close(&mut self) -> PaletteEvent {
        self.mode = PaletteMode::Searching;
        PaletteEvent::Closed
    }

    /// Cancels the current interaction and clears transient input state.
    pub fn cancel(&mut self) -> PaletteEvent {
        self.query.clear();
        self.selected = None;
        self.mode = PaletteMode::Searching;
        PaletteEvent::Cancelled
    }

    /// Replaces the query and clamps selection to the filtered result set.
    pub fn set_query(&mut self, query: impl Into<String>, actions: &[ActionSpec]) {
        self.query = query.into();
        self.clamp_selection(actions);
    }

    /// Appends a character to the query and clamps selection.
    pub fn push_query_char(&mut self, character: char, actions: &[ActionSpec]) {
        self.query.push(character);
        self.clamp_selection(actions);
    }

    /// Removes the final query character and clamps selection.
    pub fn pop_query_char(&mut self, actions: &[ActionSpec]) -> Option<char> {
        let character = self.query.pop();
        self.clamp_selection(actions);
        character
    }

    /// Moves selection within the filtered result set.
    pub fn move_selection(&mut self, movement: MoveSelection, actions: &[ActionSpec]) {
        let row_count = self.filtered_actions(actions).len();
        self.selected = move_selection(self.selected, row_count, movement);
    }

    /// Returns a renderable view of current palette state.
    pub fn view(&self, actions: &[ActionSpec]) -> PaletteView {
        let rows = self
            .filtered_actions(actions)
            .into_iter()
            .map(PaletteRow::from_action)
            .collect();

        PaletteView {
            query: self.query.clone(),
            rows,
            selected: self.selected,
            mode: self.mode.clone(),
        }
    }

    /// Accepts the selected row when it is enabled and ready to invoke.
    pub fn accept(&mut self, actions: &[ActionSpec]) -> Option<PaletteEvent> {
        let action = self.selected_action(actions)?;

        match &action.availability {
            Availability::Enabled if action.inputs.is_empty() => {
                let invocation =
                    ActionInvocation::new(action.id.clone(), InvocationSource::Palette);
                Some(PaletteEvent::Invoke(invocation))
            }
            Availability::Enabled => {
                self.mode = PaletteMode::CollectingInput {
                    action: action.id.clone(),
                    input_index: 0,
                };
                None
            }
            Availability::Disabled { .. } | Availability::Hidden => None,
        }
    }

    fn selected_action<'a>(&self, actions: &'a [ActionSpec]) -> Option<&'a ActionSpec> {
        let selected = self.selected?;
        self.filtered_actions(actions).get(selected).copied()
    }

    fn clamp_selection(&mut self, actions: &[ActionSpec]) {
        let row_count = self.filtered_actions(actions).len();

        self.selected = match (self.selected, row_count) {
            (_, 0) => None,
            (Some(selected), row_count) if selected < row_count => Some(selected),
            _ => Some(0),
        };
    }

    fn filtered_actions<'a>(&self, actions: &'a [ActionSpec]) -> Vec<&'a ActionSpec> {
        actions
            .iter()
            .filter(|action| !action.is_hidden())
            .filter(|action| matches_query(action, &self.query))
            .collect()
    }
}

fn matches_query(action: &ActionSpec, query: &str) -> bool {
    let query = query.trim();

    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();

    contains_case_insensitive(&action.title, &query)
        || action
            .category
            .as_deref()
            .is_some_and(|category| contains_case_insensitive(category, &query))
        || action
            .keywords
            .iter()
            .any(|keyword| contains_case_insensitive(keyword, &query))
}

fn contains_case_insensitive(text: &str, lowercase_query: &str) -> bool {
    text.to_lowercase().contains(lowercase_query)
}

fn move_selection(
    selected: Option<usize>,
    row_count: usize,
    movement: MoveSelection,
) -> Option<usize> {
    if row_count == 0 {
        return None;
    }

    let selected = selected.unwrap_or(0);
    let last = row_count.saturating_sub(1);

    match movement {
        MoveSelection::Next => Some((selected + 1).min(last)),
        MoveSelection::Previous => Some(selected.saturating_sub(1)),
        MoveSelection::PageDown(amount) => Some(selected.saturating_add(amount).min(last)),
        MoveSelection::PageUp(amount) => Some(selected.saturating_sub(amount)),
        MoveSelection::First => Some(0),
        MoveSelection::Last => Some(last),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionInput;

    #[test]
    fn filters_actions_by_title_category_and_keywords() {
        let mut theme = ActionSpec::new("theme.switch", "Switch Theme");
        theme.category = Some("Appearance".into());
        theme.keywords = vec!["colors".into()];
        let quit = ActionSpec::new("app.quit", "Quit");
        let actions = vec![theme, quit];

        let mut state = PaletteState::new();
        state.set_query("appear", &actions);

        assert_eq!(
            state.view(&actions).rows[0].action_id.as_str(),
            "theme.switch"
        );

        state.set_query("COLOR", &actions);

        assert_eq!(
            state.view(&actions).rows[0].action_id.as_str(),
            "theme.switch"
        );
    }

    #[test]
    fn hides_hidden_actions_from_results() {
        let mut hidden = ActionSpec::new("debug.secret", "Secret Debug Action");
        hidden.availability = Availability::Hidden;
        let visible = ActionSpec::new("app.quit", "Quit");
        let actions = vec![hidden, visible];

        let mut state = PaletteState::new();
        state.open(&actions);

        let view = state.view(&actions);
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].action_id.as_str(), "app.quit");
    }

    #[test]
    fn clamps_selection_when_query_changes() {
        let actions = vec![
            ActionSpec::new("app.quit", "Quit"),
            ActionSpec::new("theme.switch", "Switch Theme"),
        ];

        let mut state = PaletteState::new();
        state.open(&actions);
        state.move_selection(MoveSelection::Last, &actions);
        state.set_query("quit", &actions);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn moves_selection_without_wrapping() {
        let actions = vec![
            ActionSpec::new("one", "One"),
            ActionSpec::new("two", "Two"),
            ActionSpec::new("three", "Three"),
        ];

        let mut state = PaletteState::new();
        state.open(&actions);
        state.move_selection(MoveSelection::Next, &actions);
        state.move_selection(MoveSelection::PageDown(10), &actions);

        assert_eq!(state.selected(), Some(2));

        state.move_selection(MoveSelection::Previous, &actions);
        state.move_selection(MoveSelection::PageUp(10), &actions);

        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn accepts_enabled_action_without_inputs() {
        let actions = vec![ActionSpec::new("app.quit", "Quit")];

        let mut state = PaletteState::new();
        state.open(&actions);

        let event = state.accept(&actions);

        assert_eq!(
            event,
            Some(PaletteEvent::Invoke(ActionInvocation::new(
                "app.quit",
                InvocationSource::Palette
            )))
        );
    }

    #[test]
    fn does_not_accept_disabled_action() {
        let mut disabled = ActionSpec::new("workspace.close", "Close Workspace");
        disabled.availability = Availability::Disabled {
            reason: "No workspace is open".into(),
        };
        let actions = vec![disabled];

        let mut state = PaletteState::new();
        state.open(&actions);

        assert_eq!(state.accept(&actions), None);
    }

    #[test]
    fn accepting_action_with_inputs_enters_collection_mode() {
        let mut action = ActionSpec::new("theme.switch", "Switch Theme");
        action.inputs.push(ActionInput::Choice {
            id: "theme".into(),
            label: "Theme".into(),
            choices: Vec::new(),
        });
        let actions = vec![action];

        let mut state = PaletteState::new();
        state.open(&actions);

        assert_eq!(state.accept(&actions), None);
        assert_eq!(
            state.mode(),
            &PaletteMode::CollectingInput {
                action: "theme.switch".into(),
                input_index: 0,
            }
        );
    }

    #[test]
    fn cancel_clears_query_selection_and_mode() {
        let actions = vec![ActionSpec::new("app.quit", "Quit")];

        let mut state = PaletteState::new();
        state.open(&actions);
        state.set_query("quit", &actions);

        assert_eq!(state.cancel(), PaletteEvent::Cancelled);
        assert_eq!(state.query(), "");
        assert_eq!(state.selected(), None);
        assert_eq!(state.mode(), &PaletteMode::Searching);
    }
}

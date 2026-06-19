//! Command palette state machine.
//!
//! Start here when wiring command palette behavior into an application:
//!
//! - [`PaletteState::new`] creates an empty state machine.
//! - [`PaletteState::open`], [`PaletteState::close`], [`PaletteState::cancel`], and
//!   [`PaletteState::cancel_events`] manage the interaction lifecycle.
//! - [`PaletteState::set_query`], [`PaletteState::push_query_char`], and
//!   [`PaletteState::pop_query_char`] update filtering or active text input.
//! - [`PaletteState::move_selection`] changes the selected action or input choice.
//! - [`PaletteState::accept`] emits invocations or enters input collection.
//! - [`PaletteState::view`] and [`PaletteState::view_with_shortcuts`] produce [`PaletteView`]
//!   snapshots for renderers.
//! - [`PaletteState::preview_event`] returns the current preview invocation without mutating state.

use ratatui_action::id::ActionId;
use ratatui_action::input::ActionInput;
use ratatui_action::invocation::{ActionArgs, ActionInvocation, InvocationSource};
use ratatui_action::spec::{ActionSpec, Availability};

use crate::event::{MoveSelection, PaletteEvent, PaletteMode};
use crate::shortcut::ShortcutLabels;
use crate::view::{PaletteRow, PaletteView};
use crate::{matching, selection};

/// Stateful command palette model.
///
/// [`PaletteState`] owns query text, row selection, input-collection mode, and
/// partially resolved arguments. It borrows the current action list for each
/// operation so applications can build action lists dynamically.
///
/// The methods fall into these caller tasks:
///
/// - **Lifecycle:** [`new`](Self::new), [`open`](Self::open), [`close`](Self::close),
///   [`cancel`](Self::cancel), and [`cancel_events`](Self::cancel_events).
/// - **Inspection:** [`query`](Self::query), [`selected`](Self::selected), and
///   [`mode`](Self::mode).
/// - **Editing:** [`set_query`](Self::set_query), [`push_query_char`](Self::push_query_char), and
///   [`pop_query_char`](Self::pop_query_char).
/// - **Selection and dispatch:** [`move_selection`](Self::move_selection),
///   [`accept`](Self::accept), and [`preview_event`](Self::preview_event).
/// - **Rendering:** [`view`](Self::view) and [`view_with_shortcuts`](Self::view_with_shortcuts).
///
/// # Examples
///
/// ```
/// use ratatui_action::spec::ActionSpec;
/// use ratatui_command_palette::event::MoveSelection;
/// use ratatui_command_palette::state::PaletteState;
///
/// let actions = vec![
///     ActionSpec::new("document.open", "Open document"),
///     ActionSpec::new("document.close", "Close document"),
/// ];
///
/// let mut palette = PaletteState::new();
/// palette.open(&actions);
/// palette.move_selection(MoveSelection::Next, &actions);
///
/// assert_eq!(palette.selected(), Some(1));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteState {
    query: String,
    selected: Option<usize>,
    mode: PaletteMode,
    args: ActionArgs,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            query: String::new(),
            selected: None,
            mode: PaletteMode::Searching,
            args: ActionArgs::new(),
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
    ///
    /// Opening resets input collection but keeps the current query. Call
    /// [`PaletteState::cancel`] when the interaction should also clear the
    /// query and selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("document.open", "Open document")];
    /// let mut palette = PaletteState::new();
    ///
    /// assert_eq!(palette.open(&actions), PaletteEvent::Opened);
    /// assert_eq!(palette.selected(), Some(0));
    /// ```
    pub fn open(&mut self, actions: &[ActionSpec]) -> PaletteEvent {
        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();
        self.clamp_selection(actions);
        PaletteEvent::Opened
    }

    /// Closes the palette without implying cancellation.
    ///
    /// Closing clears transient input collection state but does not clear the
    /// query. Applications that treat close as an abandoned interaction should
    /// use [`PaletteState::cancel`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("document.open", "Open document")];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.set_query("open", &actions);
    ///
    /// assert_eq!(palette.close(), PaletteEvent::Closed);
    /// assert_eq!(palette.query(), "open");
    /// ```
    pub fn close(&mut self) -> PaletteEvent {
        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();
        PaletteEvent::Closed
    }

    /// Cancels the current interaction and clears transient input state.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("document.open", "Open document")];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.set_query("open", &actions);
    ///
    /// assert_eq!(palette.cancel(), PaletteEvent::Cancelled);
    /// assert_eq!(palette.query(), "");
    /// assert_eq!(palette.selected(), None);
    /// ```
    pub fn cancel(&mut self) -> PaletteEvent {
        self.cancel_events();
        PaletteEvent::Cancelled
    }

    /// Cancels the current interaction and returns all resulting events.
    ///
    /// If cancellation abandons an active preview, the returned events include
    /// `PaletteEvent::PreviewChanged(None)` before [`PaletteEvent::Cancelled`]
    /// so the application can roll back transient preview state.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::id::InputId;
    /// use ratatui_action::input::{ActionChoice, ActionInput};
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions =
    ///     vec![
    ///         ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
    ///             id: InputId::new("theme"),
    ///             label: "Theme".into(),
    ///             choices: vec![ActionChoice::new("catppuccin", "Catppuccin")],
    ///         }),
    ///     ];
    ///
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.accept(&actions);
    ///
    /// assert_eq!(
    ///     palette.cancel_events(),
    ///     vec![PaletteEvent::PreviewChanged(None), PaletteEvent::Cancelled]
    /// );
    /// ```
    pub fn cancel_events(&mut self) -> Vec<PaletteEvent> {
        let clears_preview = matches!(self.mode, PaletteMode::CollectingInput { .. });
        self.query.clear();
        self.selected = None;
        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();

        let mut events = Vec::new();
        if clears_preview {
            events.push(PaletteEvent::PreviewChanged(None));
        }
        events.push(PaletteEvent::Cancelled);
        events
    }

    /// Replaces the query and clamps selection to the filtered result set.
    ///
    /// Setting the query leaves input collection and returns to
    /// [`PaletteMode::Searching`].
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![
    ///     ActionSpec::new("document.open", "Open document"),
    ///     ActionSpec::new("theme.switch", "Switch theme"),
    /// ];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.set_query("theme", &actions);
    ///
    /// assert_eq!(palette.view(&actions).rows()[0].title(), "Switch theme");
    /// ```
    pub fn set_query(&mut self, query: impl Into<String>, actions: &[ActionSpec]) {
        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();
        self.query = query.into();
        self.select_first(actions);
    }

    /// Appends a character to the query and clamps selection.
    ///
    /// This is the low-level edit primitive used by interactive input loops.
    /// Applications may call [`PaletteState::set_query`] instead when they
    /// already have the full query string.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("theme.switch", "Switch theme")];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    ///
    /// for character in "theme".chars() {
    ///     palette.push_query_char(character, &actions);
    /// }
    ///
    /// assert_eq!(palette.query(), "theme");
    /// assert_eq!(palette.view(&actions).rows()[0].title(), "Switch theme");
    /// ```
    pub fn push_query_char(&mut self, character: char, actions: &[ActionSpec]) {
        if matches!(self.current_input(actions), Some(ActionInput::Text { .. })) {
            self.query.push(character);
            return;
        }

        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();
        self.query.push(character);
        self.select_first(actions);
    }

    /// Removes the final query character and clamps selection.
    ///
    /// Returns the removed character, or `None` when the query was already
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("theme.switch", "Switch theme")];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.set_query("theme", &actions);
    ///
    /// assert_eq!(palette.pop_query_char(&actions), Some('e'));
    /// assert_eq!(palette.query(), "them");
    /// ```
    pub fn pop_query_char(&mut self, actions: &[ActionSpec]) -> Option<char> {
        if matches!(self.current_input(actions), Some(ActionInput::Text { .. })) {
            return self.query.pop();
        }

        self.mode = PaletteMode::Searching;
        self.args = ActionArgs::new();
        let character = self.query.pop();
        self.select_first(actions);
        character
    }

    /// Moves selection within the filtered result set.
    ///
    /// Relative movement wraps at the result boundaries. In choice-input mode,
    /// movement can emit [`PaletteEvent::PreviewChanged`] so the application can
    /// preview the selected value without committing it.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::id::InputId;
    /// use ratatui_action::input::{ActionChoice, ActionInput};
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::{MoveSelection, PaletteEvent};
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions =
    ///     vec![
    ///         ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
    ///             id: InputId::new("theme"),
    ///             label: "Theme".into(),
    ///             choices: vec![
    ///                 ActionChoice::new("catppuccin", "Catppuccin"),
    ///                 ActionChoice::new("github-dark", "GitHub Dark"),
    ///             ],
    ///         }),
    ///     ];
    ///
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.accept(&actions);
    ///
    /// let event = palette.move_selection(MoveSelection::Next, &actions);
    /// assert!(matches!(event, Some(PaletteEvent::PreviewChanged(Some(_)))));
    /// ```
    pub fn move_selection(
        &mut self,
        movement: MoveSelection,
        actions: &[ActionSpec],
    ) -> Option<PaletteEvent> {
        let row_count = self.current_row_count(actions);
        self.selected = selection::move_selection(self.selected, row_count, movement);
        self.preview_event(actions)
    }

    /// Returns a renderable view of current palette state.
    ///
    /// The view is a snapshot for renderers and tests. Mutating the palette
    /// after creating a view does not update that previous view.
    pub fn view(&self, actions: &[ActionSpec]) -> PaletteView {
        self.view_with_shortcuts(actions, &ShortcutLabels::new())
    }

    /// Returns a renderable view with presentation-only shortcut labels.
    ///
    /// Shortcuts are not stored on [`ActionSpec`]. Applications can pass the
    /// currently active keymap labels here while keeping action metadata
    /// semantic and context independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::shortcut::ShortcutLabels;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("document.open", "Open document")];
    /// let mut shortcuts = ShortcutLabels::new();
    /// shortcuts.insert("document.open", "Ctrl-O");
    ///
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    ///
    /// let view = palette.view_with_shortcuts(&actions, &shortcuts);
    ///
    /// assert_eq!(view.rows()[0].shortcut(), Some("Ctrl-O"));
    /// ```
    pub fn view_with_shortcuts(
        &self,
        actions: &[ActionSpec],
        shortcuts: &ShortcutLabels,
    ) -> PaletteView {
        let rows = match &self.mode {
            PaletteMode::Searching => self
                .filtered_actions(actions)
                .into_iter()
                .map(|action| PaletteRow::from_action_with_shortcuts(action, shortcuts))
                .collect(),
            PaletteMode::CollectingInput {
                action,
                input_index,
            } => self.input_rows(actions, action, *input_index),
        };

        PaletteView {
            prompt: self.prompt(actions),
            query: match &self.mode {
                PaletteMode::Searching => self.query.clone(),
                PaletteMode::CollectingInput { .. }
                    if matches!(self.current_input(actions), Some(ActionInput::Text { .. })) =>
                {
                    self.query.clone()
                }
                PaletteMode::CollectingInput { .. } => String::new(),
            },
            rows,
            selected: self.selected,
            mode: self.mode.clone(),
        }
    }

    /// Accepts the selected row when it is enabled and ready to invoke.
    ///
    /// Accepting an enabled action with no inputs returns
    /// [`PaletteEvent::Invoke`]. Accepting an enabled action whose first input
    /// is a choice enters [`PaletteMode::CollectingInput`] and returns a
    /// preview event for the selected choice. Disabled, hidden, unsupported, or
    /// unselected actions return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions = vec![ActionSpec::new("document.open", "Open document")];
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    ///
    /// let Some(PaletteEvent::Invoke(invocation)) = palette.accept(&actions) else {
    ///     panic!("expected invocation");
    /// };
    ///
    /// assert_eq!(invocation.id().as_str(), "document.open");
    /// ```
    pub fn accept(&mut self, actions: &[ActionSpec]) -> Option<PaletteEvent> {
        if matches!(self.mode, PaletteMode::CollectingInput { .. }) {
            return self.accept_input(actions);
        }

        let action = self.selected_action(actions)?;

        match action.availability() {
            Availability::Enabled if action.inputs().is_empty() => {
                let invocation =
                    ActionInvocation::new(action.id().clone(), InvocationSource::Palette);
                self.query.clear();
                self.select_first(actions);
                Some(PaletteEvent::Invoke(invocation))
            }
            Availability::Enabled
                if action
                    .inputs()
                    .first()
                    .filter(|input| is_collectible_input(input))
                    .is_some() =>
            {
                self.mode = PaletteMode::CollectingInput {
                    action: action.id().clone(),
                    input_index: 0,
                };
                self.query.clear();
                self.selected = selection::selected_for_input(action.inputs().first());
                self.preview_event(actions)
            }
            Availability::Enabled => None,
            Availability::Disabled { .. } | Availability::Hidden => None,
        }
    }

    /// Returns a preview event for the current choice selection, when available.
    ///
    /// This method does not mutate palette state. It returns `None` outside
    /// choice-input mode or when no choice is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use ratatui_action::id::InputId;
    /// use ratatui_action::input::{ActionChoice, ActionInput};
    /// use ratatui_action::spec::ActionSpec;
    /// use ratatui_command_palette::event::PaletteEvent;
    /// use ratatui_command_palette::state::PaletteState;
    ///
    /// let actions =
    ///     vec![
    ///         ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
    ///             id: InputId::new("theme"),
    ///             label: "Theme".into(),
    ///             choices: vec![ActionChoice::new("catppuccin", "Catppuccin")],
    ///         }),
    ///     ];
    ///
    /// let mut palette = PaletteState::new();
    /// palette.open(&actions);
    /// palette.accept(&actions);
    ///
    /// let Some(PaletteEvent::PreviewChanged(Some(preview))) = palette.preview_event(&actions) else {
    ///     panic!("expected preview event");
    /// };
    ///
    /// assert_eq!(
    ///     preview.args().get(&InputId::new("theme")),
    ///     Some("catppuccin")
    /// );
    /// ```
    pub fn preview_event(&self, actions: &[ActionSpec]) -> Option<PaletteEvent> {
        let (action, input, value) = self.selected_input_value(actions)?;
        let mut args = self.args.clone();
        args.insert(input.id().clone(), value);
        let invocation =
            ActionInvocation::with_args(action.id().clone(), args, InvocationSource::Palette);

        Some(PaletteEvent::PreviewChanged(Some(invocation)))
    }

    fn accept_input(&mut self, actions: &[ActionSpec]) -> Option<PaletteEvent> {
        let (action, input, value) = self.current_input_value(actions)?;
        let input_id = input.id().clone();
        let action_id = action.id().clone();
        self.args.insert(input_id, value);

        let PaletteMode::CollectingInput { input_index, .. } = self.mode else {
            return None;
        };
        let next_index = input_index + 1;

        if next_index < action.inputs().len() {
            self.mode = PaletteMode::CollectingInput {
                action: action_id,
                input_index: next_index,
            };
            self.query.clear();
            self.selected = selection::selected_for_input(action.inputs().get(next_index));
            return self.preview_event(actions);
        }

        let invocation = ActionInvocation::with_args(
            action_id,
            std::mem::take(&mut self.args),
            InvocationSource::Palette,
        );
        self.mode = PaletteMode::Searching;
        self.query.clear();

        Some(PaletteEvent::Invoke(invocation))
    }

    fn selected_action<'a>(&self, actions: &'a [ActionSpec]) -> Option<&'a ActionSpec> {
        let selected = self.selected?;
        self.filtered_actions(actions).get(selected).copied()
    }

    fn clamp_selection(&mut self, actions: &[ActionSpec]) {
        let row_count = self.current_row_count(actions);

        self.selected = match (self.selected, row_count) {
            (_, 0) => None,
            (Some(selected), row_count) if selected < row_count => Some(selected),
            _ => Some(0),
        };
    }

    fn select_first(&mut self, actions: &[ActionSpec]) {
        self.selected = (self.current_row_count(actions) > 0).then_some(0);
    }

    fn filtered_actions<'a>(&self, actions: &'a [ActionSpec]) -> Vec<&'a ActionSpec> {
        matching::filtered_actions(actions, &self.query)
    }

    fn current_row_count(&self, actions: &[ActionSpec]) -> usize {
        match &self.mode {
            PaletteMode::Searching => self.filtered_actions(actions).len(),
            PaletteMode::CollectingInput {
                action,
                input_index,
            } => self.input_rows(actions, action, *input_index).len(),
        }
    }

    fn prompt(&self, actions: &[ActionSpec]) -> String {
        match &self.mode {
            PaletteMode::Searching => ">".into(),
            PaletteMode::CollectingInput { .. } => self
                .current_input(actions)
                .map_or_else(|| ">".into(), |input| format!("{}:", input.label())),
        }
    }

    fn current_input<'a>(&self, actions: &'a [ActionSpec]) -> Option<&'a ActionInput> {
        let PaletteMode::CollectingInput {
            action,
            input_index,
        } = &self.mode
        else {
            return None;
        };

        actions
            .iter()
            .find(|candidate| candidate.id() == action)?
            .inputs()
            .get(*input_index)
    }

    fn input_rows(
        &self,
        actions: &[ActionSpec],
        action: &ActionId,
        input_index: usize,
    ) -> Vec<PaletteRow> {
        let Some(input) = actions
            .iter()
            .find(|candidate| candidate.id() == action)
            .and_then(|action| action.inputs().get(input_index))
        else {
            return Vec::new();
        };

        match input {
            ActionInput::Choice { .. } => input
                .choices()
                .unwrap_or_default()
                .iter()
                .map(|choice| PaletteRow::from_choice(action, input, choice))
                .collect(),
            ActionInput::Bool { .. } => vec![
                PaletteRow::from_bool(action, input, true),
                PaletteRow::from_bool(action, input, false),
            ],
            ActionInput::Text { .. } => Vec::new(),
        }
    }

    fn current_input_value<'a>(
        &self,
        actions: &'a [ActionSpec],
    ) -> Option<(&'a ActionSpec, &'a ActionInput, String)> {
        let PaletteMode::CollectingInput {
            action,
            input_index,
        } = &self.mode
        else {
            return None;
        };
        let action = actions.iter().find(|candidate| candidate.id() == action)?;
        let input = action.inputs().get(*input_index)?;

        match input {
            ActionInput::Text { .. } => Some((action, input, self.query.clone())),
            ActionInput::Choice { .. } | ActionInput::Bool { .. } => {
                self.selected_input_value(actions)
            }
        }
    }

    fn selected_input_value<'a>(
        &self,
        actions: &'a [ActionSpec],
    ) -> Option<(&'a ActionSpec, &'a ActionInput, String)> {
        let PaletteMode::CollectingInput {
            action,
            input_index,
        } = &self.mode
        else {
            return None;
        };
        let selected = self.selected?;
        let action = actions.iter().find(|candidate| candidate.id() == action)?;
        let input = action.inputs().get(*input_index)?;

        match input {
            ActionInput::Choice { choices, .. } => choices
                .get(selected)
                .map(|choice| (action, input, choice.value().to_string())),
            ActionInput::Bool { .. } => match selected {
                0 => Some((action, input, "true".into())),
                1 => Some((action, input, "false".into())),
                _ => None,
            },
            ActionInput::Text { .. } => None,
        }
    }
}

fn is_collectible_input(input: &ActionInput) -> bool {
    match input {
        ActionInput::Text { .. } | ActionInput::Bool { .. } => true,
        ActionInput::Choice { choices, .. } => !choices.is_empty(),
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod state_tests;

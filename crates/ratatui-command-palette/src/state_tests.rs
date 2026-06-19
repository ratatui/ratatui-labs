use ratatui_action::input::{ActionChoice, ActionInput};
use ratatui_action::invocation::{ActionInvocation, InvocationSource};
use ratatui_action::spec::{ActionSpec, Availability};

use crate::event::{MoveSelection, PaletteEvent, PaletteMode};
use crate::shortcut::ShortcutLabels;
use crate::state::PaletteState;

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
fn query_edits_select_first_filtered_result() {
    let actions = vec![
        ActionSpec::new("document.open", "Open Document"),
        ActionSpec::new("help.open", "Open Help"),
        ActionSpec::new("debug.toggle", "Toggle Debug"),
    ];

    let mut state = PaletteState::new();
    state.open(&actions);
    state.move_selection(MoveSelection::Last, &actions);
    state.set_query("open", &actions);

    let Some(PaletteEvent::Invoke(invocation)) = state.accept(&actions) else {
        panic!("expected invocation event");
    };

    assert_eq!(invocation.id().as_str(), "document.open");
}

#[test]
fn accepting_action_without_inputs_clears_query() {
    let actions = vec![
        ActionSpec::new("document.open", "Open Document"),
        ActionSpec::new("app.quit", "Quit"),
    ];

    let mut state = PaletteState::new();
    state.open(&actions);
    state.set_query("open", &actions);

    let Some(PaletteEvent::Invoke(_)) = state.accept(&actions) else {
        panic!("expected invocation event");
    };

    assert_eq!(state.query(), "");
    assert_eq!(state.view(&actions).rows().len(), 2);
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
    let disabled = ActionSpec::new("workspace.close", "Close Workspace").with_availability(
        Availability::Disabled {
            reason: "No workspace is open".into(),
        },
    );
    let actions = vec![disabled];

    let mut state = PaletteState::new();
    state.open(&actions);

    assert_eq!(state.accept(&actions), None);
}

#[test]
fn accepting_action_with_inputs_enters_collection_mode() {
    let action = ActionSpec::new("theme.switch", "Switch Theme").with_input(ActionInput::Choice {
        id: "theme".into(),
        label: "Theme".into(),
        choices: vec![ActionChoice::new("catppuccin", "Catppuccin")],
    });
    let actions = vec![action];

    let mut state = PaletteState::new();
    state.open(&actions);

    assert!(matches!(
        state.accept(&actions),
        Some(PaletteEvent::PreviewChanged(Some(_)))
    ));
    assert_eq!(
        state.mode(),
        &PaletteMode::CollectingInput {
            action: "theme.switch".into(),
            input_index: 0,
        }
    );
}

#[test]
fn choice_input_view_shows_choices() {
    let actions = vec![theme_action()];
    let mut state = PaletteState::new();
    state.open(&actions);
    state.accept(&actions);

    let view = state.view(&actions);

    assert_eq!(view.prompt(), "Theme:");
    assert_eq!(view.rows().len(), 2);
    assert_eq!(view.rows()[0].title(), "Catppuccin");
    assert_eq!(view.selected(), Some(0));
}

#[test]
fn moving_choice_selection_emits_preview_event() {
    let actions = vec![theme_action()];
    let mut state = PaletteState::new();
    state.open(&actions);
    state.accept(&actions);

    let event = state.move_selection(MoveSelection::Next, &actions);

    let Some(PaletteEvent::PreviewChanged(Some(invocation))) = event else {
        panic!("expected preview event");
    };
    assert_eq!(invocation.id().as_str(), "theme.switch");
    assert_eq!(invocation.args().get(&"theme".into()), Some("github-dark"));
}

#[test]
fn accepting_choice_input_invokes_with_resolved_args() {
    let actions = vec![theme_action()];
    let mut state = PaletteState::new();
    state.open(&actions);
    state.accept(&actions);
    state.move_selection(MoveSelection::Next, &actions);

    let event = state.accept(&actions);

    let Some(PaletteEvent::Invoke(invocation)) = event else {
        panic!("expected invocation event");
    };
    assert_eq!(invocation.id().as_str(), "theme.switch");
    assert_eq!(invocation.args().get(&"theme".into()), Some("github-dark"));
    assert_eq!(state.mode(), &PaletteMode::Searching);
}

#[test]
fn unsupported_input_does_not_enter_empty_collection_mode() {
    let action =
        ActionSpec::new("settings.advanced", "Advanced Settings").with_input(ActionInput::Choice {
            id: "empty".into(),
            label: "Empty".into(),
            choices: Vec::new(),
        });
    let actions = vec![action];

    let mut state = PaletteState::new();
    state.open(&actions);

    assert_eq!(state.accept(&actions), None);
    assert_eq!(state.mode(), &PaletteMode::Searching);
}

#[test]
fn text_input_invokes_with_typed_value() {
    let action = ActionSpec::new("search.open", "Open Search").with_input(ActionInput::Text {
        id: "query".into(),
        label: "Query".into(),
        placeholder: Some("Search text".into()),
    });
    let actions = vec![action];

    let mut state = PaletteState::new();
    state.open(&actions);
    assert_eq!(state.accept(&actions), None);
    assert_eq!(
        state.mode(),
        &PaletteMode::CollectingInput {
            action: "search.open".into(),
            input_index: 0,
        }
    );

    for character in "widgets".chars() {
        state.push_query_char(character, &actions);
    }

    let Some(PaletteEvent::Invoke(invocation)) = state.accept(&actions) else {
        panic!("expected invocation event");
    };
    assert_eq!(invocation.id().as_str(), "search.open");
    assert_eq!(invocation.args().get(&"query".into()), Some("widgets"));
}

#[test]
fn bool_input_view_and_invocation_use_selected_value() {
    let action = ActionSpec::new("debug.toggle", "Toggle Debug").with_input(ActionInput::Bool {
        id: "enabled".into(),
        label: "Enabled".into(),
    });
    let actions = vec![action];

    let mut state = PaletteState::new();
    state.open(&actions);
    assert!(matches!(
        state.accept(&actions),
        Some(PaletteEvent::PreviewChanged(Some(_)))
    ));

    let view = state.view(&actions);
    assert_eq!(view.prompt(), "Enabled:");
    assert_eq!(view.rows()[0].title(), "Yes");
    assert_eq!(view.rows()[1].title(), "No");

    state.move_selection(MoveSelection::Next, &actions);
    let Some(PaletteEvent::Invoke(invocation)) = state.accept(&actions) else {
        panic!("expected invocation event");
    };

    assert_eq!(invocation.args().get(&"enabled".into()), Some("false"));
}

#[test]
fn view_can_include_shortcuts_from_external_keymap_labels() {
    let actions = vec![ActionSpec::new("document.open", "Open Document")];
    let mut shortcuts = ShortcutLabels::new();
    shortcuts.insert("document.open", "Ctrl-O");

    let mut state = PaletteState::new();
    state.open(&actions);

    let view = state.view_with_shortcuts(&actions, &shortcuts);

    assert_eq!(view.rows()[0].shortcut(), Some("Ctrl-O"));
}

#[test]
fn cancel_events_clear_preview_before_cancelling() {
    let actions = vec![theme_action()];
    let mut state = PaletteState::new();
    state.open(&actions);
    state.accept(&actions);

    assert_eq!(
        state.cancel_events(),
        vec![PaletteEvent::PreviewChanged(None), PaletteEvent::Cancelled]
    );
    assert_eq!(state.mode(), &PaletteMode::Searching);
}

#[test]
fn editing_query_returns_to_searching_mode() {
    let action = ActionSpec::new("theme.switch", "Switch Theme").with_input(ActionInput::Choice {
        id: "theme".into(),
        label: "Theme".into(),
        choices: vec![ActionChoice::new("catppuccin", "Catppuccin")],
    });
    let actions = vec![action];

    let mut state = PaletteState::new();
    state.open(&actions);
    state.accept(&actions);

    state.push_query_char('q', &actions);

    assert_eq!(state.mode(), &PaletteMode::Searching);
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

fn theme_action() -> ActionSpec {
    ActionSpec::new("theme.switch", "Switch Theme").with_input(ActionInput::Choice {
        id: "theme".into(),
        label: "Theme".into(),
        choices: vec![
            ActionChoice::new("catppuccin", "Catppuccin"),
            ActionChoice::new("github-dark", "GitHub Dark"),
        ],
    })
}

//! Selection movement helpers.

use ratatui_action::input::ActionInput;

use crate::event::MoveSelection;

pub(crate) fn selected_for_input(input: Option<&ActionInput>) -> Option<usize> {
    match input {
        Some(ActionInput::Choice { choices, .. }) if !choices.is_empty() => Some(0),
        Some(ActionInput::Bool { .. }) => Some(0),
        Some(ActionInput::Text { .. }) | Some(ActionInput::Choice { .. }) | None => None,
    }
}

pub(crate) fn move_selection(
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
        MoveSelection::Next => Some((selected + 1) % row_count),
        MoveSelection::Previous => Some(selected.checked_sub(1).unwrap_or(last)),
        MoveSelection::PageDown(amount) => Some((selected + amount) % row_count),
        MoveSelection::PageUp(amount) => {
            Some((row_count + selected - (amount % row_count)) % row_count)
        }
        MoveSelection::First => Some(0),
        MoveSelection::Last => Some(last),
    }
}

#[cfg(test)]
mod tests {
    use ratatui_action::input::ActionInput;

    use super::*;

    #[test]
    fn wraps_at_boundaries() {
        let selected = move_selection(Some(0), 3, MoveSelection::Next);

        assert_eq!(selected, Some(1));

        assert_eq!(move_selection(Some(2), 3, MoveSelection::Next), Some(0));
        assert_eq!(move_selection(Some(0), 3, MoveSelection::Previous), Some(2));
    }

    #[test]
    fn page_movement_wraps_by_amount() {
        assert_eq!(
            move_selection(Some(1), 3, MoveSelection::PageDown(5)),
            Some(0)
        );
        assert_eq!(
            move_selection(Some(1), 3, MoveSelection::PageUp(5)),
            Some(2)
        );
    }

    #[test]
    fn selects_collectible_choice_and_bool_inputs() {
        let text = ActionInput::Text {
            id: "query".into(),
            label: "Query".into(),
            placeholder: None,
        };
        let bool_input = ActionInput::Bool {
            id: "enabled".into(),
            label: "Enabled".into(),
        };

        assert_eq!(selected_for_input(Some(&text)), None);
        assert_eq!(selected_for_input(Some(&bool_input)), Some(0));
    }
}

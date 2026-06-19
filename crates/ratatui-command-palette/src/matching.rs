//! Action filtering for palette search.
//!
//! The default matcher is intentionally small: case-insensitive substring
//! matching over title, category, and keywords with stable input ordering.
//! Applications that need fuzzy matching, recency, frequency, or context
//! ranking should prepare or order the action list before passing it to
//! [`PaletteState`](crate::state::PaletteState).

use ratatui_action::spec::ActionSpec;

/// Returns visible actions whose metadata matches `query`.
pub(crate) fn filtered_actions<'a>(actions: &'a [ActionSpec], query: &str) -> Vec<&'a ActionSpec> {
    actions
        .iter()
        .filter(|action| !action.is_hidden())
        .filter(|action| matches_query(action, query))
        .collect()
}

fn matches_query(action: &ActionSpec, query: &str) -> bool {
    let query = query.trim();

    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();

    contains_case_insensitive(action.title(), &query)
        || action
            .category()
            .is_some_and(|category| contains_case_insensitive(category, &query))
        || action
            .keywords()
            .any(|keyword| contains_case_insensitive(keyword, &query))
}

fn contains_case_insensitive(text: &str, lowercase_query: &str) -> bool {
    text.to_lowercase().contains(lowercase_query)
}

#[cfg(test)]
mod tests {
    use ratatui_action::spec::{ActionSpec, Availability};

    use super::*;

    #[test]
    fn filters_actions_by_title_category_and_keywords() {
        let theme = ActionSpec::new("theme.switch", "Switch Theme")
            .with_category("Appearance")
            .with_keywords(["colors"]);
        let quit = ActionSpec::new("app.quit", "Quit");
        let actions = vec![theme, quit];

        assert_eq!(
            filtered_actions(&actions, "appear")[0].id().as_str(),
            "theme.switch"
        );
        assert_eq!(
            filtered_actions(&actions, "COLOR")[0].id().as_str(),
            "theme.switch"
        );
    }

    #[test]
    fn omits_hidden_actions() {
        let hidden = ActionSpec::new("debug.secret", "Secret Debug Action")
            .with_availability(Availability::Hidden);
        let visible = ActionSpec::new("app.quit", "Quit");
        let actions = vec![hidden, visible];

        let filtered = filtered_actions(&actions, "");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id().as_str(), "app.quit");
    }
}

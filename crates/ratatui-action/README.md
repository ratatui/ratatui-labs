# ratatui-action

Experimental semantic action model for Ratatui applications.

This crate is part of `ratatui-labs`. It describes application capabilities independently from the
UI surfaces that present or invoke them. APIs are experimental and may change or disappear.

The command palette experiment uses this crate for action identity, metadata, availability, inputs,
arguments, and invocation requests.

## Usage

Declare semantic actions with stable identifiers. Dispatch remains application-owned; UI surfaces
return invocation requests instead of running callbacks.

```rust
use ratatui_action::{
    id::InputId,
    input::{ActionChoice, ActionInput},
    invocation::{ActionArgs, ActionInvocation, InvocationSource},
    spec::{ActionSpec, Availability},
};

let switch_theme = ActionSpec::new("theme.switch", "Switch theme")
    .with_category("Appearance")
    .with_description("Preview and apply a terminal color theme")
    .with_keywords(["color", "style"])
    .with_input(ActionInput::Choice {
        id: InputId::new("theme"),
        label: "Theme".into(),
        choices: vec![
            ActionChoice::new("catppuccin", "Catppuccin"),
            ActionChoice::new("github-dark", "GitHub Dark"),
        ],
    });

let close_workspace = ActionSpec::new("workspace.close", "Close workspace").with_availability(
    Availability::Disabled {
        reason: "No workspace is open".into(),
    },
);

let mut args = ActionArgs::new();
args.insert("theme", "github-dark");
let invocation =
    ActionInvocation::with_args(switch_theme.id().clone(), args, InvocationSource::Palette);

assert_eq!(invocation.id().as_str(), "theme.switch");
assert_eq!(invocation.args().get(&InputId::new("theme")), Some("github-dark"));
assert!(!close_workspace.is_enabled());
```

## API Notes

- `ActionSpec` and `ActionInvocation` use private fields so the experiment can add validation or
  change storage without committing to struct literals.
- `ActionSpec` stores owned text for now, while accessors expose borrowed text or iterators so the
  storage can be revisited before publication.
- `ActionInput` describes required values, not a specific input widget.
- `ActionArgs` stores resolved input values by `InputId`, so dispatch code can ignore which surface
  collected them.

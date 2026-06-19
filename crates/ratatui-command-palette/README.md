# ratatui-command-palette

Experimental command palette state and rendering for Ratatui applications.

This crate is part of `ratatui-labs`. It consumes `ratatui-action` metadata, owns palette
interaction state, and emits events for the application to dispatch. APIs are experimental and may
change or disappear.

## Usage

Applications pass action specs into `PaletteState`, render the returned view, and handle emitted
events at their own dispatch boundary.

```rust
use ratatui_action::{
    id::InputId,
    input::{ActionChoice, ActionInput},
    spec::ActionSpec,
};
use ratatui_command_palette::{
    event::{MoveSelection, PaletteEvent},
    state::PaletteState,
};

let actions = vec![
    ActionSpec::new("document.open", "Open document").with_category("Navigation"),
    ActionSpec::new("theme.switch", "Switch theme").with_input(ActionInput::Choice {
        id: InputId::new("theme"),
        label: "Theme".into(),
        choices: vec![
            ActionChoice::new("catppuccin", "Catppuccin"),
            ActionChoice::new("github-dark", "GitHub Dark"),
        ],
    }),
];

let mut palette = PaletteState::new();
palette.open(&actions);
palette.set_query("theme", &actions);

let view = palette.view(&actions);
assert_eq!(view.rows()[0].title(), "Switch theme");

palette.accept(&actions);
let event = palette.move_selection(MoveSelection::Next, &actions);

let Some(PaletteEvent::PreviewChanged(Some(preview))) = event else {
    panic!("expected preview event");
};

assert_eq!(preview.args().get(&InputId::new("theme")), Some("github-dark"));
```

For crossterm applications, convert key events into palette commands with `PaletteKey` and keep
application policy, such as whether `Esc` exits or cancels, at the application boundary.

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_command_palette::key::PaletteKey;

let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);

assert!(matches!(PaletteKey::from_crossterm(key), PaletteKey::Move(_)));
```

The palette collects `Text`, `Choice`, and `Bool` inputs inline. `Choice` and `Bool` inputs emit
preview events as selection changes; `Text` inputs use the query line as the input field and invoke
with the typed value when accepted. Use `PaletteState::cancel_events` when the application needs an
explicit `PreviewChanged(None)` event before cancellation.

Shortcut labels are presentation data, not action metadata. Keep keybindings in the application or
keymap layer, then pass the active labels through `ShortcutLabels` when rendering a palette view.

Renderers consume `PaletteView` snapshots. The crate includes modal, flat overlay, split preview,
fullscreen, and inline dropdown renderers so applications can keep one action model while changing
presentation.

```rust
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui_action::spec::ActionSpec;
use ratatui_command_palette::{
    render::{FlatOverlayRenderer, ModalRenderer, PaletteRenderer},
    state::PaletteState,
};

let actions = vec![ActionSpec::new("document.open", "Open document")];
let mut palette = PaletteState::new();
palette.open(&actions);

let view = palette.view(&actions);
let area = Rect::new(0, 0, 40, 8);
let mut buffer = Buffer::empty(area);

ModalRenderer::new().render(area, &mut buffer, &view);
FlatOverlayRenderer::new().render(area, &mut buffer, &view);
```

Run the interactive examples with:

```sh
cargo run -p ratatui-command-palette --example command-palette
cargo run -p ratatui-command-palette --example command-palette -- --help
cargo run -p ratatui-command-palette --example command-palette -- --renderer split
cargo run -p ratatui-command-palette --example renderer-gallery
cargo run -p ratatui-command-palette --example renderer-gallery -- --help
```

`command-palette -- --renderer` accepts `modal`, `flat`, `split`, `fullscreen`, and `inline`.

Rendered validation is captured by the repository Betamax tape:

```sh
just betamax
just betamax tapes/renderers/split.tape
```

The tapes drive the real examples and write screenshots, terminal state, and GIF artifacts under
`target/betamax/`. Renderer-specific example tapes live under `tapes/renderers/`. Set
`BETAMAX_JOBS` to control default parallelism.

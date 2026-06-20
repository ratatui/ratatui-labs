# ratatui-labs

Experimental Ratatui labs-style APIs and prototype Ratatui work.

This crate is a Ratatui namespace reservation that can host small experiments
before they are ready to move into focused crates or the main Ratatui
repository. APIs in this crate are experimental and may change or disappear.

`ratatui-labs` is not intended to become a broad implementation crate. When an
experiment grows a clear concept boundary, prefer a focused crate with its own
crate-level documentation, examples, and validation. This crate can then provide
a short compatibility namespace for the experiment while the API is evaluated.

## Command Palette Experiment

The command palette experiment now lives in two concept-owned crates:

- `ratatui-action` for semantic action identity, metadata, availability, inputs, and
  invocations.
- `ratatui-command-palette` for palette state, filtering, selection, and event
  emission.

The palette does not own application state or execute application callbacks. It
consumes action metadata and emits invocation, preview, and lifecycle events
that the application handles.

```rust
use ratatui_labs::{
    action::spec::ActionSpec,
    command_palette::{event::PaletteEvent, state::PaletteState},
};

let actions = vec![ActionSpec::new("document.open", "Open document")];
let mut palette = PaletteState::new();
palette.open(&actions);

if let Some(PaletteEvent::Invoke(invocation)) = palette.accept(&actions) {
    assert_eq!(invocation.id().as_str(), "document.open");
}
```

Run the first interactive example with:

```sh
cargo run -p ratatui-command-palette --example command-palette
cargo run -p ratatui-command-palette --example command-palette -- --help
cargo run -p ratatui-command-palette --example command-palette -- --renderer split
```

Rendered validation for this example is captured by the repository Betamax tape:

```sh
just betamax
```

## Layout Experiment

The frame-local layout coordination experiment lives in `ratatui-layout`.
It records visible regions, focus targets, pointer targets, cursor requests,
viewport metadata, and scroll metrics produced by a render pass so applications
can route later input events without adopting a retained widget tree.

Start with `ratatui_layout::docs` when exploring the coordination model, or use
the umbrella re-export:

```rust
use ratatui_labs::layout::{
    frame::FrameSnapshot,
    pointer::{PointerTarget, PointerTargets},
};
use ratatui_core::layout::Rect;

let frame = FrameSnapshot::new(Rect::new(0, 0, 20, 1))
    .mouse(PointerTargets::new().target(PointerTarget::new("save", Rect::new(0, 0, 6, 1))));

assert_eq!(frame.route_position((2, 0)).unwrap().id, "save");
```

The crate points at the main Ratatui project rather than a separate
implementation repository:

- <https://github.com/ratatui/ratatui-labs>
- <https://docs.rs/ratatui/latest/ratatui/>

Documentation for this reservation crate is published at:

- <https://docs.rs/ratatui-labs>

## Related Crates And Overlap

This reservation may overlap with:

- `ratatui-unstable`, `ratatui-experimental`, and future unstable API policy.
- the experimental `ratatui-action` and `ratatui-command-palette` crates in this workspace.
- the experimental `ratatui-layout` crate in this workspace.

Any future implementation should coordinate with those crates or clearly explain
the difference before publishing a non-reservation release.

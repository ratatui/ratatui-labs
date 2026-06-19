# ratatui-labs

Experimental Ratatui labs-style APIs and prototype Ratatui work.

This crate is a Ratatui namespace reservation that can host small experiments
before they are ready to move into focused crates or the main Ratatui
repository. APIs in this crate are experimental and may change or disappear.

## Command Palette Experiment

The command palette experiment starts with two concept-owned modules:

- [`action`] for semantic action identity, metadata, availability, inputs, and
  invocations.
- [`command_palette`] for palette state, filtering, selection, and event
  emission.

The palette does not own application state or execute application callbacks.
It consumes action metadata and emits invocation or lifecycle events that the
application handles.

The crate points at the main Ratatui project rather than a separate
implementation repository:

- <https://github.com/ratatui/ratatui>
- <https://docs.rs/ratatui/latest/ratatui/>

Documentation for this reservation crate is published at:

- <https://docs.rs/ratatui-labs>

## Related Crates And Overlap

This reservation may overlap with:

- `ratatui-unstable`, `ratatui-experimental`, and future unstable API policy.
- possible future `ratatui-action` and `ratatui-command-palette` crates.

Any future implementation should coordinate with those crates or clearly explain
the difference before publishing a non-reservation release.

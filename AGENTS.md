# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace for experimental Ratatui crates and prototypes.

- `Cargo.toml` defines the workspace and shared package metadata.
- `crates/ratatui-labs/` is the initial placeholder crate copied from the reservation pattern.
- `crates/*/src/lib.rs` should contain library code or placeholder crate docs.
- `crates/*/README.md` is the public crate-facing documentation.

Add new experiments as separate crates under `crates/` when they may grow independently.

For early experiments inside an existing crate, organize modules around owned concepts instead of
putting all code in one file or broad helper buckets. Prefer named files such as `action.rs`,
`command_palette.rs`, and `command_palette/view.rs` over `mod.rs`, `utils.rs`, `common.rs`, or
`types.rs` unless local structure clearly justifies the alternative. Keep weak abstractions close to
their first use until multiple callers or a stable domain name prove they should move outward.

## Build, Test, and Development Commands

```sh
cargo +nightly fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
just validate
just betamax # when a Betamax tape exists or visible TUI behavior changed
markdownlint-cli2 README.md 'crates/**/*.md'
```

`cargo +nightly fmt --all` applies the repo rustfmt settings copied from Betamax. `cargo check`
validates all crates and targets without building tests. `cargo clippy` enforces warning-free code.
`cargo test` runs unit tests and doctests. `just validate` runs the local validation bundle.
`markdownlint-cli2` checks Markdown formatting with the repository config.

## Coding Style & Naming Conventions

Use Rust 2024 and the workspace lint settings. Unsafe code is forbidden. Prefer small, readable
modules and explicit names over premature abstractions. Crate names should use the `ratatui-*`
pattern, with directories matching package names, such as `crates/ratatui-labs`.

Markdown prose should wrap at 100 columns unless a table or URL makes that impractical.

## Testing Guidelines

Place unit tests next to the code they exercise. Prefer focused tests that document expected
behavior over broad placeholder tests. For new public APIs, include doctests or README examples when
the example helps users understand the crate.

Use Betamax for terminal-rendering validation when a change affects visible TUI behavior. The local
`just betamax` recipe should run the relevant tape and write rendered artifacts under
`target/betamax/`, especially screenshots, GIFs, and terminal state snapshots. Prefer Betamax tape
steps over PTY-only smoke tests for row selection, expansion/collapse behavior, status/title bars,
scrolling, colors, wrapping, keyboard interactions, and jj-rendered template output. Keep unit tests
for state and action behavior, but use Betamax as evidence that the actual terminal rendering and
interaction sequence work.

## Documentation Guidelines

Treat docs as part of the behavior contract. Document current behavior, not planned behavior, except
in explicitly marked PRDs, roadmaps, or issues. Keep README files as entry points and move deeper
design or reference material into named docs under `docs/`.

When public experimental APIs change, update nearby Rustdoc, crate README content, examples, and any
rendered Betamax evidence together. Examples should prove representative use, including ownership,
lifecycle, side effects, and integration shape where those matter.

## Commit & Pull Request Guidelines

Keep changes small and purpose-specific. Write commit summaries in the imperative mood, for example
`Add labs placeholder crate`. Pull requests should explain the experiment or reservation being
introduced, list validation commands run, and link relevant Ratatui issues or design notes when
available.

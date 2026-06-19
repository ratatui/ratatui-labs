# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust workspace for experimental Ratatui crates and prototypes.

- `Cargo.toml` defines the workspace and shared package metadata.
- `crates/ratatui-labs/` is the initial placeholder crate copied from the reservation pattern.
- `crates/*/src/lib.rs` should contain library code or placeholder crate docs.
- `crates/*/README.md` is the public crate-facing documentation.

Add new experiments as separate crates under `crates/` when they may grow independently.

## Build, Test, and Development Commands

```sh
cargo fmt --check
cargo check --workspace
cargo test --workspace
markdownlint-cli2 README.md 'crates/**/*.md'
```

`cargo fmt --check` verifies Rust formatting. `cargo check --workspace` validates all crates without
building tests. `cargo test --workspace` runs unit tests and doctests. `markdownlint-cli2` checks
Markdown formatting with the repository config.

## Coding Style & Naming Conventions

Use Rust 2024 and the workspace lint settings. Unsafe code is forbidden. Prefer small, readable
modules and explicit names over premature abstractions. Crate names should use the `ratatui-*`
pattern, with directories matching package names, such as `crates/ratatui-labs`.

Markdown prose should wrap at 100 columns unless a table or URL makes that impractical.

## Testing Guidelines

Place unit tests next to the code they exercise. Prefer focused tests that document expected
behavior over broad placeholder tests. For new public APIs, include doctests or README examples when
the example helps users understand the crate.

## Commit & Pull Request Guidelines

Keep changes small and purpose-specific. Write commit summaries in the imperative mood, for example
`Add labs placeholder crate`. Pull requests should explain the experiment or reservation being
introduced, list validation commands run, and link relevant Ratatui issues or design notes when
available.

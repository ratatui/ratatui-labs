# Ratatui Labs

Experimental Ratatui work lives here before it is ready to move into a focused crate or the main
Ratatui repository.

Initial crate placeholders follow the same shape as Ratatui namespace reservation crates. They can
grow into real experiments when there is a concrete design to test.

## Crates

- `crates/ratatui-labs` - placeholder for labs-style experiments and prototype Ratatui work.

## Validation

```sh
cargo +nightly fmt --all
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The repository also provides matching `just` recipes:

```sh
just validate
```

For experiments that change visible terminal behavior, use Betamax for rendered validation:

```sh
just betamax
```

The default recipe expects `tapes/jk-log-ui.tape`. Betamax tapes should render the real TUI flow and
write PNG, GIF, and state artifacts under `target/betamax/` for local inspection.

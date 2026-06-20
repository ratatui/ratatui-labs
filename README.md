# Ratatui Labs

Experimental Ratatui work lives here before it is ready to move into a focused crate or the main
Ratatui repository.

Initial crate placeholders follow the same shape as Ratatui namespace reservation crates. They can
grow into real experiments when there is a concrete design to test.

## Crates

- `crates/ratatui-action` - experimental semantic action identifiers, metadata, inputs, and
  invocation requests.
- `crates/ratatui-command-palette` - experimental command palette state, rendering, preview, and
  invocation behavior.
- `crates/ratatui-layout` - experimental frame-local UI coordination primitives for visible
  regions, focus targets, pointer targets, cursor requests, and scroll metadata.
- `crates/ratatui-labs` - umbrella crate for labs-style experiments and prototype Ratatui work.

## Examples

```sh
cargo run -p ratatui-command-palette --example command-palette
cargo run -p ratatui-command-palette --example command-palette -- --help
cargo run -p ratatui-command-palette --example command-palette -- --renderer split
cargo run -p ratatui-layout --example left_right_row
cargo run -p ratatui-layout --example frame_snapshot
```

## Docs

- [Command palette PRD](docs/prds/command-palette.md)
- [Architecture decision records](docs/adrs/)

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
just betamax tapes/command-palette.tape
```

The default recipe runs every tape under `tapes/`. Pass a tape path to run one scenario. Betamax
tapes should render the real TUI flow and write PNG, GIF, and state artifacts under
`target/betamax/` for local inspection. Set `BETAMAX_JOBS` to control default parallelism.

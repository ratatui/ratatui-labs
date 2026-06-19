set shell := ["sh", "-cu"]

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo +nightly fmt --all -- --check

check:
    cargo check --workspace --all-targets

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

lint-md:
    markdownlint-cli2 README.md AGENTS.md 'docs/**/*.md' 'crates/**/*.md'

validate: fmt-check check clippy test lint-md

betamax tape="tapes/jk-log-ui.tape":
    test -f "{{tape}}" || { echo "missing Betamax tape: {{tape}}" >&2; exit 1; }
    mkdir -p target/betamax
    cargo run --manifest-path ../betamax/Cargo.toml -- run "{{tape}}"

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

betamax tape="":
    mkdir -p target/betamax/renderers
    cargo build --manifest-path ../../betamax/Cargo.toml
    tapes="{{tape}}"; \
    if [ -n "$tapes" ]; then \
        test -f "$tapes" || { echo "missing Betamax tape: $tapes" >&2; exit 1; }; \
        ../../betamax/target/debug/betamax run "$tapes"; \
    else \
        jobs="${BETAMAX_JOBS:-4}"; \
        find tapes -name '*.tape' | sort | xargs -P "$jobs" -I{} \
            sh -c '../../betamax/target/debug/betamax run "$1"' sh {}; \
    fi

set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

fmt:
    cargo +nightly fmt --all

fmt-check:
    cargo +nightly fmt --all -- --check

check:
    cargo check --workspace --all-targets --all-features

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

clippy-beta:
    cargo +beta clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-targets --all-features

doc:
    cargo doc --workspace --all-features --no-deps

doc-test:
    cargo test --workspace --doc --all-features

package:
    cargo package --workspace --allow-dirty

dependency-policy:
    cargo deny check advisories licenses bans sources

feature-check:
    cargo hack check --workspace --lib --tests --feature-powerset

minimal-versions:
    cargo minimal-versions check --direct --workspace
    cargo update --workspace

semver-checks:
    cargo semver-checks --workspace

typos:
    typos

lint-md:
    markdownlint-cli2 README.md AGENTS.md 'docs/**/*.md' 'crates/**/*.md'

validate: fmt-check check clippy test doc doc-test lint-md

betamax tape="":
    mkdir -p target/betamax/renderers
    betamax="${BETAMAX_BIN:-}"; \
    if [ -z "$betamax" ]; then \
        if [ -x ../../betamax/target/debug/betamax ]; then \
            betamax="../../betamax/target/debug/betamax"; \
        else \
            cargo build --manifest-path ../../betamax/Cargo.toml; \
            betamax="../../betamax/target/debug/betamax"; \
        fi; \
    fi; \
    tapes="{{tape}}"; \
    if [ -n "$tapes" ]; then \
        test -f "$tapes" || { echo "missing Betamax tape: $tapes" >&2; exit 1; }; \
        REPO_DIR="$PWD" "$betamax" run "$tapes"; \
    else \
        jobs="${BETAMAX_JOBS:-4}"; \
        find tapes -name '*.tape' | sort | xargs -P "$jobs" -I{} \
            bash -euo pipefail -c 'REPO_DIR="$PWD" "$0" run "$1"' "$betamax" {}; \
    fi

release-check: validate dependency-policy feature-check minimal-versions semver-checks package betamax

# 0002 Action API Construction And Field Visibility

## Status

Accepted.

## Context

The initial action model can be represented as simple structs and enums. The main design question is
which types should expose public fields and which should use constructors, accessors, and builder
methods.

Rust public fields are ergonomic for literals, pattern matching, and direct updates. They are also a
compatibility commitment: changing storage, validating invariants, adding defaults, or hiding fields
later becomes a breaking change.

The action model includes invariant-bearing types:

- `ActionSpec` has identity, title, availability, search metadata, and required inputs.
- `ActionInvocation` carries the requested action, resolved arguments, and invocation source.

These types are central to cross-surface behavior. Their shape should be easy to evolve while the
experiment is still learning.

## Decision

Use private fields, constructors, accessors, and builder-style modifiers for public action and view
types that may need validation, storage changes, or compatibility-preserving evolution:

- `ActionSpec::new`
- `ActionSpec::with_description`
- `ActionSpec::with_category`
- `ActionSpec::with_keywords`
- `ActionSpec::with_input`
- `ActionSpec::with_availability`
- `ActionSpec::keywords` yields `&str` values rather than exposing the backing vector
- `ActionInvocation::new`
- `ActionInvocation::with_args`
- `ActionChoice::new`
- `ActionChoice::with_description`
- `PaletteRow` accessors
- `PaletteView` accessors

This follows the Rust API Guidelines direction for builders on complex values and future-proofing
public APIs. The experiment still keeps the concrete structs simple, but callers use methods rather
than depending on field layout.

## Consequences

`ActionSpec`, `ActionInvocation`, `ActionChoice`, `PaletteRow`, and `PaletteView` can add
validation, alternate storage, richer metadata, or source-specific behavior without forcing users to
update struct literals. Builder methods also keep examples readable and avoid requiring callers to
fill every optional field.

The cost is some verbosity. Callers cannot use struct update syntax or directly destructure these
types. Tests and examples must use accessors instead of field access.

## Alternatives Considered

### Public Fields Everywhere

This is the shortest API and feels natural for a data-first experiment. It becomes risky once the
types are exported from focused crates, because every public field becomes part of the compatibility
surface.

### Private Fields Everywhere

This maximizes future flexibility and is now the chosen shape for exported view data. The cost is
extra accessors, but that cost is small compared with the cost of changing public field layout
later.

### Typed Builders For Every Public Type

A separate builder type may become useful for larger specs, dynamic choices, or validation. It is
not currently necessary because `ActionSpec` has a small number of optional fields and by-value
builder methods are adequate.

## Revisit When

- `ActionSpec` construction needs fallible validation
- action metadata gains fields that make the builder chain unwieldy
- the crates move from labs experiment to stable publication
- callers need a controlled way to construct synthetic `PaletteView` values for renderer tests

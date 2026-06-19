# 0003 Owned Palette View Data

## Status

Accepted for the experiment.

## Context

`PaletteState::view` prepares data for renderers. The view can either borrow from the action list or
own the strings and rows it passes to renderers.

Borrowed view data can avoid allocation and make rendering cheaper. It also introduces lifetimes
into the renderer API and can expose storage decisions before the view model is stable.

Owned view data is easy to clone, snapshot, compare in tests, write into Betamax state artifacts,
and pass through renderers without lifetime complexity. That simplicity is useful while the palette
is still proving modes, previews, input collection, and renderer boundaries.

## Decision

Use owned `String` and `Vec` fields in `PaletteView` during the labs experiment:

```rust
pub struct PaletteView {
    prompt: String,
    query: String,
    rows: Vec<PaletteRow>,
    selected: Option<usize>,
    mode: PaletteMode,
}
```

Expose this data through accessors. Document the owned storage as an experiment-stage
simplification, not a stable performance decision.

Action metadata follows the same experiment-stage default: `ActionSpec` stores owned strings, while
accessors expose `&str` values or iterators. The owned form keeps construction and Betamax
inspection simple, and the accessor boundary leaves room to revisit `Cow<'static, str>` or richer
Ratatui text types before publication.

## Consequences

The current tests and Betamax workflow can inspect the exact render model without lifetime-heavy
fixtures. The renderer API stays simple, and the first implementation can optimize for clarity.

The cost is allocation on each view preparation. That may matter for large action lists, frequent
redraws, or slower terminals. Accessors reduce the compatibility cost of changing storage later,
but a stable crate should still revisit allocation before publication.

## Alternatives Considered

### Borrowed View Model

A borrowed view could use `&str`, slices, or `Cow` to reduce allocations. This is probably the right
direction if profiling shows view construction matters or if the renderer API stabilizes around a
clear lifetime story.

The downside is API complexity while the rest of the experiment is still changing.

### Accessor-Backed View Type

Private fields with accessors hide storage and preserve future flexibility. This is the current
shape because renderer authors do not need direct struct literals for normal use.

### Renderer Pulls Directly From `PaletteState`

This avoids a separate view model, but it couples rendering to state, matching, and input
collection. It would make alternate renderers harder to write and test.

## Revisit When

- Betamax state artifacts or tests need a different view shape
- a benchmark shows view allocation is meaningful
- the crate is prepared for stable publication
- another renderer needs a different view contract

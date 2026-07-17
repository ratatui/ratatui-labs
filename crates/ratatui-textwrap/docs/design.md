# Ratatui Textwrap Design Notes

These notes record the reasoning behind the current experiment and the follow-up ideas raised while
developing it. Sections describing future directions are questions to evaluate, not commitments or
part of the current API contract.

## Problem framing

Ratatui currently performs styled word wrapping inside `Paragraph` while it renders. That path is
hard to reuse in another widget, does not return the wrapped text, and repeats its work when a
caller needs both a line count and rendered output. Its reflow state machine also combines
grapheme measurement, whitespace policy, word assembly, overflow behavior, and line reconstruction.

The experiment separates two concerns:

1. Convert Ratatui's styled graphemes into input suitable for a line-breaking algorithm.
1. Materialize the algorithm's output as owned Ratatui `Line` and `Text` values.

This separation makes algorithm differences observable. It does not assume that a textwrap-backed
algorithm must reproduce every historical Paragraph edge case.

## Current decisions

### Owned materialization

`TextWrapper::wrap` allocates an owned `Text<'static>`. The caller can count, cache, scroll, or pass
that value to any widget. A borrowed iterator could avoid some allocations, but it would tie layout
to the source lifetime and make caching and widget composition harder to demonstrate in the first
API.

Width remains an argument to `wrap` rather than stored configuration. A caller can reuse one
wrapper as its area changes, while a cache key can include the source revision, width, and selected
algorithm explicitly.

### Algorithm selection

Line breaking is an explicit [`WrapAlgorithm`][wrap-algorithm] policy:

- `FirstFit` greedily fills lines and is the default.
- `OptimalFit` evaluates all break points with textwrap's default penalties.
- `ParagraphCompat { trim }` runs a maintained copy of Paragraph's reflow state machine.

`ParagraphCompat` is separate because Paragraph's streaming whitespace and overflow rules do not
fit textwrap's word-plus-following-whitespace fragment model. Keeping compatibility separate makes
the textwrap adapter easier to understand and lets applications choose whether historical fidelity
is valuable for a particular view.

The optimal-fit implementation falls back to first-fit when textwrap reports an infinite cost.
`TextWrapper::wrap` has no error channel, and the same fragments remain valid first-fit input, so
the fallback preserves deterministic useful output without adding a failure mode solely for an
optimizer limit.

[wrap-algorithm]: https://docs.rs/ratatui-textwrap/latest/ratatui_textwrap/enum.WrapAlgorithm.html

### Configuration API

`TextWrapper` is a small `Copy` value with a consuming `.algorithm(...)` method. A separate builder
type would duplicate a one-field configuration, while mutable setters would add a second
construction style without enabling another use case. The consuming method composes with Rust's
usual builder syntax and can grow with the configuration while it remains small.

The Paragraph `trim` boolean belongs only to `WrapAlgorithm::ParagraphCompat`. First-fit and
optimal-fit follow textwrap's fragment whitespace contract rather than presenting Paragraph's trim
policy as if it applied identically to every algorithm. This is why the original `.trim(bool)`
configuration became an algorithm-specific field.

Future options should first be evaluated as fields or variants of an owned Ratatui-facing policy.
The public API need not expose textwrap dependency types merely because an implementation delegates
to textwrap.

### Public boundary

`TextWrapper` is the whole-text entry point. The `algorithms` module exposes focused line-level
functions for custom widgets and processing pipelines. Styled pieces, fragments, pending queues,
and reconstruction helpers remain implementation details because they describe how the current
adapters work rather than what a caller needs to request.

This boundary addresses requests for reusable Paragraph reflow without making Paragraph's internal
state machine itself a general-purpose abstraction. The copied algorithm can evolve alongside its
compatibility tests while applications depend on owned wrapped lines.

### Graphemes, widths, and overflow

Shared conversion retains every grapheme and its Ratatui `CellWidth`; it does not discard content
based on the requested line width. The selected algorithm owns the decision about content that
cannot fit. This distinction keeps measurement and text conversion independent from wrapping
policy.

The textwrap algorithms retain an indivisible grapheme wider than the available area as an
overflowing line. No valid Ratatui grapheme boundary exists at which to split it. `ParagraphCompat`
omits the same grapheme because current Paragraph does so. Applications can therefore select the
compatibility behavior without forcing silent data loss on every algorithm.

## Characterization findings

The compatibility suite treats rendered buffers as the external evidence. It found that a styled
textwrap adapter can preserve text, line, and span styles while delegating line breaking, provided
that the adapter observes several constraints:

- A source span boundary inside a word is not a wrapping opportunity.
- Words wider than the line must be split at Ratatui grapheme boundaries before textwrap receives
  them because textwrap arranges complete fragments.
- Only the final split fragment inherits the original word's following whitespace.
- Ratatui must remain authoritative for terminal-cell width and whitespace classification. In
  particular, non-breaking and zero-width spaces do not follow `char::is_whitespace` in the same
  way.
- Adjacent output pieces with the same style can be coalesced after layout without changing
  rendered cells.
- Paragraph and textwrap differ most visibly around leading, trailing, repeated, whitespace-only,
  and differently styled whitespace.
- Control characters embedded in a manually constructed `Span` are filtered by Ratatui's styled
  grapheme iterator; separate `Line` values remain the hard-boundary representation.

These differences are reasons to keep both general algorithms and a compatibility algorithm. They
are not evidence that every new wrapping API should inherit Paragraph's behavior.

## Follow-up directions

### More line-breaking policy

textwrap offers more configuration than the first experiment exposes. Candidate additions include
custom optimal-fit penalties, hyphenation, different word separators or word splitters, and a
sequence of widths for hanging or shaped layouts. Each option needs a Ratatui-facing contract for
styled content and terminal cells before it belongs in the public API.

Indentation is related but not identical to separator policy. Useful forms may include continuation
indentation, alignment with the first non-whitespace cell, and a styled continuation prefix. A
prefix also changes the width available to the algorithm, so treating indentation as a final
rendering decoration would produce incorrect break points.

### Incremental and viewport-aware layout

Large editors and syntax-highlighted file previews may process only a viewport-sized source chunk.
Possible designs include a continuation token, a resumable line wrapper, or a persistent layout
object that can extend previously computed output. The design must define what happens when text
before or outside the viewport changes; wrapping only a subset of lines is not enough to guarantee
a stable scroll position.

A persistent layout type could also own caching. That would add invalidation, source identity, and
memory policy to the API, so the current crate leaves caching with the caller until those
requirements are clearer.

### Source and cursor mapping

Owned wrapped text answers where content renders but not where it came from. Editors, selectable
links, diagnostics, and mouse hit testing may need mappings between source byte or grapheme ranges
and wrapped rows and columns. Such mappings must account for filtered controls, zero-width
graphemes, omitted separator whitespace, compatibility-only omission, and any future inserted
hyphen or prefix.

Cursor mapping and source mapping should be designed together. Adding indexes after reconstruction
would lose decisions made while fragments are split and arranged.

### Ratatui integration and compatibility lifecycle

The experiment can inform a future Ratatui API on `Line` or `Text`, or an implementation used by
`Paragraph`, but it does not establish that integration yet. Adoption needs evidence that the
owned-output API composes with real widgets and that documented rendering changes are acceptable.

`ParagraphCompat` is copy-and-maintain code tied to a stated Ratatui behavior baseline. Future
releases need to decide whether to advance that baseline, name multiple compatibility policies, or
keep one stable historical policy. Silently changing the meaning of `ParagraphCompat` would defeat
the reason it exists.

### Performance evidence

Materializing owned text adds allocations but enables reuse across frames. Benchmarks become useful
when they compare a concrete application decision, such as repeated Paragraph wrapping against a
cached owned result, first-fit against optimal-fit, or full-document processing against viewport
processing. Microbenchmarks of isolated helpers would not answer those integration questions.

## Feedback questions

The experiment is most useful when reports identify the source text, styles, width, selected
algorithm, and expected rendered cells. Broader design feedback should address questions such as:

- Is an owned `Text` the right reusable result, or does a real use case require a borrowed or lazy
  view?
- Which Paragraph differences require compatibility, and which are acceptable behavior changes?
- Which textwrap configuration is needed before the public options should grow?
- What state must survive between viewport-sized wrapping calls?
- Which source and cursor mapping operations need to be efficient?

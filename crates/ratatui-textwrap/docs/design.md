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

Two public entry points serve large or changing text:

- `TextWrapper::tail` walks back from the last line and wraps only the rows in view. It needs no
  state, so it suits logs that stick to the bottom and costs nothing extra after a resize.
- `WrappedText` owns a `Text` and the row count of each of its lines at one width. Reads go through
  `Deref`; every mutation goes through `line_mut`, `lines_mut`, `push_line`, `extend`, `splice`,
  or `text_mut`, each of which records what may have changed, so the counts cannot fall out of
  step and several edits between frames are counted once.

Row positions come from a `WrappedLayout`, a read-only view that borrows the text, so the text
cannot change while the view exists. Getting one is the explicit step from "may have pending
edits" to "up to date":

- `layout(&mut self)` recounts whatever is pending and returns the view.
- `try_layout(&self)` returns the view only if nothing is pending, for drawing code that holds a
  shared reference.
- `layout_bounded(&mut self, max_lines)` recounts at most `max_lines` per call and returns either
  the view or a `PendingLayout` that reports progress and can still draw the bottom rows, so a
  full recount after `set_width` or `text_mut` can be spread across frames.

The counts themselves live in a crate-internal `RowIndex`. A public index over caller-owned
storage (an editor's rope, a preview's own `Vec<Line>`) would have to trust callers to report
every edit, and removing it later would be a breaking change; keeping it internal leaves that
door open until a real caller needs it.

The counts live in chunks of at most 512 lines with running totals per chunk. Lookups are
O(log n); edits, inserts, and removals anywhere touch one or two chunks plus the per-chunk totals.
Callers decide how the text changes, so the structure avoids assuming appends. A counted B-tree
would make inserts O(log n) in theory, but the chunked form stays in microseconds at realistic
sizes with much less code, and it is private, so it can change.

Short lines are rewrapped whole when they are visible. Lines longer than 16 rows also keep the byte
bounds of every row, recorded in the same pass that counts them, because every row this crate
produces is a contiguous run of the line's graphemes (Paragraph compatibility additionally omits
graphemes wider than the row, which is re-applied). Drawing any row of a long line then scans only
that row, so a 500 KB single line costs about a screenful per frame. A general row cache was
considered and rejected: on highlighted code the uncached frame is within about 5 µs of a fully
cached copy, and the larger gap on logs came from the very long lines that bounds now cover.
`TextWrapper::tail` keeps no state and still rescans a long last line in full.

#### Row producers

An algorithm takes part in the index by lending out rows one at a time as lifetime-free `Row`
records: byte ranges into the source spans plus a width. Producers implement a private trait
(`reset`, `next_row`, `count_remaining`), so counting builds no spans and visible rows borrow
from the source.

- `ParagraphRows` runs the unchanged Paragraph state machine over grapheme records. Upstream
  `WordWrapper` has the same shape (whole line in, rows out), so the compatibility code still
  diffs cleanly against it.
- `FirstFitRows` and `OptimalFitRows` group graphemes into the same fragments that the owned
  textwrap path builds, as index ranges. Those fragments implement textwrap's `Fragment` trait, so
  textwrap's own `wrap_first_fit` rule and `wrap_optimal_fit` algorithm choose the breaks; no
  upstream change was needed. First-fit decides one row at a time. Optimal fit minimizes a cost
  over the whole line, so its producer computes every break of a line on `reset` and then lends
  rows out. Both match the owned output cell for cell on the whole compatibility corpus, which
  shows the row and index abstractions are not specific to Paragraph.

A future better-than-Paragraph policy should be a new `WrapAlgorithm` variant with its own
producer, not a change to `ParagraphCompat`.

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

`benches/scroll.rs` compares full frame costs on generated logs (styled spans, a mix of CJK words,
5% very long lines) and on syntax-highlighted Rust source, at 100 columns and 50 rows. Text is sized
by wrapped rows, up to 65,535 so `Paragraph` can scroll to every position with its `u16` offset.
`cargo bench -p ratatui-textwrap --bench scroll` runs a quick pass over the smallest and largest
size; `SCROLL_BENCH=full` runs all four sizes with longer measurement. `benches/report.py` renders
the results as a shaded HTML table.

The implementations compared:

- **Paragraph**: Ratatui's `Paragraph` with `Wrap` and `scroll`, the widget most apps use today. It
  wraps from the first line on every render, so its cost grows with the scroll position, and a
  bottom-stuck view also calls `line_count`, which wraps everything once more.
- **yazi `WordWrapper`**: yazi's vendored copy of Paragraph's wrapper (`benches/vendor`), driven the
  way yazi's preview drives it: wrap from the top, skip rows, keep a screenful. It is the baseline
  for the issue that motivated this work.
- **Previous owned cache**: this crate before index-based wrapping (`benches/old`).
  `TextWrapper::wrap` copies the whole text into owned wrapped lines once; each frame then slices
  the cached rows. It is the fastest per frame and the most expensive to build, hold, and update.
- **`TextWrapper::tail`**: no state; walks back from the last line to fill the view.
- **`WrappedText`**: owns the text and per-line row counts, recounting only what each edit touched;
  frames wrap only the visible lines through its `WrappedLayout`.

Medians at 65,535 rows (27,304 log lines, or the equivalent amount of code) on an Apple M2 Max:

| Scenario                           | Paragraph | Previous owned cache | `WrappedText`                 |
| ---------------------------------- | --------- | -------------------- | ----------------------------- |
| Log frame scrolled to bottom       | 142 ms    | 135 µs               | 205 µs (216 µs with `tail`)   |
| Log frame scrolled to middle       | 69 ms     | 149 µs               | 238 µs                        |
| Append a line, stick to bottom     | 280 ms    | 154 µs               | 214 µs                        |
| Edit a middle line, draw bottom    | 277 ms    | 360 ms (rewrap)      | 212 µs                        |
| Insert a middle line, draw bottom  | 278 ms    | 360 ms (rewrap)      | 242 µs                        |
| Code frame scrolled to bottom      | 52 ms     | 64 µs                | 69 µs                         |
| Inside one 500 KB line             | 19 ms     | 130 µs               | 139 µs (4.6 ms with `tail`)   |
| Count every row (build, resize)    | 138 ms    | 360 ms               | 95 ms                         |
| First-fit: count every row         | n/a       | 423 ms (owned lines) | 85 ms                         |
| Optimal-fit: count every row       | n/a       | 512 ms (owned lines) | 171 ms                        |
| Retained memory (text is 10.6 MiB) | n/a       | 18.2 MiB             | 213 KiB plus long-line bounds |

Paragraph and yazi grow linearly with the scroll position and the text length; yazi tracks
Paragraph within about 20%. `WrappedText` stays flat across sizes. Counting every row (after a
resize or `text_mut`) is the only operation still above a 16 ms frame; `layout_bounded` spreads it
across frames while `tail` keeps drawing. About 130 µs of every log frame is rendering the visible
rows. The previous owned cache stays 5 to 90 µs faster per unchanged frame because it has already
wrapped everything, at the cost of a full copy of the text and a full rewrap on any edit or resize.
Long-line bounds add 32 bytes per row of each line longer than 16 rows. Paragraph's append, edit,
and insert times exclude building the `Paragraph` from the changed text.

### Decision log

Decisions made while building the scrolling support, with the alternatives considered:

- **Wrap only visible rows, using per-line row counts.** Every existing path (Paragraph, yazi, the
  owned cache) does work proportional to the text or the scroll position. Counting rows per source
  line answers "which line holds row N" and lets edits touch only the lines they change.
- **Lifetime-free row records.** Rows are byte ranges into the source spans, so counting builds no
  spans and visible rows borrow from the source. Owned per-row copies were the alternative; they
  cost an 18 MiB copy for 10.6 MiB of text.
- **Keep Paragraph's decision logic verbatim.** `ParagraphRows` moves the state machine from owned
  strings to grapheme indices and from "whole line" to "one row at a time", without changing any
  break decision. It is checked against Paragraph's rendering on the compatibility corpus and on
  every scroll window. A better policy belongs in a new `WrapAlgorithm` variant.
- **Producers behind a private trait, proven with a second and third algorithm.** First-fit and
  optimal-fit producers reuse textwrap's own algorithms through its `Fragment` trait and match
  the owned output exactly, so the abstraction is not specific to Paragraph and no upstream change
  was needed. An iterator-based first-fit upstream would help sub-line streaming later.
- **Chunked counts over a Fenwick tree or counted B-tree.** A Fenwick tree made edits O(log n) but
  needed a rebuild for mid-text inserts. Callers decide how text changes, so the structure must not
  assume appends. 512-line chunks keep every edit in microseconds with plain code; a counted B-tree
  would be O(log n) on paper for more code and no measured gain. It is private, so it can change.
- **`WrappedText` owns the text; `RowIndex` stays crate-internal.** An index over caller-owned text
  can fall out of step if the caller forgets to report an edit. Owning the text rules that out.
  Mutable access is plain `&mut` at three granularities (`line_mut`, `lines_mut`, `text_mut`) plus
  `Vec`-style structural edits, recorded and recounted lazily, so there is no guard to forget or
  leak. A public index (or `#[doc(hidden)]`, which is still public API) would be a one-way door;
  it can be added later, designed around a real caller with its own storage.
- **`WrappedLayout` as an explicit "up to date" step.** Queries need current counts. Rather than
  `&mut self` on every query, `layout` recounts once and returns a borrowed view whose queries take
  `&self`; the borrow prevents edits while the view exists. `try_layout` serves drawing code that
  holds `&self`, and `layout_bounded` returns either the view or a `PendingLayout`, like
  `binary_search` returning `Result`, so a large recount can be spread across frames.
- **Long-line row bounds instead of a row cache.** A general row cache would save about 5 µs per
  frame on code; the larger gap on logs came from a few very long lines rescanned in full. Lines
  over 16 rows keep each row's byte bounds, recorded in the same counting pass, so drawing inside a
  500 KB line drops from 4.6 ms to 139 µs while memory stays bounded by the rows of long lines.
- **Measurement before mechanism.** Each step was benchmarked against Paragraph, yazi's wrapper,
  and this crate's previous code in one process. The benchmark caught a 30% regression from lost
  inlining across modules and another from counting long lines twice; both were fixed.

### Possible next steps

- Feed real yazi output (syntect-highlighted previews, ANSI command output) through
  `WrappedText` in yazi to check the API shape against its scrolling, cancellation, and Lua
  `line_count` needs.
- Decide whether a public index over caller-owned storage is needed, with an editor or yazi's own
  line buffer as the design input.
- Replace stored long-line bounds with resumable producer state every few rows if memory for very
  long single-line files matters (a 1 MB line stores about 10,000 bounds, about 320 KiB).
- Propose an iterator-based first-fit to textwrap if sub-line streaming becomes useful.
- Track the `usize` scroll offset change in Ratatui, which removes Paragraph's 65,535-row limit
  used to size this benchmark.

## Feedback questions

The experiment is most useful when reports identify the source text, styles, width, selected
algorithm, and expected rendered cells. Broader design feedback should address questions such as:

- Is `WrappedText` plus `tail` the right lazy view, or do real users need an index over their own
  text storage, or sub-line position mapping?
- Which Paragraph differences require compatibility, and which are acceptable behavior changes?
- Which textwrap configuration is needed before the public options should grow?
- What state must survive between viewport-sized wrapping calls?
- Which source and cursor mapping operations need to be efficient?

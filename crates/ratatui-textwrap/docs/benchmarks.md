# Text Wrapping Benchmarks

These benchmarks compare application workflows rather than isolated wrapping helpers. They answer
when owned materialization pays for its allocations, when native `Paragraph` avoids full-document
work by stopping at the viewport, and how first-fit and optimal-fit change the cost.

## Compared implementations

- `paragraph-native` stores the source in `Paragraph` and uses `Wrap { trim: true }`.
- `paragraph-compat` materializes with `ParagraphCompat { trim: true }`, then renders without
  wrapping.
- `first-fit` and `optimal-fit` materialize with textwrap's two line-breaking algorithms, then
  render without wrapping.

Only `paragraph-compat` is expected to produce the same wrapped cells as native `Paragraph`.
First-fit and optimal-fit intentionally retain their documented whitespace and overflow behavior.

Cold materialized workflows include cloning the source `Text`. `TextWrapper::wrap` consumes its
input, so retaining the source for later widths requires that clone. Cached workflows construct and
retain an unwrapped `Paragraph` from the materialized result before measurement.

## Corpus and viewport

The core corpus uses the fixed seed `0x5EED_7E57_CAFE_BABE` and approximately 4 KiB, 64 KiB, and
1 MiB of text. Logical lines contain prose-like words, repeated whitespace, style changes, span
boundaries inside some words, combining characters, and occasional wide graphemes. Generation and
fixture validation happen before Criterion starts measuring.

The normal viewport is 200×50 cells. Resize sessions repeat widths 120, 160, 200, 240, and 280 at a
constant height of 50 rows. Sessions contain 60 frames.

The 4 KiB and 64 KiB fixtures participate in multi-frame core sessions. Single-operation and
viewport groups also include 1 MiB. The opt-in stress target adds 1 MiB sessions, a 4 MiB deep
scroll, and 64 KiB long-line, whitespace-heavy, Unicode-heavy, and unbreakable inputs.

## Workloads

- **`wrap-or-count`:** native fully wraps to count lines; materialized algorithms clone and wrap,
  then read `Text::lines.len()`.
- **`fresh-render`:** native wraps only far enough to fill 50 rows; materialized algorithms wrap
  the full input, then render 50 rows.
- **`count-then-render`:** native fully wraps to count, then wraps again while rendering;
  materialized algorithms wrap once and count the unwrapped `Paragraph` in constant time.
- **`render-then-count`:** native renders the viewport, then fully wraps to count; materialized
  algorithms wrap once and count in constant time.
- **`same-width-amortized-60-frames`:** native wraps every frame; materialized algorithms wrap once
  inside the measured session and reuse the result.
- **`same-width-cached-60-frames`:** native wraps every frame; materialized algorithms use a result
  prepared before measurement.
- **`resize-recomputed-60-frames`:** native rewraps the viewport at each width; materialized
  algorithms wrap the full input at every width change.
- **`resize-cached-60-frames`:** native rewraps the viewport at each width; materialized algorithms
  reuse results cached by width before measurement.
- **`viewport-cold`:** native wraps through the requested scroll row and 50 visible rows;
  materialized algorithms wrap the full input before scrolling.
- **`viewport-cached`:** native still wraps through the requested scroll row; materialized
  algorithms skip directly through pre-materialized lines.

Buffers, caches, scroll offsets, and fixture generation are outside timed regions unless the group
explicitly names cold materialization or amortized wrapping. Full-input groups report bytes per
second; session groups report frames per second.

## Running

Run the core suite from the workspace root:

```console
cargo bench -p ratatui-textwrap --bench textwrap-workflows
```

Criterion accepts a benchmark-name filter:

```console
cargo bench -p ratatui-textwrap --bench textwrap-workflows -- wrap-or-count
```

Compile and execute every core benchmark once without measuring it:

```console
cargo bench -p ratatui-textwrap --bench textwrap-workflows -- --test
```

Run the expensive stress suite separately:

```console
cargo bench -p ratatui-textwrap --bench textwrap-stress
```

The allocation diagnostic is deliberately separate from Criterion wall-time measurement. It uses
an instrumented system allocator and writes one CSV row per 64 KiB workflow and implementation:

```console
cargo bench -p ratatui-textwrap --bench textwrap-allocations -- \
  target/textwrap-allocations.csv
```

Regenerate the Markdown result tables from Criterion's stable raw CSV samples:

```console
python3 crates/ratatui-textwrap/scripts/summarize-benchmarks.py \
  --criterion-dir target/criterion \
  --allocations target/textwrap-allocations.csv \
  --sampling "Criterion defaults" \
  --output crates/ratatui-textwrap/docs/benchmark-results.md
```

The report script calculates the median measured time per iteration. For 60-frame groups it also
shows time per frame. Ratios compare each implementation with `paragraph-native` for the same group
and input.

## Tracking changes

Raw Criterion results remain under the ignored `target/` directory. Save a named baseline before
changing an implementation, then compare without overwriting it:

```console
cargo bench -p ratatui-textwrap --bench textwrap-workflows -- --save-baseline main
cargo bench -p ratatui-textwrap --bench textwrap-workflows -- --baseline main
```

Baselines are machine-local. Do not compare absolute timings collected on different hardware or
under materially different system load. The checked-in [benchmark results](benchmark-results.md)
include their environment and revision so they serve as a reference, not a regression threshold.

To profile one workload without Criterion analysis or saved results:

```console
cargo bench -p ratatui-textwrap --bench textwrap-workflows -- \
  same-width-amortized --profile-time 10
```

## Validation

Fixture preflight checks run before measurement. They verify deterministic size bounds, matching
native and paragraph-compatible line counts, matching native and paragraph-compatible buffers at
the start, middle, and end viewports, repeatable output for the textwrap algorithms, and scroll
offsets that fit `Paragraph`'s `u16` API.

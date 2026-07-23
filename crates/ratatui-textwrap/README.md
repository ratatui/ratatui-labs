# Ratatui Textwrap

`ratatui-textwrap` adapts the [`textwrap`][textwrap] crate's first-fit and optimal-fit line-breaking
algorithms to styled Ratatui text. It prepares textwrap fragments from Ratatui graphemes, then
rebuilds owned Ratatui lines while preserving their styles. A separate `ParagraphCompat` mode
reproduces Ratatui Paragraph's reflow behavior.

The crate accepts strings, spans, lines, and other values that convert into `Text`, then returns an
owned `Text<'static>` that can be cached or passed to another widget.

## Usage

```rust
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui_textwrap::{TextWrapper, WrapAlgorithm};

let line = Line::from(vec!["Styled ".blue(), "text can wrap across spans".bold()]);
let wrapping = TextWrapper::new().algorithm(WrapAlgorithm::OptimalFit);
let wrapped = wrapping.wrap(line, 20);
let paragraph = Paragraph::new(wrapped);
```

Wrapping allocates the owned `wrapped` value. Keep it while the source, algorithm, and width remain
unchanged. Width is supplied to each call, so one `TextWrapper` configuration can serve a resizable
view.

## Algorithm choice

| Algorithm         | Behavior                                                       | Use when                                                    |
| ----------------- | -------------------------------------------------------------- | ----------------------------------------------------------- |
| `FirstFit`        | Greedily fills lines; this is the default.                     | Predictable textwrap behavior is sufficient.                |
| `OptimalFit`      | Scores every break point with textwrap's default penalties.    | Line balance matters more than greedy placement.            |
| `ParagraphCompat` | Runs a copy of Ratatui Paragraph's reflow algorithm.           | Ratatui 0.30.2 behavior must be reproduced.                 |

First-fit and optimal-fit model each word together with its following whitespace, following
textwrap's [fragment contract]. A source `Span` boundary does not create a break inside a word.
Overlong words are split at Ratatui grapheme boundaries before textwrap chooses lines. Separator
whitespace after the final fragment on a generated line is omitted.

`ParagraphCompat` also accepts Paragraph's `trim` policy:

```rust
use ratatui_textwrap::{TextWrapper, WrapAlgorithm};

let wrapping = TextWrapper::new()
    .algorithm(WrapAlgorithm::ParagraphCompat { trim: true });
let wrapped = wrapping.wrap("alpha beta", 8);
```

Custom widgets can use the same implementations one line at a time:

```rust
use ratatui::text::Line;
use ratatui_textwrap::algorithms::paragraph;

let source = Line::from("alpha beta");
let wrapped = paragraph::wrap_line(&source, 8, true);
```

The `algorithms::paragraph` module exposes the copied compatibility behavior;
`algorithms::textwrap` exposes first-fit and optimal-fit. These functions return owned lines but do
not retain state between calls. Use `TextWrapper` when wrapping a complete `Text` so its top-level
style and alignment are carried into the result.

The copied implementation uses Ratatui 0.30.2 as its source baseline. Compatibility tests render it
against the compatible Ratatui release selected for development. The same tests record where
first-fit differs from Paragraph and where optimal-fit differs from first-fit. Current Paragraph
differences center on leading, trailing, repeated, whitespace-only, and styled whitespace.

## Text behavior

Each entry in `Text::lines` is an independent wrapping boundary. Generated lines preserve the
source text, line, and span styles and alignment. Adjacent output spans with equal styles may be
coalesced, so callers should rely on rendered cells rather than the original span partition.

A width of zero returns no lines while retaining top-level text style and alignment. Control
characters embedded directly in a manually constructed `Span` are filtered by Ratatui's
styled-grapheme iterator; use separate `Line` values for hard boundaries.

First-fit and optimal-fit preserve a grapheme wider than the requested width as an indivisible,
overflowing line. `ParagraphCompat` omits that grapheme, matching Paragraph's historical behavior.

## Scope

`ratatui-textwrap` is an experiment, and its API may change while the configuration and
compatibility surface are evaluated. It does not replace `Paragraph::wrap`, expose textwrap types,
or provide stateful incremental wrapping, lazy layout, source-position mapping, cursor mapping,
indentation, hyphenation, or custom optimal-fit penalties.

The [design notes](docs/design.md) record why the current API has this shape, what the Paragraph
comparison taught us, and which follow-up ideas remain open questions rather than promised
features.

## Benchmarks

The benchmark suite compares native `Paragraph` wrapping with paragraph-compatible, first-fit, and
optimal-fit materialization. It covers line counting, rendering, repeated cached rendering,
terminal resizing, and scrolled viewports using deterministic styled inputs.

See the [benchmark guide](docs/benchmarks.md) for the workload definitions and commands. The
[latest checked-in results](docs/benchmark-results.md) record one machine-specific reference run;
use Criterion baselines for before-and-after comparisons on the same machine.

The implementation follows textwrap's [fragment contract], [first-fit source], and [optimal-fit
source]. `ParagraphCompat` follows Ratatui's [WordWrapper source] and [Paragraph integration].

[first-fit source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms.rs#L336-L367
[fragment contract]: https://docs.rs/textwrap/0.16/textwrap/core/trait.Fragment.html
[optimal-fit source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms/optimal_fit.rs#L302-L381
[Paragraph integration]: https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui-widgets/src/paragraph.rs#L329-L355
[textwrap]: https://docs.rs/textwrap/0.16/textwrap/
[WordWrapper source]: https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui-widgets/src/reflow.rs#L29-L273

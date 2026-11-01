//! Runs textwrap's line-breaking algorithms on one styled Ratatui line.
//!
//! [`crate::algorithms::textwrap::wrap_first_fit`] and
//! [`crate::algorithms::textwrap::wrap_optimal_fit`] are public line-level entry points. They
//! receive a Ratatui line, prepare fragments whose widths and break points come from Ratatui
//! graphemes, and rebuild the selected slices as owned lines. Prefer [`crate::TextWrapper`] when
//! the input is a complete [`Text`](ratatui_core::text::Text) and its top-level metadata must also
//! be preserved. Reconstruction keeps whitespace between fragments on the same line and drops
//! whitespace after the final fragment, as required by textwrap's [`Fragment`] contract.
//! A grapheme wider than the requested width remains intact as an overflowing fragment because no
//! valid grapheme boundary exists at which to split it.
//!
//! Fragment construction lives in the sibling `fragment` module. Paragraph compatibility has its
//! own streaming algorithm because it does not follow textwrap's fragment model.
//!
//! [line-breaking overview]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/index.html
//! [`Fragment`]: https://docs.rs/textwrap/0.16/textwrap/core/trait.Fragment.html

use std::ops::Range;

use ::textwrap::core::Fragment;
use ::textwrap::wrap_algorithms::{
    Penalties, wrap_first_fit as textwrap_first_fit, wrap_optimal_fit as textwrap_optimal_fit,
};
use ratatui_core::text::{Line, Span};

use super::fragment::{self, StyledFragment};
use super::grapheme::{self, Grapheme};
use super::row::{Row, RowBounds, Segment};

/// Wraps one source line with textwrap's greedy first-fit algorithm.
///
/// A fragment is one word plus its following whitespace. First-fit greedily adds the next fragment
/// while it fits. Whitespace contributes to the fit between words but is omitted when its fragment
/// ends an output line. The upstream [documentation] and [source] define the generic algorithm;
/// this adapter supplies Ratatui graphemes and cell widths.
///
/// Every returned line copies `source` style and alignment and owns its span strings. A positive
/// width produces at least one line; a zero width produces no lines. The function allocates styled
/// fragments and returned line and span collections. Calls are independent.
///
/// [documentation]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/fn.wrap_first_fit.html
/// [source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms.rs#L336-L367
#[must_use]
pub fn wrap_first_fit(source: &Line<'_>, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let fragments = fragment::from_line(source, width);
    let line_widths = [f64::from(width)];
    let wrapped = textwrap_first_fit(&fragments, &line_widths);
    reconstruct_lines(source, wrapped)
}

/// Wraps one source line with textwrap's penalty-based optimal-fit algorithm.
///
/// This uses the same fragments and reconstruction rules as [`wrap_first_fit`] and passes
/// textwrap's unchanged `Penalties::new()` value. The cost accounts for line count, squared unused
/// width, overflow width, and a short final single-word line. The optimizer can therefore prefer a
/// small overflow to a poorly balanced break. Hyphen penalties have no effect because this adapter
/// does not hyphenate or report a penalty width. The upstream [documentation], [penalties], and
/// [source] define the complete generic cost calculation.
///
/// Every returned line copies `source` style and alignment and owns its span strings. A positive
/// width produces at least one line; a zero width produces no lines. The function allocates styled
/// fragments, optimization state, and returned line and span collections. Calls are independent.
/// If textwrap reports an infinite cost, this function uses first-fit so the public wrapping API
/// remains infallible and deterministic.
///
/// [documentation]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/fn.wrap_optimal_fit.html
/// [penalties]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/struct.Penalties.html
/// [source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/wrap_algorithms/optimal_fit.rs#L302-L381
#[must_use]
pub fn wrap_optimal_fit(source: &Line<'_>, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let fragments = fragment::from_line(source, width);
    let line_widths = [f64::from(width)];

    // `wrap_optimal_fit` reports `OverflowError` when its accumulated cost becomes infinite. The
    // fragments remain valid input to first-fit, and `TextWrapper::wrap` has no error channel, so
    // falling back preserves useful deterministic output if future penalties or inputs reach that
    // limit.
    let wrapped = textwrap_optimal_fit(&fragments, &line_widths, &Penalties::new())
        .unwrap_or_else(|_| textwrap_first_fit(&fragments, &line_widths));
    reconstruct_lines(source, wrapped)
}

/// Rebuilds all fragment slices selected for one source line.
fn reconstruct_lines(source: &Line<'_>, wrapped: Vec<&[StyledFragment]>) -> Vec<Line<'static>> {
    wrapped
        .into_iter()
        .map(|fragments| reconstruct_line(source, fragments))
        .collect()
}

/// Reconstructs one textwrap-selected fragment slice as an owned Ratatui line.
///
/// Keeps separator whitespace between fragments and drops it after the final fragment.
fn reconstruct_line(source: &Line<'_>, fragments: &[StyledFragment]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (index, fragment) in fragments.iter().enumerate() {
        fragment.append_word_to(&mut spans);
        if index + 1 != fragments.len() {
            fragment.append_whitespace_to(&mut spans);
        }
    }
    Line {
        style: source.style,
        alignment: source.alignment,
        spans,
    }
}

/// Reusable first-fit row producer that lends out rows as source byte ranges.
///
/// Produces exactly the rows of [`wrap_first_fit`] without copying span content: it groups the
/// line's graphemes into the same word-and-trailing-whitespace fragments, splits overlong words at
/// grapheme boundaries, and runs textwrap's first-fit rule one row at a time. Because each row is a
/// contiguous run of graphemes, rows are cheap to lend, count, or skip.
///
/// ```
/// use ratatui::text::Line;
/// use ratatui_textwrap::algorithms::textwrap::FirstFitRows;
///
/// let source = Line::from("alpha beta");
/// let mut rows = FirstFitRows::new();
/// rows.reset(&source, 5);
/// assert_eq!(
///     rows.next_row().unwrap().to_line(&source).to_string(),
///     "alpha"
/// );
/// assert_eq!(
///     rows.next_row().unwrap().to_line(&source).to_string(),
///     "beta"
/// );
/// assert!(rows.next_row().is_none());
/// ```
#[derive(Debug, Clone, Default)]
pub struct FirstFitRows {
    /// Fragments of the current source line.
    line: FragmentLine,
    /// Index of the first fragment of the next row.
    next: usize,
    /// Whether every row has been produced.
    done: bool,
    /// Rows lent out so far.
    output: RowOutput,
}

impl FirstFitRows {
    /// Creates an idle producer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            done: true,
            ..Self::default()
        }
    }

    /// Starts wrapping `source` at `width` cells, discarding any unfinished line.
    ///
    /// A zero width yields no rows; an empty line yields one empty row.
    pub fn reset(&mut self, source: &Line<'_>, width: u16) {
        self.done = !self.line.reset(source, width);
        self.next = 0;
    }

    /// Counts the rows remaining in the current line without lending them out.
    pub fn count_remaining(&mut self) -> usize {
        self.output.record = false;
        let mut count = 0;
        while self.next_row().is_some() {
            count += 1;
        }
        self.output.record = true;
        count
    }

    /// Returns the bounds of the row most recently produced.
    pub(crate) const fn row_bounds(&self) -> RowBounds {
        self.output.bounds
    }

    /// Sets whether produced rows record their segments; bounds are recorded either way.
    pub(crate) const fn set_record(&mut self, record: bool) {
        self.output.record = record;
    }

    /// Produces the next row of the current line, or `None` once the line is exhausted.
    pub fn next_row(&mut self) -> Option<Row<'_>> {
        if self.done {
            return None;
        }
        // textwrap's first-fit rule: a fragment starts a new row when its word does not fit after
        // the widths of the words and whitespace already placed, unless the row is empty.
        let fragments = &self.line.fragments;
        let start = self.next;
        let mut used = 0;
        let mut end = start;
        while let Some(fragment) = fragments.get(end) {
            if used + fragment.word_width > self.line.width && end > start {
                break;
            }
            used += fragment.word_width + fragment.whitespace_width;
            end += 1;
        }
        self.next = end;
        self.done = end >= fragments.len();
        Some(self.output.fill(&self.line, start..end))
    }
}

/// Reusable optimal-fit row producer that lends out rows as source byte ranges.
///
/// Produces exactly the rows of [`wrap_optimal_fit`] without copying span content. Optimal fit
/// chooses every break of a line together, so [`Self::reset`] runs textwrap's algorithm over the
/// line's fragments once; rows are then lent out one at a time like the other producers.
///
/// ```
/// use ratatui::text::Line;
/// use ratatui_textwrap::algorithms::textwrap::OptimalFitRows;
///
/// let source = Line::from("aaa bb cc ddddd");
/// let mut rows = OptimalFitRows::new();
/// rows.reset(&source, 6);
/// let rows: Vec<String> =
///     std::iter::from_fn(|| rows.next_row().map(|row| row.to_line(&source).to_string()))
///         .collect();
/// assert_eq!(rows, ["aaa", "bb cc", "ddddd"]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OptimalFitRows {
    /// Fragments of the current source line.
    line: FragmentLine,
    /// End fragment index of each row, in order.
    breaks: Vec<usize>,
    /// Index into `breaks` of the next row.
    next: usize,
    /// Rows lent out so far.
    output: RowOutput,
}

impl OptimalFitRows {
    /// Creates an idle producer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts wrapping `source` at `width` cells, discarding any unfinished line.
    ///
    /// A zero width yields no rows; an empty line yields one empty row.
    pub fn reset(&mut self, source: &Line<'_>, width: u16) {
        self.breaks.clear();
        self.next = 0;
        if !self.line.reset(source, width) {
            return;
        }
        let fragments = &self.line.fragments;
        let line_widths = [f64::from(width)];
        // Same fallback as `wrap_optimal_fit` in this module.
        let rows = textwrap_optimal_fit(fragments, &line_widths, &Penalties::new())
            .unwrap_or_else(|_| textwrap_first_fit(fragments, &line_widths));
        let mut end = 0;
        for row in rows {
            end += row.len();
            self.breaks.push(end);
        }
    }

    /// Counts the rows remaining in the current line without lending them out.
    pub fn count_remaining(&mut self) -> usize {
        let count = self.breaks.len() - self.next;
        self.next = self.breaks.len();
        count
    }

    /// Returns the bounds of the row most recently produced.
    pub(crate) const fn row_bounds(&self) -> RowBounds {
        self.output.bounds
    }

    /// Sets whether produced rows record their segments; bounds are recorded either way.
    pub(crate) const fn set_record(&mut self, record: bool) {
        self.output.record = record;
    }

    /// Produces the next row of the current line, or `None` once the line is exhausted.
    pub fn next_row(&mut self) -> Option<Row<'_>> {
        let end = *self.breaks.get(self.next)?;
        let start = self
            .next
            .checked_sub(1)
            .map_or(0, |previous| self.breaks[previous]);
        self.next += 1;
        Some(self.output.fill(&self.line, start..end))
    }
}

/// Graphemes of one source line grouped into textwrap fragments.
#[derive(Debug, Clone, Default)]
struct FragmentLine {
    /// Requested width in cells.
    width: u32,
    /// Graphemes of the line.
    graphemes: Vec<Grapheme>,
    /// Fragments over `graphemes`, with overlong words already split.
    fragments: Vec<GraphemeFragment>,
}

impl FragmentLine {
    /// Scans `source` into fragments for `width`. Returns `false`, with no fragments, for width 0.
    fn reset(&mut self, source: &Line<'_>, width: u16) -> bool {
        self.width = u32::from(width);
        self.graphemes.clear();
        self.fragments.clear();
        if width == 0 {
            return false;
        }
        grapheme::scan(source, &mut self.graphemes);

        let mut current = GraphemeFragment::empty(0);
        for index in 0..self.graphemes.len() {
            let grapheme = self.graphemes[index];
            if grapheme.whitespace {
                current.end = index + 1;
                current.whitespace_width += grapheme.width;
                continue;
            }
            if current.end > current.word_end {
                self.push_fragment(current);
                current = GraphemeFragment::empty(index);
            }
            current.word_end = index + 1;
            current.end = index + 1;
            current.word_width += grapheme.width;
        }
        if current.end > current.start {
            self.push_fragment(current);
        }
        true
    }

    /// Adds `fragment`, splitting a word wider than the row at grapheme boundaries.
    ///
    /// Matches `StyledFragment::split_word`: a non-zero-width grapheme starts a new piece when it
    /// would overflow a piece that already has content, and the whitespace stays on the last piece.
    fn push_fragment(&mut self, fragment: GraphemeFragment) {
        if fragment.word_width <= self.width {
            self.fragments.push(fragment);
            return;
        }
        let mut piece = GraphemeFragment::empty(fragment.start);
        for index in fragment.start..fragment.word_end {
            let width = self.graphemes[index].width;
            if width > 0 && piece.word_end > piece.start && piece.word_width + width > self.width {
                self.fragments.push(piece);
                piece = GraphemeFragment::empty(index);
            }
            piece.word_end = index + 1;
            piece.end = index + 1;
            piece.word_width += width;
        }
        piece.end = fragment.end;
        piece.whitespace_width = fragment.whitespace_width;
        self.fragments.push(piece);
    }
}

/// The segment buffer a textwrap producer lends rows from.
#[derive(Debug, Clone)]
struct RowOutput {
    /// Whether rows record segments; counting skips them.
    record: bool,
    /// Segments of the row most recently lent out.
    row: Vec<Segment>,
    /// Bounds of the row most recently produced, recorded even while counting.
    bounds: RowBounds,
}

impl Default for RowOutput {
    fn default() -> Self {
        Self {
            record: true,
            row: Vec::new(),
            bounds: RowBounds::EMPTY,
        }
    }
}

impl RowOutput {
    /// Lends the row made of `fragments`: every word and the whitespace between them, but not the
    /// whitespace after the last word, as textwrap's reconstruction does.
    fn fill(&mut self, line: &FragmentLine, fragments: Range<usize>) -> Row<'_> {
        self.row.clear();
        self.bounds = RowBounds::EMPTY;
        let mut width = 0;
        if fragments.start < fragments.end {
            let first = line.fragments[fragments.start].start;
            let last = line.fragments[fragments.end - 1].word_end;
            let graphemes = &line.graphemes[first..last];
            if let (Some(head), Some(tail)) = (graphemes.first(), graphemes.last()) {
                self.bounds = RowBounds::between(head.segment, tail.segment);
            }
            if self.record {
                for grapheme in graphemes {
                    width += grapheme.width;
                    grapheme::push_segment(&mut self.row, grapheme.segment);
                }
            }
        }
        Row {
            segments: &self.row,
            width: u16::try_from(width).unwrap_or(u16::MAX),
        }
    }
}

/// A word and its trailing whitespace as grapheme index ranges.
#[derive(Debug, Clone, Copy)]
struct GraphemeFragment {
    /// First grapheme of the word.
    start: usize,
    /// End of the word and start of the trailing whitespace.
    word_end: usize,
    /// End of the trailing whitespace.
    end: usize,
    /// Cell width of the word.
    word_width: u32,
    /// Cell width of the trailing whitespace.
    whitespace_width: u32,
}

impl GraphemeFragment {
    /// Creates an empty fragment starting at grapheme `start`.
    const fn empty(start: usize) -> Self {
        Self {
            start,
            word_end: start,
            end: start,
            word_width: 0,
            whitespace_width: 0,
        }
    }
}

/// Lets textwrap's own algorithms choose breaks over index-based fragments.
impl Fragment for GraphemeFragment {
    fn width(&self) -> f64 {
        f64::from(self.word_width)
    }

    fn whitespace_width(&self) -> f64 {
        f64::from(self.whitespace_width)
    }

    fn penalty_width(&self) -> f64 {
        0.0
    }
}

/// Unit tests for textwrap invocation and reconstruction.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the fragment contract's end-of-line whitespace rule during reconstruction.
    #[test]
    fn reconstruction_drops_only_the_final_fragment_whitespace() {
        let source = Line::from("alpha  beta  ");
        let fragments = fragment::from_line(&source, 20);

        let line = reconstruct_line(&source, &fragments);

        assert_eq!(line.to_string(), "alpha  beta");
    }

    /// Checks the common first-fit word boundary without Paragraph wrapping.
    #[test]
    fn first_fit_returns_owned_physical_lines() {
        let lines = wrap_first_fit(&Line::from("alpha beta"), 5);
        let content = lines.iter().map(ToString::to_string).collect::<Vec<_>>();

        assert_eq!(content, ["alpha", "beta"]);
    }

    /// Checks that both algorithms preserve an indivisible grapheme wider than the line.
    #[test]
    fn algorithms_preserve_grapheme_wider_than_line() {
        let source = Line::from("界");

        assert_eq!(wrap_first_fit(&source, 1)[0].to_string(), "界");
        assert_eq!(wrap_optimal_fit(&source, 1)[0].to_string(), "界");
    }
}

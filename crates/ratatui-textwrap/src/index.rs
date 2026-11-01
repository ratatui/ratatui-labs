//! Row index for scrolling wrapped text without wrapping every line each frame.
//!
//! [`RowIndex`] stores the wrapped row count of every source line at one width. Mapping a scroll
//! offset to a source line costs O(log n); recounting, inserting, or removing lines anywhere costs
//! the lines touched plus a small per-chunk bookkeeping pass. Only the rows in view are wrapped
//! into [`Line`] values. The index is crate-internal: [`WrappedText`](crate::WrappedText) owns the
//! text and is the only caller, so the counts cannot fall out of step with the lines they describe.

use std::ops::Range;

use ratatui_core::text::{Line, Text};

use crate::WrapAlgorithm;
use crate::algorithms::grapheme::{self, Grapheme};
use crate::algorithms::paragraph::ParagraphRows;
use crate::algorithms::textwrap::{FirstFitRows, OptimalFitRows};
use crate::algorithms::{Row, RowBounds, Segment};
use crate::counts::RowCounts;

/// A wrapped row's position in the source text.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RowPosition {
    /// Index into [`Text::lines`].
    pub line: usize,
    /// Row number within that source line's wrapped rows.
    pub row: usize,
}

/// Lines that wrap to more rows than this keep the byte bounds of every row.
///
/// Drawing a row of such a line then scans only that row's bytes instead of the whole line, so a
/// very long line (a minified file, a long log record) costs about a screenful per frame after it
/// is counted once. Shorter lines are cheap to rewrap and keep only their count.
const LONG_LINE_ROWS: usize = 16;

/// Wrapped row counts for one width and [`WrapAlgorithm`].
#[derive(Debug, Clone)]
pub(crate) struct RowIndex {
    /// Requested width in cells.
    width: u16,
    /// Algorithm the counts were produced with.
    algorithm: WrapAlgorithm,
    /// Row producer reused for counting.
    rows: Rows,
    /// Rows produced by each counted source line.
    counts: RowCounts,
    /// Row bounds of every counted line longer than [`LONG_LINE_ROWS`], sorted by line.
    long_lines: Vec<(usize, Box<[RowBounds]>)>,
    /// Row bounds of the line being counted, reused between lines.
    scratch: Vec<RowBounds>,
}

impl RowIndex {
    /// Creates an index that has counted no lines.
    pub(crate) fn empty(width: u16, algorithm: WrapAlgorithm) -> Self {
        Self {
            width,
            algorithm,
            rows: Rows::new(algorithm),
            counts: RowCounts::default(),
            long_lines: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Returns the width the index was built for.
    pub(crate) const fn width(&self) -> u16 {
        self.width
    }

    /// Returns the algorithm the index counts with.
    pub(crate) const fn algorithm(&self) -> WrapAlgorithm {
        self.algorithm
    }

    /// Returns the number of source lines counted.
    pub(crate) fn len(&self) -> usize {
        self.counts.len()
    }

    /// Returns the total number of wrapped rows in the counted lines.
    pub(crate) fn total_rows(&self) -> usize {
        self.counts.total()
    }

    /// Returns the number of rows produced by source line `line`.
    pub(crate) fn line_rows(&self, line: usize) -> usize {
        self.counts.get(line)
    }

    /// Returns the first wrapped row of source line `line`.
    pub(crate) fn first_row(&self, line: usize) -> usize {
        self.counts.rows_before(line)
    }

    /// Finds the source line and in-line row of wrapped row `row`, or `None` past the end.
    pub(crate) fn locate(&self, row: usize) -> Option<RowPosition> {
        if row >= self.counts.total() {
            return None;
        }
        let (line, before) = self.counts.locate(row);
        Some(RowPosition {
            line,
            row: row - before,
        })
    }

    /// Forgets every count.
    pub(crate) fn clear(&mut self) {
        self.counts.clear();
        self.long_lines.clear();
    }

    /// Matches the counted lines to `text`, counting at most `max_lines` new lines. Returns
    /// whether every line of `text` is now counted.
    pub(crate) fn sync_len_bounded(&mut self, text: &Text<'_>, max_lines: usize) -> bool {
        let len = text.lines.len();
        let counted = self.counts.len();
        if len < counted {
            self.counts.remove(len..counted);
            return true;
        }
        let end = len.min(counted.saturating_add(max_lines));
        self.insert_lines(text, counted..end);
        end == len
    }

    /// Recounts `lines` after their content changed.
    pub(crate) fn invalidate_lines(&mut self, text: &Text<'_>, lines: Range<usize>) {
        for line in lines {
            let count = self.count_line(line, &text.lines[line]);
            if self.counts.get(line) != count {
                self.counts.set(line, count);
            }
        }
    }

    /// Counts `lines`, just inserted into `text`, shifting the lines after them.
    pub(crate) fn insert_lines(&mut self, text: &Text<'_>, lines: Range<usize>) {
        for (line, _) in &mut self.long_lines {
            if *line >= lines.start {
                *line += lines.len();
            }
        }
        let counts: Vec<usize> = lines
            .clone()
            .map(|line| self.count_line(line, &text.lines[line]))
            .collect();
        self.counts.insert(lines.start, counts);
    }

    /// Forgets `lines`, just removed from the text, shifting the lines after them.
    pub(crate) fn remove_lines(&mut self, lines: Range<usize>) {
        self.long_lines.retain(|(line, _)| !lines.contains(line));
        for (line, _) in &mut self.long_lines {
            if *line >= lines.end {
                *line -= lines.len();
            }
        }
        self.counts.remove(lines);
    }

    /// Counts the rows of `source`, which is line `line`, keeping its row bounds if it is long.
    fn count_line(&mut self, line: usize, source: &Line<'_>) -> usize {
        // One pass records every row's bounds; they are kept only if the line turns out long.
        let count = self
            .rows
            .count_bounds(source, self.width, &mut self.scratch);
        let slot = self
            .long_lines
            .binary_search_by_key(&line, |(line, _)| *line);
        if count > LONG_LINE_ROWS {
            let bounds: Box<[RowBounds]> = self.scratch.as_slice().into();
            match slot {
                Ok(index) => self.long_lines[index].1 = bounds,
                Err(index) => self.long_lines.insert(index, (line, bounds)),
            }
        } else if let Ok(index) = slot {
            self.long_lines.remove(index);
        }
        count
    }

    /// Returns the row bounds of line `line`, if it is long.
    fn long_line(&self, line: usize) -> Option<&[RowBounds]> {
        let index = self
            .long_lines
            .binary_search_by_key(&line, |(line, _)| *line)
            .ok()?;
        Some(&self.long_lines[index].1)
    }

    /// Wraps the `height` rows that start at row `skip`, borrowing span content from `text`.
    ///
    /// Takes `&self` so read-only views can draw; the short-lived row producer is the only state.
    pub(crate) fn visible<'t>(&self, text: &'t Text<'_>, skip: usize, height: usize) -> Text<'t> {
        let mut lines = Vec::with_capacity(height.min(self.counts.total()));
        if let Some(start) = self.locate(skip) {
            let mut rows = Rows::new(self.algorithm);
            let mut scratch = Vec::new();
            let mut row_skip = start.row;
            let sources = &text.lines[start.line..self.counts.len()];
            for (line, source) in (start.line..).zip(sources) {
                if lines.len() == height {
                    break;
                }
                let take = height - lines.len();
                if let Some(bounds) = self.long_line(line) {
                    for bounds in bounds.iter().skip(row_skip).take(take) {
                        lines.push(self.bounded_row(source, *bounds, &mut scratch));
                    }
                } else {
                    rows.wrap_into(source, self.width, row_skip, take, &mut lines);
                }
                row_skip = 0;
            }
        }
        Text {
            alignment: text.alignment,
            style: text.style,
            lines,
        }
    }
}

impl RowIndex {
    /// Rebuilds one row of a long line from its bounds, borrowing from `source`.
    fn bounded_row<'t>(
        &self,
        source: &'t Line<'_>,
        bounds: RowBounds,
        scratch: &mut Vec<Grapheme>,
    ) -> Line<'t> {
        let mut segments: Vec<Segment> = Vec::new();
        let mut width = 0_u32;
        if bounds.start != bounds.end {
            scratch.clear();
            grapheme::scan_range(source, bounds.start, bounds.end, scratch);
            let omit_wide = matches!(self.rows, Rows::Paragraph(_));
            for grapheme in scratch.iter() {
                if omit_wide && grapheme.width > u32::from(self.width) {
                    continue;
                }
                width += grapheme.width;
                grapheme::push_segment(&mut segments, grapheme.segment);
            }
        }
        let row = Row {
            segments: &segments,
            width: u16::try_from(width).unwrap_or(u16::MAX),
        };
        row.to_line(source)
    }
}

/// Lends out the rows of one source line at a time.
///
/// Implemented by each algorithm's reusable producer. [`Row`] records are lifetime-free byte
/// ranges, so callers can count rows, skip them, or borrow only the ones they display.
trait RowProducer {
    /// Starts wrapping `source` at `width` cells.
    fn reset(&mut self, source: &Line<'_>, width: u16);

    /// Produces the next row, or `None` once the line is exhausted.
    fn next_row(&mut self) -> Option<Row<'_>>;

    /// Returns the bounds of the row most recently produced.
    fn row_bounds(&self) -> RowBounds;

    /// Sets whether produced rows record their segments; bounds are recorded either way.
    fn set_record(&mut self, record: bool);
}

/// Implements [`RowProducer`] by forwarding to a producer's inherent methods.
macro_rules! row_producer {
    ($($producer:ty),*) => {$(
        impl RowProducer for $producer {
            fn reset(&mut self, source: &Line<'_>, width: u16) {
                Self::reset(self, source, width);
            }

            fn next_row(&mut self) -> Option<Row<'_>> {
                Self::next_row(self)
            }

            fn row_bounds(&self) -> RowBounds {
                Self::row_bounds(self)
            }

            fn set_record(&mut self, record: bool) {
                Self::set_record(self, record);
            }
        }
    )*};
}

row_producer!(ParagraphRows, FirstFitRows, OptimalFitRows);

/// Row producer for one [`WrapAlgorithm`].
#[derive(Debug, Clone)]
pub(crate) enum Rows {
    /// Paragraph compatibility.
    Paragraph(ParagraphRows),
    /// Textwrap first-fit.
    FirstFit(FirstFitRows),
    /// Textwrap optimal-fit.
    OptimalFit(OptimalFitRows),
}

impl Rows {
    /// Creates the row producer for `algorithm`.
    pub(crate) fn new(algorithm: WrapAlgorithm) -> Self {
        match algorithm {
            WrapAlgorithm::ParagraphCompat { trim } => Self::Paragraph(ParagraphRows::new(trim)),
            WrapAlgorithm::FirstFit => Self::FirstFit(FirstFitRows::new()),
            WrapAlgorithm::OptimalFit => Self::OptimalFit(OptimalFitRows::new()),
        }
    }

    /// Returns the producer as a trait object.
    fn producer(&mut self) -> &mut dyn RowProducer {
        match self {
            Self::Paragraph(rows) => rows,
            Self::FirstFit(rows) => rows,
            Self::OptimalFit(rows) => rows,
        }
    }

    /// Counts the rows of `source` at `width`, replacing `bounds` with every row's bounds.
    fn count_bounds(
        &mut self,
        source: &Line<'_>,
        width: u16,
        bounds: &mut Vec<RowBounds>,
    ) -> usize {
        bounds.clear();
        let rows = self.producer();
        rows.reset(source, width);
        rows.set_record(false);
        while rows.next_row().is_some() {
            bounds.push(rows.row_bounds());
        }
        rows.set_record(true);
        bounds.len()
    }

    /// Appends up to `take` rows of `source`, after skipping `skip`, to `output`.
    pub(crate) fn wrap_into<'t>(
        &mut self,
        source: &'t Line<'_>,
        width: u16,
        mut skip: usize,
        take: usize,
        output: &mut Vec<Line<'t>>,
    ) {
        let end = output.len().saturating_add(take);
        let rows = self.producer();
        rows.reset(source, width);
        while output.len() < end
            && let Some(row) = rows.next_row()
        {
            if skip > 0 {
                skip -= 1;
            } else {
                output.push(row.to_line(source));
            }
        }
    }
}

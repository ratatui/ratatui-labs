//! Preserves Ratatui Paragraph's historical word-reflow behavior for one styled line.
//!
//! textwrap's fragment model assigns whitespace to the preceding word and assumes an atomic
//! fragment has one fixed width. Paragraph instead processes graphemes as a stream. Its observable
//! behavior can consume one whitespace grapheme at a wrap boundary, carry excess whitespace to the
//! next line, and admit a multi-cell grapheme after the pending word has crossed the limit. Those
//! cases affect line count, styles, and rendered cells.
//!
//! `ParagraphRows` maintains Paragraph's pending line, word, and whitespace queues over the
//! uncoalesced styled-grapheme stream and lends out one row at a time as byte ranges into the
//! source spans. [`wrap_line`] and `count_rows` are built on it. No textwrap fragment or
//! line-breaking API participates. Prefer [`crate::TextWrapper`] when the input is a complete
//! [`Text`](ratatui_core::text::Text) and its top-level metadata must also be preserved.
//!
//! The state machine is maintained against Ratatui 0.30.2's [`WordWrapper` implementation]. The
//! surrounding [`Paragraph integration`] shows how the widget constructs and consumes that wrapper;
//! compatibility tests render both paths and compare buffers and line counts.
//!
//! [`Paragraph integration`]: https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui-widgets/src/paragraph.rs#L329-L355
//! [`WordWrapper` implementation]: https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui-widgets/src/reflow.rs#L29-L273

use std::collections::VecDeque;

use ratatui_core::text::Line;

use super::grapheme::{self, Grapheme};
use super::row::{Row, RowBounds, Segment};

/// Wraps one source line with Paragraph's pending-word and pending-whitespace rules.
///
/// The copied behavior includes consuming one separator grapheme when whitespace crosses a
/// boundary. A positive width produces at least one line; a zero width produces no lines. Every
/// returned line copies `source` style and alignment and owns its span strings.
///
/// Graphemes containing controls and graphemes wider than `width` are omitted to reproduce
/// Paragraph's rendered behavior. Calls are independent and do not preserve wrapping state between
/// source chunks. This is a convenience over [`ParagraphRows`], which yields the same rows as
/// borrowed byte ranges without copying span content.
///
/// When `trim` is `true`, leading separator whitespace is discarded on continuation lines. When it
/// is `false`, that whitespace is preserved, matching `ratatui::widgets::Wrap::trim`.
#[must_use]
pub fn wrap_line(source: &Line<'_>, width: u16, trim: bool) -> Vec<Line<'static>> {
    let mut rows = ParagraphRows::new(trim);
    rows.reset(source, width);
    let mut output = Vec::new();
    while let Some(row) = rows.next_row() {
        output.push(row.to_owned_line(source));
    }
    output
}

/// Counts the rows [`wrap_line`] would produce without building any spans.
#[must_use]
pub fn count_rows(source: &Line<'_>, width: u16, trim: bool) -> usize {
    let mut rows = ParagraphRows::new(trim);
    rows.reset(source, width);
    rows.count_remaining()
}

/// Where the row producer is within the current source line.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Phase {
    /// Still consuming graphemes.
    Streaming,
    /// Graphemes are exhausted; flushing the trailing rows.
    Finishing,
    /// Every row has been produced.
    Done,
}

/// Reusable Paragraph-compatible row producer for one source line at a time.
///
/// [`Self::reset`] scans a line into lifetime-free grapheme records. [`Self::next_row`] then runs
/// Paragraph's state machine until the next row is decided and lends it out as source byte
/// ranges. Callers can stop early, skip rows without building spans, or turn only the rows they
/// display into borrowed [`Line`] values. Buffers are retained between lines, so wrapping many
/// lines with one producer allocates only while they grow.
///
/// ```
/// use ratatui::text::Line;
/// use ratatui_textwrap::algorithms::paragraph::ParagraphRows;
///
/// let source = Line::from("alpha beta");
/// let mut rows = ParagraphRows::new(true);
/// rows.reset(&source, 8);
/// let first = rows.next_row().unwrap().to_line(&source);
/// assert_eq!(first.to_string(), "alpha");
/// assert_eq!(rows.next_row().unwrap().width, 4);
/// assert!(rows.next_row().is_none());
/// ```
#[derive(Debug, Clone)]
pub struct ParagraphRows {
    /// Whether continuation lines discard leading separator whitespace.
    trim: bool,
    /// Requested width in cells.
    width: u32,
    /// Graphemes of the current source line.
    graphemes: Vec<Grapheme>,
    /// Index of the next grapheme to consume.
    next: usize,
    /// Committed graphemes of the row being built.
    pending_line: Vec<usize>,
    /// Graphemes of the word that may still move to the next row.
    pending_word: Vec<usize>,
    /// Whitespace graphemes that may still move to the next row.
    pending_whitespace: VecDeque<usize>,
    /// Width of `pending_line`.
    line_width: u32,
    /// Width of `pending_word`.
    word_width: u32,
    /// Width of `pending_whitespace`.
    whitespace_width: u32,
    /// Whether the previous grapheme left a non-empty pending word.
    has_pending_word: bool,
    /// Current state of the producer.
    phase: Phase,
    /// Whether Paragraph's trimmed whitespace-only empty row is still owed.
    owe_empty_row: bool,
    /// Whether any row has been produced for this line.
    emitted: bool,
    /// Whether rows record segments; counting skips them.
    record: bool,
    /// Segments of the row most recently lent out.
    row: Vec<Segment>,
    /// Width of `row`.
    row_width: u32,
    /// Bounds of the row most recently produced, recorded even while counting.
    bounds: RowBounds,
}

impl ParagraphRows {
    /// Creates an idle producer using `trim` as Paragraph's `Wrap::trim` policy.
    #[must_use]
    pub fn new(trim: bool) -> Self {
        Self {
            trim,
            width: 0,
            graphemes: Vec::new(),
            next: 0,
            pending_line: Vec::new(),
            pending_word: Vec::new(),
            pending_whitespace: VecDeque::new(),
            line_width: 0,
            word_width: 0,
            whitespace_width: 0,
            has_pending_word: false,
            phase: Phase::Done,
            owe_empty_row: false,
            emitted: false,
            record: true,
            row: Vec::new(),
            row_width: 0,
            bounds: RowBounds::EMPTY,
        }
    }

    /// Starts wrapping `source` at `width` cells, discarding any unfinished line.
    ///
    /// A zero width yields no rows.
    pub fn reset(&mut self, source: &Line<'_>, width: u16) {
        self.width = u32::from(width);
        self.graphemes.clear();
        self.next = 0;
        self.pending_line.clear();
        self.pending_word.clear();
        self.pending_whitespace.clear();
        self.line_width = 0;
        self.word_width = 0;
        self.whitespace_width = 0;
        self.has_pending_word = false;
        self.owe_empty_row = false;
        self.emitted = false;
        self.row.clear();
        self.row_width = 0;
        if width == 0 {
            self.phase = Phase::Done;
            return;
        }
        self.phase = Phase::Streaming;

        grapheme::scan(source, &mut self.graphemes);
    }

    /// Counts the rows remaining in the current line without lending them out.
    pub fn count_remaining(&mut self) -> usize {
        self.record = false;
        let mut count = 0;
        while self.next_row().is_some() {
            count += 1;
        }
        self.record = true;
        count
    }

    /// Returns the bounds of the row most recently produced.
    pub(crate) const fn row_bounds(&self) -> RowBounds {
        self.bounds
    }

    /// Sets whether produced rows record their segments; bounds are recorded either way.
    pub(crate) const fn set_record(&mut self, record: bool) {
        self.record = record;
    }

    /// Produces the next row of the current line, or `None` once the line is exhausted.
    pub fn next_row(&mut self) -> Option<Row<'_>> {
        let produced = match self.phase {
            Phase::Streaming => self.stream(),
            Phase::Finishing => self.finish(),
            Phase::Done => false,
        };
        if !produced {
            return None;
        }
        self.emitted = true;
        Some(Row {
            segments: &self.row,
            // Paragraph admits at most one extra wide grapheme, so rows stay near the u16 width.
            width: u16::try_from(self.row_width).unwrap_or(u16::MAX),
        })
    }

    /// Consumes graphemes until a row is committed or the stream ends.
    fn stream(&mut self) -> bool {
        let width = self.width;
        let trim = self.trim;
        while self.next < self.graphemes.len() {
            let index = self.next;
            self.next += 1;
            let piece = self.graphemes[index];
            // Paragraph's WordWrapper silently skips a grapheme that cannot fit in an empty line.
            if piece.width > width {
                continue;
            }
            let mut produced = false;

            let piece_width = piece.width;
            let word_ended = self.has_pending_word && piece.whitespace;
            let line_empty = self.pending_line.is_empty();
            let trimmed_overflow = line_empty && trim && self.word_width + piece_width > width;
            let whitespace_overflow =
                line_empty && trim && self.whitespace_width + piece_width > width;
            let untrimmed_overflow = line_empty
                && !trim
                && self.word_width + self.whitespace_width + piece_width > width;

            if word_ended || trimmed_overflow || whitespace_overflow || untrimmed_overflow {
                if !self.pending_line.is_empty() || !trim {
                    self.pending_line.extend(self.pending_whitespace.drain(..));
                    self.line_width += self.whitespace_width;
                }
                self.pending_line.append(&mut self.pending_word);
                self.line_width += self.word_width;
                self.pending_whitespace.clear();
                self.whitespace_width = 0;
                self.word_width = 0;
            }

            let line_full = self.line_width >= width;
            let pending_word_overflow = piece_width > 0
                && self.line_width + self.whitespace_width + self.word_width >= width;
            if line_full || pending_word_overflow {
                let mut remaining = width.saturating_sub(self.line_width);
                self.take_pending_line();
                produced = true;

                while let Some(&front) = self.pending_whitespace.front() {
                    let front_width = self.graphemes[front].width;
                    if front_width > remaining {
                        break;
                    }
                    self.whitespace_width -= front_width;
                    remaining -= front_width;
                    self.pending_whitespace.pop_front();
                }
                if piece.whitespace && self.pending_whitespace.is_empty() {
                    return true;
                }
            }

            if piece.whitespace {
                self.whitespace_width += piece_width;
                self.pending_whitespace.push_back(index);
            } else {
                self.word_width += piece_width;
                self.pending_word.push(index);
            }
            self.has_pending_word = !self.pending_word.is_empty();
            if produced {
                return true;
            }
        }

        self.phase = Phase::Finishing;
        self.owe_empty_row = self.pending_line.is_empty()
            && self.pending_word.is_empty()
            && !self.pending_whitespace.is_empty()
            && trim;
        if !self.pending_line.is_empty() || !trim {
            self.pending_line.extend(self.pending_whitespace.drain(..));
        }
        self.pending_line.append(&mut self.pending_word);
        self.finish()
    }

    /// Produces the trailing rows once graphemes are exhausted.
    fn finish(&mut self) -> bool {
        if self.owe_empty_row {
            self.owe_empty_row = false;
            self.row.clear();
            self.row_width = 0;
            self.bounds = RowBounds::EMPTY;
            return true;
        }
        if !self.pending_line.is_empty() {
            self.take_pending_line();
            return true;
        }
        self.phase = Phase::Done;
        if self.emitted {
            return false;
        }
        self.row.clear();
        self.row_width = 0;
        self.bounds = RowBounds::EMPTY;
        true
    }

    /// Moves the committed graphemes into the lent row, merging contiguous ranges.
    fn take_pending_line(&mut self) {
        self.row.clear();
        self.row_width = 0;
        self.bounds = match (self.pending_line.first(), self.pending_line.last()) {
            (Some(&first), Some(&last)) => {
                RowBounds::between(self.graphemes[first].segment, self.graphemes[last].segment)
            }
            _ => RowBounds::EMPTY,
        };
        if !self.record {
            self.pending_line.clear();
            self.line_width = 0;
            return;
        }
        for &index in &self.pending_line {
            let grapheme = self.graphemes[index];
            self.row_width += grapheme.width;
            grapheme::push_segment(&mut self.row, grapheme.segment);
        }
        self.pending_line.clear();
        self.line_width = 0;
    }
}

/// Unit tests for the copied Paragraph state machine.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks Paragraph's distinct whitespace-only behavior for both trim policies.
    #[test]
    fn whitespace_only_line_follows_trim_policy() {
        let source = Line::from("   ");

        let preserved = wrap_line(&source, 2, false);
        let trimmed = wrap_line(&source, 2, true);

        assert_eq!(
            preserved
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["  "]
        );
        assert_eq!(
            trimmed.iter().map(ToString::to_string).collect::<Vec<_>>(),
            [""]
        );
    }

    /// Checks the mixed-width overflow behavior retained from Paragraph.
    #[test]
    fn mixed_width_word_keeps_paragraph_overflow_behavior() {
        let lines = wrap_line(&Line::from("a界b"), 2, false);
        let content = lines.iter().map(ToString::to_string).collect::<Vec<_>>();

        assert_eq!(content, ["a", "界b"]);
    }

    /// Checks Paragraph's policy of omitting a grapheme wider than an otherwise empty line.
    #[test]
    fn grapheme_wider_than_line_is_omitted() {
        let lines = wrap_line(&Line::from("界"), 1, false);

        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].to_string(), "");
    }
}

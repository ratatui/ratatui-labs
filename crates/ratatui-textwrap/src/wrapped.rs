//! Owned text that keeps its wrapped row counts in step with every edit.

use std::ops::{Deref, Range, RangeBounds};

use ratatui_core::text::{Line, Text};

use crate::index::RowIndex;
use crate::{RowPosition, TextWrapper, WrapAlgorithm};

/// Text that can be scrolled without wrapping all of it, kept in step with every edit.
///
/// `WrappedText` owns a [`Text`] and the number of wrapped rows each of its lines produces at one
/// width. Those counts map any scroll offset to a source line, so a frame wraps only the rows in
/// view.
///
/// Reads go through [`Deref`] to [`Text`] and are free. Every way to change the text goes through a
/// method that records what may have changed, so the counts cannot fall out of step:
///
/// - [`Self::line_mut`] and [`Self::lines_mut`] lend lines for editing; only those lines are
///   recounted.
/// - [`Self::push_line`], [`Self::extend`], and [`Self::splice`] change the line structure; only
///   the new lines are counted.
/// - [`Self::text_mut`] lends the whole [`Text`], and [`Self::set_width`] changes the width; every
///   line is recounted.
///
/// Row positions are queried through a [`WrappedLayout`], an up-to-date view that borrows the text
/// so it cannot change while the view exists:
///
/// - [`Self::layout`] recounts whatever is pending, then returns the view.
/// - [`Self::try_layout`] returns the view through `&self`, if nothing is pending.
/// - [`Self::layout_bounded`] recounts at most a given number of lines per call, returning a
///   [`PendingLayout`] until it is done, to spread a full recount of large text across frames.
///
/// [`Self::tail`] needs no counts, so a view that sticks to the bottom stays cheap after any edit.
///
/// ```
/// use ratatui_textwrap::{WrapAlgorithm, WrappedText};
///
/// let mut log = WrappedText::new("starting", 20, WrapAlgorithm::FirstFit);
/// log.push_line("request handled in 3 ms");
/// log.line_mut(0).spans[0].content = "started".into();
///
/// let layout = log.layout();
/// assert_eq!(layout.total_rows(), 3);
/// let rows: Vec<String> = layout
///     .rows(1, 2)
///     .lines
///     .iter()
///     .map(ToString::to_string)
///     .collect();
/// assert_eq!(rows, ["request handled in 3", "ms"]);
/// assert_eq!(log.lines[0].to_string(), "started");
/// ```
#[derive(Debug, Clone)]
pub struct WrappedText<'a> {
    /// The owned text. Never exposed mutably without recording what may change.
    text: Text<'a>,
    /// Row counts for the lines counted so far.
    index: RowIndex,
    /// Counted lines whose content may have changed since they were counted.
    dirty: Vec<Range<usize>>,
}

impl<'a> WrappedText<'a> {
    /// Wraps `text` at `width` cells with `algorithm`, counting every line.
    #[must_use]
    pub fn new(text: impl Into<Text<'a>>, width: u16, algorithm: WrapAlgorithm) -> Self {
        let mut wrapped = Self::new_uncounted(text, width, algorithm);
        wrapped.index.sync_len_bounded(&wrapped.text, usize::MAX);
        wrapped
    }

    /// Like [`Self::new`], but counts nothing yet.
    ///
    /// Use with [`Self::layout_bounded`] to spread the initial count of large text across frames.
    #[must_use]
    pub fn new_uncounted(text: impl Into<Text<'a>>, width: u16, algorithm: WrapAlgorithm) -> Self {
        Self {
            text: text.into(),
            index: RowIndex::empty(width, algorithm),
            dirty: Vec::new(),
        }
    }

    /// Returns the wrapping width.
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.index.width()
    }

    /// Returns the wrapping algorithm.
    #[must_use]
    pub const fn algorithm(&self) -> WrapAlgorithm {
        self.index.algorithm()
    }

    /// Changes the wrapping width. Every line is recounted by the next layout.
    pub fn set_width(&mut self, width: u16) {
        if width != self.width() {
            self.index = RowIndex::empty(width, self.algorithm());
            self.dirty.clear();
        }
    }

    /// Lends the whole text for arbitrary edits. Every line is recounted by the next layout.
    pub fn text_mut(&mut self) -> &mut Text<'a> {
        self.index.clear();
        self.dirty.clear();
        &mut self.text
    }

    /// Lends line `line` for editing. Only that line is recounted.
    pub fn line_mut(&mut self, line: usize) -> &mut Line<'a> {
        self.dirty.push(line..line + 1);
        &mut self.text.lines[line]
    }

    /// Lends `lines` for editing. Only those lines are recounted.
    ///
    /// The slice cannot change the number of lines; use [`Self::splice`] for that.
    pub fn lines_mut(&mut self, lines: impl RangeBounds<usize>) -> &mut [Line<'a>] {
        let lines = range(lines, self.text.lines.len());
        self.dirty.push(lines.clone());
        &mut self.text.lines[lines]
    }

    /// Appends one line.
    pub fn push_line(&mut self, line: impl Into<Line<'a>>) {
        self.text.lines.push(line.into());
    }

    /// Appends lines.
    pub fn extend(&mut self, lines: impl IntoIterator<Item = Line<'a>>) {
        self.text.lines.extend(lines);
    }

    /// Replaces `lines` with `replacement`, like [`Vec::splice`].
    ///
    /// Only the replacement lines are counted; the counts of other lines shift without rewrapping.
    pub fn splice(
        &mut self,
        lines: impl RangeBounds<usize>,
        replacement: impl IntoIterator<Item = Line<'a>>,
    ) {
        self.flush_dirty();
        let removed = range(lines, self.text.lines.len());
        let before = self.text.lines.len();
        self.text.lines.splice(removed.clone(), replacement);
        let inserted = self.text.lines.len() + removed.len() - before;

        let counted = self.index.len();
        if removed.start <= counted {
            self.index
                .remove_lines(removed.start..removed.end.min(counted));
            self.index
                .insert_lines(&self.text, removed.start..removed.start + inserted);
        }
    }

    /// Returns the text, discarding the counts.
    #[must_use]
    pub fn into_text(self) -> Text<'a> {
        self.text
    }

    /// Wraps the last `height` rows, borrowing from the text.
    ///
    /// Uses the layout when nothing is pending, so rows of very long lines come from their recorded
    /// bounds. Otherwise it walks back from the last line without counts, so it stays cheap right
    /// after any edit, including [`Self::text_mut`] and [`Self::set_width`].
    #[must_use]
    pub fn tail(&self, height: usize) -> Text<'_> {
        if let Some(layout) = self.try_layout() {
            return layout.tail(height);
        }
        TextWrapper::new()
            .algorithm(self.algorithm())
            .tail(&self.text, self.width(), height)
    }

    /// Recounts whatever is pending and returns an up-to-date layout.
    pub fn layout(&mut self) -> WrappedLayout<'_> {
        self.flush_dirty();
        self.index.sync_len_bounded(&self.text, usize::MAX);
        WrappedLayout {
            text: &self.text,
            index: &self.index,
        }
    }

    /// Returns an up-to-date layout without recounting, or `None` if any edit is still pending.
    ///
    /// Suits drawing code that only holds `&self`: call [`Self::layout`] once after editing, then
    /// draw from shared references.
    #[must_use]
    pub fn try_layout(&self) -> Option<WrappedLayout<'_>> {
        let current = self.dirty.is_empty() && self.index.len() == self.text.lines.len();
        current.then_some(WrappedLayout {
            text: &self.text,
            index: &self.index,
        })
    }

    /// Recounts pending edits and at most `max_lines` uncounted lines.
    ///
    /// Returns the layout once every line is counted, or a [`PendingLayout`] that can still draw
    /// the bottom rows and report progress. Call once per frame after [`Self::set_width`] or
    /// [`Self::text_mut`] on large text to avoid one long recount.
    pub fn layout_bounded(
        &mut self,
        max_lines: usize,
    ) -> Result<WrappedLayout<'_>, PendingLayout<'_>> {
        self.flush_dirty();
        let complete = self.index.sync_len_bounded(&self.text, max_lines);
        let this = &*self;
        if complete {
            Ok(WrappedLayout {
                text: &this.text,
                index: &this.index,
            })
        } else {
            Err(PendingLayout { wrapped: this })
        }
    }

    /// Recounts lines lent out since the last layout.
    fn flush_dirty(&mut self) {
        let counted = self.index.len();
        for lines in self.dirty.drain(..) {
            let lines = lines.start.min(counted)..lines.end.min(counted);
            self.index.invalidate_lines(&self.text, lines);
        }
    }
}

impl<'a> Deref for WrappedText<'a> {
    type Target = Text<'a>;

    fn deref(&self) -> &Text<'a> {
        &self.text
    }
}

/// An up-to-date view of a [`WrappedText`]'s row positions.
///
/// Every query takes `&self`. The view borrows the text, so the text cannot change while the view
/// exists and the positions it reports always describe the current lines.
#[derive(Debug, Clone, Copy)]
pub struct WrappedLayout<'w> {
    /// The text the counts describe.
    text: &'w Text<'w>,
    /// Row counts for every line of `text`.
    index: &'w RowIndex,
}

impl<'w> WrappedLayout<'w> {
    /// Returns the total number of wrapped rows.
    #[must_use]
    pub fn total_rows(&self) -> usize {
        self.index.total_rows()
    }

    /// Returns the number of rows produced by source line `line`.
    ///
    /// # Panics
    ///
    /// Panics if `line` is out of bounds.
    #[must_use]
    pub fn line_rows(&self, line: usize) -> usize {
        self.index.line_rows(line)
    }

    /// Returns the first wrapped row of source line `line`; `line` may equal the line count.
    #[must_use]
    pub fn first_row(&self, line: usize) -> usize {
        self.index.first_row(line)
    }

    /// Finds the source line and in-line row of wrapped row `row`, or `None` past the end.
    #[must_use]
    pub fn locate(&self, row: usize) -> Option<RowPosition> {
        self.index.locate(row)
    }

    /// Wraps the `height` rows starting at row `skip`, borrowing from the text.
    ///
    /// Only the source lines that overlap the window are wrapped. The result keeps the text's
    /// style and alignment, so rendering it in an unwrapped `Paragraph` matches a wrapped
    /// `Paragraph` scrolled to `skip`.
    #[must_use]
    pub fn rows(&self, skip: usize, height: usize) -> Text<'w> {
        self.index.visible(self.text, skip, height)
    }

    /// Wraps the last `height` rows, borrowing from the text.
    #[must_use]
    pub fn tail(&self, height: usize) -> Text<'w> {
        self.rows(self.total_rows().saturating_sub(height), height)
    }
}

/// A [`WrappedText`] whose recount is still in progress.
///
/// Returned by [`WrappedText::layout_bounded`]. It can draw the bottom of the text, which needs no
/// counts, and reports how far the recount has got.
#[derive(Debug, Clone, Copy)]
pub struct PendingLayout<'w> {
    /// The text being recounted.
    wrapped: &'w WrappedText<'w>,
}

impl<'w> PendingLayout<'w> {
    /// Returns how many source lines have been counted.
    #[must_use]
    pub fn counted_lines(&self) -> usize {
        self.wrapped.index.len()
    }

    /// Returns the number of source lines.
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.wrapped.text.lines.len()
    }

    /// Wraps the last `height` rows, borrowing from the text.
    #[must_use]
    pub fn tail(&self, height: usize) -> Text<'w> {
        self.wrapped.tail(height)
    }
}

/// Resolves `bounds` against a length, panicking like slice indexing when out of range.
fn range(bounds: impl RangeBounds<usize>, len: usize) -> Range<usize> {
    use std::ops::Bound::{Excluded, Included, Unbounded};
    let start = match bounds.start_bound() {
        Included(&start) => start,
        Excluded(&start) => start + 1,
        Unbounded => 0,
    };
    let end = match bounds.end_bound() {
        Included(&end) => end + 1,
        Excluded(&end) => end,
        Unbounded => len,
    };
    assert!(
        start <= end && end <= len,
        "range {start}..{end} out of bounds for {len} lines"
    );
    start..end
}

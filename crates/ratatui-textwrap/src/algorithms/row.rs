//! Lifetime-free wrapped rows shared by row producers.
//!
//! A [`Row`] describes one wrapped row as byte ranges into the source [`Line`]'s spans, so row
//! records can be kept, counted, or skipped without copying text. Only rows that are displayed
//! need to become [`Line`] values.

use ratatui_core::text::{Line, Span};

/// One contiguous byte range of a single source span within a wrapped row.
///
/// Segments carry no lifetime. They index into the [`Line`] that produced them, so they stay valid
/// until that line's spans change.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Segment {
    /// Index into [`Line::spans`].
    pub span: usize,
    /// Start byte offset within the span content.
    pub start: usize,
    /// End byte offset (exclusive) within the span content.
    pub end: usize,
}

/// The byte bounds of one wrapped row, as (span index, byte offset) positions.
///
/// Every row produced by this crate's algorithms is a contiguous run of the line's graphemes, minus
/// graphemes that Paragraph compatibility omits for being wider than the row. Rescanning the bounds
/// and re-applying that omission reproduces the row exactly.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct RowBounds {
    /// Position of the row's first grapheme.
    pub(crate) start: (usize, usize),
    /// Position just past the row's last grapheme. Equal to `start` for an empty row.
    pub(crate) end: (usize, usize),
}

impl RowBounds {
    /// Bounds of a row with no graphemes.
    pub(crate) const EMPTY: Self = Self {
        start: (0, 0),
        end: (0, 0),
    };

    /// Bounds running from the start of `first` to the end of `last`.
    pub(crate) const fn between(first: Segment, last: Segment) -> Self {
        Self {
            start: (first.span, first.start),
            end: (last.span, last.end),
        }
    }
}

/// A wrapped row lent out by a row producer such as
/// [`ParagraphRows::next_row`](crate::algorithms::paragraph::ParagraphRows::next_row).
///
/// The borrow lasts until the producer's next call, which reuses the segment buffer.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Row<'r> {
    /// Source ranges in display order. Adjacent ranges of one span are merged.
    pub segments: &'r [Segment],
    /// Rendered width in terminal cells.
    pub width: u16,
}

impl Row<'_> {
    /// Builds a line that borrows its span content from `source`.
    ///
    /// Each segment becomes one span with its source span style. The line copies `source` style and
    /// alignment.
    #[must_use]
    pub fn to_line<'s>(&self, source: &'s Line<'_>) -> Line<'s> {
        let spans = self
            .segments
            .iter()
            .map(|segment| {
                let span = &source.spans[segment.span];
                Span::styled(&span.content[segment.start..segment.end], span.style)
            })
            .collect();
        Line {
            style: source.style,
            alignment: source.alignment,
            spans,
        }
    }

    /// Builds an owned line, coalescing adjacent segments with equal styles.
    #[must_use]
    pub fn to_owned_line(&self, source: &Line<'_>) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for segment in self.segments {
            let span = &source.spans[segment.span];
            let content = &span.content[segment.start..segment.end];
            if let Some(last) = spans.last_mut()
                && last.style == span.style
            {
                last.content.to_mut().push_str(content);
            } else {
                spans.push(Span::styled(content.to_owned(), span.style));
            }
        }
        Line {
            style: source.style,
            alignment: source.alignment,
            spans,
        }
    }
}

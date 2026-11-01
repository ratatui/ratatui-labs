//! Lifetime-free grapheme records shared by the row producers.
//!
//! [`scan`] records every grapheme Ratatui's styled-grapheme iterator yields for a line as a byte
//! range, cell width, and whitespace classification. Producers make all wrapping decisions on these
//! records and lend out rows as [`Segment`] ranges, so no span content is copied.

use ratatui_core::buffer::CellWidth;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};

use super::row::Segment;

/// Lifetime-free record of one styled grapheme in a source line.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Grapheme {
    /// Position of the grapheme in its source span.
    pub(crate) segment: Segment,
    /// Ratatui cell width.
    pub(crate) width: u32,
    /// Ratatui whitespace classification.
    pub(crate) whitespace: bool,
}

/// Appends the graphemes of `source` to `graphemes`.
///
/// Graphemes containing control characters are omitted, as `Line::styled_graphemes` omits them.
#[inline]
pub(crate) fn scan(source: &Line<'_>, graphemes: &mut Vec<Grapheme>) {
    for (index, span) in source.spans.iter().enumerate() {
        push_span(graphemes, index, &span.content, 0);
    }
}

/// Appends the graphemes of `source` between two byte positions, as (span, byte offset) pairs.
///
/// Both positions must fall on grapheme boundaries, such as the bounds of a previously produced
/// row. Segmentation restarted at a boundary yields the same graphemes as a full scan.
#[inline]
pub(crate) fn scan_range(
    source: &Line<'_>,
    start: (usize, usize),
    end: (usize, usize),
    graphemes: &mut Vec<Grapheme>,
) {
    let spans = &source.spans[start.0..=end.0];
    for (index, span) in (start.0..).zip(spans) {
        let content: &str = &span.content;
        let from = if index == start.0 { start.1 } else { 0 };
        let to = if index == end.0 { end.1 } else { content.len() };
        push_span(graphemes, index, &content[from..to], from);
    }
}

/// Appends the graphemes of `content`, which starts at byte `offset` of span `index`.
#[inline]
fn push_span(graphemes: &mut Vec<Grapheme>, index: usize, content: &str, offset: usize) {
    if content.is_ascii() {
        push_ascii(graphemes, index, content, offset);
    } else {
        push_graphemes(graphemes, index, content, offset);
    }
}

/// Appends `segment` to a row, merging it with the previous range when they are contiguous.
#[inline]
pub(crate) fn push_segment(row: &mut Vec<Segment>, segment: Segment) {
    if let Some(last) = row.last_mut()
        && last.span == segment.span
        && last.end == segment.start
    {
        last.end = segment.end;
    } else {
        row.push(segment);
    }
}

/// Records every Ratatui grapheme of span `index`, as `Span::styled_graphemes` yields them.
#[inline]
fn push_graphemes(graphemes: &mut Vec<Grapheme>, index: usize, content: &str, offset: usize) {
    let base = content.as_ptr() as usize - offset;
    let span = Span::raw(content);
    for grapheme in span.styled_graphemes(Style::new()) {
        let start = grapheme.symbol.as_ptr() as usize - base;
        let segment = Segment {
            span: index,
            start,
            end: start + grapheme.symbol.len(),
        };
        graphemes.push(Grapheme {
            segment,
            width: u32::from(grapheme.symbol.cell_width()),
            whitespace: grapheme.is_whitespace(),
        });
    }
}

/// Records the graphemes of ASCII span `index` without Unicode segmentation.
///
/// Every ASCII grapheme is one byte except CRLF, and Ratatui drops graphemes that contain a
/// control, so each remaining byte is a printable grapheme one cell wide. Only the space is
/// whitespace. The tests check this against [`push_graphemes`] for every ASCII byte.
#[inline]
fn push_ascii(graphemes: &mut Vec<Grapheme>, index: usize, content: &str, offset: usize) {
    for (start, byte) in (offset..).zip(content.bytes()) {
        if byte.is_ascii_control() {
            continue;
        }
        graphemes.push(Grapheme {
            segment: Segment {
                span: index,
                start,
                end: start + 1,
            },
            width: 1,
            whitespace: byte == b' ',
        });
    }
}

/// Unit tests for grapheme scanning.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the ASCII fast path against Ratatui's grapheme iterator for every ASCII byte.
    #[test]
    fn ascii_fast_path_matches_styled_graphemes() {
        let all: String = (0_u8..=0x7f).map(char::from).collect();
        for content in [all.as_str(), "a\r\nb", "\r\n\r", " a  b\t c\n"] {
            let (mut fast, mut slow) = (Vec::new(), Vec::new());
            push_ascii(&mut fast, 3, content, 0);
            push_graphemes(&mut slow, 3, content, 0);
            let key = |g: &Grapheme| (g.segment, g.width, g.whitespace);
            assert_eq!(
                fast.iter().map(key).collect::<Vec<_>>(),
                slow.iter().map(key).collect::<Vec<_>>(),
                "{content:?}"
            );
        }
    }
}

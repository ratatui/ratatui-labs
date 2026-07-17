//! Conversion between Ratatui graphemes and owned styled runs.
//!
//! `StyledPiece` is the owned unit shared by both wrapping implementations. A piece records text,
//! source span style, Ratatui cell width, and Ratatui whitespace classification. It does not apply
//! the containing line or text style; those remain on the reconstructed [`Line`].
//!
//! `collect_graphemes` creates one piece per styled grapheme without applying a wrapping policy.
//! The textwrap path coalesces pieces after assigning them to a word or whitespace, while Paragraph
//! compatibility keeps the stream at grapheme granularity. `append` coalesces equal adjacent styles
//! when it creates output spans.
//!
//! Ratatui's [`StyledGrapheme`] and [`CellWidth`] APIs define the classification and measurement
//! semantics retained here. The tagged [`Ratatui text source`] shows the implementation used by the
//! 0.30.2 source baseline.
//!
//! [`CellWidth`]: ratatui_core::buffer::CellWidth
//! [`Ratatui text source`]: https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui-core/src/text.rs
//! [`StyledGrapheme`]: ratatui_core::text::StyledGrapheme

use ratatui_core::buffer::CellWidth;
use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};

/// Stores an owned, uniformly styled run with one word-or-whitespace role.
///
/// A piece begins as one Ratatui [`StyledGrapheme`](ratatui_core::text::StyledGrapheme), then may
/// absorb adjacent graphemes with the same style and classification. The intermediate form avoids
/// one output [`Span`] allocation per grapheme while retaining every boundary that affects layout
/// or rendering.
#[derive(Debug, Clone)]
pub struct StyledPiece {
    /// Owned grapheme content used to build the final `Text<'static>`.
    ///
    /// Ownership removes any output lifetime dependency on the source text.
    pub content: String,

    /// The source span style before text and line styles are applied.
    ///
    /// Keeping the unpatched style prevents reset colors and modifiers from being applied twice
    /// when the output line restores its original metadata.
    pub style: Style,

    /// Cached terminal-cell width calculated with Ratatui's [`CellWidth`].
    ///
    /// The cache keeps layout independent of textwrap's optional Unicode-width feature and makes
    /// Ratatui's terminal-specific width corrections authoritative.
    pub width: u32,

    /// Ratatui's word-separation classification for every grapheme in [`Self::content`].
    ///
    /// Ratatui treats a zero-width space as whitespace and a non-breaking space as word content,
    /// which differs from recomputing the value with `char::is_whitespace`.
    pub whitespace: bool,
}

impl StyledPiece {
    /// Copies one classified content run and records its Ratatui cell width.
    ///
    /// `whitespace` comes from `StyledGrapheme::is_whitespace`, keeping segmentation aligned with
    /// Ratatui's treatment of non-breaking and zero-width spaces.
    pub fn new(content: &str, style: Style, whitespace: bool) -> Self {
        Self {
            content: content.to_owned(),
            style,
            width: u32::from(content.cell_width()),
            whitespace,
        }
    }
}

/// Collects one owned piece for each Ratatui grapheme in `line`.
///
/// `Line::styled_graphemes` filters graphemes containing control characters. Every remaining
/// grapheme is retained regardless of width so each wrapping algorithm can apply its own overflow
/// policy. Keeping one piece per grapheme lets Paragraph compatibility make its width and
/// whitespace decisions as a stream; word fragments coalesce pieces later.
pub fn collect_graphemes(line: &Line<'_>) -> Vec<StyledPiece> {
    line.spans
        .iter()
        .flat_map(|span| span.styled_graphemes(Style::new()))
        .map(|grapheme| StyledPiece::new(grapheme.symbol, grapheme.style, grapheme.is_whitespace()))
        .collect()
}

/// Builds an owned line from `pieces` and copies the source line metadata.
///
/// Keeping style and alignment on the [`Line`] preserves Ratatui's text-to-line-to-span style
/// patch order and makes every physical line generated from one source line align identically.
pub fn line_from(source: &Line<'_>, pieces: &[StyledPiece]) -> Line<'static> {
    let mut spans = Vec::new();
    append(&mut spans, pieces);
    Line {
        style: source.style,
        alignment: source.alignment,
        spans,
    }
}

/// Adds `piece`, coalescing it with an adjacent piece of the same style and role.
///
/// Requiring the same whitespace role prevents coalescing from erasing the semantic boundary
/// between a word and its following separator.
pub fn push(pieces: &mut Vec<StyledPiece>, piece: StyledPiece) {
    if let Some(last) = pieces.last_mut()
        && last.style == piece.style
        && last.whitespace == piece.whitespace
    {
        last.content.push_str(&piece.content);
        last.width += piece.width;
    } else {
        pieces.push(piece);
    }
}

/// Appends owned spans and coalesces equal adjacent source styles.
///
/// Reconstruction no longer needs the word-or-whitespace classification, so equal styles may
/// cross that boundary. Rendered cells are unchanged and the output avoids redundant spans.
pub fn append(spans: &mut Vec<Span<'static>>, pieces: &[StyledPiece]) {
    for piece in pieces {
        if let Some(last) = spans.last_mut()
            && last.style == piece.style
        {
            last.content.to_mut().push_str(&piece.content);
        } else {
            spans.push(Span::styled(piece.content.clone(), piece.style));
        }
    }
}

/// Unit tests for grapheme conversion and run coalescing.
#[cfg(test)]
mod tests {
    use ratatui_core::style::Color;

    use super::*;

    /// Checks Ratatui whitespace classification without applying an area-width policy.
    #[test]
    fn collection_keeps_ratatui_classification_and_wide_graphemes() {
        let pieces = collect_graphemes(&Line::from("\u{200b}\u{a0}界"));

        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].content, "\u{200b}");
        assert!(pieces[0].whitespace);
        assert_eq!(pieces[1].content, "\u{a0}");
        assert!(!pieces[1].whitespace);
        assert_eq!(pieces[2].content, "界");
        assert_eq!(pieces[2].width, 2);
    }

    /// Checks that semantic roles constrain piece coalescing but not final span coalescing.
    #[test]
    fn coalescing_keeps_piece_roles_and_merges_final_styles() {
        let style = Style::new().fg(Color::Green);
        let mut pieces = Vec::new();
        push(&mut pieces, StyledPiece::new("a", style, false));
        push(&mut pieces, StyledPiece::new("b", style, false));
        push(&mut pieces, StyledPiece::new(" ", style, true));

        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].content, "ab");

        let mut spans = Vec::new();
        append(&mut spans, &pieces);
        assert_eq!(spans, [Span::styled("ab ", style)]);
    }
}

//! Styled implementation of textwrap's fragment contract.
//!
//! `StyledFragment` stores one word and its following whitespace as separate styled runs. The
//! separation lets textwrap count whitespace between words without rendering it at the end of a
//! generated line. Words may cross source span boundaries, so a style change does not introduce a
//! break point.
//!
//! `from_line` groups Ratatui graphemes into fragments and splits words wider than the requested
//! width. A single grapheme wider than that width remains an indivisible overflowing fragment. The
//! line-breaking algorithms can then treat every fragment as atomic. Reconstruction reads the word
//! and whitespace portions separately to preserve their source styles.
//!
//! Ratatui remains authoritative for grapheme segmentation, whitespace classification, and cell
//! width. The upstream [fragment documentation] and [source] define the low-level textwrap
//! contract.
//!
//! [fragment documentation]: https://docs.rs/textwrap/0.16/textwrap/core/trait.Fragment.html
//! [source]: https://github.com/mgeisler/textwrap/blob/4770e55af425a0cffb9ad8496599d2a1a4f5ed14/src/core.rs#L217-L230

use ratatui_core::style::Style;
use ratatui_core::text::{Line, Span};
use textwrap::core::Fragment;

use super::piece::{self, StyledPiece};

/// Stores a styled word and its following whitespace as one textwrap fragment.
///
/// textwrap's [`Fragment`] contract measures trailing whitespace only when another fragment fits
/// on the same output line. Separate styled collections preserve the source styles while exposing
/// that measurement model. One word may contain pieces from several source spans.
#[derive(Debug, Default, Clone)]
pub struct StyledFragment {
    /// Word content that remains indivisible after overlong-word splitting.
    word: Vec<StyledPiece>,

    /// Whitespace immediately following [`Self::word`] in the source line.
    whitespace: Vec<StyledPiece>,

    /// Sum of word widths exposed through [`Fragment::width`].
    ///
    /// The cached sum avoids rescanning owned strings during line-breaking passes.
    word_width: u32,

    /// Sum of whitespace widths exposed through [`Fragment::whitespace_width`].
    ///
    /// A separate sum is necessary because textwrap excludes it when the fragment ends a line.
    whitespace_width: u32,
}

/// Builds width-bounded textwrap fragments from one Ratatui line.
///
/// Segmentation crosses source span boundaries so differently styled pieces of one lexical word
/// remain one wrapping unit. `piece::collect_graphemes` filters control characters and measures
/// cell widths. Overlong words are split before reaching textwrap, but a grapheme wider than
/// `width` remains whole and may produce an overflowing line. `width` is positive because
/// [`crate::TextWrapper`] handles zero before dispatching source lines.
pub fn from_line(line: &Line<'_>, width: u16) -> Vec<StyledFragment> {
    let mut fragments = Vec::new();
    let mut current = StyledFragment::default();

    for piece in piece::collect_graphemes(line) {
        if piece.whitespace {
            current.push_whitespace(piece);
            continue;
        }
        if !current.whitespace.is_empty() {
            fragments.push(current);
            current = StyledFragment::default();
        }
        current.push_word(piece);
    }
    if current.has_word() || !current.whitespace.is_empty() {
        fragments.push(current);
    }

    fragments
        .into_iter()
        .flat_map(|fragment| fragment.split_word(width))
        .collect()
}

impl StyledFragment {
    /// Splits this fragment's word at Ratatui grapheme boundaries when it exceeds `width`.
    ///
    /// textwrap's algorithms do not divide a [`Fragment`], so words must be bounded first. Only
    /// the final fragment inherits the source fragment's trailing whitespace; assigning it to an
    /// earlier fragment would create a false separator inside the original word. Zero-width
    /// graphemes remain with the current fragment to avoid producing empty fragments. `width` is
    /// positive because zero-width text is handled before fragment construction.
    pub fn split_word(self, width: u16) -> Vec<Self> {
        if self.word_width <= u32::from(width) {
            return vec![self];
        }

        let mut split = Vec::new();
        let mut current = Self::default();
        for piece in self.word {
            let span = Span::styled(piece.content, piece.style);
            let graphemes = span.styled_graphemes(Style::new());
            for grapheme in graphemes {
                let piece =
                    StyledPiece::new(grapheme.symbol, grapheme.style, grapheme.is_whitespace());
                let would_overflow = current.word_width + piece.width > u32::from(width);
                if piece.width > 0 && current.has_word() && would_overflow {
                    split.push(current);
                    current = Self::default();
                }
                current.push_word(piece);
            }
        }
        current.whitespace = self.whitespace;
        current.whitespace_width = self.whitespace_width;
        split.push(current);
        split
    }

    /// Appends the word portion to an output span list, coalescing equal adjacent styles.
    pub fn append_word_to(&self, spans: &mut Vec<Span<'static>>) {
        piece::append(spans, &self.word);
    }

    /// Appends trailing whitespace when reconstruction decides that it remains visible.
    pub fn append_whitespace_to(&self, spans: &mut Vec<Span<'static>>) {
        piece::append(spans, &self.whitespace);
    }

    /// Adds word content and updates its cached width.
    fn push_word(&mut self, piece: StyledPiece) {
        self.word_width += piece.width;
        piece::push(&mut self.word, piece);
    }

    /// Adds following whitespace and updates its cached width.
    fn push_whitespace(&mut self, piece: StyledPiece) {
        self.whitespace_width += piece.width;
        piece::push(&mut self.whitespace, piece);
    }

    /// Returns whether the fragment contains word content.
    ///
    /// Leading whitespace remains a whitespace-only fragment because it participates in overflow
    /// behavior even though no word precedes it.
    const fn has_word(&self) -> bool {
        !self.word.is_empty()
    }
}

impl Fragment for StyledFragment {
    /// Returns the word width used by textwrap's line-breaking algorithms.
    ///
    /// textwrap expresses widths as `f64`; the stored value remains an integer terminal-cell count
    /// until this trait boundary.
    fn width(&self) -> f64 {
        f64::from(self.word_width)
    }

    /// Returns the following whitespace width included only between same-line fragments.
    fn whitespace_width(&self) -> f64 {
        f64::from(self.whitespace_width)
    }

    /// Returns zero because this adapter inserts neither hyphens nor replacement glyphs.
    fn penalty_width(&self) -> f64 {
        0.0
    }
}

/// Unit tests for styled fragment construction.
#[cfg(test)]
mod tests {
    use ratatui_core::style::{Color, Style};

    use super::*;

    /// Checks that style changes inside a word do not introduce break points.
    #[test]
    fn style_boundaries_remain_inside_one_word_fragment() {
        let line = Line::from(vec![
            Span::styled("al", Style::new().fg(Color::Red)),
            Span::styled("pha", Style::new().fg(Color::Blue)),
            Span::raw(" beta"),
        ]);

        let fragments = from_line(&line, 20);

        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments[0].word_width, 5);
        assert_eq!(fragments[0].word.len(), 2);
        assert_eq!(fragments[0].whitespace_width, 1);
    }

    /// Checks that only the final slice of an overlong word keeps following whitespace.
    #[test]
    fn overlong_word_keeps_whitespace_on_its_final_fragment() {
        let fragments = from_line(&Line::from("abcdef  "), 3);

        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments[0].word_width, 3);
        assert_eq!(fragments[0].whitespace_width, 0);
        assert_eq!(fragments[1].word_width, 3);
        assert_eq!(fragments[1].whitespace_width, 2);
    }

    /// Checks that an indivisible grapheme becomes one overflowing fragment instead of
    /// disappearing.
    #[test]
    fn grapheme_wider_than_line_remains_an_overflowing_fragment() {
        let fragments = from_line(&Line::from("界"), 1);

        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].word_width, 2);
        assert_eq!(fragments[0].word[0].content, "界");
    }
}

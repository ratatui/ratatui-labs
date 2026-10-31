// Vendored from ratatui-labs 817e6779 (before index-based wrapping) for benchmark comparison.
//! Preserves Ratatui Paragraph's historical word-reflow behavior for one styled line.
//!
//! textwrap's fragment model assigns whitespace to the preceding word and assumes an atomic
//! fragment has one fixed width. Paragraph instead processes graphemes as a stream. Its observable
//! behavior can consume one whitespace grapheme at a wrap boundary, carry excess whitespace to the
//! next line, and admit a multi-cell grapheme after the pending word has crossed the limit. Those
//! cases affect line count, styles, and rendered cells.
//!
//! [`wrap_line`] is the public line-level compatibility entry point. It maintains Paragraph's
//! pending line, word, and whitespace queues over the uncoalesced styled-grapheme stream. The
//! output uses the same owned pieces as the textwrap path, but no textwrap fragment or
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
use std::mem;

use ratatui_core::text::Line;

use super::piece::{self, StyledPiece};

/// Wraps one source line with Paragraph's pending-word and pending-whitespace rules.
///
/// The copied behavior includes consuming one separator grapheme when whitespace crosses a
/// boundary. `pending_line` contains committed pieces; `pending_word` and `pending_whitespace`
/// remain movable until the next grapheme determines whether they fit. Every returned line copies
/// `source` style and alignment and owns its span strings. A positive width produces at least one
/// line; a zero width produces no lines.
///
/// Graphemes containing controls and graphemes wider than `width` are omitted to reproduce
/// Paragraph's rendered behavior. The function allocates its pending queues and returned lines.
/// Calls are independent and do not preserve wrapping state between source chunks.
///
/// When `trim` is `true`, leading separator whitespace is discarded on continuation lines. When it
/// is `false`, that whitespace is preserved, matching `ratatui::widgets::Wrap::trim`.
#[must_use]
pub fn wrap_line(source: &Line<'_>, width: u16, trim: bool) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let mut output = VecDeque::new();
    let mut pending_line = Vec::new();
    let mut pending_word = Vec::new();
    let mut pending_whitespace: VecDeque<StyledPiece> = VecDeque::new();
    let mut line_width = 0_u32;
    let mut word_width = 0_u32;
    let mut whitespace_width = 0_u32;
    let mut has_pending_word = false;

    for piece in piece::collect_graphemes(source) {
        // Paragraph's WordWrapper silently skips a grapheme that cannot fit in an empty line. Keep
        // that compatibility policy here rather than deleting content during shared conversion.
        if piece.width > u32::from(width) {
            continue;
        }

        let piece_width = piece.width;
        let word_ended = has_pending_word && piece.whitespace;
        let trimmed_overflow =
            pending_line.is_empty() && trim && word_width + piece_width > u32::from(width);
        let whitespace_overflow =
            pending_line.is_empty() && trim && whitespace_width + piece_width > u32::from(width);
        let untrimmed_overflow = pending_line.is_empty()
            && !trim
            && word_width + whitespace_width + piece_width > u32::from(width);

        if word_ended || trimmed_overflow || whitespace_overflow || untrimmed_overflow {
            if !pending_line.is_empty() || !trim {
                pending_line.extend(pending_whitespace.drain(..));
                line_width += whitespace_width;
            }
            pending_line.append(&mut pending_word);
            line_width += word_width;
            pending_whitespace.clear();
            whitespace_width = 0;
            word_width = 0;
        }

        let line_full = line_width >= u32::from(width);
        let pending_word_overflow =
            piece_width > 0 && line_width + whitespace_width + word_width >= u32::from(width);
        if line_full || pending_word_overflow {
            let mut remaining = u32::from(width).saturating_sub(line_width);
            output.push_back(mem::take(&mut pending_line));
            line_width = 0;

            while let Some(front) = pending_whitespace.front() {
                if front.width > remaining {
                    break;
                }
                whitespace_width -= front.width;
                remaining -= front.width;
                pending_whitespace.pop_front();
            }
            if piece.whitespace && pending_whitespace.is_empty() {
                continue;
            }
        }

        if piece.whitespace {
            whitespace_width += piece_width;
            pending_whitespace.push_back(piece);
        } else {
            word_width += piece_width;
            pending_word.push(piece);
        }
        has_pending_word = !pending_word.is_empty();
    }

    if pending_line.is_empty() && pending_word.is_empty() && !pending_whitespace.is_empty() && trim
    {
        output.push_back(Vec::new());
    }
    if !pending_line.is_empty() || !trim {
        pending_line.extend(pending_whitespace);
    }
    pending_line.append(&mut pending_word);
    if !pending_line.is_empty() {
        output.push_back(pending_line);
    }
    if output.is_empty() {
        output.push_back(Vec::new());
    }

    output
        .into_iter()
        .map(|pieces| piece::line_from(source, &pieces))
        .collect()
}

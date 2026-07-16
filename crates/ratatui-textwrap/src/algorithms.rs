//! Wraps individual styled lines with the crate's line-breaking algorithms.
//!
//! [`WrapAlgorithm`] is the public choice stored by [`TextWrapper`](crate::TextWrapper).
//! `wrap_line` applies that choice to one source line. First-fit and optimal-fit share the same
//! styled fragments; Paragraph compatibility uses a separate grapheme-stream state machine.
//!
//! # Entry points
//!
//! - [`crate::algorithms::wrap_line`] dispatches a [`WrapAlgorithm`] selected at runtime.
//! - [`crate::algorithms::paragraph::wrap_line`] reproduces Paragraph's historical wrapping
//!   behavior.
//! - [`crate::algorithms::textwrap::wrap_first_fit`] and
//!   [`crate::algorithms::textwrap::wrap_optimal_fit`] invoke one textwrap-backed algorithm
//!   directly.
//!
//! These functions suit custom widgets and processing pipelines that already divide input into
//! independent line boundaries. Calls do not share wrapping state. All entry points allocate owned
//! span strings and return lines that can outlive `source`; use [`crate::TextWrapper`] when
//! top-level [`Text`](ratatui_core::text::Text) metadata must also be preserved.
//!
//! # Example
//!
//! ```
//! use ratatui::text::Line;
//! use ratatui_textwrap::algorithms::paragraph;
//!
//! let source = Line::from("alpha beta");
//! let wrapped = paragraph::wrap_line(&source, 8, true);
//!
//! assert_eq!(wrapped[0].to_string(), "alpha");
//! assert_eq!(wrapped[1].to_string(), "beta");
//! ```
//!
//! textwrap's [line-breaking overview] describes the generic first-fit and optimal-fit algorithms.
//! Ratatui supplies the grapheme segmentation, whitespace classification, and terminal-cell widths
//! used by this adapter.
//!
//! [line-breaking overview]: https://docs.rs/textwrap/0.16/textwrap/wrap_algorithms/index.html

use ratatui_core::text::Line;

/// Styled word fragments consumed by textwrap.
mod fragment;
/// Ratatui Paragraph's copied legacy line-wrapping algorithm.
pub mod paragraph;
/// Owned styled runs shared by line-breaking strategies.
mod piece;
/// Integration with textwrap's low-level line-breaking algorithms.
pub mod textwrap;

/// Chooses the line-breaking behavior used by [`TextWrapper`](crate::TextWrapper).
///
/// [`Self::FirstFit`] and [`Self::OptimalFit`] use textwrap with Ratatui graphemes and cell widths.
/// They split overlong words at grapheme boundaries and omit separator whitespace at the end of a
/// generated line. An indivisible grapheme wider than the requested width remains on an
/// overflowing line. [`Self::ParagraphCompat`] preserves Ratatui Paragraph's different streaming
/// behavior, including omitting such a grapheme.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum WrapAlgorithm {
    /// Fills each line greedily with the next fragment that fits.
    ///
    /// This is the default strategy. See [`crate::algorithms::textwrap::wrap_first_fit`] for its
    /// line-level contract.
    #[default]
    FirstFit,

    /// Scores every break point in a source line and chooses the lowest-cost layout.
    ///
    /// This can prefer a small overflow to a poorly balanced break. See
    /// [`crate::algorithms::textwrap::wrap_optimal_fit`] for its cost and fallback contract.
    OptimalFit,

    /// Reproduces Ratatui 0.30.2 Paragraph word reflow.
    ///
    /// This uses the maintained compatibility implementation documented by
    /// [`crate::algorithms::paragraph::wrap_line`], rather than textwrap.
    ParagraphCompat {
        /// Controls leading separator whitespace on continuation lines.
        ///
        /// `true` discards it and `false` preserves it, matching
        /// `ratatui::widgets::Wrap::trim`.
        trim: bool,
    },
}

/// Wraps one source line with `algorithm`.
///
/// Every returned line copies `source` style and alignment and owns its span strings. A positive
/// width produces at least one output line, including for an empty source line. A zero width
/// produces no lines, matching [`TextWrapper::wrap`](crate::TextWrapper::wrap).
///
/// This function allocates temporary wrapping state and the returned line and span collections.
/// Calls are independent and do not continue a partially wrapped source line from an earlier call.
#[must_use]
pub fn wrap_line(algorithm: WrapAlgorithm, source: &Line<'_>, width: u16) -> Vec<Line<'static>> {
    match algorithm {
        WrapAlgorithm::FirstFit => textwrap::wrap_first_fit(source, width),
        WrapAlgorithm::OptimalFit => textwrap::wrap_optimal_fit(source, width),
        WrapAlgorithm::ParagraphCompat { trim } => paragraph::wrap_line(source, width, trim),
    }
}

/// Unit tests for strategy dispatch.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that each public variant reaches its corresponding implementation.
    #[test]
    fn variants_dispatch_to_their_implementation() {
        let source = Line::from("alpha  beta gamma");

        assert_eq!(
            wrap_line(WrapAlgorithm::FirstFit, &source, 8),
            textwrap::wrap_first_fit(&source, 8)
        );
        assert_eq!(
            wrap_line(WrapAlgorithm::OptimalFit, &source, 8),
            textwrap::wrap_optimal_fit(&source, 8)
        );
        assert_eq!(
            wrap_line(WrapAlgorithm::ParagraphCompat { trim: true }, &source, 8),
            paragraph::wrap_line(&source, 8, true)
        );
    }
}

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

use ::textwrap::wrap_algorithms::{
    Penalties, wrap_first_fit as textwrap_first_fit, wrap_optimal_fit as textwrap_optimal_fit,
};
use ratatui_core::text::{Line, Span};

use super::fragment::{self, StyledFragment};

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

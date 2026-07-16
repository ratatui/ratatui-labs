//! Whole-text configuration and wrapping.
//!
//! [`TextWrapper`] accepts values that convert into [`Text`](ratatui_core::text::Text), preserves
//! their top-level metadata, and sends each source line to the selected [`WrapAlgorithm`]. A
//! zero-width request stops before line breaking and returns the same text style and alignment with
//! no lines.

use ratatui_core::text::Text;

use crate::{WrapAlgorithm, algorithms};

/// Configures how Ratatui text is wrapped into an owned value.
///
/// The wrapper stores a [`WrapAlgorithm`]; [`Self::wrap`] receives the terminal width for each
/// call. Reuse the same wrapper when a view changes size, and recompute the text when its source,
/// width, or algorithm changes. [`Self::new`] and [`Self::default`] select
/// [`WrapAlgorithm::FirstFit`].
///
/// The returned [`Text<'static>`](Text) owns every span string and can outlive borrowed input.
///
/// # Examples
///
/// ```
/// use ratatui::text::Line;
/// use ratatui::widgets::Paragraph;
/// use ratatui_textwrap::{TextWrapper, WrapAlgorithm};
///
/// let wrapping = TextWrapper::new().algorithm(WrapAlgorithm::OptimalFit);
/// let wrapped = wrapping.wrap(Line::from("alpha beta gamma"), 10);
/// let paragraph = Paragraph::new(wrapped);
/// ```
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TextWrapper {
    /// Strategy applied independently to every entry in [`Text::lines`].
    algorithm: WrapAlgorithm,
}

impl TextWrapper {
    /// Creates a wrapper that uses textwrap's first-fit algorithm.
    ///
    /// This is equivalent to [`Self::default`].
    pub const fn new() -> Self {
        Self {
            algorithm: WrapAlgorithm::FirstFit,
        }
    }

    /// Returns this wrapper configured to use `algorithm`.
    ///
    /// Existing configuration is replaced. The same algorithm applies independently to every
    /// entry in [`Text::lines`].
    #[must_use]
    pub const fn algorithm(mut self, algorithm: WrapAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Wraps `input` into owned text for `width` terminal cells.
    ///
    /// Strings, spans, lines, and text values use this same entry point through `Into<Text>`. Each
    /// entry in [`Text::lines`] is an independent wrapping boundary and produces at least one
    /// output line when `width` is positive. The output preserves [`Text::style`],
    /// [`Text::alignment`], and the style and alignment of each source
    /// [`Line`](ratatui_core::text::Line). Span styles remain attached across splits; adjacent
    /// spans with equal styles may be coalesced.
    ///
    /// A zero width returns no lines while retaining top-level text style and alignment. Ratatui's
    /// styled-grapheme iterator filters embedded control characters. A newline inside a manually
    /// constructed [`Span`](ratatui_core::text::Span) therefore does not create a source line;
    /// convert a string containing newlines or construct separate lines when that boundary matters.
    /// First-fit and optimal-fit retain a grapheme wider than `width` on an overflowing line;
    /// Paragraph compatibility omits it to reproduce Paragraph's behavior.
    ///
    /// # Allocation
    ///
    /// The returned `Text<'static>` owns its span strings and line collections. Wrapping also
    /// allocates temporary owned fragments. Caching the result avoids those allocations while the
    /// source, width, and algorithm remain unchanged.
    ///
    /// # Examples
    ///
    /// The output does not borrow from the input:
    ///
    /// ```
    /// use ratatui_textwrap::TextWrapper;
    ///
    /// let wrapped = {
    ///     let source = String::from("owned after wrapping");
    ///     TextWrapper::new().wrap(source.as_str(), 8)
    /// };
    ///
    /// assert_eq!(wrapped.lines.len(), 3);
    /// ```
    #[must_use]
    pub fn wrap<'a>(&self, input: impl Into<Text<'a>>, width: u16) -> Text<'static> {
        let input = input.into();
        if width == 0 {
            return Text {
                alignment: input.alignment,
                style: input.style,
                lines: Vec::new(),
            };
        }

        let lines = input
            .lines
            .iter()
            .flat_map(|line| algorithms::wrap_line(self.algorithm, line, width))
            .collect();
        Text {
            alignment: input.alignment,
            style: input.style,
            lines,
        }
    }
}

/// Unit tests for whole-text configuration.
#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that a later algorithm selection replaces the stored strategy.
    #[test]
    fn algorithm_replaces_the_previous_selection() {
        let wrapper = TextWrapper::new()
            .algorithm(WrapAlgorithm::OptimalFit)
            .algorithm(WrapAlgorithm::ParagraphCompat { trim: true });

        assert_eq!(
            wrapper.algorithm,
            WrapAlgorithm::ParagraphCompat { trim: true }
        );
    }
}

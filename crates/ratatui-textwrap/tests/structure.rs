//! Structural contracts that rendered buffers cannot show.
//!
//! These tests inspect ownership, `Into<Text>` conversions, metadata, span coalescing, defaults,
//! and line structure in the returned `Text`. Each setup stays beside its assertions because the
//! inputs are small and do not share policy.

use pretty_assertions::assert_eq;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui_textwrap::algorithms::{self, paragraph, textwrap};
use ratatui_textwrap::{TextWrapper, WrapAlgorithm};
use rstest::rstest;

/// Drops borrowed input before inspecting the owned output spans.
#[test]
fn output_owns_borrowed_input() {
    let wrapped = {
        let input = String::from("owned after wrapping");
        TextWrapper::new().wrap(input.as_str(), 8)
    };

    assert_eq!(wrapped.lines.len(), 3);
    assert!(wrapped.lines.iter().all(|line| {
        line.spans
            .iter()
            .all(|span| matches!(span.content, std::borrow::Cow::Owned(_)))
    }));
}

/// Exercises string, span, line, and text conversions through the same wrap method.
#[test]
fn common_text_conversions_share_the_wrap_method() {
    let wrapper = TextWrapper::new();
    let expected = wrapper.wrap("alpha beta", 5);

    assert_eq!(wrapper.wrap(String::from("alpha beta"), 5), expected);
    assert_eq!(wrapper.wrap(Span::raw("alpha beta"), 5), expected);
    assert_eq!(wrapper.wrap(Line::from("alpha beta"), 5), expected);
    assert_eq!(wrapper.wrap(Text::from("alpha beta"), 5), expected);
}

/// Checks text and source-line metadata on every generated line.
#[rstest]
#[case::first_fit(WrapAlgorithm::FirstFit)]
#[case::optimal_fit(WrapAlgorithm::OptimalFit)]
#[case::paragraph_preserve_whitespace(WrapAlgorithm::ParagraphCompat { trim: false })]
#[case::paragraph_trim_whitespace(WrapAlgorithm::ParagraphCompat { trim: true })]
fn text_and_line_metadata_survive_every_generated_line(#[case] algorithm: WrapAlgorithm) {
    let line_style = Style::new().fg(Color::Red);
    let text_style = Style::new().bg(Color::Blue);
    let source = Text::from(
        Line::from("alpha beta gamma")
            .style(line_style)
            .right_aligned(),
    )
    .style(text_style)
    .alignment(Alignment::Center);

    let wrapped = TextWrapper::new().algorithm(algorithm).wrap(source, 5);

    assert_eq!(wrapped.style, text_style);
    assert_eq!(wrapped.alignment, Some(Alignment::Center));
    assert_eq!(wrapped.lines.len(), 3);
    for line in wrapped.lines {
        assert_eq!(line.style, line_style);
        assert_eq!(line.alignment, Some(Alignment::Right));
    }
}

/// Checks that equal adjacent source styles produce one output span.
#[test]
fn adjacent_equal_styles_coalesce() {
    let style = Style::new().fg(Color::Green);
    let source = Line::from(vec![
        Span::styled("al", style),
        Span::styled("pha", style),
        Span::styled(" ", style),
        Span::styled("beta", style),
    ]);

    let wrapped = TextWrapper::new().wrap(source, 20);

    assert_eq!(
        wrapped.lines[0].spans,
        vec![Span::styled("alpha beta", style)]
    );
}

/// Checks the line shape and retained metadata at width zero.
#[test]
fn zero_width_preserves_text_metadata_without_lines() {
    let source = Text::from("alpha").yellow().alignment(Alignment::Right);
    let wrapped = TextWrapper::new().wrap(source, 0);

    assert_eq!(wrapped.style, Style::new().fg(Color::Yellow));
    assert_eq!(wrapped.alignment, Some(Alignment::Right));
    assert!(wrapped.lines.is_empty());
}

/// Checks that empty source lines remain hard boundaries at positive widths.
#[test]
fn positive_width_preserves_empty_source_line_boundaries() {
    let source = Text::from(vec![Line::from(""), Line::from("alpha"), Line::from("")]);

    let algorithm = WrapAlgorithm::ParagraphCompat { trim: true };
    let wrapped = TextWrapper::new().algorithm(algorithm).wrap(source, 20);

    assert_eq!(wrapped.lines.len(), 3);
    assert!(wrapped.lines[0].spans.is_empty());
    assert!(wrapped.lines[2].spans.is_empty());
}

/// Checks that the constructor and derived defaults both select first-fit.
#[test]
fn new_matches_default_and_first_fit() {
    assert_eq!(TextWrapper::new(), TextWrapper::default());
    assert_eq!(WrapAlgorithm::default(), WrapAlgorithm::FirstFit);
    assert_eq!(
        TextWrapper::new().wrap("  alpha", 20),
        TextWrapper::default()
            .algorithm(WrapAlgorithm::FirstFit)
            .wrap("  alpha", 20)
    );
}

/// Records the different mixed-width word behavior of textwrap and Paragraph compatibility.
#[test]
fn mixed_width_words_characterize_algorithm_boundaries() {
    let first_fit = TextWrapper::new().wrap("a界b", 2);
    let paragraph = TextWrapper::new()
        .algorithm(WrapAlgorithm::ParagraphCompat { trim: false })
        .wrap("a界b", 2);

    let first_fit_lines = first_fit
        .lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let paragraph_lines = paragraph
        .lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(first_fit_lines, ["a", "界", "b"]);
    assert_eq!(paragraph_lines, ["a", "界b"]);
}

/// Checks the algorithm-specific policy for a grapheme that cannot fit on an empty line.
#[rstest]
#[case::first_fit(WrapAlgorithm::FirstFit)]
#[case::optimal_fit(WrapAlgorithm::OptimalFit)]
fn textwrap_algorithms_preserve_graphemes_wider_than_the_line(#[case] algorithm: WrapAlgorithm) {
    let red = Style::new().fg(Color::Red);
    let blue = Style::new().fg(Color::Blue);
    let source = Line::from(vec![Span::styled("界", red), Span::styled("界", blue)]);

    let wrapped = TextWrapper::new().algorithm(algorithm).wrap(source, 1);

    assert_eq!(wrapped.lines.len(), 2);
    assert_eq!(wrapped.lines[0].spans, [Span::styled("界", red)]);
    assert_eq!(wrapped.lines[1].spans, [Span::styled("界", blue)]);
}

/// Checks that Paragraph compatibility retains Paragraph's lossy too-wide policy.
#[test]
fn paragraph_compat_omits_graphemes_wider_than_the_line() {
    let wrapped = TextWrapper::new()
        .algorithm(WrapAlgorithm::ParagraphCompat { trim: false })
        .wrap("界", 1);

    assert_eq!(wrapped.lines.len(), 1);
    assert_eq!(wrapped.lines[0].to_string(), "");
}

/// Exercises every public line-level entry point without the whole-text wrapper.
#[test]
fn public_algorithms_wrap_owned_lines_and_handle_zero_width() {
    let paragraph_lines = {
        let source = Line::from("alpha beta").right_aligned();
        algorithms::wrap_line(WrapAlgorithm::ParagraphCompat { trim: true }, &source, 8)
    };

    let paragraph_source = Line::from("alpha beta").right_aligned();
    assert_eq!(
        paragraph_lines,
        paragraph::wrap_line(&paragraph_source, 8, true)
    );
    assert_eq!(paragraph_lines[0].alignment, Some(Alignment::Right));
    assert!(paragraph_lines.iter().all(|line| {
        line.spans
            .iter()
            .all(|span| matches!(span.content, std::borrow::Cow::Owned(_)))
    }));

    let source = Line::from("alpha beta");
    assert_eq!(
        algorithms::wrap_line(WrapAlgorithm::FirstFit, &source, 8),
        textwrap::wrap_first_fit(&source, 8)
    );
    assert_eq!(
        algorithms::wrap_line(WrapAlgorithm::OptimalFit, &source, 8),
        textwrap::wrap_optimal_fit(&source, 8)
    );
    assert!(paragraph::wrap_line(&source, 0, true).is_empty());
    assert!(textwrap::wrap_first_fit(&source, 0).is_empty());
    assert!(textwrap::wrap_optimal_fit(&source, 0).is_empty());
}

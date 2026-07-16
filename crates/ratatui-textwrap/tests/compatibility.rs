//! Rendered comparisons between the available algorithms and Ratatui Paragraph.
//!
//! The copied compatibility path must match Paragraph for every case, width, and trim policy in the
//! corpus. The textwrap paths are allowed to differ; stable inventories make each known difference
//! visible in review.
//!
//! The development requirement accepts compatible Ratatui 0.30 releases. A lockfile update can
//! therefore reveal drift from the 0.30.2 source baseline instead of silently preserving an older
//! oracle.

use std::fmt::Write;

use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui_textwrap::{TextWrapper, WrapAlgorithm};
use rstest::rstest;

/// Compares the copied compatibility algorithm with both Paragraph trim policies.
#[rstest]
#[case::preserve_whitespace(false)]
#[case::trim_whitespace(true)]
fn paragraph_compat_matches_the_resolved_paragraph_oracle(#[case] trim: bool) {
    for (case, source) in cases() {
        let max_width = source.width().min(40) as u16 + 3;
        for width in 0..=max_width {
            let algorithm = WrapAlgorithm::ParagraphCompat { trim };
            let expected = render_paragraph(source.clone(), trim, width);
            let actual = render_wrapped(source.clone(), algorithm, width);
            assert_eq!(
                actual, expected,
                "Paragraph compatibility mismatch: case={case:?}, trim={trim}, width={width}"
            );
        }
    }
}

/// Checks the reviewed widths where first-fit differs from Paragraph.
#[test]
fn first_fit_paragraph_differences_are_reviewed() {
    let differences = paragraph_differences(WrapAlgorithm::FirstFit);
    assert_eq!(differences, FIRST_FIT_PARAGRAPH_DIFFERENCES);
}

/// Checks the reviewed widths where optimal-fit differs from first-fit.
#[test]
fn optimal_fit_first_fit_differences_are_reviewed() {
    let differences = algorithm_differences(WrapAlgorithm::OptimalFit, WrapAlgorithm::FirstFit);
    assert_eq!(differences, OPTIMAL_FIT_FIRST_FIT_DIFFERENCES);
}

/// Reviewed first-fit differences, grouped by input and Paragraph trim policy.
const FIRST_FIT_PARAGRAPH_DIFFERENCES: &str = concat!(
    "leading spaces | trim=false | widths=[1, 2, 3, 6, 7, 8, 9]\n",
    "leading spaces | trim=true | widths=[1, 5, 10, 11, 12, 13, 14, 15, 16, 17, 18]\n",
    "trailing spaces | trim=false | widths=[1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13]\n",
    "trailing spaces | trim=true | widths=[1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 13]\n",
    "repeated spaces | trim=false | widths=[1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 18, 19]\n",
    "repeated spaces | trim=true | widths=[2, 3, 4, 5, 6]\n",
    "whitespace only | trim=false | widths=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]\n",
    "styled whitespace | trim=false | widths=[1, 2, 3, 4, 5, 6, 7, 8, 9]\n",
    "styled whitespace | trim=true | widths=[3, 4, 6]\n",
);

/// Reviewed optimal-fit differences from first-fit, grouped by input.
const OPTIMAL_FIT_FIRST_FIT_DIFFERENCES: &str = concat!(
    "lookahead balance | widths=[9, 15, 16, 17, 43]\n",
    "punctuation | widths=[18]\n",
);

/// Formats Paragraph mismatches as a stable inventory of case names and widths.
fn paragraph_differences(algorithm: WrapAlgorithm) -> String {
    let mut output = String::new();
    for (case, source) in cases() {
        let max_width = source.width().min(40) as u16 + 3;
        for trim in [false, true] {
            let widths = (0..=max_width)
                .filter(|&width| {
                    render_wrapped(source.clone(), algorithm, width)
                        != render_paragraph(source.clone(), trim, width)
                })
                .collect::<Vec<_>>();
            if !widths.is_empty() {
                writeln!(output, "{case} | trim={trim} | widths={widths:?}")
                    .expect("writing to a String cannot fail");
            }
        }
    }
    output
}

/// Formats pairwise algorithm mismatches as a stable inventory of case names and widths.
fn algorithm_differences(left: WrapAlgorithm, right: WrapAlgorithm) -> String {
    let mut output = String::new();
    for (case, source) in cases() {
        let max_width = source.width().min(40) as u16 + 3;
        let widths = (0..=max_width)
            .filter(|&width| {
                render_wrapped(source.clone(), left, width)
                    != render_wrapped(source.clone(), right, width)
            })
            .collect::<Vec<_>>();
        if !widths.is_empty() {
            writeln!(output, "{case} | widths={widths:?}")
                .expect("writing to a String cannot fail");
        }
    }
    output
}

/// Renders Paragraph and returns its buffer and reported physical line count.
fn render_paragraph(source: Text<'static>, trim: bool, width: u16) -> (Buffer, usize) {
    let area = Rect::new(0, 0, width, 80);
    let paragraph = Paragraph::new(source).wrap(Wrap { trim });
    let line_count = paragraph.line_count(width);
    let mut buffer = Buffer::empty(area);
    paragraph.render(area, &mut buffer);
    (buffer, line_count)
}

/// Wraps with `algorithm`, then renders the owned text without Paragraph wrapping.
fn render_wrapped(source: Text<'static>, algorithm: WrapAlgorithm, width: u16) -> (Buffer, usize) {
    let area = Rect::new(0, 0, width, 80);
    let wrapped = TextWrapper::new().algorithm(algorithm).wrap(source, width);
    let line_count = wrapped.lines.len();
    let mut buffer = Buffer::empty(area);
    Paragraph::new(wrapped).render(area, &mut buffer);
    (buffer, line_count)
}

/// Returns the styled, whitespace, metadata, and Unicode comparison corpus.
fn cases() -> Vec<(&'static str, Text<'static>)> {
    let red = Style::new().fg(Color::Red);
    let blue = Style::new().fg(Color::Blue);
    let green = Style::new().fg(Color::Green);
    vec![
        ("empty input", Text::from("")),
        ("no text lines", Text::default()),
        (
            "blank and trailing source lines",
            Text::from(vec![Line::from(""), Line::from("tail"), Line::from("")]),
        ),
        (
            "multiple explicit lines",
            Text::from(vec![Line::from("alpha beta"), Line::from("gamma delta")]),
        ),
        (
            "embedded span newline",
            Text::from(Line::from(Span::raw("alpha\nbeta"))),
        ),
        (
            "embedded tab",
            Text::from(Line::from(Span::raw("alpha\tbeta"))),
        ),
        ("leading spaces", Text::from("     alpha beta")),
        ("trailing spaces", Text::from("alpha beta     ")),
        ("repeated spaces", Text::from("alpha      beta      gamma")),
        ("whitespace only", Text::from("               ")),
        (
            "long unbreakable word",
            Text::from("supercalifragilisticexpialidocious"),
        ),
        (
            "lookahead balance",
            Text::from("These few words will unfortunately not wrap nicely."),
        ),
        (
            "exact fit word and following space",
            Text::from("12345 6789"),
        ),
        ("punctuation", Text::from("Wait... what? yes; no!")),
        ("non-breaking space", Text::from("alpha\u{a0}beta gamma")),
        ("CJK double width", Text::from("你")),
        ("combining marks", Text::from("Cafe\u{301} au lait")),
        ("emoji ZWJ", Text::from("👨‍👩‍👧‍👦")),
        (
            "zero width spaces",
            Text::from("alpha\u{200b}beta\u{200b}gamma"),
        ),
        ("wide grapheme", Text::from("界")),
        ("zero width combining grapheme", Text::from("\u{301}")),
        (
            "styles around wrap points",
            Text::from(Line::from(vec![
                Span::styled("alpha", red),
                Span::styled(" ", green),
                Span::styled("beta", blue),
                Span::styled(" gamma", red),
            ])),
        ),
        (
            "one word across styles",
            Text::from(Line::from(vec![
                Span::styled("super", red),
                Span::styled("cali", blue),
                Span::styled("fragilistic", green),
            ])),
        ),
        (
            "styled whitespace",
            Text::from(Line::from(vec![
                Span::styled("alpha", red),
                Span::styled("      ", blue),
                Span::styled("beta", green),
            ])),
        ),
        (
            "text and line styles",
            Text::from(
                Line::from(vec![Span::raw("alpha "), Span::styled("beta gamma", red)]).on_blue(),
            )
            .style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ),
        (
            "left alignment",
            Text::from(Line::from("alpha beta gamma").left_aligned()),
        ),
        (
            "center alignment",
            Text::from(Line::from("alpha beta gamma").centered()),
        ),
        (
            "right alignment",
            Text::from(Line::from("alpha beta gamma").right_aligned()),
        ),
        (
            "top level alignment",
            Text::from("alpha beta gamma").alignment(Alignment::Center),
        ),
    ]
}

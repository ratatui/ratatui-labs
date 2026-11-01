//! Agreement between the Paragraph-compatible row paths and Ratatui Paragraph.
//!
//! Every path built on `ParagraphRows` must agree: owned `wrap_line`, borrowed rows, row counts,
//! and `WrappedLayout` windows. Windows are also rendered against Paragraph's own wrapped
//! scrolling.

use pretty_assertions::assert_eq;
use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui_textwrap::algorithms::paragraph::{self, ParagraphRows};
use ratatui_textwrap::algorithms::textwrap::{FirstFitRows, wrap_first_fit};
use ratatui_textwrap::{TextWrapper, WrapAlgorithm, WrappedText};

/// Checks that borrowed rows, owned lines, and counts describe the same layout.
#[test]
fn row_paths_agree() {
    for (case, text) in cases() {
        for trim in [false, true] {
            let mut rows = ParagraphRows::new(trim);
            for width in 0..=24 {
                for source in &text.lines {
                    let owned = paragraph::wrap_line(source, width, trim);
                    assert_eq!(
                        paragraph::count_rows(source, width, trim),
                        owned.len(),
                        "count: case={case:?} trim={trim} width={width}"
                    );

                    rows.reset(source, width);
                    let mut borrowed = Vec::new();
                    while let Some(row) = rows.next_row() {
                        let line = row.to_line(source);
                        assert_eq!(usize::from(row.width), line.width(), "case={case:?}");
                        borrowed.push(line);
                    }
                    assert_eq!(
                        cells(&borrowed),
                        cells(&owned),
                        "rows: case={case:?} trim={trim} width={width}"
                    );
                }
            }
        }
    }
}

/// Checks that the streaming first-fit producer matches owned first-fit wrapping exactly.
#[test]
fn first_fit_rows_match_owned_first_fit() {
    let mut rows = FirstFitRows::new();
    for (case, text) in cases() {
        for width in 0..=24 {
            for source in &text.lines {
                let owned = wrap_first_fit(source, width);
                rows.reset(source, width);
                let mut borrowed = Vec::new();
                while let Some(row) = rows.next_row() {
                    let line = row.to_line(source);
                    assert_eq!(usize::from(row.width), line.width(), "case={case:?}");
                    borrowed.push(line);
                }
                assert_eq!(
                    cells(&borrowed),
                    cells(&owned),
                    "case={case:?} width={width}"
                );
                rows.reset(source, width);
                assert_eq!(
                    rows.count_remaining(),
                    owned.len(),
                    "case={case:?} width={width}"
                );
            }
        }
    }
}

/// Checks `WrappedText` against a fresh wrap after every kind of edit.
#[test]
fn wrapped_text_stays_in_step_with_edits() {
    for algorithm in [
        WrapAlgorithm::FirstFit,
        WrapAlgorithm::ParagraphCompat { trim: false },
    ] {
        let source: Text = cases()
            .into_iter()
            .flat_map(|(_, text)| text.lines)
            .collect();
        let extra = cases()[3].1.lines[0].clone();
        let mut wrapped = WrappedText::new(source, 9, algorithm);
        let check = |wrapped: &mut WrappedText<'_>, step: &str| {
            let expected = TextWrapper::new()
                .algorithm(algorithm)
                .wrap((**wrapped).clone(), wrapped.width());
            let layout = wrapped.layout();
            assert_eq!(layout.total_rows(), expected.lines.len(), "{step}");
            for skip in (0..=expected.lines.len()).step_by(3) {
                let end = (skip + 4).min(expected.lines.len());
                let visible = layout.rows(skip, 4);
                assert_eq!(
                    cells(&visible.lines),
                    cells(&expected.lines[skip..end]),
                    "{step}"
                );
            }
            let start = expected.lines.len().saturating_sub(4);
            assert_eq!(
                cells(&wrapped.tail(4).lines),
                cells(&expected.lines[start..]),
                "{step}"
            );
        };
        check(&mut wrapped, "new");

        wrapped.line_mut(2).spans = vec![Span::raw("rewritten into several short words")];
        check(&mut wrapped, "line_mut");
        for line in wrapped.lines_mut(4..7) {
            line.spans.clear();
        }
        check(&mut wrapped, "lines_mut");
        wrapped.push_line(extra.clone());
        wrapped.extend([Line::from("a b"), Line::default()]);
        check(&mut wrapped, "push and extend");
        wrapped.splice(3..5, [extra.clone(), Line::from("x"), extra.clone()]);
        check(&mut wrapped, "splice grow");
        wrapped.splice(1..9, []);
        check(&mut wrapped, "splice shrink");
        wrapped.line_mut(0).spans = vec![Span::raw("edited then spliced")];
        wrapped.splice(0..0, [Line::from("inserted first")]);
        check(&mut wrapped, "edit before splice");
        wrapped.set_width(5);
        let pending = wrapped.layout_bounded(2).unwrap_err();
        assert_eq!(
            (pending.counted_lines(), pending.total_lines()),
            (2, wrapped.lines.len())
        );
        assert!(wrapped.try_layout().is_none());
        check(&mut wrapped, "set_width");
        wrapped.text_mut().lines.truncate(3);
        check(&mut wrapped, "text_mut");
    }
}

/// Checks `WrappedText` against a fresh wrap after every step of seeded random edit sequences.
///
/// Each sequence mixes every mutation the type offers, including edits that are recounted lazily
/// and splices that land before, inside, and after pending edits.
#[test]
fn wrapped_text_matches_fresh_wrap_under_random_edits() {
    let pool: Vec<Line<'static>> = cases()
        .into_iter()
        .flat_map(|(_, text)| text.lines)
        .collect();
    let algorithms = [
        WrapAlgorithm::FirstFit,
        WrapAlgorithm::OptimalFit,
        WrapAlgorithm::ParagraphCompat { trim: false },
        WrapAlgorithm::ParagraphCompat { trim: true },
    ];
    for seed in 1..=64_u64 {
        let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let mut next = move |bound: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            usize::try_from(state % bound.max(1) as u64).unwrap()
        };
        let algorithm = algorithms[next(algorithms.len())];
        let initial: Vec<Line> = (0..next(12))
            .map(|_| pool[next(pool.len())].clone())
            .collect();
        let mut wrapped = WrappedText::new(Text::from(initial), 1 + next(30) as u16, algorithm);

        for step in 0..40 {
            let len = wrapped.lines.len();
            let op = next(9);
            match op {
                0 if len > 0 => *wrapped.line_mut(next(len)) = pool[next(pool.len())].clone(),
                1 if len > 0 => {
                    let start = next(len);
                    let end = start + next(len - start + 1);
                    for line in wrapped.lines_mut(start..end) {
                        *line = pool[next(pool.len())].clone();
                    }
                }
                2 => wrapped.push_line(pool[next(pool.len())].clone()),
                3 => wrapped.extend((0..next(4)).map(|_| pool[next(pool.len())].clone())),
                4 | 5 => {
                    let start = next(len + 1);
                    let end = start + next(len - start + 1);
                    let replacement: Vec<Line> = (0..next(4))
                        .map(|_| pool[next(pool.len())].clone())
                        .collect();
                    wrapped.splice(start..end, replacement);
                }
                6 => wrapped.set_width(next(30) as u16),
                7 => {
                    let keep = next(len + 1);
                    wrapped.text_mut().lines.truncate(keep);
                }
                _ => {
                    let _ = wrapped.layout_bounded(next(5));
                }
            }

            let context = format!("seed={seed} step={step} op={op} algorithm={algorithm:?}");
            let expected = TextWrapper::new()
                .algorithm(algorithm)
                .wrap((*wrapped).clone(), wrapped.width());
            let height = 1 + next(6);
            let start = expected.lines.len().saturating_sub(height);
            assert_eq!(
                cells(&wrapped.tail(height).lines),
                cells(&expected.lines[start..]),
                "tail: {context}"
            );
            // Querying only every few steps leaves edits pending across later mutations.
            if next(3) == 0 {
                let layout = wrapped.layout();
                assert_eq!(
                    layout.total_rows(),
                    expected.lines.len(),
                    "total: {context}"
                );
                let skip = next(expected.lines.len() + 1);
                let end = (skip + height).min(expected.lines.len());
                assert_eq!(
                    cells(&layout.rows(skip, height).lines),
                    cells(&expected.lines[skip..end]),
                    "visible: {context}"
                );
            }
        }
    }
}

/// Checks every scroll window of the layout against Paragraph's wrapped scroll.
#[test]
fn layout_windows_match_paragraph_scroll() {
    let height = 4_u16;
    for (case, text) in cases() {
        for trim in [false, true] {
            for width in 1..=24 {
                let mut wrapped =
                    WrappedText::new(text.clone(), width, WrapAlgorithm::ParagraphCompat { trim });
                let layout = wrapped.layout();
                let total = layout.total_rows();
                let expected_total = Paragraph::new(text.clone())
                    .wrap(Wrap { trim })
                    .line_count(width);
                assert_eq!(total, expected_total, "total: case={case:?} width={width}");

                for skip in 0..=total {
                    let area = Rect::new(0, 0, width, height);
                    let mut expected = Buffer::empty(area);
                    Paragraph::new(text.clone())
                        .wrap(Wrap { trim })
                        .scroll((u16::try_from(skip).unwrap(), 0))
                        .render(area, &mut expected);

                    let visible = layout.rows(skip, usize::from(height));
                    let actual = render_rows(&visible.lines, area);
                    assert_eq!(
                        actual, expected,
                        "window: case={case:?} trim={trim} width={width} skip={skip}"
                    );
                }
            }
        }
    }
}

/// Checks lookup, per-line recounts, structural edits, and the layout states.
#[test]
fn layout_locates_rows_and_tracks_edits() {
    let algorithm = WrapAlgorithm::ParagraphCompat { trim: true };
    let mut wrapped = WrappedText::new("aaaa bbbb\ncc\ndddd eeee ffff", 5, algorithm);
    let layout = wrapped.layout();
    assert_eq!(layout.total_rows(), 6);
    let position = layout.locate(3).unwrap();
    assert_eq!((position.line, position.row), (2, 0));
    assert!(layout.locate(6).is_none());
    assert!(wrapped.try_layout().is_some());

    *wrapped.line_mut(1) = Line::from("cccc dddd eeee");
    assert!(wrapped.try_layout().is_none(), "an edit is pending");
    let layout = wrapped.layout();
    assert_eq!(layout.total_rows(), 8);
    assert_eq!(layout.first_row(2), 5);
    assert_eq!(layout.line_rows(1), 3);
    let position = layout.locate(4).unwrap();
    assert_eq!((position.line, position.row), (1, 2));

    wrapped.push_line("gggg hhhh");
    assert_eq!(wrapped.layout().total_rows(), 10);
    wrapped.splice(2.., []);
    assert_eq!(wrapped.layout().total_rows(), 5);

    let mut partial = WrappedText::new_uncounted(wrapped.clone().into_text(), 5, algorithm);
    let pending = partial.layout_bounded(1).unwrap_err();
    assert_eq!((pending.counted_lines(), pending.total_lines()), (1, 2));
    assert_eq!(pending.tail(1).lines[0].to_string(), "eeee");
    let layout = partial.layout_bounded(1).unwrap();
    assert_eq!(layout.total_rows(), 5);
}

/// Checks that every algorithm's layout windows and tails equal slices of owned wrapping.
#[test]
fn windows_and_tails_match_owned_wrapping_for_every_algorithm() {
    let algorithms = [
        WrapAlgorithm::FirstFit,
        WrapAlgorithm::OptimalFit,
        WrapAlgorithm::ParagraphCompat { trim: false },
        WrapAlgorithm::ParagraphCompat { trim: true },
    ];
    for algorithm in algorithms {
        let wrapper = TextWrapper::new().algorithm(algorithm);
        for (case, text) in cases() {
            let text = text.style(Style::new().italic()).right_aligned();
            for width in 0..=24 {
                let owned = wrapper.wrap(text.clone(), width);
                let mut wrapped = WrappedText::new(text.clone(), width, algorithm);
                let layout = wrapped.layout();
                assert_eq!(layout.total_rows(), owned.lines.len(), "case={case:?}");
                for height in [0, 1, 3] {
                    for skip in 0..=owned.lines.len() {
                        let end = (skip + height).min(owned.lines.len());
                        let visible = layout.rows(skip, height);
                        assert_eq!(visible.style, text.style);
                        assert_eq!(visible.alignment, text.alignment);
                        assert_eq!(
                            cells(&visible.lines),
                            cells(&owned.lines[skip..end]),
                            "window: {algorithm:?} case={case:?} width={width} skip={skip}"
                        );
                    }
                    let start = owned.lines.len().saturating_sub(height);
                    let expected = cells(&owned.lines[start..]);
                    let tail = wrapper.tail(&text, width, height);
                    assert_eq!(tail.style, text.style);
                    assert_eq!(cells(&tail.lines), expected, "tail: {algorithm:?} {case:?}");
                    assert_eq!(cells(&layout.tail(height).lines), expected);
                }
            }
        }
    }
}

/// Draws already-wrapped rows the way Paragraph's wrapped path does.
///
/// Paragraph can admit a wide grapheme past the row width, so rendering through an unwrapped
/// Paragraph would truncate differently. Cells beyond the area are dropped.
fn render_rows(rows: &[Line<'_>], area: Rect) -> Buffer {
    let mut buffer = Buffer::empty(area);
    for (y, row) in (0..area.height).zip(rows) {
        let row_width = u16::try_from(row.width()).unwrap();
        let mut x = match row.alignment {
            Some(Alignment::Center) => (area.width / 2).saturating_sub(row_width / 2),
            Some(Alignment::Right) => area.width.saturating_sub(row_width),
            _ => 0,
        };
        for grapheme in row.styled_graphemes(Style::new()) {
            let width = grapheme.symbol.cell_width();
            if width == 0 {
                continue;
            }
            if x < area.width {
                buffer[(x, y)]
                    .set_symbol(grapheme.symbol)
                    .set_style(grapheme.style);
            }
            x += width;
        }
    }
    buffer
}

/// Concatenates rendered content and per-grapheme style for comparison across span partitions.
fn cells(lines: &[Line<'_>]) -> Vec<Vec<(String, Style)>> {
    lines
        .iter()
        .map(|line| {
            line.styled_graphemes(Style::new())
                .map(|grapheme| (grapheme.symbol.to_owned(), grapheme.style))
                .collect()
        })
        .collect()
}

/// Yazi-style inputs: expanded tabs, multi-span highlighting, long lines, wide graphemes.
fn cases() -> Vec<(&'static str, Text<'static>)> {
    let keyword = Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD);
    let ident = Style::new().fg(Color::Cyan);
    let syntect = Line::from(vec![
        Span::styled("fn", keyword),
        Span::raw(" "),
        Span::styled("main", ident),
        Span::raw("() {"),
        Span::styled(
            "    let x = \"hello world\";",
            Style::new().fg(Color::Green),
        ),
        Span::raw(" }"),
    ]);
    let ansi = Line::from(vec![
        "error".red().bold(),
        ": ".into(),
        "cannot find value ".into(),
        "`x`".yellow(),
        " in this scope".into(),
    ]);
    let long = "lorem ipsum dolor sit amet ".repeat(12);
    vec![
        (
            "expanded tabs",
            Text::from("    indented\tline\n        deeper  x"),
        ),
        (
            "syntect",
            Text::from(vec![syntect.clone(), Line::default(), syntect]),
        ),
        ("ansi", Text::from(ansi)),
        ("long line", Text::from(long)),
        ("long word", Text::from("x".repeat(70))),
        ("cjk", Text::from("日本語のテキスト 折り返し テスト")),
        ("emoji", Text::from("👩‍👩‍👧 family 🇯🇵 flag ok👍🏽")),
        ("whitespace", Text::from("   \n  a  \n\n")),
        (
            "styled line",
            Text::from(Line::from("aligned and styled text").centered().on_blue()),
        ),
    ]
}

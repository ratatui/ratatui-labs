//! Scrolling wrapped logs: Paragraph, yazi, the previous owned path, and the row index.
//!
//! Each group runs over generated log text sized to 16k, 32k, 48k, and 65535 wrapped rows (the
//! most Paragraph can scroll with its `u16` offset) at 100 columns and a 50-row viewport.
//! Implementations:
//!
//! - `paragraph`: `ratatui::widgets::Paragraph` with `Wrap` and `scroll`, rendered by reference.
//! - `yazi`: yazi's vendored copy of Ratatui's `WordWrapper`, wrapping from the first line and
//!   skipping rows, as yazi's preview does (`vendor/yazi_wrapper.rs`, yazi `ec4ce8f2`).
//! - `pre`: this crate before index-based wrapping (`old/`), which wraps into owned lines. A frame
//!   then slices the cached owned rows.
//! - `post_tail`: [`TextWrapper::tail`], walking back from the end with no index.
//! - `wrapped_text`: [`WrappedText`], which owns the text and keeps its index in step.
//!
//! In the append and edit groups, `paragraph` counts rows with `line_count` and renders scrolled
//! to the bottom; building the `Paragraph` from the changed text is excluded from its timing.
//!
//! The `code` group repeats the frame and append cases on this crate's own Rust source with one
//! styled span per token, approximating syntect-highlighted previews such as yazi's.
//!
//! The `long_line` group appends one 500 KB line to a log and views it from inside the line and
//! from the bottom.
//!
//! Every frame renders its rows into a reused buffer so the groups compare full frame costs.

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui_textwrap::algorithms::{self, paragraph};
use ratatui_textwrap::{TextWrapper, WrapAlgorithm, WrappedText};

/// This crate's `ParagraphCompat` implementation before index-based wrapping.
#[allow(dead_code, clippy::all)]
mod old {
    pub mod paragraph;
    mod piece;
}

/// yazi's vendored Ratatui `WordWrapper`.
#[allow(dead_code, clippy::all)]
#[rustfmt::skip]
#[path = "vendor/yazi_wrapper.rs"]
mod yazi;

use yazi::LineComposer;

/// Viewport width.
const WIDTH: u16 = 100;
/// Viewport height.
const HEIGHT: u16 = 50;
/// Paragraph's `Wrap { trim }` used by every implementation.
const TRIM: bool = false;
/// Target wrapped row counts, up to Paragraph's `u16` scroll limit.
///
/// Set `SCROLL_BENCH=full` for all four sizes and longer measurement; the default quick run uses
/// the smallest and largest, which is enough to see whether a cost is flat or linear.
fn sizes() -> Vec<usize> {
    if full() {
        vec![16_384, 32_768, 49_152, 65_535]
    } else {
        vec![16_384, 65_535]
    }
}

/// Returns whether `SCROLL_BENCH=full` requests the complete run.
fn full() -> bool {
    std::env::var("SCROLL_BENCH").is_ok_and(|value| value == "full")
}

/// Builds deterministic log-like text: styled timestamp, level, target, and a message of varying
/// length with occasional CJK words and very long lines.
fn log_text(lines: usize) -> Text<'static> {
    const WORDS: [&str; 12] = [
        "request", "handled", "in", "ms", "user", "id", "cache", "miss", "retrying", "連続",
        "接続", "ok",
    ];
    let mut seed = 0x2545_f491_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    (0..lines)
        .map(|i| {
            let words = match next() % 20 {
                0 => 200 + next() % 200,
                1..=3 => 0,
                _ => 3 + next() % 30,
            };
            let message = (0..words)
                .map(|_| WORDS[(next() % 12) as usize])
                .collect::<Vec<_>>()
                .join(" ");
            let (level, color) = match next() % 10 {
                0 => ("ERROR", Color::Red),
                1 | 2 => ("WARN ", Color::Yellow),
                _ => ("INFO ", Color::Green),
            };
            Line::from(vec![
                Span::raw(format!("2026-10-31T12:{:02}:{:02}Z ", i / 60 % 60, i % 60)).dim(),
                Span::styled(level, Style::new().fg(color).bold()),
                Span::raw(" app::server: ").cyan(),
                Span::raw(message),
            ])
        })
        .collect()
}

/// Rust sources from this crate, used as realistic previewed code.
const SOURCES: [&str; 6] = [
    include_str!("../src/algorithms/paragraph.rs"),
    include_str!("../src/algorithms/textwrap.rs"),
    include_str!("../src/counts.rs"),
    include_str!("../src/index.rs"),
    include_str!("../src/wrapped.rs"),
    include_str!("../src/wrapper.rs"),
];

/// Builds syntax-highlighted code text whose wrapped rows reach at most `rows`.
///
/// Mimics what a previewer such as yazi hands to the wrapper after syntect highlighting: real
/// source lines, mostly ASCII, indented, with one styled span per token. The token classes are
/// approximate; the span density and line-length distribution are what matter here.
fn code_rows(rows: usize) -> Text<'static> {
    let mut text = Text::default();
    let mut total = 0;
    'fill: loop {
        for source in SOURCES {
            for line in source.lines() {
                let line = highlight(line);
                total += paragraph::count_rows(&line, WIDTH, TRIM);
                if total > rows {
                    break 'fill;
                }
                text.lines.push(line);
            }
        }
    }
    text
}

/// Splits one source line into styled token spans, like a syntax highlighter's output.
fn highlight(line: &str) -> Line<'static> {
    const KEYWORDS: [&str; 16] = [
        "fn", "let", "mut", "pub", "use", "impl", "for", "in", "if", "else", "match", "return",
        "struct", "enum", "self", "Self",
    ];
    if let Some(start) = line.find("//") {
        let mut spans = highlight(&line[..start]).spans;
        spans.push(Span::raw(line[start..].to_owned()).dark_gray().italic());
        return Line::from(spans);
    }
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(first) = rest.chars().next() {
        let end = if first == '"' {
            rest[1..].find('"').map_or(rest.len(), |end| end + 2)
        } else if first.is_alphanumeric() || first == '_' {
            rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len())
        } else if first == ' ' {
            rest.find(|c: char| c != ' ').unwrap_or(rest.len())
        } else {
            first.len_utf8()
        };
        let token = &rest[..end];
        let span = Span::raw(token.to_owned());
        spans.push(if first == '"' {
            span.green()
        } else if KEYWORDS.contains(&token) {
            span.magenta().bold()
        } else if first.is_ascii_digit() {
            span.yellow()
        } else if first.is_uppercase() {
            span.cyan()
        } else if first.is_alphanumeric() || first == '_' || first == ' ' {
            span
        } else {
            span.gray()
        });
        rest = &rest[end..];
    }
    Line::from(spans)
}

/// Builds log text whose wrapped rows reach at most `rows`, the largest such prefix.
fn log_rows(rows: usize) -> Text<'static> {
    let mut text = log_text(rows);
    let mut total = 0;
    let keep = text
        .lines
        .iter()
        .take_while(|line| {
            total += paragraph::count_rows(line, WIDTH, TRIM);
            total <= rows
        })
        .count();
    text.lines.truncate(keep);
    text
}

/// Renders already-wrapped text into `buffer`.
fn render(text: Text<'_>, buffer: &mut Buffer) {
    Paragraph::new(text).render(buffer.area, buffer);
}

/// Draws the last rows of a wrapped Paragraph: count every row, then scroll to the bottom.
fn paragraph_bottom(paragraph: Paragraph<'_>, buffer: &mut Buffer) {
    let rows = paragraph.line_count(WIDTH);
    let skip = u16::try_from(rows.saturating_sub(usize::from(HEIGHT))).expect("u16 rows");
    paragraph.scroll((skip, 0)).render(buffer.area, buffer);
}

/// Wraps `text` with yazi's `WordWrapper`, skips `skip` rows, and returns the grapheme count of
/// the next `HEIGHT` rows. With `skip == usize::MAX` it counts every row instead.
fn yazi_rows(text: &Text<'_>, skip: usize) -> usize {
    let lines = text
        .lines
        .iter()
        .map(|line| (line.styled_graphemes(Style::new()), Alignment::Left));
    let mut wrapper = yazi::WordWrapper::new(lines, WIDTH, TRIM);
    let mut rows = 0;
    let mut graphemes = 0;
    while let Some(row) = wrapper.next_line() {
        if rows >= skip {
            graphemes += row.graphemes.len();
            if rows - skip + 1 == usize::from(HEIGHT) {
                break;
            }
        }
        rows += 1;
    }
    if skip == usize::MAX { rows } else { graphemes }
}

/// Wraps every line into owned rows with the previous implementation.
fn pre_wrap(text: &Text<'_>) -> Vec<Line<'static>> {
    text.lines
        .iter()
        .flat_map(|line| old::paragraph::wrap_line(line, WIDTH, TRIM))
        .collect()
}

/// Slices the cached owned rows for a frame.
fn pre_window(cached: &[Line<'static>], skip: usize) -> Text<'static> {
    let end = (skip + usize::from(HEIGHT)).min(cached.len());
    Text::from(cached[skip.min(end)..end].to_vec())
}

/// Cost of producing every row once: building a cache, resizing, or answering `line_count`.
fn rewrap_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("rewrap_all");
    group.sample_size(10);
    for size in sizes() {
        let text = log_rows(size);
        let paragraph = Paragraph::new(text.clone()).wrap(Wrap { trim: TRIM });
        group.bench_function(BenchmarkId::new("paragraph_line_count", size), |b| {
            b.iter(|| black_box(paragraph.line_count(WIDTH)));
        });
        group.bench_function(BenchmarkId::new("yazi_count", size), |b| {
            b.iter(|| black_box(yazi_rows(&text, usize::MAX)));
        });
        group.bench_function(BenchmarkId::new("pre_owned_wrap", size), |b| {
            b.iter(|| black_box(pre_wrap(&text)));
        });
        group.bench_function(BenchmarkId::new("post_owned_wrap", size), |b| {
            b.iter(|| {
                let rows: Vec<_> = text
                    .lines
                    .iter()
                    .flat_map(|line| paragraph::wrap_line(line, WIDTH, TRIM))
                    .collect();
                black_box(rows)
            });
        });
        // Counting every line's rows, excluding the copy of the text into `WrappedText`.
        let textwrap_algorithms = [
            ("", algorithm()),
            ("first_fit_", WrapAlgorithm::FirstFit),
            ("optimal_fit_", WrapAlgorithm::OptimalFit),
        ];
        for (prefix, wrap_algorithm) in textwrap_algorithms {
            group.bench_function(
                BenchmarkId::new(format!("{prefix}wrapped_text"), size),
                |b| {
                    b.iter_batched(
                        || text.clone(),
                        |text| black_box(WrappedText::new(text, WIDTH, wrap_algorithm)),
                        BatchSize::LargeInput,
                    );
                },
            );
            if prefix.is_empty() {
                continue;
            }
            // The same counts through owned lines, as the textwrap algorithms did before.
            group.bench_function(
                BenchmarkId::new(format!("{prefix}owned_count"), size),
                |b| {
                    b.iter(|| {
                        let rows: usize = text
                            .lines
                            .iter()
                            .map(|line| algorithms::wrap_line(wrap_algorithm, line, WIDTH).len())
                            .sum();
                        black_box(rows)
                    });
                },
            );
        }
    }
    group.finish();
}

/// Cost of one frame of unchanged text scrolled to the top, middle, or bottom.
fn frame(c: &mut Criterion) {
    let wrapper = TextWrapper::new().algorithm(WrapAlgorithm::ParagraphCompat { trim: TRIM });
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("frame");
    group.sample_size(10);
    for size in sizes() {
        let text = log_rows(size);
        let cached = pre_wrap(&text);
        let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
        let layout = wrapped.layout();
        let total = layout.total_rows();
        let bottom = total.saturating_sub(usize::from(HEIGHT));
        for (position, skip) in [("top", 0), ("middle", total / 2), ("bottom", bottom)] {
            let id = |name: &str| BenchmarkId::new(format!("{name}/{position}"), size);
            let mut buffer = Buffer::empty(area);
            {
                let scroll = u16::try_from(skip).expect("sizes stay within u16 rows");
                let paragraph = Paragraph::new(text.clone())
                    .wrap(Wrap { trim: TRIM })
                    .scroll((scroll, 0));
                group.bench_function(id("paragraph"), |b| {
                    b.iter(|| (&paragraph).render(area, &mut buffer));
                });
            }
            group.bench_function(id("yazi"), |b| b.iter(|| black_box(yazi_rows(&text, skip))));
            group.bench_function(id("pre_cached"), |b| {
                b.iter(|| render(pre_window(&cached, skip), &mut buffer));
            });
            group.bench_function(id("wrapped_text"), |b| {
                b.iter(|| render(layout.rows(skip, usize::from(HEIGHT)), &mut buffer));
            });
            if position == "bottom" {
                group.bench_function(id("post_tail"), |b| {
                    b.iter(|| render(wrapper.tail(&text, WIDTH, usize::from(HEIGHT)), &mut buffer));
                });
            }
        }
    }
    group.finish();
}

/// Cost of a log that sticks to the bottom: append a line, then draw the last rows.
///
/// Each iteration removes the appended line afterwards so the text keeps its size; removal is
/// cheap for every implementation and is included in the timing.
fn append_stick_bottom(c: &mut Criterion) {
    let wrapper = TextWrapper::new().algorithm(WrapAlgorithm::ParagraphCompat { trim: TRIM });
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("append_stick_bottom");
    group.sample_size(10);
    for size in sizes() {
        let mut text = log_rows(size);
        let appended = log_text(16).lines;
        let mut buffer = Buffer::empty(area);

        group.bench_function(BenchmarkId::new("paragraph", size), |b| {
            let mut i = 0;
            b.iter_batched(
                || {
                    let mut grown = text.clone();
                    grown.lines.push(appended[i % appended.len()].clone());
                    i += 1;
                    Paragraph::new(grown).wrap(Wrap { trim: TRIM })
                },
                |paragraph| paragraph_bottom(paragraph, &mut buffer),
                BatchSize::LargeInput,
            );
        });
        group.bench_function(BenchmarkId::new("yazi", size), |b| {
            let mut i = 0;
            b.iter(|| {
                text.lines.push(appended[i % appended.len()].clone());
                i += 1;
                let total = yazi_rows(&text, usize::MAX);
                black_box(yazi_rows(&text, total.saturating_sub(usize::from(HEIGHT))));
                text.lines.pop();
            });
        });
        group.bench_function(BenchmarkId::new("pre_cached", size), |b| {
            let mut cached = pre_wrap(&text);
            let mut i = 0;
            b.iter(|| {
                let new_rows =
                    old::paragraph::wrap_line(&appended[i % appended.len()], WIDTH, TRIM);
                i += 1;
                let added = new_rows.len();
                cached.extend(new_rows);
                let skip = cached.len().saturating_sub(usize::from(HEIGHT));
                render(pre_window(&cached, skip), &mut buffer);
                cached.truncate(cached.len() - added);
            });
        });
        group.bench_function(BenchmarkId::new("post_tail", size), |b| {
            let mut i = 0;
            b.iter(|| {
                text.lines.push(appended[i % appended.len()].clone());
                i += 1;
                render(wrapper.tail(&text, WIDTH, usize::from(HEIGHT)), &mut buffer);
                text.lines.pop();
            });
        });
        group.bench_function(BenchmarkId::new("wrapped_text", size), |b| {
            let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
            let mut i = 0;
            b.iter(|| {
                wrapped.push_line(appended[i % appended.len()].clone());
                i += 1;
                draw_bottom(&mut wrapped, &mut buffer);
                let last = wrapped.lines.len() - 1;
                wrapped.splice(last.., []);
            });
        });
    }
    group.finish();
}

/// Cost of editing a line in the middle of the text, then drawing the bottom.
///
/// The previous implementation had no row offsets per source line, so it rewraps everything.
fn edit_middle(c: &mut Criterion) {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("edit_middle");
    group.sample_size(10);
    for size in sizes() {
        let mut text = log_rows(size);
        let replacements = log_text(16).lines;
        let middle = text.lines.len() / 2;
        let mut buffer = Buffer::empty(area);

        group.bench_function(BenchmarkId::new("paragraph", size), |b| {
            let mut i = 0;
            b.iter_batched(
                || {
                    let mut edited = text.clone();
                    edited.lines[middle] = replacements[i % replacements.len()].clone();
                    i += 1;
                    Paragraph::new(edited).wrap(Wrap { trim: TRIM })
                },
                |paragraph| paragraph_bottom(paragraph, &mut buffer),
                BatchSize::LargeInput,
            );
        });
        group.bench_function(BenchmarkId::new("yazi", size), |b| {
            let mut i = 0;
            b.iter(|| {
                text.lines[middle] = replacements[i % replacements.len()].clone();
                i += 1;
                let total = yazi_rows(&text, usize::MAX);
                black_box(yazi_rows(&text, total.saturating_sub(usize::from(HEIGHT))));
            });
        });
        group.bench_function(BenchmarkId::new("pre_rewrap", size), |b| {
            let mut i = 0;
            b.iter(|| {
                text.lines[middle] = replacements[i % replacements.len()].clone();
                i += 1;
                let cached = pre_wrap(&text);
                let skip = cached.len().saturating_sub(usize::from(HEIGHT));
                render(pre_window(&cached, skip), &mut buffer);
            });
        });
        group.bench_function(BenchmarkId::new("wrapped_text", size), |b| {
            let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
            let mut i = 0;
            b.iter(|| {
                *wrapped.line_mut(middle) = replacements[i % replacements.len()].clone();
                i += 1;
                draw_bottom(&mut wrapped, &mut buffer);
            });
        });
    }
    group.finish();
}

/// Cost of inserting a line in the middle of the text, then drawing the bottom.
///
/// Each iteration removes the inserted line afterwards so the text keeps its size; removal is
/// included in the timing for the incremental implementations.
fn insert_middle(c: &mut Criterion) {
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("insert_middle");
    group.sample_size(10);
    for size in sizes() {
        let mut text = log_rows(size);
        let inserted = log_text(16).lines;
        let middle = text.lines.len() / 2;
        let mut buffer = Buffer::empty(area);

        group.bench_function(BenchmarkId::new("paragraph", size), |b| {
            let mut i = 0;
            b.iter_batched(
                || {
                    let mut grown = text.clone();
                    grown
                        .lines
                        .insert(middle, inserted[i % inserted.len()].clone());
                    i += 1;
                    Paragraph::new(grown).wrap(Wrap { trim: TRIM })
                },
                |paragraph| paragraph_bottom(paragraph, &mut buffer),
                BatchSize::LargeInput,
            );
        });
        group.bench_function(BenchmarkId::new("pre_rewrap", size), |b| {
            let mut i = 0;
            b.iter(|| {
                text.lines
                    .insert(middle, inserted[i % inserted.len()].clone());
                i += 1;
                let cached = pre_wrap(&text);
                let skip = cached.len().saturating_sub(usize::from(HEIGHT));
                render(pre_window(&cached, skip), &mut buffer);
                text.lines.remove(middle);
            });
        });
        group.bench_function(BenchmarkId::new("wrapped_text", size), |b| {
            let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
            let mut i = 0;
            b.iter(|| {
                wrapped.splice(middle..middle, [inserted[i % inserted.len()].clone()]);
                i += 1;
                draw_bottom(&mut wrapped, &mut buffer);
                wrapped.splice(middle..=middle, []);
            });
        });
    }
    group.finish();
}

/// Previewing highlighted source code: frames at the bottom and middle, and appending a line.
fn code(c: &mut Criterion) {
    let wrapper = TextWrapper::new().algorithm(algorithm());
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("code");
    group.sample_size(10);
    for size in sizes() {
        let text = code_rows(size);
        let cached = pre_wrap(&text);
        let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
        let total = wrapped.layout().total_rows();
        let mut buffer = Buffer::empty(area);
        for (position, skip) in [
            ("middle", total / 2),
            ("bottom", total.saturating_sub(usize::from(HEIGHT))),
        ] {
            let id = |name: &str| BenchmarkId::new(format!("{name}_{position}"), size);
            let scroll = u16::try_from(skip).expect("sizes stay within u16 rows");
            let paragraph = Paragraph::new(text.clone())
                .wrap(Wrap { trim: TRIM })
                .scroll((scroll, 0));
            group.bench_function(id("paragraph"), |b| {
                b.iter(|| (&paragraph).render(area, &mut buffer));
            });
            group.bench_function(id("yazi"), |b| b.iter(|| black_box(yazi_rows(&text, skip))));
            group.bench_function(id("pre_cached"), |b| {
                b.iter(|| render(pre_window(&cached, skip), &mut buffer));
            });
            let layout = wrapped.layout();
            group.bench_function(id("wrapped_text"), |b| {
                b.iter(|| render(layout.rows(skip, usize::from(HEIGHT)), &mut buffer));
            });
        }
        group.bench_function(BenchmarkId::new("tail_bottom", size), |b| {
            b.iter(|| render(wrapper.tail(&text, WIDTH, usize::from(HEIGHT)), &mut buffer));
        });
        let appended = code_rows(200).lines;
        group.bench_function(BenchmarkId::new("wrapped_text_append", size), |b| {
            let mut i = 0;
            b.iter(|| {
                wrapped.push_line(appended[i % appended.len()].clone());
                i += 1;
                draw_bottom(&mut wrapped, &mut buffer);
                let last = wrapped.lines.len() - 1;
                wrapped.splice(last.., []);
            });
        });
    }
    group.finish();
}

/// A 500 KB single line (minified code, a long log record) at the end of a log.
///
/// `WrappedText` draws rows of long lines from the byte bounds it recorded while counting, so a
/// frame scans about a screenful of the line. `TextWrapper::tail` keeps no state and rewraps the
/// whole last line every frame, which is the cost every path paid before row bounds.
fn long_line(c: &mut Criterion) {
    let wrapper = TextWrapper::new().algorithm(algorithm());
    let area = Rect::new(0, 0, WIDTH, HEIGHT);
    let mut group = c.benchmark_group("long_line");
    group.sample_size(10);
    let mut text = log_rows(4_096);
    let word = "token=value; ";
    text.lines
        .push(Line::from(word.repeat(500_000 / word.len())).cyan());
    let cached = pre_wrap(&text);
    let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
    let layout = wrapped.layout();
    let total = layout.total_rows();
    let inside = total - layout.line_rows(text.lines.len() - 1) / 2;
    let size = text.lines.last().map_or(0, |line| line.width());
    let mut buffer = Buffer::empty(area);
    for (position, skip) in [
        ("inside", inside),
        ("bottom", total.saturating_sub(usize::from(HEIGHT))),
    ] {
        let id = |name: &str| BenchmarkId::new(format!("{name}_{position}"), size);
        let scroll = u16::try_from(skip).expect("the text stays within u16 rows");
        let paragraph = Paragraph::new(text.clone())
            .wrap(Wrap { trim: TRIM })
            .scroll((scroll, 0));
        group.bench_function(id("paragraph"), |b| {
            b.iter(|| (&paragraph).render(area, &mut buffer));
        });
        group.bench_function(id("pre_cached"), |b| {
            b.iter(|| render(pre_window(&cached, skip), &mut buffer));
        });
        group.bench_function(id("wrapped_text"), |b| {
            b.iter(|| render(layout.rows(skip, usize::from(HEIGHT)), &mut buffer));
        });
    }
    group.bench_function(BenchmarkId::new("tail_bottom", size), |b| {
        b.iter(|| render(wrapper.tail(&text, WIDTH, usize::from(HEIGHT)), &mut buffer));
    });
    group.finish();
}

/// The algorithm every implementation is compared under.
const fn algorithm() -> WrapAlgorithm {
    WrapAlgorithm::ParagraphCompat { trim: TRIM }
}

/// Brings `wrapped` up to date and draws its last rows through the layout.
fn draw_bottom(wrapped: &mut WrappedText<'_>, buffer: &mut Buffer) {
    render(wrapped.layout().tail(usize::from(HEIGHT)), buffer);
}

/// Reports retained heap bytes for the owned cache and the index once, before timing.
fn report_memory() {
    for size in sizes() {
        let text = log_rows(size);
        let source = owned_bytes(&text.lines);
        let cached = owned_bytes(&pre_wrap(&text));
        let mut wrapped = WrappedText::new(text.clone(), WIDTH, algorithm());
        let total_rows = wrapped.layout().total_rows();
        // One usize per line, plus two per 512-line chunk.
        let index_bytes = text.lines.len() * size_of::<usize>();
        eprintln!(
            "memory {} lines ({} rows): source {} KiB, pre owned cache {} KiB, index {} KiB",
            text.lines.len(),
            total_rows,
            source / 1024,
            cached / 1024,
            index_bytes / 1024
        );
    }
}

/// Approximates the heap retained by owned lines: line and span vectors plus string contents.
fn owned_bytes(lines: &[Line<'_>]) -> usize {
    size_of_val(lines)
        + lines
            .iter()
            .map(|line| {
                line.spans.capacity() * size_of::<Span>()
                    + line
                        .spans
                        .iter()
                        .map(|span| span.content.len())
                        .sum::<usize>()
            })
            .sum::<usize>()
}

/// Runs the memory report, then every timing group.
fn benches(c: &mut Criterion) {
    report_memory();
    rewrap_all(c);
    frame(c);
    append_stick_bottom(c);
    edit_middle(c);
    insert_middle(c);
    code(c);
    long_line(c);
}

/// Measurement budget per benchmark: short by default, longer with `SCROLL_BENCH=full`.
///
/// Every benchmark gets the same fixed budget whatever its cost, so total run time is roughly the
/// number of benchmarks times the budget. Fast operations run many iterations inside it; slow
/// ones are capped by the 10-sample minimum.
fn config() -> Criterion {
    let (warm_up, measurement) = if full() { (500, 2000) } else { (200, 700) };
    Criterion::default()
        .warm_up_time(Duration::from_millis(warm_up))
        .measurement_time(Duration::from_millis(measurement))
}

criterion_group! {
    name = scroll;
    config = config();
    targets = benches
}
criterion_main!(scroll);

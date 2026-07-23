//! Shared deterministic fixtures and workflow operations for text wrapping benchmarks.

#![expect(
    dead_code,
    reason = "each benchmark binary uses a different subset of the shared support module"
)]

use std::hint::black_box;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui_textwrap::{TextWrapper, WrapAlgorithm};

pub const VIEWPORT_WIDTH: u16 = 200;
pub const VIEWPORT_HEIGHT: u16 = 50;
pub const RESIZE_WIDTHS: [u16; 5] = [120, 160, 200, 240, 280];
pub const SESSION_FRAMES: usize = 60;
pub const FIXTURE_SEED: u64 = 0x5EED_7E57_CAFE_BABE;

const CORE_SIZES: [(&str, usize); 3] = [
    ("4-kib", 4 * 1024),
    ("64-kib", 64 * 1024),
    ("1-mib", 1024 * 1024),
];
const SESSION_SIZES: [&str; 2] = ["4-kib", "64-kib"];
const STYLE_COUNT: usize = 6;

/// One deterministic, styled text input.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: &'static str,
    pub target_bytes: usize,
    pub source_bytes: usize,
    pub text: Text<'static>,
}

impl Fixture {
    pub fn line_count(&self, implementation: Implementation, width: u16) -> usize {
        match implementation {
            Implementation::NativeParagraph => native_paragraph(self, 0).line_count(width),
            _ => wrap(self, implementation, width).lines.len(),
        }
    }
}

/// Implementations compared by every workflow.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Implementation {
    NativeParagraph,
    ParagraphCompat,
    FirstFit,
    OptimalFit,
}

impl Implementation {
    pub const ALL: [Self; 4] = [
        Self::NativeParagraph,
        Self::ParagraphCompat,
        Self::FirstFit,
        Self::OptimalFit,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::NativeParagraph => "paragraph-native",
            Self::ParagraphCompat => "paragraph-compat",
            Self::FirstFit => "first-fit",
            Self::OptimalFit => "optimal-fit",
        }
    }

    const fn algorithm(self) -> Option<WrapAlgorithm> {
        match self {
            Self::NativeParagraph => None,
            Self::ParagraphCompat => Some(WrapAlgorithm::ParagraphCompat { trim: true }),
            Self::FirstFit => Some(WrapAlgorithm::FirstFit),
            Self::OptimalFit => Some(WrapAlgorithm::OptimalFit),
        }
    }
}

/// A viewport location expressed relative to the wrapped output.
#[derive(Debug, Clone, Copy)]
pub enum Viewport {
    Start,
    Middle,
    End,
}

impl Viewport {
    pub const ALL: [Self; 3] = [Self::Start, Self::Middle, Self::End];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }

    pub fn scroll(self, line_count: usize) -> u16 {
        let last_page = line_count.saturating_sub(usize::from(VIEWPORT_HEIGHT));
        let offset = match self {
            Self::Start => 0,
            Self::Middle => last_page / 2,
            Self::End => last_page,
        };
        u16::try_from(offset).expect("fixture scroll offset must fit Paragraph's u16 API")
    }
}

/// Generates and validates the standard benchmark corpus.
pub fn core_fixtures() -> Vec<Fixture> {
    CORE_SIZES
        .into_iter()
        .enumerate()
        .map(|(index, (name, size))| {
            let seed = FIXTURE_SEED.wrapping_add(index as u64);
            let fixture = mixed_styled_fixture(name, size, seed);
            verify_fixture(&fixture);
            fixture
        })
        .collect()
}

/// Returns the fixtures used by multi-frame benchmark sessions.
pub fn session_fixtures(fixtures: &[Fixture]) -> impl Iterator<Item = &Fixture> {
    fixtures
        .iter()
        .filter(|fixture| SESSION_SIZES.contains(&fixture.name))
}

/// Generates a deterministic fixture for an opt-in stress profile.
pub fn stress_fixture(name: &'static str, target_bytes: usize, profile: StressProfile) -> Fixture {
    let text = match profile {
        StressProfile::LongLine => {
            let content =
                "alpha beta gamma delta epsilon zeta eta theta ".repeat(target_bytes.div_ceil(46));
            Text::from(Line::from(content))
        }
        StressProfile::Whitespace => {
            let content = "alpha     beta\tgamma      delta   ".repeat(target_bytes.div_ceil(35));
            Text::from(Line::from(content))
        }
        StressProfile::Unicode => {
            let content = "界 café e\u{301} 🐀 alpha βeta ".repeat(target_bytes.div_ceil(29));
            Text::from(Line::from(content))
        }
        StressProfile::Unbreakable => {
            Text::from(Line::from("abcdefghij".repeat(target_bytes.div_ceil(10))))
        }
    };
    let source_bytes = text.lines.iter().map(line_bytes).sum();
    Fixture {
        name,
        target_bytes,
        source_bytes,
        text,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StressProfile {
    LongLine,
    Whitespace,
    Unicode,
    Unbreakable,
}

/// Materializes one fixture with a non-native implementation.
#[must_use]
pub fn wrap(fixture: &Fixture, implementation: Implementation, width: u16) -> Text<'static> {
    let algorithm = implementation
        .algorithm()
        .expect("native Paragraph does not materialize wrapped Text");
    TextWrapper::new()
        .algorithm(algorithm)
        .wrap(fixture.text.clone(), width)
}

/// Builds a native wrapping Paragraph that owns a clone of the fixture.
#[must_use]
pub fn native_paragraph(fixture: &Fixture, scroll: u16) -> Paragraph<'static> {
    Paragraph::new(fixture.text.clone())
        .wrap(Wrap { trim: true })
        .scroll((scroll, 0))
}

/// Builds an unwrapped Paragraph from materialized text.
#[must_use]
pub fn materialized_paragraph(text: Text<'static>, scroll: u16) -> Paragraph<'static> {
    Paragraph::new(text).scroll((scroll, 0))
}

/// Allocates an empty viewport buffer.
#[must_use]
pub fn viewport_buffer(width: u16) -> Buffer {
    Buffer::empty(Rect::new(0, 0, width, VIEWPORT_HEIGHT))
}

/// Renders a reusable Paragraph into a viewport and keeps the result observable.
pub fn render(paragraph: &Paragraph<'_>, width: u16, buffer: &mut Buffer) {
    let area = Rect::new(0, 0, width, VIEWPORT_HEIGHT);
    paragraph.render(area, buffer);
    black_box(buffer);
}

fn mixed_styled_fixture(name: &'static str, target_bytes: usize, seed: u64) -> Fixture {
    let mut random = Random::new(seed);
    let mut lines = Vec::new();
    let mut source_bytes = 0;

    while source_bytes < target_bytes {
        let word_count = 55 + random.index(50);
        let line = mixed_styled_line(&mut random, word_count);
        source_bytes += line_bytes(&line) + usize::from(!lines.is_empty());
        lines.push(line);
    }

    Fixture {
        name,
        target_bytes,
        source_bytes,
        text: Text::from(lines),
    }
}

fn mixed_styled_line(random: &mut Random, word_count: usize) -> Line<'static> {
    let mut spans = Vec::new();
    let mut pending = String::new();
    let mut style_index = random.index(STYLE_COUNT);

    for index in 0..word_count {
        let word = random_word(random);
        if random.one_in(11) && word.is_ascii() && word.len() >= 6 {
            let midpoint = word.len() / 2;
            pending.push_str(&word[..midpoint]);
            flush_span(&mut spans, &mut pending, style_index);
            style_index = (style_index + 1) % STYLE_COUNT;
            pending.push_str(&word[midpoint..]);
        } else {
            pending.push_str(word);
        }

        if index + 1 != word_count {
            let spaces = if random.one_in(13) { "  " } else { " " };
            pending.push_str(spaces);
        }

        if random.one_in(5) {
            flush_span(&mut spans, &mut pending, style_index);
            style_index = random.index(STYLE_COUNT);
        }
    }
    flush_span(&mut spans, &mut pending, style_index);
    Line::from(spans)
}

fn random_word(random: &mut Random) -> &'static str {
    const ASCII_WORDS: [&str; 24] = [
        "alpha",
        "benchmark",
        "cache",
        "delta",
        "editor",
        "fragment",
        "grapheme",
        "horizontal",
        "indentation",
        "layout",
        "materialized",
        "paragraph",
        "performance",
        "render",
        "resize",
        "scroll",
        "separator",
        "styled",
        "terminal",
        "throughput",
        "unicode",
        "viewport",
        "whitespace",
        "wrapping",
    ];
    const UNICODE_WORDS: [&str; 8] = [
        "café",
        "e\u{301}lan",
        "naïve",
        "βeta",
        "界",
        "東",
        "🐀",
        "résumé",
    ];

    if random.one_in(12) {
        UNICODE_WORDS[random.index(UNICODE_WORDS.len())]
    } else {
        ASCII_WORDS[random.index(ASCII_WORDS.len())]
    }
}

fn flush_span(spans: &mut Vec<Span<'static>>, pending: &mut String, style_index: usize) {
    if pending.is_empty() {
        return;
    }
    spans.push(Span::styled(std::mem::take(pending), style(style_index)));
}

const fn style(index: usize) -> Style {
    match index {
        0 => Style::new(),
        1 => Style::new().fg(Color::Blue),
        2 => Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        3 => Style::new().fg(Color::Yellow),
        4 => Style::new()
            .fg(Color::Magenta)
            .add_modifier(Modifier::ITALIC),
        _ => Style::new().fg(Color::Cyan),
    }
}

fn line_bytes(line: &Line<'_>) -> usize {
    line.spans.iter().map(|span| span.content.len()).sum()
}

fn verify_fixture(fixture: &Fixture) {
    assert!(fixture.source_bytes >= fixture.target_bytes);
    assert!(fixture.source_bytes <= fixture.target_bytes + 2048);

    let native_count = fixture.line_count(Implementation::NativeParagraph, VIEWPORT_WIDTH);
    let compat = wrap(fixture, Implementation::ParagraphCompat, VIEWPORT_WIDTH);
    assert_eq!(native_count, compat.lines.len());

    for viewport in Viewport::ALL {
        let scroll = viewport.scroll(native_count);
        let native = native_paragraph(fixture, scroll);
        let materialized = materialized_paragraph(compat.clone(), scroll);
        let mut native_buffer = viewport_buffer(VIEWPORT_WIDTH);
        let mut materialized_buffer = viewport_buffer(VIEWPORT_WIDTH);
        render(&native, VIEWPORT_WIDTH, &mut native_buffer);
        render(&materialized, VIEWPORT_WIDTH, &mut materialized_buffer);
        if native_buffer != materialized_buffer {
            let (index, (native, materialized)) = native_buffer
                .content
                .iter()
                .zip(&materialized_buffer.content)
                .enumerate()
                .find(|(_, (native, materialized))| native != materialized)
                .expect("different buffers must contain a different cell");
            let x = index % usize::from(VIEWPORT_WIDTH);
            let y = index / usize::from(VIEWPORT_WIDTH);
            panic!(
                "Paragraph compatibility mismatch: fixture={}, viewport={}, x={x}, y={y}, \
                 native={native:?}, materialized={materialized:?}",
                fixture.name,
                viewport.name(),
            );
        }
    }

    for implementation in [Implementation::FirstFit, Implementation::OptimalFit] {
        let first = wrap(fixture, implementation, VIEWPORT_WIDTH);
        let second = wrap(fixture, implementation, VIEWPORT_WIDTH);
        let first = materialized_paragraph(first, 0);
        let second = materialized_paragraph(second, 0);
        let mut first_buffer = viewport_buffer(VIEWPORT_WIDTH);
        let mut second_buffer = viewport_buffer(VIEWPORT_WIDTH);
        render(&first, VIEWPORT_WIDTH, &mut first_buffer);
        render(&second, VIEWPORT_WIDTH, &mut second_buffer);
        assert_eq!(first_buffer, second_buffer);
    }
}

/// Small deterministic generator kept local to benchmark fixture construction.
#[derive(Debug, Clone, Copy)]
struct Random(u64);

impl Random {
    const fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }

    fn one_in(&mut self, denominator: u64) -> bool {
        self.next().is_multiple_of(denominator)
    }
}

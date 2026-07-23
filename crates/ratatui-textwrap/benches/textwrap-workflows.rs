//! Criterion benchmarks for application-level text wrapping workflows.

mod support;

use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};

use support::{
    Fixture, Implementation, RESIZE_WIDTHS, SESSION_FRAMES, VIEWPORT_WIDTH, Viewport,
    core_fixtures, materialized_paragraph, native_paragraph, render, session_fixtures,
    viewport_buffer, wrap,
};

fn workflows(criterion: &mut Criterion) {
    let fixtures = core_fixtures();

    wrap_or_count(criterion, &fixtures);
    fresh_render(criterion, &fixtures);
    count_and_render(criterion, &fixtures, CountOrder::Before);
    count_and_render(criterion, &fixtures, CountOrder::After);
    same_width_amortized(criterion, &fixtures);
    same_width_cached(criterion, &fixtures);
    resize_recomputed(criterion, &fixtures);
    resize_cached(criterion, &fixtures);
    viewport_render(criterion, &fixtures, CacheState::Cold);
    viewport_render(criterion, &fixtures, CacheState::Cached);
}

fn wrap_or_count(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/wrap-or-count");
    for fixture in fixtures {
        group.throughput(Throughput::Bytes(fixture.source_bytes as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    group.bench_with_input(id, fixture, move |bencher, _| {
                        bencher.iter(|| black_box(paragraph.line_count(VIEWPORT_WIDTH)));
                    });
                }
                _ => {
                    group.bench_with_input(id, fixture, move |bencher, fixture| {
                        bencher.iter(|| {
                            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                            black_box(wrapped.lines.len());
                        });
                    });
                }
            }
        }
    }
    group.finish();
}

fn fresh_render(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/fresh-render");
    for fixture in fixtures {
        group.throughput(Throughput::Bytes(fixture.source_bytes as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    bench_render(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        render(&paragraph, VIEWPORT_WIDTH, buffer);
                    });
                }
                _ => {
                    bench_render(&mut group, id, fixture, move |fixture, buffer| {
                        let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                        let paragraph = materialized_paragraph(wrapped, 0);
                        render(&paragraph, VIEWPORT_WIDTH, buffer);
                    });
                }
            }
        }
    }
    group.finish();
}

#[derive(Debug, Clone, Copy)]
enum CountOrder {
    Before,
    After,
}

impl CountOrder {
    const fn group_name(self) -> &'static str {
        match self {
            Self::Before => "textwrap/count-then-render",
            Self::After => "textwrap/render-then-count",
        }
    }
}

fn count_and_render(criterion: &mut Criterion, fixtures: &[Fixture], order: CountOrder) {
    let mut group = criterion.benchmark_group(order.group_name());
    for fixture in fixtures {
        group.throughput(Throughput::Bytes(fixture.source_bytes as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    bench_render(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        count_and_render_paragraph(&paragraph, VIEWPORT_WIDTH, buffer, order);
                    });
                }
                _ => {
                    bench_render(&mut group, id, fixture, move |fixture, buffer| {
                        let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                        let paragraph = materialized_paragraph(wrapped, 0);
                        count_and_render_paragraph(&paragraph, VIEWPORT_WIDTH, buffer, order);
                    });
                }
            }
        }
    }
    group.finish();
}

fn count_and_render_paragraph(
    paragraph: &ratatui::widgets::Paragraph<'_>,
    width: u16,
    buffer: &mut ratatui::buffer::Buffer,
    order: CountOrder,
) {
    match order {
        CountOrder::Before => {
            black_box(paragraph.line_count(width));
            render(paragraph, width, buffer);
        }
        CountOrder::After => {
            render(paragraph, width, buffer);
            black_box(paragraph.line_count(width));
        }
    }
}

fn same_width_amortized(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/same-width-amortized-60-frames");
    for fixture in session_fixtures(fixtures) {
        group.throughput(Throughput::Elements(SESSION_FRAMES as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    bench_session(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        for _ in 0..SESSION_FRAMES {
                            render(&paragraph, VIEWPORT_WIDTH, buffer);
                        }
                    });
                }
                _ => {
                    bench_session(&mut group, id, fixture, move |fixture, buffer| {
                        let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                        let paragraph = materialized_paragraph(wrapped, 0);
                        for _ in 0..SESSION_FRAMES {
                            render(&paragraph, VIEWPORT_WIDTH, buffer);
                        }
                    });
                }
            }
        }
    }
    group.finish();
}

fn same_width_cached(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/same-width-cached-60-frames");
    for fixture in session_fixtures(fixtures) {
        group.throughput(Throughput::Elements(SESSION_FRAMES as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            let paragraph = match implementation {
                Implementation::NativeParagraph => native_paragraph(fixture, 0),
                _ => {
                    let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                    materialized_paragraph(wrapped, 0)
                }
            };
            bench_session(&mut group, id, fixture, move |fixture, buffer| {
                black_box(fixture);
                for _ in 0..SESSION_FRAMES {
                    render(&paragraph, VIEWPORT_WIDTH, buffer);
                }
            });
        }
    }
    group.finish();
}

fn resize_recomputed(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/resize-recomputed-60-frames");
    for fixture in session_fixtures(fixtures) {
        group.throughput(Throughput::Elements(SESSION_FRAMES as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    bench_resize(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        for width in resize_sequence() {
                            render(&paragraph, width, buffer);
                        }
                    });
                }
                _ => {
                    bench_resize(&mut group, id, fixture, move |fixture, buffer| {
                        for width in resize_sequence() {
                            let wrapped = wrap(fixture, implementation, width);
                            let paragraph = materialized_paragraph(wrapped, 0);
                            render(&paragraph, width, buffer);
                        }
                    });
                }
            }
        }
    }
    group.finish();
}

fn resize_cached(criterion: &mut Criterion, fixtures: &[Fixture]) {
    let mut group = criterion.benchmark_group("textwrap/resize-cached-60-frames");
    for fixture in session_fixtures(fixtures) {
        group.throughput(Throughput::Elements(SESSION_FRAMES as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    bench_resize(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        for width in resize_sequence() {
                            render(&paragraph, width, buffer);
                        }
                    });
                }
                _ => {
                    let paragraphs = RESIZE_WIDTHS.map(|width| {
                        let wrapped = wrap(fixture, implementation, width);
                        materialized_paragraph(wrapped, 0)
                    });
                    bench_resize(&mut group, id, fixture, move |fixture, buffer| {
                        black_box(fixture);
                        for index in resize_index_sequence() {
                            let width = RESIZE_WIDTHS[index];
                            render(&paragraphs[index], width, buffer);
                        }
                    });
                }
            }
        }
    }
    group.finish();
}

#[derive(Debug, Clone, Copy)]
enum CacheState {
    Cold,
    Cached,
}

impl CacheState {
    const fn group_name(self) -> &'static str {
        match self {
            Self::Cold => "textwrap/viewport-cold",
            Self::Cached => "textwrap/viewport-cached",
        }
    }
}

fn viewport_render(criterion: &mut Criterion, fixtures: &[Fixture], cache_state: CacheState) {
    let mut group = criterion.benchmark_group(cache_state.group_name());
    for fixture in fixtures {
        for implementation in Implementation::ALL {
            let line_count = fixture.line_count(implementation, VIEWPORT_WIDTH);
            for viewport in Viewport::ALL {
                let parameter = format!("{}/{}", fixture.name, viewport.name());
                let id = BenchmarkId::new(implementation.name(), parameter);
                let scroll = viewport.scroll(line_count);
                match (implementation, cache_state) {
                    (Implementation::NativeParagraph, _) => {
                        let paragraph = native_paragraph(fixture, scroll);
                        bench_render(&mut group, id, fixture, move |fixture, buffer| {
                            black_box(fixture);
                            render(&paragraph, VIEWPORT_WIDTH, buffer);
                        });
                    }
                    (_, CacheState::Cold) => {
                        bench_render(&mut group, id, fixture, move |fixture, buffer| {
                            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                            let paragraph = materialized_paragraph(wrapped, scroll);
                            render(&paragraph, VIEWPORT_WIDTH, buffer);
                        });
                    }
                    (_, CacheState::Cached) => {
                        let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                        let paragraph = materialized_paragraph(wrapped, scroll);
                        bench_render(&mut group, id, fixture, move |fixture, buffer| {
                            black_box(fixture);
                            render(&paragraph, VIEWPORT_WIDTH, buffer);
                        });
                    }
                }
            }
        }
    }
    group.finish();
}

fn bench_render(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    fixture: &Fixture,
    mut operation: impl FnMut(&Fixture, &mut ratatui::buffer::Buffer),
) {
    group.bench_with_input(id, fixture, |bencher, fixture| {
        bencher.iter_batched(
            || viewport_buffer(VIEWPORT_WIDTH),
            |mut buffer| operation(fixture, &mut buffer),
            BatchSize::LargeInput,
        );
    });
}

fn bench_session(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    fixture: &Fixture,
    mut operation: impl FnMut(&Fixture, &mut ratatui::buffer::Buffer),
) {
    bench_render(group, id, fixture, move |fixture, buffer| {
        operation(fixture, buffer);
    });
}

fn bench_resize(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    fixture: &Fixture,
    mut operation: impl FnMut(&Fixture, &mut ratatui::buffer::Buffer),
) {
    let max_width = *RESIZE_WIDTHS
        .iter()
        .max()
        .expect("resize widths must not be empty");
    group.bench_with_input(id, fixture, |bencher, fixture| {
        bencher.iter_batched(
            || viewport_buffer(max_width),
            |mut buffer| operation(fixture, &mut buffer),
            BatchSize::LargeInput,
        );
    });
}

fn resize_sequence() -> impl Iterator<Item = u16> {
    resize_index_sequence().map(|index| RESIZE_WIDTHS[index])
}

fn resize_index_sequence() -> impl Iterator<Item = usize> {
    (0..SESSION_FRAMES).map(|frame| frame % RESIZE_WIDTHS.len())
}

criterion_group!(benches, workflows);
criterion_main!(benches);

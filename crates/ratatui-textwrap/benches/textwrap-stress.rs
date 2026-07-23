//! Opt-in Criterion benchmarks for expensive or pathological wrapping inputs.

mod support;

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use support::{
    Fixture, Implementation, RESIZE_WIDTHS, SESSION_FRAMES, StressProfile, VIEWPORT_WIDTH,
    Viewport, core_fixtures, materialized_paragraph, native_paragraph, render, stress_fixture,
    viewport_buffer, wrap,
};

fn stress(criterion: &mut Criterion) {
    stress_profiles(criterion);

    let fixtures = core_fixtures();
    let large = fixtures
        .iter()
        .find(|fixture| fixture.name == "1-mib")
        .expect("core corpus must contain the 1 MiB fixture");
    large_sessions(criterion, large);
    deep_scroll(criterion);
}

fn stress_profiles(criterion: &mut Criterion) {
    let fixtures = [
        stress_fixture("long-line-64-kib", 64 * 1024, StressProfile::LongLine),
        stress_fixture("whitespace-64-kib", 64 * 1024, StressProfile::Whitespace),
        stress_fixture("unicode-64-kib", 64 * 1024, StressProfile::Unicode),
        stress_fixture("unbreakable-64-kib", 64 * 1024, StressProfile::Unbreakable),
    ];
    let mut group = criterion.benchmark_group("textwrap-stress/wrap-or-count");
    group.sample_size(10);

    for fixture in &fixtures {
        group.throughput(Throughput::Bytes(fixture.source_bytes as u64));
        for implementation in Implementation::ALL {
            let id = BenchmarkId::new(implementation.name(), fixture.name);
            match implementation {
                Implementation::NativeParagraph => {
                    let paragraph = native_paragraph(fixture, 0);
                    group.bench_function(id, move |bencher| {
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

fn large_sessions(criterion: &mut Criterion, fixture: &Fixture) {
    let mut same_width = criterion.benchmark_group("textwrap-stress/one-mib-amortized-60-frames");
    same_width.sample_size(10);
    same_width.throughput(Throughput::Elements(SESSION_FRAMES as u64));

    for implementation in Implementation::ALL {
        let id = BenchmarkId::from_parameter(implementation.name());
        match implementation {
            Implementation::NativeParagraph => {
                let paragraph = native_paragraph(fixture, 0);
                same_width.bench_function(id, move |bencher| {
                    bencher.iter_batched(
                        || viewport_buffer(VIEWPORT_WIDTH),
                        |mut buffer| {
                            for _ in 0..SESSION_FRAMES {
                                render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
                            }
                        },
                        BatchSize::LargeInput,
                    );
                });
            }
            _ => {
                same_width.bench_with_input(id, fixture, move |bencher, fixture| {
                    bencher.iter_batched(
                        || viewport_buffer(VIEWPORT_WIDTH),
                        |mut buffer| {
                            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                            let paragraph = materialized_paragraph(wrapped, 0);
                            for _ in 0..SESSION_FRAMES {
                                render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
                            }
                        },
                        BatchSize::LargeInput,
                    );
                });
            }
        }
    }
    same_width.finish();

    let mut resize = criterion.benchmark_group("textwrap-stress/one-mib-resize-60-frames");
    resize.sample_size(10);
    resize.throughput(Throughput::Elements(SESSION_FRAMES as u64));
    let max_width = *RESIZE_WIDTHS
        .iter()
        .max()
        .expect("resize widths must not be empty");

    for implementation in Implementation::ALL {
        let id = BenchmarkId::from_parameter(implementation.name());
        match implementation {
            Implementation::NativeParagraph => {
                let paragraph = native_paragraph(fixture, 0);
                resize.bench_function(id, move |bencher| {
                    bencher.iter_batched(
                        || viewport_buffer(max_width),
                        |mut buffer| {
                            for frame in 0..SESSION_FRAMES {
                                let width = RESIZE_WIDTHS[frame % RESIZE_WIDTHS.len()];
                                render(&paragraph, width, &mut buffer);
                            }
                        },
                        BatchSize::LargeInput,
                    );
                });
            }
            _ => {
                resize.bench_with_input(id, fixture, move |bencher, fixture| {
                    bencher.iter_batched(
                        || viewport_buffer(max_width),
                        |mut buffer| {
                            for frame in 0..SESSION_FRAMES {
                                let width = RESIZE_WIDTHS[frame % RESIZE_WIDTHS.len()];
                                let wrapped = wrap(fixture, implementation, width);
                                let paragraph = materialized_paragraph(wrapped, 0);
                                render(&paragraph, width, &mut buffer);
                            }
                        },
                        BatchSize::LargeInput,
                    );
                });
            }
        }
    }
    resize.finish();
}

fn deep_scroll(criterion: &mut Criterion) {
    let fixture = stress_fixture(
        "deep-scroll-4-mib",
        4 * 1024 * 1024,
        StressProfile::LongLine,
    );
    let mut cold = criterion.benchmark_group("textwrap-stress/deep-scroll-cold");
    cold.sample_size(10);

    for implementation in Implementation::ALL {
        let line_count = fixture.line_count(implementation, VIEWPORT_WIDTH);
        let scroll = Viewport::End.scroll(line_count);
        let id = BenchmarkId::from_parameter(implementation.name());
        match implementation {
            Implementation::NativeParagraph => {
                let paragraph = native_paragraph(&fixture, scroll);
                cold.bench_function(id, move |bencher| {
                    bencher.iter_batched(
                        || viewport_buffer(VIEWPORT_WIDTH),
                        |mut buffer| render(&paragraph, VIEWPORT_WIDTH, &mut buffer),
                        BatchSize::LargeInput,
                    );
                });
            }
            _ => {
                cold.bench_with_input(id, &fixture, move |bencher, fixture| {
                    bencher.iter_batched(
                        || viewport_buffer(VIEWPORT_WIDTH),
                        |mut buffer| {
                            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                            let paragraph = materialized_paragraph(wrapped, scroll);
                            render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
                        },
                        BatchSize::LargeInput,
                    );
                });
            }
        }
    }
    cold.finish();

    let mut cached = criterion.benchmark_group("textwrap-stress/deep-scroll-cached");
    cached.sample_size(10);
    for implementation in Implementation::ALL {
        let line_count = fixture.line_count(implementation, VIEWPORT_WIDTH);
        let scroll = Viewport::End.scroll(line_count);
        let paragraph = match implementation {
            Implementation::NativeParagraph => native_paragraph(&fixture, scroll),
            _ => {
                let wrapped = wrap(&fixture, implementation, VIEWPORT_WIDTH);
                materialized_paragraph(wrapped, scroll)
            }
        };
        cached.bench_function(
            BenchmarkId::from_parameter(implementation.name()),
            move |bencher| {
                bencher.iter_batched(
                    || viewport_buffer(VIEWPORT_WIDTH),
                    |mut buffer| render(&paragraph, VIEWPORT_WIDTH, &mut buffer),
                    BatchSize::LargeInput,
                );
            },
        );
    }
    cached.finish();
}

criterion_group!(benches, stress);
criterion_main!(benches);

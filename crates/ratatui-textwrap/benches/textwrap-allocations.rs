//! One-shot allocation diagnostics for representative text wrapping workflows.

mod support;

use std::alloc::System;
use std::error::Error;
use std::fmt::Write;
use std::hint::black_box;
use std::path::PathBuf;

use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};

use support::{
    Fixture, Implementation, RESIZE_WIDTHS, SESSION_FRAMES, VIEWPORT_WIDTH, Viewport,
    core_fixtures, materialized_paragraph, native_paragraph, render, viewport_buffer, wrap,
};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() -> Result<(), Box<dyn Error>> {
    let fixtures = core_fixtures();
    let fixture = fixtures
        .iter()
        .find(|fixture| fixture.name == "64-kib")
        .expect("core corpus must contain the 64 KiB fixture");
    let rows = allocation_rows(fixture);
    let csv = format_csv(&rows);

    if let Some(path) = std::env::args_os().nth(1) {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path)
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, csv)?;
    } else {
        print!("{csv}");
    }
    Ok(())
}

#[derive(Debug)]
struct AllocationRow {
    workload: &'static str,
    implementation: &'static str,
    stats: Stats,
}

fn allocation_rows(fixture: &Fixture) -> Vec<AllocationRow> {
    let mut rows = Vec::new();
    for implementation in Implementation::ALL {
        rows.push(measure_wrap_or_count(fixture, implementation));
        rows.push(measure_count_then_render(fixture, implementation));
        rows.push(measure_same_width(fixture, implementation, false));
        rows.push(measure_same_width(fixture, implementation, true));
        rows.push(measure_resize(fixture, implementation, false));
        rows.push(measure_resize(fixture, implementation, true));
        rows.push(measure_viewport(fixture, implementation, false));
        rows.push(measure_viewport(fixture, implementation, true));
    }
    rows
}

fn measure_wrap_or_count(fixture: &Fixture, implementation: Implementation) -> AllocationRow {
    let paragraph =
        (implementation == Implementation::NativeParagraph).then(|| native_paragraph(fixture, 0));
    row("wrap-or-count", implementation, || match implementation {
        Implementation::NativeParagraph => {
            black_box(
                paragraph
                    .as_ref()
                    .expect("native paragraph must exist")
                    .line_count(VIEWPORT_WIDTH),
            );
            None
        }
        _ => {
            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
            black_box(wrapped.lines.len());
            Some(wrapped)
        }
    })
}

fn measure_count_then_render(fixture: &Fixture, implementation: Implementation) -> AllocationRow {
    let native =
        (implementation == Implementation::NativeParagraph).then(|| native_paragraph(fixture, 0));
    let mut buffer = viewport_buffer(VIEWPORT_WIDTH);
    row("count-then-render", implementation, || {
        let paragraph = match implementation {
            Implementation::NativeParagraph => native.expect("native paragraph must exist"),
            _ => {
                let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                materialized_paragraph(wrapped, 0)
            }
        };
        black_box(paragraph.line_count(VIEWPORT_WIDTH));
        render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
        paragraph
    })
}

fn measure_same_width(
    fixture: &Fixture,
    implementation: Implementation,
    cached: bool,
) -> AllocationRow {
    let cached_paragraph = cached.then(|| match implementation {
        Implementation::NativeParagraph => native_paragraph(fixture, 0),
        _ => {
            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
            materialized_paragraph(wrapped, 0)
        }
    });
    let native = (!cached && implementation == Implementation::NativeParagraph)
        .then(|| native_paragraph(fixture, 0));
    let mut buffer = viewport_buffer(VIEWPORT_WIDTH);
    let workload = if cached {
        "same-width-cached-60-frames"
    } else {
        "same-width-amortized-60-frames"
    };

    row(workload, implementation, || {
        let paragraph = match (cached_paragraph, native) {
            (Some(paragraph), _) | (_, Some(paragraph)) => paragraph,
            (None, None) => {
                let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                materialized_paragraph(wrapped, 0)
            }
        };
        for _ in 0..SESSION_FRAMES {
            render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
        }
        paragraph
    })
}

fn measure_resize(
    fixture: &Fixture,
    implementation: Implementation,
    cached: bool,
) -> AllocationRow {
    let max_width = *RESIZE_WIDTHS
        .iter()
        .max()
        .expect("resize widths must not be empty");
    let mut buffer = viewport_buffer(max_width);
    let native =
        (implementation == Implementation::NativeParagraph).then(|| native_paragraph(fixture, 0));
    let cached_paragraphs =
        (cached && implementation != Implementation::NativeParagraph).then(|| {
            RESIZE_WIDTHS.map(|width| {
                let wrapped = wrap(fixture, implementation, width);
                materialized_paragraph(wrapped, 0)
            })
        });
    let workload = if cached {
        "resize-cached-60-frames"
    } else {
        "resize-recomputed-60-frames"
    };

    row(workload, implementation, || {
        for frame in 0..SESSION_FRAMES {
            let index = frame % RESIZE_WIDTHS.len();
            let width = RESIZE_WIDTHS[index];
            if let Some(paragraph) = &native {
                render(paragraph, width, &mut buffer);
            } else if let Some(paragraphs) = &cached_paragraphs {
                render(&paragraphs[index], width, &mut buffer);
            } else {
                let wrapped = wrap(fixture, implementation, width);
                let paragraph = materialized_paragraph(wrapped, 0);
                render(&paragraph, width, &mut buffer);
            }
        }
    })
}

fn measure_viewport(
    fixture: &Fixture,
    implementation: Implementation,
    cached: bool,
) -> AllocationRow {
    let line_count = fixture.line_count(implementation, VIEWPORT_WIDTH);
    let scroll = Viewport::Middle.scroll(line_count);
    let cached_paragraph = cached.then(|| match implementation {
        Implementation::NativeParagraph => native_paragraph(fixture, scroll),
        _ => {
            let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
            materialized_paragraph(wrapped, scroll)
        }
    });
    let native = (!cached && implementation == Implementation::NativeParagraph)
        .then(|| native_paragraph(fixture, scroll));
    let mut buffer = viewport_buffer(VIEWPORT_WIDTH);
    let workload = if cached {
        "viewport-middle-cached"
    } else {
        "viewport-middle-cold"
    };

    row(workload, implementation, || {
        let paragraph = match (cached_paragraph, native) {
            (Some(paragraph), _) | (_, Some(paragraph)) => paragraph,
            (None, None) => {
                let wrapped = wrap(fixture, implementation, VIEWPORT_WIDTH);
                materialized_paragraph(wrapped, scroll)
            }
        };
        render(&paragraph, VIEWPORT_WIDTH, &mut buffer);
        paragraph
    })
}

fn row<T>(
    workload: &'static str,
    implementation: Implementation,
    operation: impl FnOnce() -> T,
) -> AllocationRow {
    let region = Region::new(GLOBAL);
    let output = operation();
    black_box(&output);
    let stats = region.change();
    AllocationRow {
        workload,
        implementation: implementation.name(),
        stats,
    }
}

fn format_csv(rows: &[AllocationRow]) -> String {
    let mut output = String::from(
        "workload,implementation,allocations,reallocations,bytes_allocated,\
         bytes_reallocated,deallocations,bytes_deallocated\n",
    );
    for row in rows {
        writeln!(
            output,
            "{},{},{},{},{},{},{},{}",
            row.workload,
            row.implementation,
            row.stats.allocations,
            row.stats.reallocations,
            row.stats.bytes_allocated,
            row.stats.bytes_reallocated,
            row.stats.deallocations,
            row.stats.bytes_deallocated,
        )
        .expect("writing to a String cannot fail");
    }
    output
}

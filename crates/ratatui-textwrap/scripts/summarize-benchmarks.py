#!/usr/bin/env python3
"""Summarize Criterion raw samples and allocation diagnostics as Markdown."""

from __future__ import annotations

import argparse
import csv
import platform
import statistics
import subprocess
import textwrap
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


BASELINE = "paragraph-native"
FRAMES_PER_SESSION = 60
IMPLEMENTATION_ORDER = {
    "paragraph-native": 0,
    "paragraph-compat": 1,
    "first-fit": 2,
    "optimal-fit": 3,
}
MATERIALIZED = ("paragraph-compat", "first-fit", "optimal-fit")


@dataclass(frozen=True)
class Timing:
    group: str
    function: str
    value: str
    median_ns: float
    throughput: str


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--criterion-dir",
        type=Path,
        default=Path("target/criterion"),
        help="Criterion output directory",
    )
    parser.add_argument(
        "--allocations",
        type=Path,
        help="CSV produced by the textwrap-allocations benchmark",
    )
    parser.add_argument(
        "--sampling",
        default="not recorded in Criterion's raw CSV",
        help="sampling settings to record in the generated report",
    )
    parser.add_argument("--output", type=Path, help="write Markdown to this path")
    return parser.parse_args()


def read_timings(root: Path) -> list[Timing]:
    samples: dict[tuple[str, str, str], list[float]] = defaultdict(list)
    throughput: dict[tuple[str, str, str], tuple[str, float]] = {}

    for path in sorted(root.glob("**/new/raw.csv")):
        with path.open(newline="", encoding="utf-8") as source:
            for row in csv.DictReader(source):
                key = (row["group"], row["function"], row["value"])
                measured = float(row["sample_measured_value"])
                iterations = float(row["iteration_count"])
                samples[key].append(measured / iterations)
                if row["throughput_type"]:
                    throughput[key] = (
                        row["throughput_type"],
                        float(row["throughput_num"]),
                    )

    timings = []
    for key, values in samples.items():
        group, function, value = key
        median_ns = statistics.median(values)
        timings.append(
            Timing(
                group,
                function,
                value,
                median_ns,
                format_throughput(median_ns, throughput.get(key)),
            )
        )
    return sorted(
        timings,
        key=lambda timing: (
            timing.group,
            timing.value,
            IMPLEMENTATION_ORDER.get(timing.function, 99),
        ),
    )


def format_throughput(
    median_ns: float,
    throughput: tuple[str, float] | None,
) -> str:
    if throughput is None or median_ns == 0:
        return "—"
    kind, amount = throughput
    per_second = amount / (median_ns / 1_000_000_000)
    if kind == "bytes":
        return f"{per_second / (1024 * 1024):.2f} MiB/s"
    if kind == "elements":
        return f"{per_second:.1f} frames/s"
    return f"{per_second:.1f} {kind}/s"


def timing_markdown(timings: list[Timing]) -> list[str]:
    if not timings:
        return ["_No Criterion `raw.csv` samples were found._", ""]

    lines = []
    by_group: dict[str, list[Timing]] = defaultdict(list)
    for timing in timings:
        by_group[timing.group].append(timing)

    for group, group_timings in by_group.items():
        lines.extend([f"### `{group}`", ""])
        per_frame = "60-frames" in group
        label = "Median/frame" if per_frame else "Median"
        rows = []
        baselines = {
            timing.value: timing.median_ns
            for timing in group_timings
            if timing.function == BASELINE
        }
        for timing in group_timings:
            displayed_ns = (
                timing.median_ns / FRAMES_PER_SESSION if per_frame else timing.median_ns
            )
            baseline = baselines.get(timing.value)
            ratio = timing.median_ns / baseline if baseline else None
            ratio_text = f"{ratio:.2f}×" if ratio is not None else "—"
            rows.append(
                [
                    timing.value,
                    f"`{timing.function}`",
                    format_duration(displayed_ns),
                    timing.throughput,
                    ratio_text,
                ]
            )
        lines.extend(
            markdown_table(
                ["Input", "Implementation", label, "Throughput", "vs. Paragraph"],
                rows,
                right_aligned={2, 3, 4},
            )
        )
        lines.append("")
    return lines


def allocation_markdown(path: Path | None) -> list[str]:
    if path is None or not path.exists():
        return ["_No allocation diagnostic CSV was supplied._", ""]

    with path.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source))
    rows.sort(
        key=lambda row: (
            row["workload"],
            IMPLEMENTATION_ORDER.get(row["implementation"], 99),
        )
    )
    baselines = {
        row["workload"]: int(row["bytes_allocated"])
        for row in rows
        if row["implementation"] == BASELINE
    }
    table_rows = []
    for row in rows:
        allocated = int(row["bytes_allocated"])
        baseline = baselines.get(row["workload"])
        ratio = allocated / baseline if baseline else None
        ratio_text = f"{ratio:.2f}×" if ratio is not None else "—"
        table_rows.append(
            [
                row["workload"],
                f"`{row['implementation']}`",
                f"{int(row['allocations']):,}",
                f"{int(row['reallocations']):,}",
                f"{allocated:,}",
                ratio_text,
            ]
        )
    lines = markdown_table(
        [
            "Workload",
            "Implementation",
            "Allocations",
            "Reallocations",
            "Bytes allocated",
            "vs. Paragraph",
        ],
        table_rows,
        right_aligned={2, 3, 4, 5},
    )
    lines.append("")
    return lines


def markdown_table(
    headers: list[str],
    rows: list[list[str]],
    *,
    right_aligned: set[int],
) -> list[str]:
    widths = [
        max(3, len(header), *(len(row[index]) for row in rows))
        for index, header in enumerate(headers)
    ]

    def cells(values: list[str]) -> str:
        formatted = [
            value.rjust(widths[index])
            if index in right_aligned
            else value.ljust(widths[index])
            for index, value in enumerate(values)
        ]
        return f"| {' | '.join(formatted)} |"

    delimiters = [
        "-" * (width - 1) + ":" if index in right_aligned else "-" * width
        for index, width in enumerate(widths)
    ]
    return [cells(headers), cells(delimiters), *(cells(row) for row in rows)]


def format_duration(nanoseconds: float) -> str:
    if nanoseconds < 1_000:
        return f"{nanoseconds:.1f} ns"
    if nanoseconds < 1_000_000:
        return f"{nanoseconds / 1_000:.2f} µs"
    if nanoseconds < 1_000_000_000:
        return f"{nanoseconds / 1_000_000:.2f} ms"
    return f"{nanoseconds / 1_000_000_000:.2f} s"


def measured_observations(timings: list[Timing]) -> list[str]:
    indexed = {
        (timing.group, timing.value, timing.function): timing.median_ns
        for timing in timings
    }

    def ratios(group: str, value: str) -> tuple[float, float]:
        baseline = indexed[(group, value, BASELINE)]
        values = [
            indexed[(group, value, implementation)] / baseline
            for implementation in MATERIALIZED
        ]
        return min(values), max(values)

    def ratio_text(group: str, value: str) -> str:
        low, high = ratios(group, value)
        if f"{low:.2f}" == f"{high:.2f}":
            return f"{low:.2f}×"
        return f"{low:.2f}–{high:.2f}×"

    try:
        observations = [
            "- In this run, cold 1 MiB viewport rendering with full materialization took "
            f"{ratio_text('textwrap/fresh-render', '1-mib')} native time because native stopped "
            "after 50 rows.",
            "- The 1 MiB count-then-render workflow took "
            f"{ratio_text('textwrap/count-then-render', '1-mib')} native time; avoiding the second "
            "wrap did not offset the current materialization cost.",
            "- At 64 KiB, steady same-width cached rendering took "
            f"{ratio_text('textwrap/same-width-cached-60-frames', '64-kib')} native time, while "
            "wrap-once amortized rendering took "
            f"{ratio_text('textwrap/same-width-amortized-60-frames', '64-kib')}.",
            "- At 64 KiB, recomputing across resize widths took "
            f"{ratio_text('textwrap/resize-recomputed-60-frames', '64-kib')} native time; caching "
            "all five widths reduced that to "
            f"{ratio_text('textwrap/resize-cached-60-frames', '64-kib')}.",
            "- For a cached 1 MiB document, the middle viewport took "
            f"{ratio_text('textwrap/viewport-cached', '1-mib/middle')} native time and the end "
            f"viewport took {ratio_text('textwrap/viewport-cached', '1-mib/end')}.",
        ]
        return [
            line
            for observation in observations
            for line in textwrap.wrap(
                observation,
                width=100,
                subsequent_indent="  ",
            )
        ]
    except KeyError:
        return [
            "- The expected core benchmark matrix was incomplete, so measured summary deltas "
            "could not be generated.",
        ]


def command_output(*command: str) -> str:
    try:
        return subprocess.check_output(command, text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unavailable"


def cpu_name() -> str:
    if platform.system() == "Darwin":
        name = command_output("sysctl", "-n", "machdep.cpu.brand_string")
        if name != "unavailable":
            return name
    return platform.processor() or "unavailable"


def report(timings: list[Timing], allocations: Path | None, sampling: str) -> str:
    rustc = command_output("rustc", "--version")
    revision = command_output(
        "jj",
        "log",
        "-r",
        "@",
        "--no-graph",
        "-T",
        "change_id.short()",
    )
    lines = [
        "# `ratatui-textwrap` Benchmark Results",
        "",
        "This report is generated from Criterion raw samples and the one-shot allocation "
        "diagnostic.",
        "",
        "## Environment",
        "",
        f"- Platform: {platform.platform()}",
        f"- Processor: {cpu_name()}",
        f"- Rust: `{rustc}`",
        "- Benchmark dependencies: Criterion 0.8.2 and `stats_alloc` 0.1.10",
        "- Compared dependencies: Ratatui 0.30.2 and textwrap 0.16.2",
        f"- Change: `{revision}`",
        "",
        "## Reference configuration",
        "",
        "- Seed: `0x5EED_7E57_CAFE_BABE`",
        "- Core inputs: approximately 4 KiB, 64 KiB, and 1 MiB",
        "- Viewport: 200×50 cells",
        "- Resize widths: 120, 160, 200, 240, and 280 cells",
        "- Session length: 60 frames",
        f"- Sampling: {sampling}",
        "",
        "Use Criterion's default settings when evaluating a suspected regression.",
        "",
        "## Interpretation",
        "",
        "- Cold materialization processes the complete input, while native `Paragraph` can stop",
        "  after filling the viewport.",
        "- Count-and-render groups expose native Paragraph's repeated wrapping and the owned",
        "  result's constant-time line count.",
        "- Cached session and viewport groups isolate reuse after wrapping; resize-cached also",
        "  shows the effect of caching by width.",
        "- Allocation results are one-shot diagnostics from an instrumented allocator and are not",
        "  part of Criterion's wall-time samples.",
        "",
        "### Observed deltas",
        "",
    ]
    lines.extend(measured_observations(timings))
    lines.extend(
        [
            "",
        "## Timing",
        "",
        ]
    )
    lines.extend(timing_markdown(timings))
    lines.extend(["## Allocations", ""])
    lines.extend(allocation_markdown(allocations))
    return "\n".join(lines)


def main() -> None:
    options = arguments()
    markdown = report(
        read_timings(options.criterion_dir),
        options.allocations,
        options.sampling,
    )
    if options.output:
        options.output.write_text(markdown, encoding="utf-8")
    else:
        print(markdown)


if __name__ == "__main__":
    main()

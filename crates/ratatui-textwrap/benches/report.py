#!/usr/bin/env python3
"""Render the `scroll` bench results as a shaded HTML table.

Run after `cargo bench -p ratatui-textwrap --bench scroll`:

    python3 crates/ratatui-textwrap/benches/report.py [target-dir] [output.html]

Reads criterion's median estimates from `<target-dir>/criterion` (default `target`) and writes
`<target-dir>/criterion/scroll-report.html` unless an output path is given. Cells are shaded on a
log scale: green under 1 ms (deeper is faster), amber from 1 to 16 ms, and red over 16 ms, faint
near one 60 fps frame, clear by 100 ms, and deep near 400 ms.
"""

import json
import math
import sys
from pathlib import Path

SIZES = ["16384", "32768", "49152", "65535"]
LABELS = {"16384": "16k", "32768": "32k", "49152": "48k", "65535": "65,535"}
# Results written this long before the newest result come from an earlier run and are greyed out.
STALE_SECONDS = 15 * 60
NEW = {"TextWrapper::tail", "WrappedText", "WrappedText build", "New owned wrap",
       "First-fit WrappedText build", "Optimal-fit WrappedText build", "WrappedText, bottom", "TextWrapper::tail, bottom", "WrappedText, middle",
       "WrappedText, append", "WrappedText, inside the line"}

# (title, [(label, criterion id without the size)])
GROUPS = [
    ("Frame scrolled to bottom (stick-to-bottom view)", [
        ("Paragraph", "frame/paragraph_bottom"),
        ("yazi WordWrapper", "frame/yazi_bottom"),
        ("Old owned cache", "frame/pre_cached_bottom"),
        ("WrappedText", "frame/wrapped_text_bottom"),
        ("TextWrapper::tail", "frame/post_tail_bottom"),
    ]),
    ("Frame scrolled to middle", [
        ("Paragraph", "frame/paragraph_middle"),
        ("yazi WordWrapper", "frame/yazi_middle"),
        ("Old owned cache", "frame/pre_cached_middle"),
        ("WrappedText", "frame/wrapped_text_middle"),
    ]),
    ("Frame at top", [
        ("Paragraph", "frame/paragraph_top"),
        ("yazi WordWrapper", "frame/yazi_top"),
        ("Old owned cache", "frame/pre_cached_top"),
        ("WrappedText", "frame/wrapped_text_top"),
    ]),
    ("Append a line, stay at bottom", [
        ("Paragraph (count + render)", "append_stick_bottom/paragraph"),
        ("yazi WordWrapper", "append_stick_bottom/yazi"),
        ("Old owned cache", "append_stick_bottom/pre_cached"),
        ("TextWrapper::tail", "append_stick_bottom/post_tail"),
        ("WrappedText", "append_stick_bottom/wrapped_text"),
    ]),
    ("Edit a middle line, draw bottom", [
        ("Paragraph", "edit_middle/paragraph"),
        ("yazi WordWrapper", "edit_middle/yazi"),
        ("Old (full rewrap)", "edit_middle/pre_rewrap"),
        ("WrappedText", "edit_middle/wrapped_text"),
    ]),
    ("Insert a middle line, draw bottom", [
        ("Paragraph", "insert_middle/paragraph"),
        ("Old (full rewrap)", "insert_middle/pre_rewrap"),
        ("WrappedText", "insert_middle/wrapped_text"),
    ]),
    ("Code preview (highlighted Rust source)", [
        ("Paragraph, bottom", "code/paragraph_bottom"),
        ("yazi WordWrapper, bottom", "code/yazi_bottom"),
        ("Old owned cache, bottom", "code/pre_cached_bottom"),
        ("WrappedText, bottom", "code/wrapped_text_bottom"),
        ("TextWrapper::tail, bottom", "code/tail_bottom"),
        ("Paragraph, middle", "code/paragraph_middle"),
        ("WrappedText, middle", "code/wrapped_text_middle"),
        ("WrappedText, append", "code/wrapped_text_append"),
    ]),
    ("One 500 KB line at the end of a log (column = line width in cells)", [
        ("Paragraph, inside the line", "long_line/paragraph_inside"),
        ("Old owned cache, inside the line", "long_line/pre_cached_inside"),
        ("WrappedText, inside the line", "long_line/wrapped_text_inside"),
        ("Paragraph, bottom", "long_line/paragraph_bottom"),
        ("Old owned cache, bottom", "long_line/pre_cached_bottom"),
        ("WrappedText, bottom", "long_line/wrapped_text_bottom"),
        ("TextWrapper::tail, bottom", "long_line/tail_bottom"),
    ]),
    ("Wrap or count every row (resize / build)", [
        ("Paragraph line_count", "rewrap_all/paragraph_line_count"),
        ("yazi count", "rewrap_all/yazi_count"),
        ("Old owned wrap", "rewrap_all/pre_owned_wrap"),
        ("New owned wrap", "rewrap_all/post_owned_wrap"),
        ("WrappedText build", "rewrap_all/wrapped_text"),
        ("First-fit owned count", "rewrap_all/first_fit_owned_count"),
        ("First-fit WrappedText build", "rewrap_all/first_fit_wrapped_text"),
        ("Optimal-fit owned count", "rewrap_all/optimal_fit_owned_count"),
        ("Optimal-fit WrappedText build", "rewrap_all/optimal_fit_wrapped_text"),
    ]),
]

STYLE = """
body{margin:0}
.wrap{font-family:var(--font-sans,system-ui);color:var(--foreground,#ddd)}
.legend{display:flex;gap:14px;font-size:12px;color:var(--muted-foreground,#999);margin:0 0 10px;
  flex-wrap:wrap}
.legend span{display:inline-flex;align-items:center;gap:6px}
.sw{width:12px;height:12px;border-radius:3px;display:inline-block}
table{width:100%;border-collapse:collapse;font-size:13px}
th,td{padding:5px 8px;text-align:right;border-bottom:1px solid var(--border,#333)}
td:first-child,th:first-child{text-align:left}
thead th{font-weight:600;color:var(--muted-foreground,#999);font-size:12px}
tr.g th{text-align:left;padding-top:14px;font-weight:600}
td{font-family:var(--font-mono,ui-monospace);font-variant-numeric:tabular-nums}
td:first-child{font-family:var(--font-sans,system-ui)}
td.new{font-weight:600}
td.stale{opacity:.45;font-style:italic}
.note{font-size:12px;color:var(--muted-foreground,#999);margin-top:10px;line-height:1.45}
"""

COLORS = {"success": "#22c55e", "warning": "#f59e0b", "destructive": "#ef4444"}


def shade(ns):
    """Returns a background style for a duration in nanoseconds."""
    log = math.log10
    if ns < 1e6:
        t = min(max((log(1e6) - log(ns)) / (log(1e6) - log(1.2e5)), 0), 1)
        name, percent = "success", 10 + 50 * t**1.6
    elif ns <= 16e6:
        t = (log(ns) - log(1e6)) / (log(16e6) - log(1e6))
        name, percent = "warning", 15 + 30 * t
    else:
        t = min(max((log(ns) - log(16e6)) / (log(4e8) - log(16e6)), 0), 1)
        name, percent = "destructive", 10 + 60 * t**1.8
    color = f"var(--{name},{COLORS[name]})"
    return f"background:color-mix(in srgb,{color} {percent:.0f}%,transparent)"


def fmt(ns):
    """Formats a duration in nanoseconds."""
    for unit, scale in (("s", 1e9), ("ms", 1e6), ("µs", 1e3)):
        if ns >= scale:
            return f"{ns / scale:.3g}&nbsp;{unit}"
    return f"{ns:.3g}&nbsp;ns"


def main():
    target = Path(sys.argv[1] if len(sys.argv) > 1 else "target")
    criterion = target / "criterion"
    output = Path(sys.argv[2]) if len(sys.argv) > 2 else criterion / "scroll-report.html"

    def estimate(bench, size):
        return criterion / bench / size / "new" / "estimates.json"

    paths = [estimate(bench, size) for _, benches in GROUPS for _, bench in benches for size in SIZES]
    present = [path for path in paths if path.exists()]
    newest = max(path.stat().st_mtime for path in present)
    sizes = [size for size in SIZES if any(estimate(b, size).exists() for _, g in GROUPS for _, b in g)]

    def median(bench, size):
        path = estimate(bench, size)
        if not path.exists():
            return None, False
        stale = newest - path.stat().st_mtime > STALE_SECONDS
        return json.loads(path.read_text())["median"]["point_estimate"], stale

    rows = []
    for title, benches in GROUPS:
        rows.append(f'<tr class="g"><th colspan="{len(sizes) + 1}">{title}</th></tr>')
        for label, bench in benches:
            cells = []
            for size in sizes:
                value, stale = median(bench, size)
                if value is None:
                    cells.append("<td>–</td>")
                elif stale:
                    cells.append(f'<td class="stale" title="earlier run">{fmt(value)}</td>')
                else:
                    cells.append(f'<td style="{shade(value)}">{fmt(value)}</td>')
            name = f'<td class="new">{label}</td>' if label in NEW else f"<td>{label}</td>"
            rows.append(f"<tr>{name}{''.join(cells)}</tr>")

    def swatch(ns):
        return f'<i class="sw" style="{shade(ns)}"></i>'

    legend = (
        f"<span>{swatch(1.25e5)}{swatch(2.5e5)}{swatch(6e5)}&lt; 1 ms (deeper = faster)</span>"
        f"<span>{swatch(2e6)}{swatch(1.2e7)}1–16 ms</span>"
        f"<span>{swatch(2.3e7)}{swatch(1e8)}{swatch(3.8e8)}&gt; 16 ms: faint at 20 ms, clear at"
        " 100 ms, deep near 400 ms</span><span><b>Bold</b> = added by index-based wrapping</span>"
    )
    header = "".join(f"<th>{LABELS[size]}</th>" for size in sizes)
    html = f"""<!doctype html><html><head><meta charset="utf-8"><style>{STYLE}</style></head>
<body><div class="wrap"><div class="legend">{legend}</div>
<table><thead><tr><th>Wrapped rows →</th>{header}</tr></thead><tbody>
{"".join(rows)}
</tbody></table>
<div class="note">Criterion medians per operation. Generated logs at 100 columns × 50 visible rows.
Frame times include rendering the visible rows. Greyed italic cells come from an earlier run. Paragraph's append, edit, and insert times exclude
building the Paragraph from the changed text.</div></div></body></html>
"""
    output.write_text(html)
    print(output)


if __name__ == "__main__":
    main()

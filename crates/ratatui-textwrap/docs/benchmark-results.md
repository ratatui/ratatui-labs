# `ratatui-textwrap` Benchmark Results

This report is generated from Criterion raw samples and the one-shot allocation diagnostic.

## Environment

- Platform: macOS-15.7.7-arm64-arm-64bit-Mach-O
- Processor: Apple M2 Max
- Rust: `rustc 1.97.1 (8bab26f4f 2026-07-14) (Homebrew)`
- Benchmark dependencies: Criterion 0.8.2 and `stats_alloc` 0.1.10
- Compared dependencies: Ratatui 0.30.2 and textwrap 0.16.2
- Change: `qusnlzuqzqtq`

## Reference configuration

- Seed: `0x5EED_7E57_CAFE_BABE`
- Core inputs: approximately 4 KiB, 64 KiB, and 1 MiB
- Viewport: 200×50 cells
- Resize widths: 120, 160, 200, 240, and 280 cells
- Session length: 60 frames
- Sampling: 100 ms warm-up, 200 ms target measurement, 10 samples

Use Criterion's default settings when evaluating a suspected regression.

## Interpretation

- Cold materialization processes the complete input, while native `Paragraph` can stop
  after filling the viewport.
- Count-and-render groups expose native Paragraph's repeated wrapping and the owned
  result's constant-time line count.
- Cached session and viewport groups isolate reuse after wrapping; resize-cached also
  shows the effect of caching by width.
- Allocation results are one-shot diagnostics from an instrumented allocator and are not
  part of Criterion's wall-time samples.

### Observed deltas

- In this run, cold 1 MiB viewport rendering with full materialization took 205.08–295.05× native
  time because native stopped after 50 rows.
- The 1 MiB count-then-render workflow took 3.59–4.65× native time; avoiding the second wrap did not
  offset the current materialization cost.
- At 64 KiB, steady same-width cached rendering took 0.84× native time, while wrap-once amortized
  rendering took 1.11–1.22×.
- At 64 KiB, recomputing across resize widths took 18.69–24.61× native time; caching all five widths
  reduced that to 0.82–0.83×.
- For a cached 1 MiB document, the middle viewport took 0.02× native time and the end viewport took
  0.01×.

## Timing

### `textwrap/count-then-render`

| Input  | Implementation     |    Median |  Throughput | vs. Paragraph |
| ------ | ------------------ | --------: | ----------: | ------------: |
| 1-mib  | `paragraph-native` |  27.30 ms | 36.64 MiB/s |         1.00× |
| 1-mib  | `paragraph-compat` |  98.07 ms | 10.20 MiB/s |         3.59× |
| 1-mib  | `first-fit`        | 113.58 ms |  8.81 MiB/s |         4.16× |
| 1-mib  | `optimal-fit`      | 126.81 ms |  7.89 MiB/s |         4.65× |
| 4-kib  | `paragraph-native` | 313.62 µs | 14.08 MiB/s |         1.00× |
| 4-kib  | `paragraph-compat` | 623.07 µs |  7.09 MiB/s |         1.99× |
| 4-kib  | `first-fit`        | 692.46 µs |  6.38 MiB/s |         2.21× |
| 4-kib  | `optimal-fit`      | 750.58 µs |  5.88 MiB/s |         2.39× |
| 64-kib | `paragraph-native` |   1.97 ms | 31.95 MiB/s |         1.00× |
| 64-kib | `paragraph-compat` |   6.42 ms |  9.79 MiB/s |         3.26× |
| 64-kib | `first-fit`        |   7.69 ms |  8.17 MiB/s |         3.91× |
| 64-kib | `optimal-fit`      |   8.51 ms |  7.38 MiB/s |         4.33× |

### `textwrap/fresh-render`

| Input  | Implementation     |    Median |    Throughput | vs. Paragraph |
| ------ | ------------------ | --------: | ------------: | ------------: |
| 1-mib  | `paragraph-native` | 595.40 µs | 1680.15 MiB/s |         1.00× |
| 1-mib  | `paragraph-compat` | 175.67 ms |    5.69 MiB/s |       295.05× |
| 1-mib  | `first-fit`        | 122.11 ms |    8.19 MiB/s |       205.08× |
| 1-mib  | `optimal-fit`      | 130.38 ms |    7.67 MiB/s |       218.97× |
| 4-kib  | `paragraph-native` | 196.96 µs |   22.42 MiB/s |         1.00× |
| 4-kib  | `paragraph-compat` | 854.81 µs |    5.17 MiB/s |         4.34× |
| 4-kib  | `first-fit`        | 861.50 µs |    5.13 MiB/s |         4.37× |
| 4-kib  | `optimal-fit`      | 953.09 µs |    4.63 MiB/s |         4.84× |
| 64-kib | `paragraph-native` | 759.27 µs |   82.74 MiB/s |         1.00× |
| 64-kib | `paragraph-compat` |  10.47 ms |    6.00 MiB/s |        13.80× |
| 64-kib | `first-fit`        |  10.39 ms |    6.04 MiB/s |        13.69× |
| 64-kib | `optimal-fit`      |  12.47 ms |    5.04 MiB/s |        16.43× |

### `textwrap/render-then-count`

| Input  | Implementation     |    Median |  Throughput | vs. Paragraph |
| ------ | ------------------ | --------: | ----------: | ------------: |
| 1-mib  | `paragraph-native` |  26.54 ms | 37.69 MiB/s |         1.00× |
| 1-mib  | `paragraph-compat` |  98.54 ms | 10.15 MiB/s |         3.71× |
| 1-mib  | `first-fit`        | 113.95 ms |  8.78 MiB/s |         4.29× |
| 1-mib  | `optimal-fit`      | 127.05 ms |  7.87 MiB/s |         4.79× |
| 4-kib  | `paragraph-native` | 310.82 µs | 14.21 MiB/s |         1.00× |
| 4-kib  | `paragraph-compat` | 591.30 µs |  7.47 MiB/s |         1.90× |
| 4-kib  | `first-fit`        | 687.43 µs |  6.42 MiB/s |         2.21× |
| 4-kib  | `optimal-fit`      | 747.81 µs |  5.90 MiB/s |         2.41× |
| 64-kib | `paragraph-native` |   1.99 ms | 31.64 MiB/s |         1.00× |
| 64-kib | `paragraph-compat` |   6.48 ms |  9.70 MiB/s |         3.26× |
| 64-kib | `first-fit`        |   7.69 ms |  8.17 MiB/s |         3.87× |
| 64-kib | `optimal-fit`      |   8.39 ms |  7.48 MiB/s |         4.23× |

### `textwrap/resize-cached-60-frames`

| Input  | Implementation     | Median/frame |      Throughput | vs. Paragraph |
| ------ | ------------------ | -----------: | --------------: | ------------: |
| 4-kib  | `paragraph-native` |    196.28 µs | 5094.8 frames/s |         1.00× |
| 4-kib  | `paragraph-compat` |    170.57 µs | 5862.6 frames/s |         0.87× |
| 4-kib  | `first-fit`        |    168.50 µs | 5934.7 frames/s |         0.86× |
| 4-kib  | `optimal-fit`      |    169.80 µs | 5889.1 frames/s |         0.87× |
| 64-kib | `paragraph-native` |    342.25 µs | 2921.9 frames/s |         1.00× |
| 64-kib | `paragraph-compat` |    279.63 µs | 3576.2 frames/s |         0.82× |
| 64-kib | `first-fit`        |    283.15 µs | 3531.6 frames/s |         0.83× |
| 64-kib | `optimal-fit`      |    282.82 µs | 3535.8 frames/s |         0.83× |

### `textwrap/resize-recomputed-60-frames`

| Input  | Implementation     | Median/frame |      Throughput | vs. Paragraph |
| ------ | ------------------ | -----------: | --------------: | ------------: |
| 4-kib  | `paragraph-native` |    196.21 µs | 5096.6 frames/s |         1.00× |
| 4-kib  | `paragraph-compat` |    592.02 µs | 1689.1 frames/s |         3.02× |
| 4-kib  | `first-fit`        |    683.01 µs | 1464.1 frames/s |         3.48× |
| 4-kib  | `optimal-fit`      |    740.27 µs | 1350.9 frames/s |         3.77× |
| 64-kib | `paragraph-native` |    343.01 µs | 2915.4 frames/s |         1.00× |
| 64-kib | `paragraph-compat` |      6.41 ms |  156.0 frames/s |        18.69× |
| 64-kib | `first-fit`        |      7.60 ms |  131.6 frames/s |        22.15× |
| 64-kib | `optimal-fit`      |      8.44 ms |  118.4 frames/s |        24.61× |

### `textwrap/same-width-amortized-60-frames`

| Input  | Implementation     | Median/frame |      Throughput | vs. Paragraph |
| ------ | ------------------ | -----------: | --------------: | ------------: |
| 4-kib  | `paragraph-native` |    195.01 µs | 5127.9 frames/s |         1.00× |
| 4-kib  | `paragraph-compat` |    174.68 µs | 5724.7 frames/s |         0.90× |
| 4-kib  | `first-fit`        |    177.20 µs | 5643.3 frames/s |         0.91× |
| 4-kib  | `optimal-fit`      |    178.71 µs | 5595.5 frames/s |         0.92× |
| 64-kib | `paragraph-native` |    350.07 µs | 2856.6 frames/s |         1.00× |
| 64-kib | `paragraph-compat` |    389.89 µs | 2564.8 frames/s |         1.11× |
| 64-kib | `first-fit`        |    414.61 µs | 2411.9 frames/s |         1.18× |
| 64-kib | `optimal-fit`      |    428.57 µs | 2333.4 frames/s |         1.22× |

### `textwrap/same-width-cached-60-frames`

| Input  | Implementation     | Median/frame |      Throughput | vs. Paragraph |
| ------ | ------------------ | -----------: | --------------: | ------------: |
| 4-kib  | `paragraph-native` |    196.46 µs | 5090.1 frames/s |         1.00× |
| 4-kib  | `paragraph-compat` |    171.54 µs | 5829.7 frames/s |         0.87× |
| 4-kib  | `first-fit`        |    173.96 µs | 5748.3 frames/s |         0.89× |
| 4-kib  | `optimal-fit`      |    169.56 µs | 5897.6 frames/s |         0.86× |
| 64-kib | `paragraph-native` |    350.32 µs | 2854.6 frames/s |         1.00× |
| 64-kib | `paragraph-compat` |    292.61 µs | 3417.6 frames/s |         0.84× |
| 64-kib | `first-fit`        |    293.65 µs | 3405.4 frames/s |         0.84× |
| 64-kib | `optimal-fit`      |    293.60 µs | 3406.0 frames/s |         0.84× |

### `textwrap/viewport-cached`

| Input         | Implementation     |    Median | Throughput | vs. Paragraph |
| ------------- | ------------------ | --------: | ---------: | ------------: |
| 1-mib/end     | `paragraph-native` |  26.19 ms |          — |         1.00× |
| 1-mib/end     | `paragraph-compat` | 280.59 µs |          — |         0.01× |
| 1-mib/end     | `first-fit`        | 281.44 µs |          — |         0.01× |
| 1-mib/end     | `optimal-fit`      | 282.02 µs |          — |         0.01× |
| 1-mib/middle  | `paragraph-native` |  13.31 ms |          — |         1.00× |
| 1-mib/middle  | `paragraph-compat` | 285.91 µs |          — |         0.02× |
| 1-mib/middle  | `first-fit`        | 284.40 µs |          — |         0.02× |
| 1-mib/middle  | `optimal-fit`      | 289.62 µs |          — |         0.02× |
| 1-mib/start   | `paragraph-native` | 339.80 µs |          — |         1.00× |
| 1-mib/start   | `paragraph-compat` | 284.58 µs |          — |         0.84× |
| 1-mib/start   | `first-fit`        | 282.70 µs |          — |         0.83× |
| 1-mib/start   | `optimal-fit`      | 283.24 µs |          — |         0.83× |
| 4-kib/end     | `paragraph-native` | 200.76 µs |          — |         1.00× |
| 4-kib/end     | `paragraph-compat` | 173.47 µs |          — |         0.86× |
| 4-kib/end     | `first-fit`        | 173.05 µs |          — |         0.86× |
| 4-kib/end     | `optimal-fit`      | 172.81 µs |          — |         0.86× |
| 4-kib/middle  | `paragraph-native` | 200.96 µs |          — |         1.00× |
| 4-kib/middle  | `paragraph-compat` | 173.74 µs |          — |         0.86× |
| 4-kib/middle  | `first-fit`        | 175.08 µs |          — |         0.87× |
| 4-kib/middle  | `optimal-fit`      | 174.12 µs |          — |         0.87× |
| 4-kib/start   | `paragraph-native` | 201.46 µs |          — |         1.00× |
| 4-kib/start   | `paragraph-compat` | 173.10 µs |          — |         0.86× |
| 4-kib/start   | `first-fit`        | 173.05 µs |          — |         0.86× |
| 4-kib/start   | `optimal-fit`      | 174.95 µs |          — |         0.87× |
| 64-kib/end    | `paragraph-native` |   1.71 ms |          — |         1.00× |
| 64-kib/end    | `paragraph-compat` | 272.91 µs |          — |         0.16× |
| 64-kib/end    | `first-fit`        | 277.20 µs |          — |         0.16× |
| 64-kib/end    | `optimal-fit`      | 278.11 µs |          — |         0.16× |
| 64-kib/middle | `paragraph-native` |   1.06 ms |          — |         1.00× |
| 64-kib/middle | `paragraph-compat` | 287.77 µs |          — |         0.27× |
| 64-kib/middle | `first-fit`        | 293.29 µs |          — |         0.28× |
| 64-kib/middle | `optimal-fit`      | 281.22 µs |          — |         0.27× |
| 64-kib/start  | `paragraph-native` | 352.20 µs |          — |         1.00× |
| 64-kib/start  | `paragraph-compat` | 294.68 µs |          — |         0.84× |
| 64-kib/start  | `first-fit`        | 296.70 µs |          — |         0.84× |
| 64-kib/start  | `optimal-fit`      | 291.55 µs |          — |         0.83× |

### `textwrap/viewport-cold`

| Input         | Implementation     |    Median | Throughput | vs. Paragraph |
| ------------- | ------------------ | --------: | ---------: | ------------: |
| 1-mib/end     | `paragraph-native` |  25.43 ms |          — |         1.00× |
| 1-mib/end     | `paragraph-compat` |  98.57 ms |          — |         3.88× |
| 1-mib/end     | `first-fit`        | 114.26 ms |          — |         4.49× |
| 1-mib/end     | `optimal-fit`      | 128.23 ms |          — |         5.04× |
| 1-mib/middle  | `paragraph-native` |  13.14 ms |          — |         1.00× |
| 1-mib/middle  | `paragraph-compat` |  97.98 ms |          — |         7.45× |
| 1-mib/middle  | `first-fit`        | 114.75 ms |          — |         8.73× |
| 1-mib/middle  | `optimal-fit`      | 128.22 ms |          — |         9.76× |
| 1-mib/start   | `paragraph-native` | 335.67 µs |          — |         1.00× |
| 1-mib/start   | `paragraph-compat` |  97.64 ms |          — |       290.88× |
| 1-mib/start   | `first-fit`        | 114.46 ms |          — |       340.99× |
| 1-mib/start   | `optimal-fit`      | 127.69 ms |          — |       380.40× |
| 4-kib/end     | `paragraph-native` | 197.16 µs |          — |         1.00× |
| 4-kib/end     | `paragraph-compat` | 583.81 µs |          — |         2.96× |
| 4-kib/end     | `first-fit`        | 704.21 µs |          — |         3.57× |
| 4-kib/end     | `optimal-fit`      | 770.75 µs |          — |         3.91× |
| 4-kib/middle  | `paragraph-native` | 200.55 µs |          — |         1.00× |
| 4-kib/middle  | `paragraph-compat` | 574.89 µs |          — |         2.87× |
| 4-kib/middle  | `first-fit`        | 698.21 µs |          — |         3.48× |
| 4-kib/middle  | `optimal-fit`      | 783.65 µs |          — |         3.91× |
| 4-kib/start   | `paragraph-native` | 200.79 µs |          — |         1.00× |
| 4-kib/start   | `paragraph-compat` | 589.19 µs |          — |         2.93× |
| 4-kib/start   | `first-fit`        | 700.10 µs |          — |         3.49× |
| 4-kib/start   | `optimal-fit`      | 779.45 µs |          — |         3.88× |
| 64-kib/end    | `paragraph-native` |   1.78 ms |          — |         1.00× |
| 64-kib/end    | `paragraph-compat` |   6.48 ms |          — |         3.65× |
| 64-kib/end    | `first-fit`        |   7.58 ms |          — |         4.26× |
| 64-kib/end    | `optimal-fit`      |   8.48 ms |          — |         4.77× |
| 64-kib/middle | `paragraph-native` |   1.08 ms |          — |         1.00× |
| 64-kib/middle | `paragraph-compat` |   6.49 ms |          — |         6.01× |
| 64-kib/middle | `first-fit`        |   7.70 ms |          — |         7.13× |
| 64-kib/middle | `optimal-fit`      |   8.53 ms |          — |         7.90× |
| 64-kib/start  | `paragraph-native` | 361.37 µs |          — |         1.00× |
| 64-kib/start  | `paragraph-compat` |   6.47 ms |          — |        17.91× |
| 64-kib/start  | `first-fit`        |   7.70 ms |          — |        21.31× |
| 64-kib/start  | `optimal-fit`      |   8.52 ms |          — |        23.57× |

### `textwrap/wrap-or-count`

| Input  | Implementation     |    Median |  Throughput | vs. Paragraph |
| ------ | ------------------ | --------: | ----------: | ------------: |
| 1-mib  | `paragraph-native` |  26.26 ms | 38.09 MiB/s |         1.00× |
| 1-mib  | `paragraph-compat` |  95.81 ms | 10.44 MiB/s |         3.65× |
| 1-mib  | `first-fit`        | 109.04 ms |  9.17 MiB/s |         4.15× |
| 1-mib  | `optimal-fit`      | 124.41 ms |  8.04 MiB/s |         4.74× |
| 4-kib  | `paragraph-native` | 110.42 µs | 39.99 MiB/s |         1.00× |
| 4-kib  | `paragraph-compat` | 403.87 µs | 10.93 MiB/s |         3.66× |
| 4-kib  | `first-fit`        | 493.12 µs |  8.95 MiB/s |         4.47× |
| 4-kib  | `optimal-fit`      | 549.26 µs |  8.04 MiB/s |         4.97× |
| 64-kib | `paragraph-native` |   1.54 ms | 40.67 MiB/s |         1.00× |
| 64-kib | `paragraph-compat` |   5.98 ms | 10.50 MiB/s |         3.87× |
| 64-kib | `first-fit`        |   7.14 ms |  8.81 MiB/s |         4.62× |
| 64-kib | `optimal-fit`      |   7.98 ms |  7.87 MiB/s |         5.17× |

## Allocations

| Workload                       | Implementation     | Allocations | Reallocations | Bytes allocated | vs. Paragraph |
| ------------------------------ | ------------------ | ----------: | ------------: | --------------: | ------------: |
| count-then-render              | `paragraph-native` |         334 |         1,581 |       2,710,528 |         1.00× |
| count-then-render              | `paragraph-compat` |      70,304 |         9,346 |       9,801,077 |         3.62× |
| count-then-render              | `first-fit`        |      92,618 |        17,587 |       9,501,034 |         3.51× |
| count-then-render              | `optimal-fit`      |      93,178 |        17,653 |      10,174,548 |         3.75× |
| resize-cached-60-frames        | `paragraph-native` |       2,712 |        11,784 |      17,884,416 |         1.00× |
| resize-cached-60-frames        | `paragraph-compat` |          60 |           360 |         540,672 |         0.03× |
| resize-cached-60-frames        | `first-fit`        |          60 |           360 |         540,672 |         0.03× |
| resize-cached-60-frames        | `optimal-fit`      |          60 |           360 |         540,672 |         0.03× |
| resize-recomputed-60-frames    | `paragraph-native` |       2,712 |        11,784 |      17,884,416 |         1.00× |
| resize-recomputed-60-frames    | `paragraph-compat` |   4,223,796 |       565,752 |     563,992,176 |        31.54× |
| resize-recomputed-60-frames    | `first-fit`        |   5,560,812 |     1,055,340 |     570,475,308 |        31.90× |
| resize-recomputed-60-frames    | `optimal-fit`      |   5,594,244 |     1,058,844 |     610,864,464 |        34.16× |
| same-width-amortized-60-frames | `paragraph-native` |       2,640 |        12,180 |      20,286,720 |         1.00× |
| same-width-amortized-60-frames | `paragraph-compat` |      70,363 |         9,700 |      10,284,405 |         0.51× |
| same-width-amortized-60-frames | `first-fit`        |      92,677 |        17,941 |       9,984,362 |         0.49× |
| same-width-amortized-60-frames | `optimal-fit`      |      93,237 |        18,007 |      10,657,876 |         0.53× |
| same-width-cached-60-frames    | `paragraph-native` |       2,640 |        12,180 |      20,286,720 |         1.00× |
| same-width-cached-60-frames    | `paragraph-compat` |          60 |           360 |         491,520 |         0.02× |
| same-width-cached-60-frames    | `first-fit`        |          60 |           360 |         491,520 |         0.02× |
| same-width-cached-60-frames    | `optimal-fit`      |          60 |           360 |         491,520 |         0.02× |
| viewport-middle-cached         | `paragraph-native` |         168 |           804 |       1,376,544 |         1.00× |
| viewport-middle-cached         | `paragraph-compat` |           1 |             6 |           8,192 |         0.01× |
| viewport-middle-cached         | `first-fit`        |           1 |             6 |           8,192 |         0.01× |
| viewport-middle-cached         | `optimal-fit`      |           1 |             6 |           8,192 |         0.01× |
| viewport-middle-cold           | `paragraph-native` |         168 |           804 |       1,376,544 |         1.00× |
| viewport-middle-cold           | `paragraph-compat` |      70,304 |         9,346 |       9,801,077 |         7.12× |
| viewport-middle-cold           | `first-fit`        |      92,618 |        17,587 |       9,501,034 |         6.90× |
| viewport-middle-cold           | `optimal-fit`      |      93,178 |        17,653 |      10,174,548 |         7.39× |
| wrap-or-count                  | `paragraph-native` |         290 |         1,378 |       2,372,416 |         1.00× |
| wrap-or-count                  | `paragraph-compat` |      70,303 |         9,340 |       9,792,885 |         4.13× |
| wrap-or-count                  | `first-fit`        |      92,617 |        17,581 |       9,492,842 |         4.00× |
| wrap-or-count                  | `optimal-fit`      |      93,177 |        17,647 |      10,166,356 |         4.29× |

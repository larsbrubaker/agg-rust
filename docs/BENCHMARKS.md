# AGG.rs Benchmarks — Rust vs C++

- **Generated:** 2026-07-22
- **Machine:** Intel(R) Core(TM) i7-7660U CPU @ 2.50GHz
- **OS:** Microsoft Windows [Version 10.0.19045.7548]
- **Rust compiler:** rustc 1.91.0 (f8297e351 2025-10-28)
- **C++ compiler:** MSVC 19.44.35219.0
- **Iterations:** 4 passes x 25 timed + 2 warmups per pass

## Methodology

Each demo is rendered by both the Rust port (in-process, via `pixel-compare`) and the original AGG 2.6 C++ library (the `agg-render` subprocess). Timings cover **the render call only** — no process startup, asset loading, or file I/O is included. Each side runs the same number of untimed warmup iterations followed by the same number of timed iterations. From those per-iteration samples both the **best (minimum)** and the **median** are reported, computed identically on both sides. **Compare best-of first** — the fastest run is the least contaminated by OS scheduling jitter, so it is the primary signal; the median is a secondary sanity check that resists outliers. Single runs are noise, and differences below roughly ±2 ms are at or under the measurement floor on this machine. Both sides render at identical sizes with identical parameters.

This table is generated with **4 passes x 25 iterations, 2 warmups per pass**; each pass sweeps every demo (interleaving the Rust and C++ measurement of one demo before moving to the next), and the **best and median are computed over all pooled samples** from every pass. Pooling across passes is what makes the numbers robust on a machine that is not perfectly idle: a transient load spike can only inflate the samples in one pass, so it cannot survive the pooled minimum, and the pooled median stabilizes as the sample count grows.

Critically, both renderers draw **the same scene** at the same size. Every demo in the table below is byte-identical: a committed pixel-compare reference test pins the Rust output byte-for-byte against the C++ output, so each speed difference reflects the implementation rather than a difference in what is drawn.

## Results

| Demo | Size | Byte-identical | C++ best (ms) | Rust best (ms) | Best Rust / C++ | C++ median (ms) | Rust median (ms) | Median Rust / C++ |
|------|------|----------------|---------------|----------------|-----------------|-----------------|------------------|-------------------|
| simple_line | 512x512 | yes | 0.34 | 0.22 | 0.66x | 0.52 | 0.29 | 0.56x |
| lion_outline | 512x512 | yes | 2.65 | 2.28 | 0.86x | 4.05 | 3.10 | 0.77x |
| rasterizers2 | 500x450 | yes | 1.86 | 2.00 | 1.08x | 2.46 | 3.09 | 1.26x |
| conv_dash_marker | 500x330 | yes | 1.40 | 1.42 | 1.02x | 1.71 | 2.10 | 1.22x |
| perspective | 600x600 | yes | 3.11 | 2.99 | 0.96x | 4.42 | 3.88 | 0.88x |
| image_perspective | 600x600 | yes | 6.19 | 6.06 | 0.98x | 8.79 | 8.74 | 0.99x |
| image_transforms | 430x340 | yes | 2.23 | 1.62 | 0.73x | 2.92 | 2.33 | 0.80x |
| image_filters | 430x340 | yes | 3.91 | 3.39 | 0.87x | 5.34 | 4.54 | 0.85x |
| compositing2 | 600x400 | yes | 4.84 | 4.77 | 0.99x | 6.80 | 6.71 | 0.99x |
| flash_rasterizer | 655x520 | yes | 2.73 | 2.15 | 0.79x | 3.69 | 2.93 | 0.79x |
| flash_rasterizer2 | 655x520 | yes | 2.58 | 1.97 | 0.76x | 3.38 | 2.63 | 0.78x |

The **Byte-identical** column records, per row, that the demo's Rust output is pinned byte-for-byte against the C++ reference by a committed test — the invariant that makes each timing an apples-to-apples comparison.

## Regenerating

```bash
# 1. Build the Rust benchmark tool (release):
cargo build --release -p pixel-compare

# 2. Build the C++ reference renderer (release):
cmake -S tools/cpp-renderer -B tools/cpp-renderer/build -A x64
cmake --build tools/cpp-renderer/build --config Release

# 3. Run the full suite and regenerate this file:
target\release\pixel-compare bench-compare \
  --cpp tools\cpp-renderer\build\Release\agg-render.exe \
  --passes 4 --iters 25 --date 2026-07-22 --out docs\BENCHMARKS.md
```

# AGG.rs Benchmarks — Rust vs C++

- **Generated:** 2026-07-22
- **Machine:** Intel(R) Core(TM) i7-7660U CPU @ 2.50GHz
- **OS:** Microsoft Windows [Version 10.0.19045.7548]
- **Rust compiler:** rustc 1.91.0 (f8297e351 2025-10-28)
- **C++ compiler:** MSVC 19.44.35219.0
- **Iterations:** 25 timed + 2 warmups per demo

## Methodology

Each demo is rendered by both the Rust port (in-process, via `pixel-compare`) and the original AGG 2.6 C++ library (the `agg-render` subprocess). Timings cover **the render call only** — no process startup, asset loading, or file I/O is included. Each side runs the same number of untimed warmup iterations followed by the same number of timed iterations. From those per-iteration samples both the **best (minimum)** and the **median** are reported, computed identically on both sides. **Compare best-of first** — the fastest run is the least contaminated by OS scheduling jitter, so it is the primary signal; the median is a secondary sanity check that resists outliers. Single runs are noise, and differences below roughly ±2 ms are at or under the measurement floor on this machine. Both sides render at identical sizes with identical parameters.

Critically, both renderers draw **the same scene** at the same size. Every demo in the table below is byte-identical: a committed pixel-compare reference test pins the Rust output byte-for-byte against the C++ output, so each speed difference reflects the implementation rather than a difference in what is drawn.

## Results

| Demo | Size | Byte-identical | C++ best (ms) | Rust best (ms) | Best Rust / C++ | C++ median (ms) | Rust median (ms) | Median Rust / C++ |
|------|------|----------------|---------------|----------------|-----------------|-----------------|------------------|-------------------|
| simple_line | 512x512 | yes | 0.39 | 0.72 | 1.84x | 0.49 | 0.80 | 1.63x |
| lion_outline | 512x512 | yes | 2.65 | 2.67 | 1.01x | 2.69 | 3.08 | 1.14x |
| rasterizers2 | 500x450 | yes | 1.79 | 2.46 | 1.37x | 1.96 | 3.00 | 1.53x |
| conv_dash_marker | 500x330 | yes | 1.35 | 1.64 | 1.21x | 1.50 | 2.37 | 1.58x |
| perspective | 600x600 | yes | 2.98 | 3.55 | 1.19x | 3.26 | 3.90 | 1.20x |
| image_perspective | 600x600 | yes | 6.04 | 7.15 | 1.18x | 6.22 | 7.81 | 1.26x |
| image_transforms | 430x340 | yes | 2.37 | 2.16 | 0.91x | 2.73 | 3.55 | 1.30x |
| image_filters | 430x340 | yes | 3.77 | 3.70 | 0.98x | 4.10 | 7.17 | 1.75x |
| compositing2 | 600x400 | yes | 4.77 | 5.01 | 1.05x | 4.86 | 5.33 | 1.10x |
| flash_rasterizer | 655x520 | yes | 2.60 | 2.81 | 1.08x | 2.65 | 3.62 | 1.37x |
| flash_rasterizer2 | 655x520 | yes | 2.53 | 2.47 | 0.98x | 2.71 | 2.62 | 0.97x |

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
  --date 2026-07-22 --out docs\BENCHMARKS.md
```

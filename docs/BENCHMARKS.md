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
| simple_line | 512x512 | yes | 0.42 | 0.74 | 1.76x | 0.47 | 0.81 | 1.71x |
| lion_outline | 512x512 | yes | 2.67 | 3.21 | 1.20x | 3.10 | 3.28 | 1.06x |
| rasterizers2 | 500x450 | yes | 1.96 | 2.37 | 1.21x | 2.06 | 2.67 | 1.30x |
| conv_dash_marker | 500x330 | yes | 1.33 | 1.66 | 1.25x | 1.55 | 1.68 | 1.08x |
| perspective | 600x600 | yes | 3.01 | 3.78 | 1.26x | 3.35 | 5.15 | 1.54x |
| image_perspective | 600x600 | yes | 6.24 | 7.20 | 1.15x | 7.61 | 8.36 | 1.10x |
| image_transforms | 430x340 | yes | 2.29 | 1.80 | 0.79x | 2.42 | 2.02 | 0.83x |
| image_filters | 430x340 | yes | 3.87 | 3.65 | 0.94x | 4.33 | 3.83 | 0.88x |
| compositing2 | 600x400 | yes | 4.78 | 4.81 | 1.01x | 6.40 | 5.67 | 0.89x |
| flash_rasterizer | 655x520 | yes | 2.55 | 4.91 | 1.93x | 2.73 | 5.54 | 2.03x |
| flash_rasterizer2 | 655x520 | yes | 2.44 | 4.68 | 1.92x | 2.76 | 5.22 | 1.89x |

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

# AGG.rs Benchmarks — Rust vs C++

- **Generated:** 2026-07-22
- **Machine:** Intel(R) Core(TM) i7-7660U CPU @ 2.50GHz
- **OS:** Microsoft Windows [Version 10.0.19045.7548]
- **Rust compiler:** rustc 1.91.0 (f8297e351 2025-10-28)
- **C++ compiler:** MSVC 19.44.35219.0
- **Iterations:** 10 timed + 2 warmups per demo

## Methodology

Each demo is rendered by both the Rust port (in-process, via `pixel-compare`) and the original AGG 2.6 C++ library (the `agg-render` subprocess). Timings cover **the render call only** — no process startup, asset loading, or file I/O is included. Each side runs the same number of untimed warmup iterations followed by the same number of timed iterations, and the median of the per-iteration samples is reported (medians resist outliers from OS scheduling jitter). Both sides render at identical sizes with identical parameters.

Critically, both renderers draw **the same scene** at the same size. For the demos marked byte-identical in the table below, a committed pixel-compare reference test pins the Rust output byte-for-byte against the C++ output, so their speed difference reflects the implementation rather than a difference in what is drawn. The remaining demos render the same scene and match visually, but are not yet pinned by a byte-compare test.

## Results

| Demo | Size | Byte-identical | C++ median (ms) | Rust median (ms) | Rust / C++ |
|------|------|----------------|-----------------|------------------|------------|
| simple_line | 512x512 | — | 0.41 | 0.86 | 2.09x |
| lion_outline | 512x512 | yes | 3.10 | 3.12 | 1.00x |
| rasterizers2 | 500x450 | yes | 2.55 | 2.97 | 1.16x |
| conv_dash_marker | 500x330 | yes | 1.39 | 2.10 | 1.51x |
| perspective | 600x600 | — | 3.96 | 4.42 | 1.12x |
| image_perspective | 600x600 | — | 6.96 | 7.98 | 1.15x |
| image_transforms | 430x340 | — | 3.76 | 2.69 | 0.72x |
| image_filters | 430x340 | — | 5.33 | 4.03 | 0.76x |
| compositing2 | 600x400 | yes | 6.63 | 8.24 | 1.24x |
| flash_rasterizer | 655x520 | yes | 5.78 | 6.74 | 1.17x |
| flash_rasterizer2 | 655x520 | yes | 4.28 | 11.97 | 2.80x |

The **Byte-identical** column marks demos whose Rust output is pinned byte-for-byte against the C++ reference by a committed test. A `—` means the scene matches visually but is not yet covered by a byte-compare test.
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

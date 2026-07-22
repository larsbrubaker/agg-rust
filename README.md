# AGG.rs — Anti-Grain Geometry for Rust

[![License: BSD-3-Clause](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/agg-rust.svg)](https://crates.io/crates/agg-rust)
[![Tests](https://img.shields.io/badge/tests-983_passing-brightgreen.svg)](#)
[![Modules](https://img.shields.io/badge/modules-88_ported-brightgreen.svg)](#)
[![Demos](https://img.shields.io/badge/demos-64_interactive-orange.svg)](https://larsbrubaker.github.io/agg-rust/)

## Support the Project

<a href="https://buymeacoffee.com/larsbrubaker"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50" width="210"></a>

AGG.rs is open-source and free to use, maintained in spare time as a labor of love. Friends James Smith and Dan Ruskin help out from time to time too.

If you find it useful, here are a few ways to help keep development going:

- **Donations:** [Buy Me a Coffee](https://buymeacoffee.com/larsbrubaker) — every coffee helps.
- **Star the repo:** Costs nothing and helps others find the project.
- **Report issues:** [Open an issue](https://github.com/larsbrubaker/agg-rust/issues) for bugs or feature ideas.
- **Contribute:** PRs welcome — open an issue first to discuss larger changes.

A pure Rust port of [Anti-Grain Geometry (AGG) 2.6](https://github.com/ghaerr/agg-2.6) — the legendary high-quality 2D software rendering library originally written in C++ by [Maxim Shemanarev](http://www.antigrain.com). Zero external dependencies. Pixel-perfect anti-aliased output. No GPU required.

**[Try the Interactive Demo](https://larsbrubaker.github.io/agg-rust/)** — 64 demos running entirely in your browser via WebAssembly.

Crate listing: **[`agg-rust` on crates.io](https://crates.io/crates/agg-rust)**.

> Part of the [rust-apps](https://github.com/larsbrubaker/rust-apps) suite — a collection of Rust graphics and geometry libraries by Lars Brubaker.

<p align="center">
  <a href="https://larsbrubaker.github.io/agg-rust/">
    <img src="docs/screenshot.png" alt="AGG.rs Interactive Demo — Lion rendering with anti-aliased vector graphics" width="800">
  </a>
</p>

## Features

AGG is a software rendering engine that produces pixel images in memory from vectorial data. It is platform-independent and achieves exceptional rendering quality through:

- **Anti-Aliasing** — subpixel-accurate scanline rasterization
- **Affine & Perspective Transforms** — rotation, scaling, skewing, and full perspective warps
- **Gradient Fills** — linear, radial, focal-point, and custom gradient functions with multi-stop color interpolation
- **Gouraud Shading** — smooth per-vertex color interpolation across triangles and meshes
- **Image Filtering** — 17 interpolation filters including bilinear, bicubic, sinc, Blackman, and more
- **30+ Compositing Modes** — full SVG 1.2 compatible Porter-Duff and blend operations
- **Stroke & Dash Generation** — configurable line joins, caps, dashes, and markers
- **Alpha Masking** — arbitrary clip regions through grayscale alpha masks
- **Stack Blur** — fast approximate Gaussian blur with adjustable radius
- **Pattern Fills** — tiled and resampled pattern rendering with perspective support
- **Built-in Fonts** — 34 embedded bitmap fonts plus vector text via GSV text engine
- **Boolean Operations** — scanline-level union, intersection, difference, and XOR

## Architecture

AGG uses a five-stage rendering pipeline with interchangeable components:

```
Vertex Source → Coordinate Conversion → Scanline Rasterizer → Scanline Container → Renderer
```

Each stage is a trait in the Rust port, allowing components to be freely mixed and matched:

| Stage | Purpose | Examples |
|-------|---------|----------|
| **Vertex Source** | Generates path vertices | `PathStorage`, `Ellipse`, `RoundedRect`, `GsvText` |
| **Coordinate Conversion** | Transforms and processes paths | `ConvCurve`, `ConvStroke`, `ConvDash`, `ConvTransform` |
| **Scanline Rasterizer** | Converts paths to scanlines | `RasterizerScanlineAa`, `RasterizerCompoundAa` |
| **Scanline Container** | Stores scanline coverage data | `ScanlineU8`, `ScanlineP8`, `ScanlineBin` |
| **Renderer** | Writes pixels to the buffer | `RendererScanlineAaSolid`, `RendererBase`, pixel formats |

## Interactive Demos

All 64 demos run in-browser via WebAssembly with no server-side processing. Categories include:

| Category | Demos | Highlights |
|----------|-------|------------|
| **Anti-Aliasing** | AA Demo, Rasterizers, Gamma, AA Test | Subpixel rendering quality visualization |
| **Rendering** | Lion, Perspective, Circles, Alpha Masks, Blur | Complex vector scenes with transforms |
| **Gradients** | Linear/Radial, Gouraud, Focal Point, Mesh | Multi-stop color interpolation |
| **Paths & Strokes** | Stroke, Contour, Dash, Line Patterns | Join/cap styles, dash patterns |
| **Curves** | Bezier, B-Spline, Text on Curve | Interactive control point editing |
| **Images & Filters** | 17 Filter Types, Perspective, Resample | Image transform and filter quality |
| **Compositing** | SVG Blend Modes, Flash Rasterizer | Porter-Duff and blend operations |
| **Patterns** | Pattern Fill, Perspective, Resample | Tiled pattern rendering |
| **Text** | GSV Vector Text, 34 Raster Fonts | Built-in font rendering |

**[Browse all demos →](https://larsbrubaker.github.io/agg-rust/)**

## Quick Start

```toml
[dependencies]
agg-rust = "1.0"
```

```rust
use agg_rust::*;

// Create a rendering buffer
let mut buf = vec![0u8; width * height * 4];
let mut rbuf = RenderingBuffer::new(&mut buf, width, height, width * 4);

// Set up the pixel format and renderer
let mut pixfmt = PixfmtRgba32::new(&mut rbuf);
let mut ren_base = RendererBase::new(&mut pixfmt);
ren_base.clear(&Rgba8::new(255, 255, 255, 255));

// Create a path and rasterize it
let mut path = PathStorage::new();
path.move_to(10.0, 10.0);
path.line_to(100.0, 50.0);
path.line_to(50.0, 100.0);
path.close_polygon();

let mut ras = RasterizerScanlineAa::new();
let mut sl = ScanlineU8::new();
ras.add_path(&mut path);
render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren_base, &Rgba8::new(200, 80, 80, 255));
```

## Development

### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- wasm-pack (for WASM demos)
- Bun (for demo dev server)

### Building & Testing

```bash
cargo build
cargo test                    # 983 tests
cargo clippy -- -D warnings
```

### Running the Demo Locally

```bash
cd demo
bun install
bun run build:wasm
bun run dev
```

Then open `http://localhost:3000` in your browser.

## Project Status

All 88 core library modules have been ported from the C++ original with 983 tests passing. All 64 applicable demos are fully implemented and running via WebAssembly.

| Metric | Value |
|--------|-------|
| Core modules ported | 88 |
| Tests passing | 983 |
| Interactive demos | 64 |
| External dependencies | 0 |
| GPU dependencies | 0 |

## Benchmarks

The Rust port is benchmarked head-to-head against the original AGG 2.6 C++ library
on a shared set of demos. Both sides render **the same scene at the same size**, and
timings cover **the render call only** (no process startup, asset loading, or file
I/O). Each demo runs 2 untimed warmups followed by 10 timed iterations, and the
**median** of the per-iteration samples is reported.

The key credibility claim: both renderers draw the same scene. For the demos marked
**byte-identical** below, a committed `pixel-compare` reference test pins the Rust
output byte-for-byte against the C++ output, so their speed difference reflects the
implementation — not a difference in what is drawn. The remaining demos render the
same scene and match visually, but are not yet pinned by a byte-compare test.

Measured on an Intel Core i7-7660U @ 2.50GHz (Windows 10 19045), rustc 1.91.0 vs
MSVC 19.44 (2026-07-22). Run-to-run variance from OS scheduling is expected:

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

The **Byte-identical** column marks demos whose Rust output is pinned byte-for-byte
against the C++ reference by a committed test; a `—` means the scene matches visually
but is not yet covered by a byte-compare test.

Full methodology, machine details, and compiler versions are in
[docs/BENCHMARKS.md](docs/BENCHMARKS.md). Regenerate the whole suite (build both
sides in release, run every demo, rewrite the doc) with:

```bash
cargo build --release -p pixel-compare
cmake -S tools/cpp-renderer -B tools/cpp-renderer/build -A x64
cmake --build tools/cpp-renderer/build --config Release
target\release\pixel-compare bench-compare \
  --cpp tools\cpp-renderer\build\Release\agg-render.exe --out docs\BENCHMARKS.md
```

## License

BSD-3-Clause — see [LICENSE](LICENSE).

Based on the original Anti-Grain Geometry library by Maxim Shemanarev, dual-licensed under the Modified BSD License and the Anti-Grain Geometry Public License.

**Note**: The GPC (General Polygon Clipper) component from the original library is excluded from this port due to its non-commercial license restriction.

## Acknowledgments

- **Maxim Shemanarev** (1966–2013) — creator of Anti-Grain Geometry, a masterwork of C++ library design
- **[ghaerr/agg-2.6](https://github.com/ghaerr/agg-2.6)** — the GitHub mirror of AGG 2.6 used as reference
- Ported by **Lars Brubaker**, sponsored by **[MatterHackers](https://www.matterhackers.com)**

# agg-rust

Pure Rust port of [Anti-Grain Geometry (AGG) 2.6](https://github.com/ghaerr/agg-2.6) — a high quality 2D vector graphics rendering library originally written in C++ by [Maxim Shemanarev](http://www.antigrain.com).

> **Status: In Development** — Porting in progress. See [Porting Plan](docs/track/porting-plan.md) for roadmap.

## What is AGG?

Anti-Grain Geometry is an Open Source 2D vector graphics rendering engine that produces pixel images in memory from vectorial data. It is platform-independent, has zero external dependencies, and achieves exceptional rendering quality through:

- **Anti-Aliasing** with subpixel accuracy
- **Affine and perspective** transformations
- **Gradient, pattern, and Gouraud shading** fills
- **Image filtering** with multiple interpolation filters (bilinear, bicubic, sinc, Blackman)
- **30+ compositing modes** (SVG 1.2 compatible)
- **Stroke, dash, and contour** generation
- **Alpha masking** and multi-clip regions
- **Stack blur** and recursive Gaussian filter
- **Built-in vector and raster fonts**

## Architecture

AGG uses a five-stage rendering pipeline that can be composed from interchangeable components:

```
Vertex Source → Coordinate Conversion → Scanline Rasterizer → Scanline Container → Renderer
```

Each stage is a trait in the Rust port, allowing components to be freely mixed and matched.

## Quick Start

```toml
[dependencies]
agg-rust = "0.1"
```

```rust
use agg_rust::*;

// Coming soon — Phase 1 implementation in progress
```

## Development

### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- wasm-pack (for WASM demos)
- Bun (for demo dev server)

### Building

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

### Running Examples

```bash
cargo run --example <name>
```

### Demo Website

```bash
cd demo
bun install
bun run build:wasm
bun run dev
```

## Porting Progress

| Phase | Description | Status |
|-------|------------|--------|
| 0 | Project Setup | Done |
| 1 | Foundation Types & Math | Pending |
| 2 | Memory & Geometry Primitives | Pending |
| 3 | Scanline Rasterizer (Core) | Pending |
| 4 | Pixel Formats & Renderers | Pending |
| 5 | Converter Pipeline | Pending |
| 6 | Span Generators & Image Processing | Pending |
| 7 | Text & Advanced Transforms | Pending |
| 8 | WASM Demo Website | Pending |

## License

BSD-3-Clause — see [LICENSE](LICENSE).

Based on the original Anti-Grain Geometry library by Maxim Shemanarev, dual-licensed under the Modified BSD License and the Anti-Grain Geometry Public License.

**Note**: The GPC (General Polygon Clipper) component from the original library is excluded from this port due to its non-commercial license restriction.

## Acknowledgments

- **Maxim Shemanarev** (1966–2013) — creator of Anti-Grain Geometry, a masterwork of C++ library design
- **[ghaerr/agg-2.6](https://github.com/ghaerr/agg-2.6)** — the GitHub mirror of AGG 2.6 used as reference
- **[clipper2-rust](https://github.com/anthropics/clipper2-rust)** — our previous Rust port whose tooling and methodology we build upon

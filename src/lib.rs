//! # agg-rust
//!
//! Pure Rust port of Anti-Grain Geometry (AGG) 2.6 — a high quality 2D vector
//! graphics rendering library originally written in C++ by Maxim Shemanarev.
//!
//! AGG produces pixel images in memory from vectorial data. It features:
//!
//! - Anti-aliased rendering with subpixel accuracy
//! - Affine and perspective transformations
//! - Multiple pixel format renderers (RGBA, RGB, grayscale, packed)
//! - Gradient and image pattern fills
//! - Stroke, dash, and contour generation
//! - Gouraud shading
//! - Image filtering and resampling
//! - Alpha masking
//! - Compositing modes (SVG 1.2 compatible)
//! - Built-in vector and raster fonts
//!
//! ## Architecture
//!
//! AGG uses a five-stage rendering pipeline:
//!
//! 1. **Vertex Source** — generates polygon/polyline vertices
//! 2. **Coordinate Conversion** — transforms, strokes, dashes, curves
//! 3. **Scanline Rasterizer** — converts vectors to anti-aliased scanlines
//! 4. **Scanline Container** — stores coverage data between stages
//! 5. **Renderer** — blends pixels into the output buffer

// Phase 1: Foundation Types & Math
pub mod array;
pub mod basics;
pub mod color;
pub mod gamma;
pub mod math;

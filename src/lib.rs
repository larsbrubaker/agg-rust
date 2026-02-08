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

// Phase 2: Memory & Geometry Primitives
pub mod arc;
pub mod arrowhead;
pub mod bezier_arc;
pub mod bounding_rect;
pub mod bspline;
pub mod clip_liang_barsky;
pub mod curves;
pub mod dda_line;
pub mod ellipse;
pub mod math_stroke;
pub mod path_storage;
pub mod rendering_buffer;
pub mod rounded_rect;
pub mod simul_eq;
pub mod trans_affine;

// Phase 3: Scanline Rasterizer
pub mod rasterizer_cells_aa;
pub mod rasterizer_scanline_aa;
pub mod rasterizer_sl_clip;
pub mod scanline_bin;
pub mod scanline_p;
pub mod scanline_u;

// Phase 3C: Pixel Formats & Renderers
pub mod pixfmt_rgba;
pub mod renderer_base;
pub mod renderer_scanline;

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
pub mod pixfmt_gray;
pub mod pixfmt_lcd;
pub mod pixfmt_rgb;
pub mod pixfmt_rgba;
pub mod renderer_base;
pub mod renderer_scanline;

// Phase 4: Converter Pipeline
pub mod conv_adaptor_vcgen;
pub mod conv_contour;
pub mod conv_curve;
pub mod conv_dash;
pub mod conv_smooth_poly1;
pub mod conv_stroke;
pub mod conv_transform;
pub mod vcgen_contour;
pub mod vcgen_dash;
pub mod vcgen_smooth_poly1;
pub mod vcgen_stroke;

// Phase 5: Span Generators & Gradients
pub mod gradient_lut;
pub mod span_allocator;
pub mod span_gouraud;
pub mod span_gouraud_rgba;
pub mod span_gradient;
pub mod span_interpolator_linear;
pub mod span_solid;

// Phase 6: Transforms, Image Filters, Text & Alpha Masking
pub mod alpha_mask_u8;
pub mod conv_marker;
pub mod gsv_text;
pub mod image_accessors;
pub mod image_filters;
pub mod span_image_filter;
pub mod trans_bilinear;
pub mod trans_perspective;

// Phase 7: Image Span Filters & Demo Infrastructure
pub mod ellipse_bresenham;
pub mod rasterizer_outline;
pub mod renderer_primitives;
pub mod span_converter;
pub mod span_image_filter_rgba;
pub mod span_interpolator_trans;
pub mod span_subdiv_adaptor;
pub mod trans_viewport;

// Phase 8: Controls (interactive UI widgets rendered via AGG pipeline)
pub mod ctrl;

// Phase 9: Quick Transforms & Segmentator
pub mod conv_adaptor_vpgen;
pub mod conv_segmentator;
pub mod span_interpolator_adaptor;
pub mod trans_polar;
pub mod trans_single_path;
pub mod trans_warp_magnifier;
pub mod vpgen_segmentator;

// Phase 10: Patterns & Perspective Interpolator
pub mod pattern_filters_rgba;
pub mod span_interpolator_persp;
pub mod span_pattern_rgba;

// Phase 11: Blur & Multi-Clip
pub mod blur;
pub mod pixfmt_transposer;
pub mod renderer_mclip;

// Phase 12: Scanline Boolean Algebra
pub mod scanline_boolean_algebra;
pub mod scanline_storage_aa;
pub mod scanline_storage_bin;

// Phase 13: Outline AA Rasterizer
pub mod line_aa_basics;
pub mod rasterizer_outline_aa;
pub mod renderer_outline_aa;
pub mod renderer_outline_image;

// Phase 14: Compositing & Compound Rasterizer
pub mod comp_op;
pub mod rasterizer_compound_aa;

// Phase 15: Raster Fonts
pub mod embedded_raster_fonts;
pub mod glyph_raster_bin;
pub mod renderer_raster_text;

// Phase 15b: Double-path transformer
pub mod trans_double_path;

// Phase 16: TrueType Font Engine (optional, requires `font` feature)
#[cfg(feature = "font")]
pub mod font_engine;
#[cfg(feature = "font")]
pub mod font_cache;

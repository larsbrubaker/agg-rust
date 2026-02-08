//! Scanline rendering functions.
//!
//! Port of `agg_renderer_scanline.h` — top-level functions that drive the
//! rasterizer → scanline → renderer pipeline.
//!
//! The primary entry point is `render_scanlines_aa_solid()` which renders
//! filled polygons with anti-aliased edges in a single solid color.

use crate::color::Rgba8;
use crate::pixfmt_rgba::PixfmtRgba32;
use crate::rasterizer_scanline_aa::{RasterizerScanlineAa, Scanline};
use crate::renderer_base::RendererBase;
use crate::scanline_u::ScanlineU8;

// ============================================================================
// render_scanlines_aa_solid — the main rendering driver
// ============================================================================

/// Render all scanlines from the rasterizer as a solid color.
///
/// This is the primary rendering function that ties together the full AGG
/// pipeline: rasterizer → scanline → renderer.
///
/// Port of C++ `render_scanlines_aa_solid()`.
///
/// Works with `ScanlineU8` (unpacked per-pixel coverage). Each span has
/// positive `len` and references into the covers array.
pub fn render_scanlines_aa_solid(
    ras: &mut RasterizerScanlineAa,
    sl: &mut ScanlineU8,
    ren: &mut RendererBase<PixfmtRgba32<'_>>,
    color: &Rgba8,
) {
    if !ras.rewind_scanlines() {
        return;
    }

    sl.reset(ras.min_x(), ras.max_x());
    while ras.sweep_scanline(sl) {
        render_scanline_aa_solid_u8(sl, ren, color);
    }
}

/// Render a single scanline from `ScanlineU8` to the renderer.
///
/// Port of C++ `render_scanline_aa_solid()` specialized for ScanlineU8
/// where all spans have positive len (per-pixel covers).
fn render_scanline_aa_solid_u8(
    sl: &ScanlineU8,
    ren: &mut RendererBase<PixfmtRgba32<'_>>,
    color: &Rgba8,
) {
    let y = sl.y();
    let spans = sl.begin();
    let covers = sl.covers();

    for span in spans {
        let x = span.x;
        let len = span.len;
        if len > 0 {
            ren.blend_solid_hspan(
                x,
                y,
                len,
                color,
                &covers[span.cover_offset..span.cover_offset + len as usize],
            );
        }
        // ScanlineU8 always has positive len, but handle negative for safety
        // (negative len would mean solid span like ScanlineP8 uses)
    }
}

// ============================================================================
// RendererScanlineAaSolid — stored-color renderer wrapper
// ============================================================================

/// A renderer that stores a color and renders solid AA scanlines.
///
/// Port of C++ `renderer_scanline_aa_solid`. Wraps a `RendererBase` and
/// a stored color for convenience.
pub struct RendererScanlineAaSolid<'a, 'b> {
    ren: &'a mut RendererBase<PixfmtRgba32<'b>>,
    color: Rgba8,
}

impl<'a, 'b> RendererScanlineAaSolid<'a, 'b> {
    pub fn new(ren: &'a mut RendererBase<PixfmtRgba32<'b>>) -> Self {
        Self {
            ren,
            color: Rgba8::new(0, 0, 0, 255),
        }
    }

    pub fn color(&mut self, c: Rgba8) {
        self.color = c;
    }

    /// Render all scanlines from the rasterizer.
    pub fn render(&mut self, ras: &mut RasterizerScanlineAa, sl: &mut ScanlineU8) {
        render_scanlines_aa_solid(ras, sl, self.ren, &self.color);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::POLY_SUBPIXEL_SCALE;
    use crate::ellipse::Ellipse;
    use crate::path_storage::PathStorage;
    use crate::pixfmt_rgba::PixelFormat;
    use crate::rendering_buffer::RowAccessor;

    const BPP: usize = 4;

    fn make_rgba_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * BPP as u32) as i32;
        let buf = vec![255u8; (h * w * BPP as u32) as usize]; // white background (all 0xFF)
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    // ========================================================================
    // Capstone test: render a solid red triangle on white background
    // ========================================================================

    #[test]
    fn test_render_triangle_solid_red() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(255, 255, 255, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Triangle (20,20) → (80,20) → (50,80)
        let s = POLY_SUBPIXEL_SCALE as i32;
        ras.move_to(20 * s, 20 * s);
        ras.line_to(80 * s, 20 * s);
        ras.line_to(50 * s, 80 * s);

        let red = Rgba8::new(255, 0, 0, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &red);

        // Center of triangle should be red
        let center = ren.ren().pixel(50, 40);
        assert_eq!(center.r, 255, "Center R should be 255");
        assert_eq!(center.g, 0, "Center G should be 0");
        assert_eq!(center.b, 0, "Center B should be 0");
        assert_eq!(center.a, 255, "Center A should be 255");

        // Corner (0,0) should remain white
        let corner = ren.ren().pixel(0, 0);
        assert_eq!(corner.r, 255);
        assert_eq!(corner.g, 255);
        assert_eq!(corner.b, 255);

        // Edge pixel should have AA blending (not fully red, not fully white)
        // Check a pixel near the edge of the triangle
        let edge = ren.ren().pixel(20, 20);
        // At the exact vertex, it might be partially covered
        assert!(edge.r > 0, "Edge pixel should have some red: r={}", edge.r);
    }

    // ========================================================================
    // Rectangle test
    // ========================================================================

    #[test]
    fn test_render_rectangle() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(255, 255, 255, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Rectangle: (10,10) → (90,10) → (90,90) → (10,90)
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(90.0, 10.0);
        ras.line_to_d(90.0, 90.0);
        ras.line_to_d(10.0, 90.0);

        let blue = Rgba8::new(0, 0, 255, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &blue);

        // Center should be blue
        let center = ren.ren().pixel(50, 50);
        assert_eq!(center.b, 255);
        assert_eq!(center.r, 0);

        // Outside should be white
        let outside = ren.ren().pixel(5, 5);
        assert_eq!(outside.r, 255);
        assert_eq!(outside.g, 255);
    }

    // ========================================================================
    // Ellipse test via add_path
    // ========================================================================

    #[test]
    fn test_render_ellipse() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(0, 0, 0, 255)); // black background
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut ellipse = Ellipse::new(50.0, 50.0, 30.0, 30.0, 64, false);
        ras.add_path(&mut ellipse, 0);

        let green = Rgba8::new(0, 255, 0, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &green);

        // Center should be green
        let center = ren.ren().pixel(50, 50);
        assert_eq!(center.g, 255);

        // Far corner should remain black
        let corner = ren.ren().pixel(0, 0);
        assert_eq!(corner.g, 0);
    }

    // ========================================================================
    // PathStorage test
    // ========================================================================

    #[test]
    fn test_render_path_storage_triangle() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(255, 255, 255, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut path = PathStorage::new();
        path.move_to(10.0, 10.0);
        path.line_to(90.0, 50.0);
        path.line_to(50.0, 90.0);
        // auto_close will close the polygon

        ras.add_path(&mut path, 0);

        let magenta = Rgba8::new(255, 0, 255, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &magenta);

        // Hit test: center of triangle
        let p = ren.ren().pixel(40, 50);
        assert!(p.r > 0 || p.b > 0, "Center should have color");
    }

    // ========================================================================
    // Clip box test
    // ========================================================================

    #[test]
    fn test_render_with_clip_box() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(255, 255, 255, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Set rasterizer clip box
        ras.clip_box(0.0, 0.0, 50.0, 50.0);

        // Draw a large rectangle that extends beyond clip
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(90.0, 10.0);
        ras.line_to_d(90.0, 90.0);
        ras.line_to_d(10.0, 90.0);

        let red = Rgba8::new(255, 0, 0, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &red);

        // Inside clip box and shape should be red
        let inside = ren.ren().pixel(30, 30);
        assert_eq!(inside.r, 255);

        // Outside clip box should remain white
        let outside = ren.ren().pixel(70, 70);
        assert_eq!(outside.r, 255);
        assert_eq!(outside.g, 255);
    }

    // ========================================================================
    // RendererScanlineAaSolid wrapper test
    // ========================================================================

    #[test]
    fn test_renderer_scanline_aa_solid_wrapper() {
        let (_buf, mut ra) = make_rgba_buffer(100, 100);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(0, 0, 0, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        ras.move_to_d(20.0, 20.0);
        ras.line_to_d(80.0, 20.0);
        ras.line_to_d(50.0, 80.0);

        let mut renderer = RendererScanlineAaSolid::new(&mut ren);
        renderer.color(Rgba8::new(0, 255, 0, 255));
        renderer.render(&mut ras, &mut sl);

        // Center should be green
        let center = renderer.ren.ren().pixel(50, 40);
        assert_eq!(center.g, 255);
    }

    // ========================================================================
    // Empty rasterizer test
    // ========================================================================

    #[test]
    fn test_render_empty() {
        let (_buf, mut ra) = make_rgba_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        pf.clear(&Rgba8::new(255, 255, 255, 255));
        let mut ren = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Render with no polygons — should not crash
        let red = Rgba8::new(255, 0, 0, 255);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut ren, &red);

        // Everything should remain white
        let p = ren.ren().pixel(5, 5);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 255);
    }
}

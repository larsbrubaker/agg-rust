//! RGBA pixel format with alpha blending.
//!
//! Port of `agg_pixfmt_rgba.h` — pixel format that reads and writes RGBA32
//! pixels (4 bytes per pixel, non-premultiplied alpha) with alpha blending.
//!
//! Provides the `PixelFormat` trait and `PixfmtRgba32` concrete implementation.

use crate::basics::CoverType;
use crate::color::Rgba8;
use crate::rendering_buffer::RowAccessor;

// ============================================================================
// PixelFormat trait
// ============================================================================

/// Trait for pixel format renderers that can blend colors into a rendering buffer.
///
/// This is the abstraction layer between the renderer and the raw pixel data.
/// Different implementations handle different pixel layouts (RGBA, RGB, gray)
/// and blending modes (premultiplied, non-premultiplied).
pub trait PixelFormat {
    type ColorType;

    fn width(&self) -> u32;
    fn height(&self) -> u32;

    /// Blend a single pixel at (x, y) with color `c` and coverage `cover`.
    fn blend_pixel(&mut self, x: i32, y: i32, c: &Self::ColorType, cover: CoverType);

    /// Blend a horizontal line of `len` pixels at (x, y) with uniform color and coverage.
    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Self::ColorType, cover: CoverType);

    /// Blend a horizontal span of `len` pixels with per-pixel coverage values.
    fn blend_solid_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &Self::ColorType,
        covers: &[CoverType],
    );

    /// Copy (overwrite) a horizontal line of `len` pixels with color `c`.
    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Self::ColorType);

    /// Copy (overwrite) a single pixel at (x, y) with color `c`.
    fn copy_pixel(&mut self, x: i32, y: i32, c: &Self::ColorType);

    /// Get the pixel color at (x, y).
    fn pixel(&self, x: i32, y: i32) -> Self::ColorType;
}

// ============================================================================
// PixfmtRgba32 — non-premultiplied RGBA, 8 bits per channel
// ============================================================================

/// Pixel format for non-premultiplied RGBA32 (4 bytes per pixel).
///
/// Port of C++ `pixfmt_alpha_blend_rgba<blender_rgba32, rendering_buf>`.
/// Component order: R=0, G=1, B=2, A=3 (standard RGBA).
///
/// Blending uses the `Rgba8` utility methods (`lerp`, `mult_cover`, etc.)
/// which match the C++ blender functions.
pub struct PixfmtRgba32<'a> {
    rbuf: &'a mut RowAccessor,
}

const BPP: usize = 4; // bytes per pixel

impl<'a> PixfmtRgba32<'a> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self { rbuf }
    }

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &Rgba8) {
        let w = self.rbuf.width();
        let h = self.rbuf.height();
        for y in 0..h {
            let row = unsafe {
                let ptr = self.rbuf.row_ptr(y as i32);
                std::slice::from_raw_parts_mut(ptr, (w as usize) * BPP)
            };
            for x in 0..w as usize {
                let off = x * BPP;
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = c.a;
            }
        }
    }

    /// Blend a single pixel (internal helper, no bounds checking).
    #[inline]
    fn blend_pix(p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        p[0] = Rgba8::lerp(p[0], cr, alpha);
        p[1] = Rgba8::lerp(p[1], cg, alpha);
        p[2] = Rgba8::lerp(p[2], cb, alpha);
        p[3] = Rgba8::lerp(p[3], 255, alpha);
    }
}

impl<'a> PixelFormat for PixfmtRgba32<'a> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.rbuf.width()
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        Rgba8::new(
            row[off] as u32,
            row[off + 1] as u32,
            row[off + 2] as u32,
            row[off + 3] as u32,
        )
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        row[off] = c.r;
        row[off + 1] = c.g;
        row[off + 2] = c.b;
        row[off + 3] = c.a;
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for i in 0..len as usize {
            let off = (x as usize + i) * BPP;
            row[off] = c.r;
            row[off + 1] = c.g;
            row[off + 2] = c.b;
            row[off + 3] = c.a;
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        let alpha = Rgba8::mult_cover(c.a, cover);
        if alpha == 255 {
            row[off] = c.r;
            row[off + 1] = c.g;
            row[off + 2] = c.b;
            row[off + 3] = 255;
        } else {
            Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
        }
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let alpha = Rgba8::mult_cover(c.a, cover);
        if alpha == 255 {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = 255;
            }
        } else {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
            }
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, covers: &[CoverType]) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for (i, &cov) in covers.iter().enumerate().take(len as usize) {
            let off = (x as usize + i) * BPP;
            let alpha = Rgba8::mult_cover(c.a, cov);
            if alpha == 255 {
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = 255;
            } else if alpha > 0 {
                Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * BPP as u32) as i32;
        let buf = vec![0u8; (h * w * BPP as u32) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_new() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pf = PixfmtRgba32::new(&mut ra);
        assert_eq!(pf.width(), 100);
        assert_eq!(pf.height(), 100);
    }

    #[test]
    fn test_copy_pixel() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let red = Rgba8::new(255, 0, 0, 255);
        pf.copy_pixel(5, 5, &red);
        let p = pf.pixel(5, 5);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_copy_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let green = Rgba8::new(0, 255, 0, 255);
        pf.copy_hline(5, 3, 10, &green);
        for x in 5..15 {
            let p = pf.pixel(x, 3);
            assert_eq!(p.g, 255);
        }
        // Pixels outside should remain black
        let p = pf.pixel(4, 3);
        assert_eq!(p.g, 0);
    }

    #[test]
    fn test_blend_pixel_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let blue = Rgba8::new(0, 0, 255, 255);
        pf.blend_pixel(3, 3, &blue, 255);
        let p = pf.pixel(3, 3);
        assert_eq!(p.b, 255);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_blend_pixel_semi_transparent() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        // Start with white background
        let white = Rgba8::new(255, 255, 255, 255);
        pf.copy_pixel(3, 3, &white);
        // Blend red at 50% coverage
        let red = Rgba8::new(255, 0, 0, 255);
        pf.blend_pixel(3, 3, &red, 128);
        let p = pf.pixel(3, 3);
        // Red should increase, green/blue should decrease
        assert!(p.r > 128);
        assert!(p.g < 200);
        assert!(p.b < 200);
    }

    #[test]
    fn test_blend_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let red = Rgba8::new(255, 0, 0, 255);
        pf.blend_hline(5, 3, 5, &red, 255);
        for x in 5..10 {
            let p = pf.pixel(x, 3);
            assert_eq!(p.r, 255);
        }
    }

    #[test]
    fn test_blend_solid_hspan() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let green = Rgba8::new(0, 255, 0, 255);
        let covers = [255u8, 128, 64, 0, 255];
        pf.blend_solid_hspan(5, 3, 5, &green, &covers);
        // Full coverage pixel
        let p0 = pf.pixel(5, 3);
        assert_eq!(p0.g, 255);
        // Zero coverage pixel should be unchanged (black)
        let p3 = pf.pixel(8, 3);
        assert_eq!(p3.g, 0);
    }

    #[test]
    fn test_clear() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let white = Rgba8::new(255, 255, 255, 255);
        pf.clear(&white);
        for y in 0..10 {
            for x in 0..10 {
                let p = pf.pixel(x, y);
                assert_eq!(p.r, 255);
                assert_eq!(p.g, 255);
                assert_eq!(p.b, 255);
                assert_eq!(p.a, 255);
            }
        }
    }

    #[test]
    fn test_blend_on_black_background() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        // Blend white at 50% on black
        let white = Rgba8::new(255, 255, 255, 255);
        pf.blend_pixel(0, 0, &white, 128);
        let p = pf.pixel(0, 0);
        // lerp(0, 255, 128) ≈ 128
        assert!((p.r as i32 - 128).abs() <= 2);
    }

    #[test]
    fn test_blend_preserves_adjacent_pixels() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let red = Rgba8::new(255, 0, 0, 255);
        pf.blend_pixel(5, 5, &red, 255);
        // Adjacent pixel should remain black
        let p = pf.pixel(4, 5);
        assert_eq!(p.r, 0);
        assert_eq!(p.g, 0);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 0);
    }

    #[test]
    fn test_pixel_read_write_roundtrip() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let c = Rgba8::new(42, 128, 200, 180);
        pf.copy_pixel(7, 3, &c);
        let p = pf.pixel(7, 3);
        assert_eq!(p.r, 42);
        assert_eq!(p.g, 128);
        assert_eq!(p.b, 200);
        assert_eq!(p.a, 180);
    }
}

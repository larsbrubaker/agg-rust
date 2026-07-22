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

    /// Blend a horizontal span with per-pixel colors and optional per-pixel coverage.
    ///
    /// If `covers` is non-empty, each pixel uses its corresponding coverage.
    /// If `covers` is empty, all pixels use the uniform `cover` value.
    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Self::ColorType],
        covers: &[CoverType],
        cover: CoverType,
    );

    /// Get the pixel color at (x, y).
    fn pixel(&self, x: i32, y: i32) -> Self::ColorType;
}

const BPP: usize = 4; // bytes per pixel

// ============================================================================
// RgbaRaw — shared non-blend plumbing for the RGBA32 pixel formats
// ============================================================================

/// Shared non-blend plumbing for the RGBA32 pixel formats. Owns the row
/// accessor and centralizes the unsafe row-slice construction; the wrapping
/// formats differ only in their blend arithmetic (lerp vs prelerp).
struct RgbaRaw<'a> {
    rbuf: &'a mut RowAccessor,
}

impl<'a> RgbaRaw<'a> {
    fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self { rbuf }
    }

    fn width(&self) -> u32 {
        self.rbuf.width()
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    #[inline]
    fn row(&self, y: i32) -> &[u8] {
        unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, (self.rbuf.width() as usize) * BPP)
        }
    }

    #[inline]
    fn row_mut(&mut self, y: i32) -> &mut [u8] {
        unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        }
    }

    /// Clear the entire buffer to a solid color.
    fn clear(&mut self, c: &Rgba8) {
        let w = self.width();
        let h = self.height();
        for y in 0..h {
            let row = self.row_mut(y as i32);
            for x in 0..w as usize {
                let off = x * BPP;
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = c.a;
            }
        }
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let row = self.row(y);
        let off = x as usize * BPP;
        Rgba8::new(
            row[off] as u32,
            row[off + 1] as u32,
            row[off + 2] as u32,
            row[off + 3] as u32,
        )
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        let row = self.row_mut(y);
        let off = x as usize * BPP;
        row[off] = c.r;
        row[off + 1] = c.g;
        row[off + 2] = c.b;
        row[off + 3] = c.a;
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        let row = self.row_mut(y);
        let start = x as usize * BPP;
        // Fill a whole pixel at a time via a chunked pattern copy. The C++ equivalent
        // stores one pixel_type per pixel; writing the 4-byte pattern over
        // chunks_exact_mut lets the optimizer vectorize the fill instead of emitting
        // four individually-indexed byte stores per pixel.
        let pat = [c.r, c.g, c.b, c.a];
        for px in row[start..start + len as usize * BPP].chunks_exact_mut(BPP) {
            px.copy_from_slice(&pat);
        }
    }
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
    raw: RgbaRaw<'a>,
}

impl<'a> PixfmtRgba32<'a> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self {
            raw: RgbaRaw::new(rbuf),
        }
    }

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &Rgba8) {
        self.raw.clear(c);
    }

    /// Blend a single pixel (internal helper, no bounds checking).
    #[inline]
    fn blend_pix(p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        p[0] = Rgba8::lerp(p[0], cr, alpha);
        p[1] = Rgba8::lerp(p[1], cg, alpha);
        p[2] = Rgba8::lerp(p[2], cb, alpha);
        p[3] = Rgba8::lerp(p[3], 255, alpha);
    }

    /// Apply inverse gamma correction to every pixel in the buffer.
    ///
    /// For each pixel, applies `gamma.inv()` to the R, G, B channels,
    /// leaving the alpha channel unchanged. This matches the C++
    /// `pixfmt_rgb24::apply_gamma_inv()` from `agg_pixfmt_rgb.h`.
    pub fn apply_gamma_inv(&mut self, gamma: &crate::gamma::GammaLut) {
        let w = self.raw.width();
        let h = self.raw.height();
        for y in 0..h {
            let row = self.raw.row_mut(y as i32);
            for x in 0..w as usize {
                let off = x * BPP;
                row[off] = gamma.inv(row[off]);
                row[off + 1] = gamma.inv(row[off + 1]);
                row[off + 2] = gamma.inv(row[off + 2]);
                // row[off + 3] (alpha) is left unchanged
            }
        }
    }

    /// Apply forward gamma correction to every pixel in the buffer.
    ///
    /// For each pixel, applies `gamma.dir()` to the R, G, B channels,
    /// leaving the alpha channel unchanged.
    pub fn apply_gamma_dir(&mut self, gamma: &crate::gamma::GammaLut) {
        let w = self.raw.width();
        let h = self.raw.height();
        for y in 0..h {
            let row = self.raw.row_mut(y as i32);
            for x in 0..w as usize {
                let off = x * BPP;
                row[off] = gamma.dir(row[off]);
                row[off + 1] = gamma.dir(row[off + 1]);
                row[off + 2] = gamma.dir(row[off + 2]);
            }
        }
    }
}

impl<'a> PixelFormat for PixfmtRgba32<'a> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.raw.width()
    }

    fn height(&self) -> u32 {
        self.raw.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        self.raw.pixel(x, y)
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        self.raw.copy_pixel(x, y, c);
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        self.raw.copy_hline(x, y, len, c);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let row = self.raw.row_mut(y);
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
        let row = self.raw.row_mut(y);
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
        let row = self.raw.row_mut(y);
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

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba8],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        let row = self.raw.row_mut(y);
        if !covers.is_empty() {
            // Per-pixel coverage from covers array
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                let c = &colors[i];
                let alpha = Rgba8::mult_cover(c.a, covers[i]);
                if alpha == 255 {
                    row[off] = c.r;
                    row[off + 1] = c.g;
                    row[off + 2] = c.b;
                    row[off + 3] = 255;
                } else if alpha > 0 {
                    Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
                }
            }
        } else if cover == 255 {
            // Full coverage, direct copy/blend
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                if c.a == 255 {
                    row[off] = c.r;
                    row[off + 1] = c.g;
                    row[off + 2] = c.b;
                    row[off + 3] = 255;
                } else if c.a > 0 {
                    Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, c.a);
                }
            }
        } else {
            // Uniform coverage for all pixels
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                let alpha = Rgba8::mult_cover(c.a, cover);
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
}

// ============================================================================
// PixfmtRgba32Pre — premultiplied RGBA, 8 bits per channel
// ============================================================================

/// Pixel format for premultiplied RGBA32 (4 bytes per pixel).
///
/// Port of C++ `pixfmt_alpha_blend_rgba<blender_rgba_pre, rendering_buf>`
/// (a.k.a. `pixfmt_rgba32_pre`). Component order: R=0, G=1, B=2, A=3.
///
/// Blending uses the premultiplied form of Alvy-Ray Smith's compositing
/// function (`prelerp`) rather than the straight-alpha `lerp` used by
/// [`PixfmtRgba32`]. The two produce slightly different rounding: `lerp`
/// rounds a single `(q - p) * a` term, while the premultiplied path rounds
/// `mult_cover(...)` and `multiply(...)` independently, which can differ by
/// one least-significant bit. The C++ `rasterizers2` demo draws into a
/// `*_pre` pixel format, so matching it byte-for-byte requires this format.
///
/// Note that for an opaque backdrop the alpha channel is preserved at 255:
/// `prelerp(255, a, a) == 255` for all `a`, so this format keeps a fully
/// opaque buffer opaque, matching the RGB (`bgr24_pre`) reference output.
pub struct PixfmtRgba32Pre<'a> {
    raw: RgbaRaw<'a>,
}

impl<'a> PixfmtRgba32Pre<'a> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self {
            raw: RgbaRaw::new(rbuf),
        }
    }

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &Rgba8) {
        self.raw.clear(c);
    }

    /// Premultiplied blend of a pixel with already-covered (premultiplied)
    /// color components. Port of `blender_rgba_pre::blend_pix` (no cover).
    #[inline]
    fn blend_pix_pre(p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        p[0] = Rgba8::prelerp(p[0], cr, alpha);
        p[1] = Rgba8::prelerp(p[1], cg, alpha);
        p[2] = Rgba8::prelerp(p[2], cb, alpha);
        p[3] = Rgba8::prelerp(p[3], alpha, alpha);
    }

    /// Premultiplied blend folding a coverage value into the color.
    /// Port of `blender_rgba_pre::blend_pix` (with cover).
    #[inline]
    fn blend_pix_cover(p: &mut [u8], c: &Rgba8, cover: CoverType) {
        Self::blend_pix_pre(
            p,
            Rgba8::mult_cover(c.r, cover),
            Rgba8::mult_cover(c.g, cover),
            Rgba8::mult_cover(c.b, cover),
            Rgba8::mult_cover(c.a, cover),
        );
    }

    /// Port of `pixfmt_alpha_blend_rgba::copy_or_blend_pix` (with cover).
    #[inline]
    fn copy_or_blend_cover(p: &mut [u8], c: &Rgba8, cover: CoverType) {
        if c.a != 0 {
            if c.a == 255 && cover == 255 {
                p[0] = c.r;
                p[1] = c.g;
                p[2] = c.b;
                p[3] = c.a;
            } else {
                Self::blend_pix_cover(p, c, cover);
            }
        }
    }

    /// Port of `pixfmt_alpha_blend_rgba::copy_or_blend_pix` (no cover).
    #[inline]
    fn copy_or_blend(p: &mut [u8], c: &Rgba8) {
        if c.a != 0 {
            if c.a == 255 {
                p[0] = c.r;
                p[1] = c.g;
                p[2] = c.b;
                p[3] = c.a;
            } else {
                Self::blend_pix_pre(p, c.r, c.g, c.b, c.a);
            }
        }
    }
}

impl<'a> PixelFormat for PixfmtRgba32Pre<'a> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.raw.width()
    }

    fn height(&self) -> u32 {
        self.raw.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        self.raw.pixel(x, y)
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        self.raw.copy_pixel(x, y, c);
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        self.raw.copy_hline(x, y, len, c);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let row = self.raw.row_mut(y);
        let off = x as usize * BPP;
        Self::copy_or_blend_cover(&mut row[off..off + BPP], c, cover);
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, cover: CoverType) {
        if c.a == 0 {
            return;
        }
        let row = self.raw.row_mut(y);
        if c.a == 255 && cover == 255 {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = c.a;
            }
        } else {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                Self::blend_pix_cover(&mut row[off..off + BPP], c, cover);
            }
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, covers: &[CoverType]) {
        if c.a == 0 {
            return;
        }
        let row = self.raw.row_mut(y);
        for (i, &cov) in covers.iter().enumerate().take(len as usize) {
            let off = (x as usize + i) * BPP;
            if c.a == 255 && cov == 255 {
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = c.a;
            } else {
                Self::blend_pix_cover(&mut row[off..off + BPP], c, cov);
            }
        }
    }

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba8],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        let row = self.raw.row_mut(y);
        if !covers.is_empty() {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                Self::copy_or_blend_cover(&mut row[off..off + BPP], &colors[i], covers[i]);
            }
        } else if cover == 255 {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                Self::copy_or_blend(&mut row[off..off + BPP], c);
            }
        } else {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                Self::copy_or_blend_cover(&mut row[off..off + BPP], c, cover);
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

    #[test]
    fn test_blend_color_hspan_per_pixel_covers() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let colors = [
            Rgba8::new(255, 0, 0, 255),
            Rgba8::new(0, 255, 0, 255),
            Rgba8::new(0, 0, 255, 255),
        ];
        let covers = [255u8, 128, 0];
        pf.blend_color_hspan(5, 3, 3, &colors, &covers, 0);
        // Full coverage red
        let p0 = pf.pixel(5, 3);
        assert_eq!(p0.r, 255);
        assert_eq!(p0.g, 0);
        // Half coverage green on black
        let p1 = pf.pixel(6, 3);
        assert!(p1.g > 64);
        // Zero coverage blue: unchanged (black)
        let p2 = pf.pixel(7, 3);
        assert_eq!(p2.b, 0);
    }

    #[test]
    fn test_blend_color_hspan_uniform_cover() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let colors = [Rgba8::new(255, 0, 0, 255), Rgba8::new(0, 255, 0, 255)];
        // Empty covers slice → uniform cover of 255
        pf.blend_color_hspan(3, 3, 2, &colors, &[], 255);
        let p0 = pf.pixel(3, 3);
        assert_eq!(p0.r, 255);
        let p1 = pf.pixel(4, 3);
        assert_eq!(p1.g, 255);
    }

    #[test]
    fn test_blend_color_hspan_full_cover_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32::new(&mut ra);
        let white = Rgba8::new(255, 255, 255, 255);
        pf.clear(&white);
        let colors = [Rgba8::new(100, 150, 200, 255)];
        pf.blend_color_hspan(0, 0, 1, &colors, &[], 255);
        let p = pf.pixel(0, 0);
        assert_eq!(p.r, 100);
        assert_eq!(p.g, 150);
        assert_eq!(p.b, 200);
    }

    // ------------------------------------------------------------------------
    // PixfmtRgba32Pre — premultiplied blend paths.
    //
    // Expected bytes are computed by hand from the C++ `blender_rgba_pre`
    // formulas (prelerp / mult_cover / multiply) and hard-coded here as
    // documented constants, so the test verifies the production code rather
    // than re-deriving prelerp in Rust.
    // ------------------------------------------------------------------------

    /// blend_pixel with a partly transparent, cover-folded color.
    ///
    /// C++ `blender_rgba_pre::blend_pix(p, c, cover)`:
    ///   q  = mult_cover(c.rgb, cover); a' = mult_cover(c.a, cover)
    ///   p  = prelerp(p, q, a')  (alpha channel: prelerp(p.a, a', a'))
    /// backdrop (10,20,30,255), color (100,200,50,200), cover 128:
    ///   q=(50,100,25), a'=100  ->  (56,112,43,255)
    #[test]
    fn test_pre_blend_pixel_cover_folding() {
        let (_buf, mut ra) = make_buffer(4, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        pf.copy_pixel(1, 1, &Rgba8::new(10, 20, 30, 255));
        pf.blend_pixel(1, 1, &Rgba8::new(100, 200, 50, 200), 128);
        let p = pf.pixel(1, 1);
        assert_eq!((p.r, p.g, p.b, p.a), (56, 112, 43, 255));
    }

    /// blend_pixel opaque color with full cover takes the copy shortcut
    /// (`c.a == 255 && cover == 255` -> `set`).
    #[test]
    fn test_pre_blend_pixel_opaque_full_cover_is_copy() {
        let (_buf, mut ra) = make_buffer(4, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        pf.copy_pixel(2, 2, &Rgba8::new(240, 240, 230, 255));
        pf.blend_pixel(2, 2, &Rgba8::new(102, 77, 26, 255), 255);
        let p = pf.pixel(2, 2);
        assert_eq!((p.r, p.g, p.b, p.a), (102, 77, 26, 255));
    }

    /// blend_pixel with a fully transparent color is a no-op (`c.a == 0`
    /// early return), regardless of cover.
    #[test]
    fn test_pre_blend_pixel_transparent_is_noop() {
        let (_buf, mut ra) = make_buffer(4, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        let backdrop = Rgba8::new(17, 34, 51, 255);
        pf.copy_pixel(0, 0, &backdrop);
        pf.blend_pixel(0, 0, &Rgba8::new(200, 100, 50, 0), 255);
        let p = pf.pixel(0, 0);
        assert_eq!((p.r, p.g, p.b, p.a), (17, 34, 51, 255));
    }

    /// blend_solid_hspan over covers [255, 128, 0] with an opaque color:
    ///   cover 255 -> copy shortcut -> (102,77,26)
    ///   cover 128 -> q=(51,39,13), a'=mult_cover(255,128)=128 ->
    ///                prelerp on (240,240,230) -> (171,159,128)
    ///   cover 0   -> prelerp(p, 0, 0) == p -> unchanged (240,240,230)
    #[test]
    fn test_pre_blend_solid_hspan_covers_and_zero_noop() {
        let (_buf, mut ra) = make_buffer(8, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        let backdrop = Rgba8::new(240, 240, 230, 255);
        for x in 0..3 {
            pf.copy_pixel(x, 0, &backdrop);
        }
        let color = Rgba8::new(102, 77, 26, 255);
        pf.blend_solid_hspan(0, 0, 3, &color, &[255, 128, 0]);
        assert_eq!(
            {
                let p = pf.pixel(0, 0);
                (p.r, p.g, p.b, p.a)
            },
            (102, 77, 26, 255)
        );
        assert_eq!(
            {
                let p = pf.pixel(1, 0);
                (p.r, p.g, p.b, p.a)
            },
            (171, 159, 128, 255)
        );
        assert_eq!(
            {
                let p = pf.pixel(2, 0);
                (p.r, p.g, p.b, p.a)
            },
            (240, 240, 230, 255)
        );
    }

    /// blend_color_hspan with a non-empty covers slice folds each pixel's
    /// cover into its (premultiplied) color via copy_or_blend_pix.
    /// backdrop black (0,0,0,255), color (60,40,20,180), cover 64:
    ///   q=(15,10,5), a'=mult_cover(180,64)=45 -> prelerp -> (15,10,5,255)
    #[test]
    fn test_pre_blend_color_hspan_with_covers_slice() {
        let (_buf, mut ra) = make_buffer(8, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        let backdrop = Rgba8::new(0, 0, 0, 255);
        pf.copy_pixel(0, 0, &backdrop);
        let colors = [Rgba8::new(60, 40, 20, 180)];
        pf.blend_color_hspan(0, 0, 1, &colors, &[64], 0);
        let p = pf.pixel(0, 0);
        assert_eq!((p.r, p.g, p.b, p.a), (15, 10, 5, 255));
    }

    /// blend_color_hspan with an empty covers slice and full uniform cover
    /// blends a premultiplied, partly transparent color with prelerp (no
    /// cover folding). backdrop (200,100,50,255), color (40,20,10,128):
    ///   prelerp -> (140,70,35,255).
    #[test]
    fn test_pre_blend_color_hspan_full_cover_premultiplied() {
        let (_buf, mut ra) = make_buffer(8, 4);
        let mut pf = PixfmtRgba32Pre::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(200, 100, 50, 255));
        let colors = [Rgba8::new(40, 20, 10, 128)];
        pf.blend_color_hspan(0, 0, 1, &colors, &[], 255);
        let p = pf.pixel(0, 0);
        assert_eq!((p.r, p.g, p.b, p.a), (140, 70, 35, 255));
    }
}

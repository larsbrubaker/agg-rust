//! RGB pixel format with alpha blending (no alpha channel in buffer).
//!
//! Port of `agg_pixfmt_rgb.h` — pixel format that reads and writes RGB24
//! pixels (3 bytes per pixel) with non-premultiplied alpha blending.
//!
//! The alpha value comes from the source color only; the buffer stores
//! no alpha channel. Blending treats missing alpha as fully opaque.

use crate::basics::CoverType;
use crate::color::Rgba8;
use crate::pixfmt_rgba::PixelFormat;
use crate::rendering_buffer::RowAccessor;

/// Bytes per pixel for RGB24.
const BPP: usize = 3;

/// Pixel format for non-premultiplied RGB24 (3 bytes per pixel).
///
/// Port of C++ `pixfmt_alpha_blend_rgb<blender_rgb<rgba8>, rendering_buf>`.
/// Component order: R=0, G=1, B=2.
///
/// Since there is no alpha channel stored in the buffer, `pixel()` always
/// returns `a=255`. Blending uses the source color's alpha to interpolate
/// each RGB component.
pub struct PixfmtRgb24<'a> {
    rbuf: &'a mut RowAccessor,
}

impl<'a> PixfmtRgb24<'a> {
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
            }
        }
    }

    /// Blend a single pixel (internal helper, no bounds checking).
    /// Non-premultiplied blending of R, G, B channels only.
    #[inline]
    fn blend_pix(p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        p[0] = Rgba8::lerp(p[0], cr, alpha);
        p[1] = Rgba8::lerp(p[1], cg, alpha);
        p[2] = Rgba8::lerp(p[2], cb, alpha);
    }
}

impl<'a> PixelFormat for PixfmtRgb24<'a> {
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
        Rgba8::new(row[off] as u32, row[off + 1] as u32, row[off + 2] as u32, 255)
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
        } else if alpha > 0 {
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
            }
        } else if alpha > 0 {
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
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        if !covers.is_empty() {
            for i in 0..len as usize {
                let off = (x as usize + i) * BPP;
                let c = &colors[i];
                let alpha = Rgba8::mult_cover(c.a, covers[i]);
                if alpha == 255 {
                    row[off] = c.r;
                    row[off + 1] = c.g;
                    row[off + 2] = c.b;
                } else if alpha > 0 {
                    Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
                }
            }
        } else if cover == 255 {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                if c.a == 255 {
                    row[off] = c.r;
                    row[off + 1] = c.g;
                    row[off + 2] = c.b;
                } else if c.a > 0 {
                    Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, c.a);
                }
            }
        } else {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let off = (x as usize + i) * BPP;
                let alpha = Rgba8::mult_cover(c.a, cover);
                if alpha == 255 {
                    row[off] = c.r;
                    row[off + 1] = c.g;
                    row[off + 2] = c.b;
                } else if alpha > 0 {
                    Self::blend_pix(&mut row[off..off + BPP], c.r, c.g, c.b, alpha);
                }
            }
        }
    }
}

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
        let pf = PixfmtRgb24::new(&mut ra);
        assert_eq!(pf.width(), 100);
        assert_eq!(pf.height(), 100);
    }

    #[test]
    fn test_copy_pixel() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        let red = Rgba8::new(255, 0, 0, 255);
        pf.copy_pixel(5, 5, &red);
        let p = pf.pixel(5, 5);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 255); // Always 255 for RGB format
    }

    #[test]
    fn test_copy_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        let green = Rgba8::new(0, 255, 0, 255);
        pf.copy_hline(5, 3, 10, &green);
        for x in 5..15 {
            let p = pf.pixel(x, 3);
            assert_eq!(p.g, 255);
        }
        let p = pf.pixel(4, 3);
        assert_eq!(p.g, 0);
    }

    #[test]
    fn test_blend_pixel_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        let blue = Rgba8::new(0, 0, 255, 255);
        pf.blend_pixel(3, 3, &blue, 255);
        let p = pf.pixel(3, 3);
        assert_eq!(p.b, 255);
    }

    #[test]
    fn test_blend_pixel_semitransparent() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        // Start with white background
        let white = Rgba8::new(255, 255, 255, 255);
        pf.copy_hline(0, 0, 10, &white);

        // Blend 50% red over white
        let red_50 = Rgba8::new(255, 0, 0, 128);
        pf.blend_pixel(5, 0, &red_50, 255);
        let p = pf.pixel(5, 0);
        // R should be midway between 255 and 255 → 255
        assert_eq!(p.r, 255);
        // G should be midway between 255 and 0 → ~128
        assert!(p.g > 120 && p.g < 140, "g={}", p.g);
        // B should be midway between 255 and 0 → ~128
        assert!(p.b > 120 && p.b < 140, "b={}", p.b);
    }

    #[test]
    fn test_blend_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        let c = Rgba8::new(100, 100, 100, 255);
        pf.blend_hline(2, 2, 5, &c, 255);
        for x in 2..7 {
            let p = pf.pixel(x, 2);
            assert_eq!(p.r, 100);
        }
    }

    #[test]
    fn test_blend_solid_hspan() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        let c = Rgba8::new(200, 100, 50, 255);
        let covers = [255u8, 128, 64, 0];
        pf.blend_solid_hspan(0, 0, 4, &c, &covers);

        let p0 = pf.pixel(0, 0);
        assert_eq!(p0.r, 200); // full cover

        let p3 = pf.pixel(3, 0);
        assert_eq!(p3.r, 0); // zero cover, no change
    }

    #[test]
    fn test_clear() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgb24::new(&mut ra);
        pf.clear(&Rgba8::new(128, 64, 32, 255));
        let p = pf.pixel(5, 5);
        assert_eq!(p.r, 128);
        assert_eq!(p.g, 64);
        assert_eq!(p.b, 32);
    }

    #[test]
    fn test_pixel_always_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let pf = PixfmtRgb24::new(&mut ra);
        // Even on a zeroed buffer, alpha reads as 255
        let p = pf.pixel(0, 0);
        assert_eq!(p.a, 255);
    }
}

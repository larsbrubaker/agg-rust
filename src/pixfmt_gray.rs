//! Grayscale pixel format with alpha blending.
//!
//! Port of `agg_pixfmt_gray.h` — pixel format that reads and writes 8-bit
//! grayscale pixels (1 byte per pixel) with non-premultiplied alpha blending.
//!
//! The alpha value comes from the source color only; the buffer stores
//! only a single gray value channel. Blending treats missing alpha as
//! fully opaque.

use crate::basics::CoverType;
use crate::color::Gray8;
use crate::pixfmt_rgba::PixelFormat;
use crate::rendering_buffer::RowAccessor;

/// Bytes per pixel for Gray8.
const BPP: usize = 1;

/// Pixel format for non-premultiplied Gray8 (1 byte per pixel).
///
/// Port of C++ `pixfmt_alpha_blend_gray<blender_gray<gray8>, rendering_buf, 1, 0>`.
/// Each pixel is a single byte representing luminance.
///
/// Since there is no alpha channel stored in the buffer, `pixel()` always
/// returns `a=255`. Blending uses the source color's alpha to interpolate
/// the gray value.
pub struct PixfmtGray8<'a> {
    rbuf: &'a mut RowAccessor,
}

impl<'a> PixfmtGray8<'a> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self { rbuf }
    }

    /// Clear the entire buffer to a solid gray value.
    pub fn clear(&mut self, c: &Gray8) {
        let w = self.rbuf.width();
        let h = self.rbuf.height();
        for y in 0..h {
            let row = unsafe {
                let ptr = self.rbuf.row_ptr(y as i32);
                std::slice::from_raw_parts_mut(ptr, w as usize * BPP)
            };
            for x in 0..w as usize {
                row[x] = c.v;
            }
        }
    }

    /// Blend a single pixel (internal helper, no bounds checking).
    #[inline]
    fn blend_pix(p: &mut u8, cv: u8, alpha: u8) {
        *p = Gray8::lerp(*p, cv, alpha);
    }
}

impl<'a> PixelFormat for PixfmtGray8<'a> {
    type ColorType = Gray8;

    fn width(&self) -> u32 {
        self.rbuf.width()
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Gray8 {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, self.rbuf.width() as usize * BPP)
        };
        Gray8::new(row[x as usize] as u32, 255)
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Gray8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        row[x as usize] = c.v;
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Gray8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        for i in 0..len as usize {
            row[x as usize + i] = c.v;
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Gray8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        let alpha = Gray8::mult_cover(c.a, cover);
        if alpha == 255 {
            row[x as usize] = c.v;
        } else if alpha > 0 {
            Self::blend_pix(&mut row[x as usize], c.v, alpha);
        }
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Gray8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        let alpha = Gray8::mult_cover(c.a, cover);
        if alpha == 255 {
            for i in 0..len as usize {
                row[x as usize + i] = c.v;
            }
        } else if alpha > 0 {
            for i in 0..len as usize {
                Self::blend_pix(&mut row[x as usize + i], c.v, alpha);
            }
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Gray8, covers: &[CoverType]) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        for (i, &cov) in covers.iter().enumerate().take(len as usize) {
            let alpha = Gray8::mult_cover(c.a, cov);
            if alpha == 255 {
                row[x as usize + i] = c.v;
            } else if alpha > 0 {
                Self::blend_pix(&mut row[x as usize + i], c.v, alpha);
            }
        }
    }

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Gray8],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        if !covers.is_empty() {
            for i in 0..len as usize {
                let c = &colors[i];
                let alpha = Gray8::mult_cover(c.a, covers[i]);
                if alpha == 255 {
                    row[x as usize + i] = c.v;
                } else if alpha > 0 {
                    Self::blend_pix(&mut row[x as usize + i], c.v, alpha);
                }
            }
        } else if cover == 255 {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                if c.a == 255 {
                    row[x as usize + i] = c.v;
                } else if c.a > 0 {
                    Self::blend_pix(&mut row[x as usize + i], c.v, c.a);
                }
            }
        } else {
            for (i, c) in colors.iter().enumerate().take(len as usize) {
                let alpha = Gray8::mult_cover(c.a, cover);
                if alpha == 255 {
                    row[x as usize + i] = c.v;
                } else if alpha > 0 {
                    Self::blend_pix(&mut row[x as usize + i], c.v, alpha);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = w as i32;
        let buf = vec![0u8; (h * w) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_new() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pf = PixfmtGray8::new(&mut ra);
        assert_eq!(pf.width(), 100);
        assert_eq!(pf.height(), 100);
    }

    #[test]
    fn test_copy_pixel() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let white = Gray8::new(255, 255);
        pf.copy_pixel(5, 5, &white);
        let p = pf.pixel(5, 5);
        assert_eq!(p.v, 255);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_copy_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let mid = Gray8::new(128, 255);
        pf.copy_hline(5, 3, 10, &mid);
        for x in 5..15 {
            let p = pf.pixel(x, 3);
            assert_eq!(p.v, 128);
        }
        let p = pf.pixel(4, 3);
        assert_eq!(p.v, 0);
    }

    #[test]
    fn test_blend_pixel_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let c = Gray8::new(200, 255);
        pf.blend_pixel(3, 3, &c, 255);
        let p = pf.pixel(3, 3);
        assert_eq!(p.v, 200);
    }

    #[test]
    fn test_blend_pixel_semitransparent() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        // Start with white background
        let white = Gray8::new(255, 255);
        pf.copy_hline(0, 0, 10, &white);

        // Blend 50% black over white
        let black_50 = Gray8::new(0, 128);
        pf.blend_pixel(5, 0, &black_50, 255);
        let p = pf.pixel(5, 0);
        // Should be midway between 255 and 0 → ~128
        assert!(p.v > 120 && p.v < 140, "v={}", p.v);
    }

    #[test]
    fn test_blend_hline() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let c = Gray8::new(100, 255);
        pf.blend_hline(2, 2, 5, &c, 255);
        for x in 2..7 {
            let p = pf.pixel(x, 2);
            assert_eq!(p.v, 100);
        }
    }

    #[test]
    fn test_blend_solid_hspan() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let c = Gray8::new(200, 255);
        let covers = [255u8, 128, 64, 0];
        pf.blend_solid_hspan(0, 0, 4, &c, &covers);

        let p0 = pf.pixel(0, 0);
        assert_eq!(p0.v, 200); // full cover

        let p3 = pf.pixel(3, 0);
        assert_eq!(p3.v, 0); // zero cover, no change
    }

    #[test]
    fn test_clear() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        pf.clear(&Gray8::new(128, 255));
        let p = pf.pixel(5, 5);
        assert_eq!(p.v, 128);
    }

    #[test]
    fn test_pixel_always_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let pf = PixfmtGray8::new(&mut ra);
        let p = pf.pixel(0, 0);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_blend_color_hspan_with_covers() {
        let (_buf, mut ra) = make_buffer(20, 10);
        let mut pf = PixfmtGray8::new(&mut ra);
        let colors = [
            Gray8::new(100, 255),
            Gray8::new(200, 255),
            Gray8::new(50, 128),
        ];
        let covers = [255u8, 255, 255];
        pf.blend_color_hspan(0, 0, 3, &colors, &covers, 255);

        assert_eq!(pf.pixel(0, 0).v, 100);
        assert_eq!(pf.pixel(1, 0).v, 200);
        // Third pixel: alpha=mult_cover(128, 255)=128, blended from 0→50
        let p2 = pf.pixel(2, 0);
        assert!(p2.v > 20 && p2.v < 35, "v={}", p2.v);
    }
}

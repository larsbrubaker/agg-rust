//! Pixel format transposer.
//!
//! Port of `agg_pixfmt_transposer.h`.
//! Wraps a `PixelFormat`, swapping x/y coordinates for all operations.
//! Used by blur for vertical pass.

use crate::basics::CoverType;
use crate::pixfmt_rgba::PixelFormat;

/// Pixel format wrapper that swaps x and y coordinates.
///
/// Port of C++ `pixfmt_transposer<PixFmt>`.
/// Makes horizontal operations act vertically and vice versa.
pub struct PixfmtTransposer<PF> {
    pixf: PF,
}

impl<PF: PixelFormat> PixfmtTransposer<PF> {
    pub fn new(pixf: PF) -> Self {
        Self { pixf }
    }

    pub fn inner(&self) -> &PF {
        &self.pixf
    }

    pub fn inner_mut(&mut self) -> &mut PF {
        &mut self.pixf
    }
}

impl<PF: PixelFormat> PixelFormat for PixfmtTransposer<PF> {
    type ColorType = PF::ColorType;

    fn width(&self) -> u32 {
        self.pixf.height()
    }

    fn height(&self) -> u32 {
        self.pixf.width()
    }

    fn pixel(&self, x: i32, y: i32) -> PF::ColorType {
        self.pixf.pixel(y, x)
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType) {
        self.pixf.copy_pixel(y, x, c);
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType) {
        // Transposed: copy_hline becomes a vertical column
        for i in 0..len as i32 {
            self.pixf.copy_pixel(y, x + i, c);
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType, cover: CoverType) {
        self.pixf.blend_pixel(y, x, c, cover);
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType, cover: CoverType) {
        for i in 0..len as i32 {
            self.pixf.blend_pixel(y, x + i, c, cover);
        }
    }

    fn blend_solid_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &PF::ColorType,
        covers: &[CoverType],
    ) {
        for (i, &cov) in covers.iter().enumerate().take(len as usize) {
            self.pixf.blend_pixel(y, x + i as i32, c, cov);
        }
    }

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[PF::ColorType],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        if !covers.is_empty() {
            for i in 0..len as usize {
                self.pixf.blend_pixel(y, x + i as i32, &colors[i], covers[i]);
            }
        } else {
            for i in 0..len as usize {
                self.pixf.blend_pixel(y, x + i as i32, &colors[i], cover);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
    use crate::rendering_buffer::RowAccessor;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * 4) as i32;
        let buf = vec![0u8; (h * w * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_transposed_dimensions() {
        let (_buf, mut ra) = make_buffer(100, 50);
        let pixf = PixfmtRgba32::new(&mut ra);
        let trans = PixfmtTransposer::new(pixf);
        assert_eq!(trans.width(), 50); // height becomes width
        assert_eq!(trans.height(), 100); // width becomes height
    }

    #[test]
    fn test_transposed_pixel() {
        let (_buf, mut ra) = make_buffer(10, 20);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut trans = PixfmtTransposer::new(pixf);

        let red = Rgba8::new(255, 0, 0, 255);
        trans.copy_pixel(5, 3, &red); // In transposed space: (5,3) → actual (3,5)

        let p = trans.pixel(5, 3);
        assert_eq!(p.r, 255);

        // Also check via inner at actual coords (3,5)
        let p2 = trans.inner().pixel(3, 5);
        assert_eq!(p2.r, 255);
    }
}

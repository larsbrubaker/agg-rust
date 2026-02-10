//! LCD subpixel pixel format for RGBA32 buffers.
//!
//! Port of `agg_pixfmt_rgb24_lcd.h` — specialized pixel format that performs
//! LCD subpixel rendering by distributing coverage across the R, G, B channels
//! of adjacent pixels. Adapted for RGBA32 (4 bytes per pixel) buffers.
//!
//! The rasterizer operates at 3x horizontal resolution. Each "subpixel" maps
//! to one color channel (R, G, or B) of an actual RGBA pixel.
//!
//! Copyright (c) 2025. BSD-3-Clause License.

use crate::basics::CoverType;
use crate::color::Rgba8;
use crate::pixfmt_rgba::PixelFormat;
use crate::rendering_buffer::RowAccessor;

// ============================================================================
// LcdDistributionLut
// ============================================================================

/// Lookup table for LCD subpixel coverage distribution.
///
/// Distributes each coverage value across primary (center), secondary
/// (adjacent), and tertiary (2-away) positions. This implements the
/// energy distribution described in Steve Gibson's subpixel rendering guide.
///
/// Port of C++ `lcd_distribution_lut` from `agg_pixfmt_rgb24_lcd.h`.
pub struct LcdDistributionLut {
    primary_lut: [u8; 256],
    secondary_lut: [u8; 256],
    tertiary_lut: [u8; 256],
}

impl LcdDistributionLut {
    /// Create a new LCD distribution lookup table.
    ///
    /// # Arguments
    /// * `prim` — Weight for the primary (center) subpixel
    /// * `second` — Weight for the secondary (adjacent) subpixels
    /// * `tert` — Weight for the tertiary (2-away) subpixels
    ///
    /// Weights are normalized so that `prim + 2*second + 2*tert = 1.0`.
    pub fn new(prim: f64, second: f64, tert: f64) -> Self {
        let norm = 1.0 / (prim + second * 2.0 + tert * 2.0);
        let prim = prim * norm;
        let second = second * norm;
        let tert = tert * norm;

        let mut primary_lut = [0u8; 256];
        let mut secondary_lut = [0u8; 256];
        let mut tertiary_lut = [0u8; 256];

        for i in 0..256 {
            primary_lut[i] = (prim * i as f64).floor() as u8;
            secondary_lut[i] = (second * i as f64).floor() as u8;
            tertiary_lut[i] = (tert * i as f64).floor() as u8;
        }

        Self {
            primary_lut,
            secondary_lut,
            tertiary_lut,
        }
    }

    /// Get the primary (center) distribution for a coverage value.
    #[inline]
    pub fn primary(&self, v: u8) -> u8 {
        self.primary_lut[v as usize]
    }

    /// Get the secondary (adjacent) distribution for a coverage value.
    #[inline]
    pub fn secondary(&self, v: u8) -> u8 {
        self.secondary_lut[v as usize]
    }

    /// Get the tertiary (2-away) distribution for a coverage value.
    #[inline]
    pub fn tertiary(&self, v: u8) -> u8 {
        self.tertiary_lut[v as usize]
    }
}

// ============================================================================
// PixfmtRgba32Lcd
// ============================================================================

const BPP: usize = 4; // bytes per pixel in RGBA32

/// LCD subpixel pixel format for RGBA32 rendering buffers.
///
/// Adapted from C++ `pixfmt_rgb24_lcd` for RGBA32 (4 bytes per pixel) buffers.
/// Reports width as `actual_width * 3` so the rasterizer operates at 3x
/// horizontal resolution. Each "subpixel" maps to one R, G, or B channel
/// of an actual RGBA pixel:
///
/// - Subpixel `sp` → pixel `sp / 3`, channel `sp % 3` (0=R, 1=G, 2=B)
/// - Byte offset: `(sp / 3) * 4 + (sp % 3)`
///
/// The `blend_solid_hspan` method distributes each coverage value across
/// 5 neighboring subpixels (tertiary, secondary, primary, secondary, tertiary)
/// using the `LcdDistributionLut`, matching the C++ implementation exactly.
pub struct PixfmtRgba32Lcd<'a> {
    rbuf: &'a mut RowAccessor,
    lut: &'a LcdDistributionLut,
}

impl<'a> PixfmtRgba32Lcd<'a> {
    /// Create a new LCD pixel format wrapping an RGBA32 rendering buffer.
    pub fn new(rbuf: &'a mut RowAccessor, lut: &'a LcdDistributionLut) -> Self {
        Self { rbuf, lut }
    }

    /// Get the actual (non-subpixel) width.
    #[inline]
    fn actual_width(&self) -> u32 {
        self.rbuf.width()
    }

    /// Blend a single byte in the RGBA buffer using the C++ alpha formula.
    ///
    /// `*p = (((rgb_val - *p) * alpha) + (*p << 16)) >> 16`
    #[inline]
    fn blend_byte(dst: u8, src: u8, alpha: i32) -> u8 {
        (((src as i32 - dst as i32) * alpha + ((dst as i32) << 16)) >> 16) as u8
    }
}

impl<'a> PixelFormat for PixfmtRgba32Lcd<'a> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.rbuf.width() * 3
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        // Map subpixel x to actual pixel
        let pixel = x as usize / 3;
        let actual_w = self.actual_width() as usize;
        if pixel >= actual_w {
            return Rgba8::new(0, 0, 0, 0);
        }
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, actual_w * BPP)
        };
        let off = pixel * BPP;
        Rgba8::new(
            row[off] as u32,
            row[off + 1] as u32,
            row[off + 2] as u32,
            row[off + 3] as u32,
        )
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        let pixel = x as usize / 3;
        let actual_w = self.actual_width() as usize;
        if pixel >= actual_w {
            return;
        }
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };
        let off = pixel * BPP;
        row[off] = c.r;
        row[off + 1] = c.g;
        row[off + 2] = c.b;
        row[off + 3] = c.a;
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        // Copy len subpixels — sets whole pixels for each affected pixel
        let actual_w = self.actual_width() as usize;
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };
        for k in 0..len as usize {
            let sp = x as usize + k;
            let pixel = sp / 3;
            let channel = sp % 3;
            if pixel >= actual_w {
                break;
            }
            let byte_off = pixel * BPP + channel;
            row[byte_off] = [c.r, c.g, c.b][channel];
            // Set alpha to 255 when we touch any channel of a pixel
            row[pixel * BPP + 3] = 255;
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let sp = x as usize;
        let pixel = sp / 3;
        let channel = sp % 3;
        let actual_w = self.actual_width() as usize;
        if pixel >= actual_w {
            return;
        }
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };
        let byte_off = pixel * BPP + channel;
        let rgb = [c.r, c.g, c.b];
        let alpha = cover as i32 * c.a as i32;
        if alpha != 0 {
            if alpha == 255 * 255 {
                row[byte_off] = rgb[channel];
            } else {
                row[byte_off] = Self::blend_byte(row[byte_off], rgb[channel], alpha);
            }
            row[pixel * BPP + 3] = 255;
        }
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, cover: CoverType) {
        let actual_w = self.actual_width() as usize;
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };
        let alpha = cover as i32 * c.a as i32;
        if alpha == 0 {
            return;
        }
        let rgb = [c.r, c.g, c.b];

        for k in 0..len as usize {
            let sp = x as usize + k;
            let pixel = sp / 3;
            let channel = sp % 3;
            if pixel >= actual_w {
                break;
            }
            let byte_off = pixel * BPP + channel;
            if alpha == 255 * 255 {
                row[byte_off] = rgb[channel];
            } else {
                row[byte_off] = Self::blend_byte(row[byte_off], rgb[channel], alpha);
            }
            row[pixel * BPP + 3] = 255;
        }
    }

    /// LCD subpixel coverage distribution — the core of LCD rendering.
    ///
    /// Distributes each coverage value across 5 neighboring subpixel positions
    /// (tertiary, secondary, primary, secondary, tertiary) using the LUT, then
    /// blends the distributed coverage into the RGBA buffer.
    ///
    /// Exact port of C++ `pixfmt_rgb24_lcd::blend_solid_hspan`, adapted for
    /// RGBA32 byte layout.
    fn blend_solid_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &Rgba8,
        covers: &[CoverType],
    ) {
        let len = len as usize;

        // Step 1: Distribute coverage across 5-tap kernel
        // Matching C++: c3[i+0] += tertiary, c3[i+1] += secondary,
        //               c3[i+2] += primary,  c3[i+3] += secondary,
        //               c3[i+4] += tertiary
        let dist_len = len + 4;
        let mut c3 = vec![0u8; dist_len];

        for i in 0..len {
            let cv = covers[i];
            c3[i] = c3[i].wrapping_add(self.lut.tertiary(cv));
            c3[i + 1] = c3[i + 1].wrapping_add(self.lut.secondary(cv));
            c3[i + 2] = c3[i + 2].wrapping_add(self.lut.primary(cv));
            c3[i + 3] = c3[i + 3].wrapping_add(self.lut.secondary(cv));
            c3[i + 4] = c3[i + 4].wrapping_add(self.lut.tertiary(cv));
        }

        // Step 2: Adjust start position (distribution extends 2 subpixels before)
        let mut sp_start = x as i32 - 2;
        let mut c3_offset = 0usize;
        let mut remaining = dist_len;

        if sp_start < 0 {
            let skip = (-sp_start) as usize;
            c3_offset = skip;
            if skip >= remaining {
                return;
            }
            remaining -= skip;
            sp_start = 0;
        }

        // Step 3: Apply distributed covers to RGBA buffer
        let actual_w = self.actual_width() as usize;
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };

        let rgb = [c.r, c.g, c.b];
        // Channel cycling: which RGB channel does sp_start map to?
        // Matching C++: i = x % 3 (after x -= 2)
        // sp_start % 3 gives us the starting channel

        for k in 0..remaining {
            let sp = sp_start as usize + k;
            let pixel = sp / 3;
            let channel = sp % 3;

            if pixel >= actual_w {
                break;
            }

            let cover = c3[c3_offset + k];
            let alpha = cover as i32 * c.a as i32;

            if alpha != 0 {
                let byte_off = pixel * BPP + channel;
                if alpha == 255 * 255 {
                    row[byte_off] = rgb[channel];
                } else {
                    row[byte_off] =
                        Self::blend_byte(row[byte_off], rgb[channel], alpha);
                }
                // Ensure alpha channel is opaque for any touched pixel
                row[pixel * BPP + 3] = 255;
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
        let actual_w = self.actual_width() as usize;
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, actual_w * BPP)
        };

        for k in 0..len as usize {
            let sp = x as usize + k;
            let pixel = sp / 3;
            let channel = sp % 3;
            if pixel >= actual_w {
                break;
            }

            let c = &colors[k];
            let cov = if !covers.is_empty() {
                covers[k]
            } else {
                cover
            };
            let alpha = cov as i32 * c.a as i32;
            if alpha != 0 {
                let byte_off = pixel * BPP + channel;
                let rgb = [c.r, c.g, c.b];
                if alpha == 255 * 255 {
                    row[byte_off] = rgb[channel];
                } else {
                    row[byte_off] =
                        Self::blend_byte(row[byte_off], rgb[channel], alpha);
                }
                row[pixel * BPP + 3] = 255;
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

    #[test]
    fn test_lcd_distribution_lut_construction() {
        // Default weights from the C++ demo: primary=1/3, secondary=2/9, tertiary=1/9
        let lut = LcdDistributionLut::new(1.0 / 3.0, 2.0 / 9.0, 1.0 / 9.0);

        // Coverage 0 should distribute to 0
        assert_eq!(lut.primary(0), 0);
        assert_eq!(lut.secondary(0), 0);
        assert_eq!(lut.tertiary(0), 0);

        // Coverage 255 should distribute fully
        // prim + 2*sec + 2*tert should approximately equal 255
        let total = lut.primary(255) as u32
            + 2 * lut.secondary(255) as u32
            + 2 * lut.tertiary(255) as u32;
        // Due to floor(), total may be slightly less than 255
        assert!(total <= 255);
        assert!(total >= 250, "total distribution = {}", total);
    }

    #[test]
    fn test_lcd_distribution_lut_normalization() {
        // Custom weights that don't sum to 1
        let lut = LcdDistributionLut::new(3.0, 2.0, 1.0);
        // After normalization: prim=3/9, sec=2/9, tert=1/9
        // primary(255) = floor(3/9 * 255) = floor(85) = 85
        assert_eq!(lut.primary(255), 85);
    }

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * BPP as u32) as i32;
        let buf = vec![255u8; (h * w * BPP as u32) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_pixfmt_lcd_width_height() {
        let (_buf, mut ra) = make_buffer(100, 50);
        let lut = LcdDistributionLut::new(1.0 / 3.0, 2.0 / 9.0, 1.0 / 9.0);
        let pf = PixfmtRgba32Lcd::new(&mut ra, &lut);
        assert_eq!(pf.width(), 300); // 100 * 3
        assert_eq!(pf.height(), 50);
    }

    #[test]
    fn test_lcd_blend_solid_hspan_black_on_white() {
        // Blend black text on white background — should darken the pixels
        let (_buf, mut ra) = make_buffer(100, 10);
        let lut = LcdDistributionLut::new(1.0 / 3.0, 2.0 / 9.0, 1.0 / 9.0);
        let mut pf = PixfmtRgba32Lcd::new(&mut ra, &lut);

        // Full-coverage span of 6 subpixels at position 30 (pixel 10)
        let covers = [255u8; 6];
        let black = Rgba8::new(0, 0, 0, 255);
        pf.blend_solid_hspan(30, 5, 6, &black, &covers);

        // The affected pixels should be darker than 255 (white)
        let p = pf.pixel(30, 5); // pixel 10
        assert!(
            p.r < 255 || p.g < 255 || p.b < 255,
            "Expected darkened pixel, got {:?}",
            (p.r, p.g, p.b)
        );
    }
}

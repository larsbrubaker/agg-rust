//! Binary raster glyph generator.
//!
//! Port of `agg_glyph_raster_bin.h`.
//! Extracts binary (1-bit per pixel) glyph bitmaps from embedded raster font data.

use crate::basics::CoverType;

// ============================================================================
// GlyphRect — bounding box of a glyph
// ============================================================================

/// Bounding box and advance of a raster glyph.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlyphRect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub dx: f64,
    pub dy: f64,
}

// ============================================================================
// GlyphRasterBin — extracts glyph bitmaps from embedded font data
// ============================================================================

/// Binary raster glyph generator.
///
/// Port of C++ `glyph_raster_bin<ColorT>`.
/// Reads glyph bitmaps from embedded font data arrays (see `embedded_raster_fonts`).
///
/// Font data format:
/// - Byte 0: height
/// - Byte 1: baseline
/// - Byte 2: start_char (first ASCII code)
/// - Byte 3: num_chars
/// - Bytes 4..4+num_chars*2: glyph offset table (little-endian u16)
/// - Remaining bytes: glyph bitmap data (1 bit per pixel, packed 8 per byte)
pub struct GlyphRasterBin<'a> {
    font: &'a [u8],
    span: Vec<CoverType>,
    bits: &'a [u8],
    glyph_width: u32,
    glyph_byte_width: u32,
}

impl<'a> GlyphRasterBin<'a> {
    pub fn new(font: &'a [u8]) -> Self {
        Self {
            font,
            span: vec![0; 32],
            bits: &[],
            glyph_width: 0,
            glyph_byte_width: 0,
        }
    }

    pub fn font(&self) -> &'a [u8] {
        self.font
    }

    pub fn set_font(&mut self, font: &'a [u8]) {
        self.font = font;
    }

    /// Font height in pixels.
    pub fn height(&self) -> f64 {
        self.font[0] as f64
    }

    /// Baseline offset from top.
    pub fn base_line(&self) -> f64 {
        self.font[1] as f64
    }

    /// Calculate total width of a string.
    pub fn width(&self, s: &str) -> f64 {
        let start_char = self.font[2] as u32;
        let num_chars = self.font[3] as u32;
        let mut w = 0u32;
        for ch in s.bytes() {
            let glyph = ch as u32;
            if glyph >= start_char && glyph < start_char + num_chars {
                let offset = self.value(4 + (glyph - start_char) as usize * 2);
                let bits_start = 4 + num_chars as usize * 2 + offset as usize;
                w += self.font[bits_start] as u32;
            }
        }
        w as f64
    }

    /// Prepare a glyph for rendering.
    ///
    /// Sets up internal state and fills `r` with the glyph's bounding box.
    /// `glyph` is the ASCII character code. `flip` inverts Y direction.
    pub fn prepare(&mut self, r: &mut GlyphRect, x: f64, y: f64, glyph: u32, flip: bool) {
        let start_char = self.font[2] as u32;
        let num_chars = self.font[3] as u32;

        // Skip characters outside the font's range
        if glyph < start_char || glyph >= start_char + num_chars {
            r.x1 = 1;
            r.x2 = 0; // x2 < x1 signals "no glyph" to the renderer
            r.dx = 0.0;
            r.dy = 0.0;
            self.glyph_width = 0;
            self.glyph_byte_width = 0;
            self.bits = &[];
            return;
        }

        let offset = self.value(4 + (glyph - start_char) as usize * 2);
        let bits_start = 4 + num_chars as usize * 2 + offset as usize;

        self.glyph_width = self.font[bits_start] as u32;
        self.glyph_byte_width = (self.glyph_width + 7) >> 3;
        self.bits = &self.font[bits_start + 1..];

        // Ensure span buffer is large enough for this glyph
        if self.span.len() < self.glyph_width as usize {
            self.span.resize(self.glyph_width as usize, 0);
        }

        r.x1 = x as i32;
        r.x2 = r.x1 + self.glyph_width as i32 - 1;
        if flip {
            r.y1 = y as i32 - self.font[0] as i32 + self.font[1] as i32;
            r.y2 = r.y1 + self.font[0] as i32 - 1;
        } else {
            r.y1 = y as i32 - self.font[1] as i32 + 1;
            r.y2 = r.y1 + self.font[0] as i32 - 1;
        }
        r.dx = self.glyph_width as f64;
        r.dy = 0.0;
    }

    /// Get coverage data for scanline `i` (0 = top of glyph).
    ///
    /// Returns a slice of `CoverType` values (0 or 255) for each pixel.
    pub fn span(&mut self, i: u32) -> &[CoverType] {
        if self.glyph_width == 0 || self.bits.is_empty() {
            return &self.span[..0];
        }
        // Font stores rows bottom-to-top, so invert
        let row = self.font[0] as u32 - i - 1;
        let row_start = (row * self.glyph_byte_width) as usize;
        if row_start >= self.bits.len() {
            // Glyph data is truncated — return empty coverage
            for j in 0..self.glyph_width as usize {
                self.span[j] = 0;
            }
            return &self.span[..self.glyph_width as usize];
        }
        let bits = &self.bits[row_start..];
        let mut val = bits[0];
        let mut nb = 0u32;
        let mut bit_idx = 0usize;
        for j in 0..self.glyph_width as usize {
            self.span[j] = if (val & 0x80) != 0 { 255 } else { 0 };
            val <<= 1;
            nb += 1;
            if nb >= 8 {
                bit_idx += 1;
                if bit_idx < bits.len() {
                    val = bits[bit_idx];
                }
                nb = 0;
            }
        }
        &self.span[..self.glyph_width as usize]
    }

    /// Read a little-endian u16 from font data at offset.
    fn value(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.font[offset], self.font[offset + 1]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal fake font: height=2, baseline=0, start_char=65('A'), num_chars=1
    // Glyph offset for 'A': 0
    // Glyph data: width=2, then 1 byte per row (2 rows)
    // Row 0: 0b11000000 = both pixels on
    // Row 1: 0b01000000 = only second pixel on
    fn make_test_font() -> Vec<u8> {
        let mut font = Vec::new();
        // Header
        font.push(2); // height
        font.push(0); // baseline
        font.push(65); // start_char = 'A'
        font.push(1); // num_chars = 1

        // Glyph offset table (1 entry, little-endian u16 = 0)
        font.push(0);
        font.push(0);

        // Glyph data for 'A'
        font.push(2); // glyph_width = 2
        font.push(0b1100_0000); // row 0: both pixels on
        font.push(0b0100_0000); // row 1: only pixel 1 on

        font
    }

    #[test]
    fn test_font_properties() {
        let font = make_test_font();
        let glyph = GlyphRasterBin::new(&font);
        assert_eq!(glyph.height(), 2.0);
        assert_eq!(glyph.base_line(), 0.0);
    }

    #[test]
    fn test_prepare_glyph() {
        let font = make_test_font();
        let mut glyph = GlyphRasterBin::new(&font);
        let mut r = GlyphRect::default();
        glyph.prepare(&mut r, 10.0, 5.0, 65, false);
        assert_eq!(r.x1, 10);
        assert_eq!(r.x2, 11); // width 2
        assert_eq!(r.dx, 2.0);
    }

    #[test]
    fn test_span() {
        let font = make_test_font();
        let mut glyph = GlyphRasterBin::new(&font);
        let mut r = GlyphRect::default();
        glyph.prepare(&mut r, 0.0, 0.0, 65, false);

        // Span 0 (top row) = row 1 of font data (inverted) = 0b01000000
        let s0 = glyph.span(0);
        assert_eq!(s0.len(), 2);
        assert_eq!(s0[0], 0); // first pixel off
        assert_eq!(s0[1], 255); // second pixel on

        // Span 1 (bottom row) = row 0 of font data = 0b11000000
        let s1 = glyph.span(1);
        assert_eq!(s1[0], 255); // both on
        assert_eq!(s1[1], 255);
    }

    #[test]
    fn test_width_calculation() {
        let font = make_test_font();
        let glyph = GlyphRasterBin::new(&font);
        assert_eq!(glyph.width("A"), 2.0);
    }
}

//! Packed scanline container (ScanlineP8).
//!
//! Port of `agg_scanline_p.h` — stores coverage data in a packed/RLE format.
//! Solid spans (uniform coverage) use negative `len` with a single cover value,
//! saving memory for large filled areas.

use crate::rasterizer_scanline_aa::Scanline;

// ============================================================================
// PackedSpan — a span in a packed scanline
// ============================================================================

/// A span in a packed scanline.
///
/// - `len > 0`: per-pixel covers, `cover_offset` indexes into covers array
///   for `len` values
/// - `len < 0`: solid span of `-len` pixels, all with the single cover value
///   at `cover_offset`
#[derive(Debug, Clone, Copy, Default)]
pub struct PackedSpan {
    pub x: i32,
    pub len: i32,
    pub cover_offset: usize,
}

// ============================================================================
// ScanlineP8 — packed scanline with RLE for solid spans
// ============================================================================

/// Packed scanline container with RLE compression for solid (uniform-cover) spans.
///
/// Port of C++ `scanline_p8`. Solid spans store a negative `len` and a single
/// cover value, reducing memory for large filled areas.
pub struct ScanlineP8 {
    last_x: i32,
    y_val: i32,
    covers: Vec<u8>,
    cover_ptr: usize,
    spans: Vec<PackedSpan>,
    cur_span: usize,
}

impl ScanlineP8 {
    pub fn new() -> Self {
        Self {
            last_x: 0x7FFF_FFF0,
            y_val: 0,
            covers: Vec::new(),
            cover_ptr: 0,
            spans: Vec::new(),
            cur_span: 0,
        }
    }

    /// Prepare for a new scanline with the given X range.
    pub fn reset(&mut self, _min_x: i32, max_x: i32) {
        let max_len = (max_x + 3) as usize;
        if max_len > self.spans.len() {
            self.spans.resize(max_len, PackedSpan::default());
            self.covers.resize(max_len, 0);
        }
        self.last_x = 0x7FFF_FFF0;
        self.cover_ptr = 0;
        self.cur_span = 0;
        self.spans[0].len = 0;
    }

    /// Get the slice of active spans (for renderer iteration).
    pub fn begin(&self) -> &[PackedSpan] {
        &self.spans[1..=self.cur_span]
    }

    /// Get the full covers array (spans reference into this via `cover_offset`).
    pub fn covers(&self) -> &[u8] {
        &self.covers
    }
}

impl Scanline for ScanlineP8 {
    fn reset_spans(&mut self) {
        self.last_x = 0x7FFF_FFF0;
        self.cover_ptr = 0;
        self.cur_span = 0;
        self.spans[0].len = 0;
    }

    fn add_cell(&mut self, x: i32, cover: u32) {
        self.covers[self.cover_ptr] = cover as u8;
        if x == self.last_x + 1 && self.spans[self.cur_span].len > 0 {
            self.spans[self.cur_span].len += 1;
        } else {
            self.cur_span += 1;
            self.spans[self.cur_span].cover_offset = self.cover_ptr;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = 1;
        }
        self.last_x = x;
        self.cover_ptr += 1;
    }

    fn add_span(&mut self, x: i32, len: u32, cover: u32) {
        if x == self.last_x + 1
            && self.spans[self.cur_span].len < 0
            && cover as u8 == self.covers[self.spans[self.cur_span].cover_offset]
        {
            // Extend the existing solid span
            self.spans[self.cur_span].len -= len as i32;
        } else {
            self.covers[self.cover_ptr] = cover as u8;
            self.cur_span += 1;
            self.spans[self.cur_span].cover_offset = self.cover_ptr;
            self.cover_ptr += 1;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = -(len as i32);
        }
        self.last_x = x + len as i32 - 1;
    }

    fn finalize(&mut self, y: i32) {
        self.y_val = y;
    }

    fn num_spans(&self) -> u32 {
        self.cur_span as u32
    }

    fn y(&self) -> i32 {
        self.y_val
    }
}

impl Default for ScanlineP8 {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let sl = ScanlineP8::new();
        assert_eq!(sl.num_spans(), 0);
    }

    #[test]
    fn test_add_cell() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 1); // positive = per-pixel
        assert_eq!(sl.covers()[spans[0].cover_offset], 128);
    }

    #[test]
    fn test_adjacent_cells_form_per_pixel_span() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 100);
        sl.add_cell(11, 200);
        sl.add_cell(12, 150);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].len, 3); // positive = per-pixel covers
    }

    #[test]
    fn test_add_span_creates_solid_span() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 5);
        assert_eq!(spans[0].len, -10); // negative = solid span
        assert_eq!(sl.covers()[spans[0].cover_offset], 255);
    }

    #[test]
    fn test_adjacent_solid_spans_merge() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        sl.add_span(15, 5, 255); // adjacent, same cover → merge
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].len, -15); // merged solid span
    }

    #[test]
    fn test_adjacent_solid_spans_different_cover_no_merge() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        sl.add_span(15, 5, 128); // adjacent but different cover
        assert_eq!(sl.num_spans(), 2);
    }

    #[test]
    fn test_cell_after_solid_span_new_span() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_span(5, 3, 200);
        sl.add_cell(8, 100); // adjacent to solid span end, but cell creates per-pixel span
        assert_eq!(sl.num_spans(), 2);
    }

    #[test]
    fn test_reset_spans() {
        let mut sl = ScanlineP8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        sl.reset_spans();
        assert_eq!(sl.num_spans(), 0);
    }
}

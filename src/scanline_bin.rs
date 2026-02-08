//! Binary scanline container (ScanlineBin).
//!
//! Port of `agg_scanline_bin.h` — simplest scanline type with no coverage
//! data. Each span just records X and length. Used for non-anti-aliased
//! rendering or as a clipping mask.

use crate::rasterizer_scanline_aa::Scanline;

// ============================================================================
// BinSpan — a span in a binary scanline
// ============================================================================

/// A horizontal span in a binary scanline (no coverage data).
#[derive(Debug, Clone, Copy, Default)]
pub struct BinSpan {
    pub x: i32,
    pub len: i32,
}

// ============================================================================
// ScanlineBin — binary scanline (no coverage data)
// ============================================================================

/// Binary scanline container — no per-pixel coverage, just on/off spans.
///
/// Port of C++ `scanline_bin`. Useful for non-AA rendering or clipping masks.
pub struct ScanlineBin {
    last_x: i32,
    y_val: i32,
    spans: Vec<BinSpan>,
    cur_span: usize,
}

impl ScanlineBin {
    pub fn new() -> Self {
        Self {
            last_x: 0x7FFF_FFF0,
            y_val: 0,
            spans: Vec::new(),
            cur_span: 0,
        }
    }

    /// Prepare for a new scanline with the given X range.
    pub fn reset(&mut self, _min_x: i32, max_x: i32) {
        let max_len = (max_x + 3) as usize;
        if max_len > self.spans.len() {
            self.spans.resize(max_len, BinSpan::default());
        }
        self.last_x = 0x7FFF_FFF0;
        self.cur_span = 0;
    }

    /// Get the slice of active spans (for renderer iteration).
    pub fn begin(&self) -> &[BinSpan] {
        &self.spans[1..=self.cur_span]
    }
}

impl Scanline for ScanlineBin {
    fn reset_spans(&mut self) {
        self.last_x = 0x7FFF_FFF0;
        self.cur_span = 0;
    }

    fn add_cell(&mut self, x: i32, _cover: u32) {
        if x == self.last_x + 1 {
            self.spans[self.cur_span].len += 1;
        } else {
            self.cur_span += 1;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = 1;
        }
        self.last_x = x;
    }

    fn add_span(&mut self, x: i32, len: u32, _cover: u32) {
        if x == self.last_x + 1 {
            self.spans[self.cur_span].len += len as i32;
        } else {
            self.cur_span += 1;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = len as i32;
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

impl Default for ScanlineBin {
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
        let sl = ScanlineBin::new();
        assert_eq!(sl.num_spans(), 0);
    }

    #[test]
    fn test_add_cell() {
        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_cell(10, 255);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 1);
    }

    #[test]
    fn test_adjacent_cells_merge() {
        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_cell(10, 255);
        sl.add_cell(11, 128);
        sl.add_cell(12, 64);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 3);
    }

    #[test]
    fn test_add_span() {
        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 5);
        assert_eq!(spans[0].len, 10);
    }

    #[test]
    fn test_separate_spans() {
        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_cell(10, 255);
        sl.add_cell(20, 255);
        assert_eq!(sl.num_spans(), 2);
    }

    #[test]
    fn test_reset_spans() {
        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_cell(10, 255);
        sl.reset_spans();
        assert_eq!(sl.num_spans(), 0);
    }
}

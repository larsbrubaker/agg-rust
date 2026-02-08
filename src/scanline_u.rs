//! Unpacked scanline container (ScanlineU8).
//!
//! Port of `agg_scanline_u.h` — stores per-pixel coverage values in a flat
//! array, with spans referencing into it. This is the most commonly used
//! scanline type for anti-aliased rendering.

use crate::rasterizer_scanline_aa::Scanline;

// ============================================================================
// ScanlineSpan — a horizontal run within a scanline
// ============================================================================

/// A horizontal span within a scanline, referencing coverage data.
///
/// For `ScanlineU8`: `len` is always positive, `cover_offset` indexes into
/// the covers array.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanlineSpan {
    pub x: i32,
    pub len: i32,
    pub cover_offset: usize,
}

// ============================================================================
// ScanlineU8 — unpacked scanline with per-pixel coverage
// ============================================================================

/// Unpacked scanline container with per-pixel u8 coverage values.
///
/// Port of C++ `scanline_u8`. Each pixel in a span has its own coverage
/// byte, stored in a flat array indexed by `x - min_x`.
///
/// Usage protocol:
/// 1. `reset(min_x, max_x)` — allocate covers and spans arrays
/// 2. `add_cell()` / `add_span()` — accumulate span data (X must be monotonically increasing)
/// 3. `finalize(y)` — set the Y coordinate
/// 4. Iterate with `begin()` and `covers()` for rendering
/// 5. `reset_spans()` — prepare for next scanline
pub struct ScanlineU8 {
    min_x: i32,
    last_x: i32,
    y_val: i32,
    covers: Vec<u8>,
    spans: Vec<ScanlineSpan>,
    cur_span: usize, // index of current span (0 = sentinel, spans start at 1)
}

impl ScanlineU8 {
    pub fn new() -> Self {
        Self {
            min_x: 0,
            last_x: 0x7FFF_FFF0,
            y_val: 0,
            covers: Vec::new(),
            spans: Vec::new(),
            cur_span: 0,
        }
    }

    /// Prepare for a new scanline with the given X range.
    pub fn reset(&mut self, min_x: i32, max_x: i32) {
        let max_len = (max_x - min_x + 2) as usize;
        if max_len > self.spans.len() {
            self.spans.resize(max_len, ScanlineSpan::default());
            self.covers.resize(max_len, 0);
        }
        self.last_x = 0x7FFF_FFF0;
        self.min_x = min_x;
        self.cur_span = 0;
    }

    /// Get the slice of active spans (for renderer iteration).
    /// Spans are 1-indexed; index 0 is a sentinel.
    pub fn begin(&self) -> &[ScanlineSpan] {
        &self.spans[1..=self.cur_span]
    }

    /// Get the full covers array (spans reference into this via `cover_offset`).
    pub fn covers(&self) -> &[u8] {
        &self.covers
    }
}

impl Scanline for ScanlineU8 {
    fn reset_spans(&mut self) {
        self.last_x = 0x7FFF_FFF0;
        self.cur_span = 0;
    }

    fn add_cell(&mut self, x: i32, cover: u32) {
        let xi = (x - self.min_x) as usize;
        self.covers[xi] = cover as u8;
        if xi as i32 == self.last_x + 1 {
            self.spans[self.cur_span].len += 1;
        } else {
            self.cur_span += 1;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = 1;
            self.spans[self.cur_span].cover_offset = xi;
        }
        self.last_x = xi as i32;
    }

    fn add_span(&mut self, x: i32, len: u32, cover: u32) {
        let xi = (x - self.min_x) as usize;
        // Fill covers with the same value
        for i in 0..len as usize {
            self.covers[xi + i] = cover as u8;
        }
        if xi as i32 == self.last_x + 1 {
            self.spans[self.cur_span].len += len as i32;
        } else {
            self.cur_span += 1;
            self.spans[self.cur_span].x = x;
            self.spans[self.cur_span].len = len as i32;
            self.spans[self.cur_span].cover_offset = xi;
        }
        self.last_x = xi as i32 + len as i32 - 1;
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

impl Default for ScanlineU8 {
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
        let sl = ScanlineU8::new();
        assert_eq!(sl.num_spans(), 0);
        assert_eq!(sl.y(), 0);
    }

    #[test]
    fn test_reset_and_add_cell() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 1);
        assert_eq!(sl.covers()[spans[0].cover_offset], 128);
    }

    #[test]
    fn test_adjacent_cells_merge() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 100);
        sl.add_cell(11, 200);
        sl.add_cell(12, 150);
        // Adjacent cells should form a single span
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 3);
        assert_eq!(sl.covers()[spans[0].cover_offset], 100);
        assert_eq!(sl.covers()[spans[0].cover_offset + 1], 200);
        assert_eq!(sl.covers()[spans[0].cover_offset + 2], 150);
    }

    #[test]
    fn test_non_adjacent_cells_separate_spans() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 100);
        sl.add_cell(20, 200);
        assert_eq!(sl.num_spans(), 2);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[1].x, 20);
    }

    #[test]
    fn test_add_span() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 5);
        assert_eq!(spans[0].len, 10);
        // All covers should be 255
        for i in 0..10 {
            assert_eq!(sl.covers()[spans[0].cover_offset + i], 255);
        }
    }

    #[test]
    fn test_finalize() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        sl.finalize(42);
        assert_eq!(sl.y(), 42);
    }

    #[test]
    fn test_reset_spans() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        assert_eq!(sl.num_spans(), 1);
        sl.reset_spans();
        assert_eq!(sl.num_spans(), 0);
    }

    #[test]
    fn test_span_then_adjacent_cell() {
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_span(5, 3, 200);
        sl.add_cell(8, 100); // adjacent to span end (5+3-1=7, next=8)
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].len, 4);
    }

    #[test]
    fn test_with_min_x_offset() {
        let mut sl = ScanlineU8::new();
        sl.reset(50, 150);
        sl.add_cell(60, 128);
        sl.add_cell(61, 64);
        assert_eq!(sl.num_spans(), 1);
        let spans = sl.begin();
        assert_eq!(spans[0].x, 60);
        assert_eq!(spans[0].len, 2);
        assert_eq!(sl.covers()[spans[0].cover_offset], 128);
        assert_eq!(sl.covers()[spans[0].cover_offset + 1], 64);
    }
}

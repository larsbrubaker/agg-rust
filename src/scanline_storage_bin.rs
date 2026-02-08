//! Binary (non-AA) scanline storage.
//!
//! Port of `agg_scanline_storage_bin.h`.
//! Stores rasterized scanlines without coverage data, for boolean
//! operations on binary (non-anti-aliased) shapes.

use crate::rasterizer_scanline_aa::Scanline;

/// Stored span data: x and len only (no coverage).
#[derive(Debug, Clone, Copy, Default)]
struct SpanData {
    x: i32,
    len: i32,
}

/// Per-scanline metadata.
#[derive(Debug, Clone, Copy, Default)]
struct ScanlineData {
    y: i32,
    num_spans: u32,
    start_span: usize,
}

/// Binary (non-AA) scanline storage.
///
/// Port of C++ `scanline_storage_bin`.
/// Like `ScanlineStorageAa` but stores only span extents, no coverage.
pub struct ScanlineStorageBin {
    spans: Vec<SpanData>,
    scanlines: Vec<ScanlineData>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    cur_scanline: usize,
}

impl ScanlineStorageBin {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            scanlines: Vec::new(),
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: i32::MIN,
            max_y: i32::MIN,
            cur_scanline: 0,
        }
    }

    /// Prepare for new rendering (clear all stored data).
    pub fn prepare(&mut self) {
        self.spans.clear();
        self.scanlines.clear();
        self.min_x = i32::MAX;
        self.min_y = i32::MAX;
        self.max_x = i32::MIN;
        self.max_y = i32::MIN;
        self.cur_scanline = 0;
    }

    /// Store a scanline from a ScanlineBin.
    pub fn render_scanline_bin(&mut self, sl: &crate::scanline_bin::ScanlineBin) {
        let y = sl.y();
        if y < self.min_y {
            self.min_y = y;
        }
        if y > self.max_y {
            self.max_y = y;
        }

        let start_span = self.spans.len();
        let mut num_spans = 0u32;

        let spans = sl.begin();
        for sp in spans {
            let x = sp.x;
            let len = sp.len;
            let xe = x + len - 1;
            if x < self.min_x {
                self.min_x = x;
            }
            if xe > self.max_x {
                self.max_x = xe;
            }
            self.spans.push(SpanData { x, len });
            num_spans += 1;
        }

        self.scanlines.push(ScanlineData {
            y,
            num_spans,
            start_span,
        });
    }

    /// Store a scanline from any Scanline type (uses add_span with cover_full).
    pub fn render_scanline_u8(&mut self, sl: &crate::scanline_u::ScanlineU8) {
        let y = sl.y();
        if y < self.min_y {
            self.min_y = y;
        }
        if y > self.max_y {
            self.max_y = y;
        }

        let start_span = self.spans.len();
        let mut num_spans = 0u32;

        let spans = sl.begin();
        for sp in spans {
            let x = sp.x;
            let len = sp.len;
            let xe = x + len - 1;
            if x < self.min_x {
                self.min_x = x;
            }
            if xe > self.max_x {
                self.max_x = xe;
            }
            self.spans.push(SpanData { x, len });
            num_spans += 1;
        }

        self.scanlines.push(ScanlineData {
            y,
            num_spans,
            start_span,
        });
    }

    /// Reset the scanline iteration pointer.
    pub fn rewind_scanlines(&mut self) -> bool {
        self.cur_scanline = 0;
        !self.scanlines.is_empty()
    }

    /// Sweep the next stored scanline into an output scanline container.
    pub fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool {
        sl.reset_spans();
        loop {
            if self.cur_scanline >= self.scanlines.len() {
                return false;
            }
            let sld = self.scanlines[self.cur_scanline];
            let num_spans = sld.num_spans;
            self.cur_scanline += 1;

            if num_spans == 0 {
                continue;
            }

            for i in 0..num_spans as usize {
                let sp = self.spans[sld.start_span + i];
                sl.add_span(sp.x, sp.len as u32, 255); // cover_full for binary
            }
            sl.finalize(sld.y);
            return true;
        }
    }

    pub fn min_x(&self) -> i32 {
        self.min_x
    }
    pub fn min_y(&self) -> i32 {
        self.min_y
    }
    pub fn max_x(&self) -> i32 {
        self.max_x
    }
    pub fn max_y(&self) -> i32 {
        self.max_y
    }

    pub fn num_scanlines(&self) -> usize {
        self.scanlines.len()
    }

    /// Get the Y coordinate for a stored scanline by index.
    pub fn scanline_y(&self, idx: usize) -> i32 {
        self.scanlines[idx].y
    }

    /// Get the number of spans for a stored scanline.
    pub fn scanline_num_spans(&self, idx: usize) -> u32 {
        self.scanlines[idx].num_spans
    }

    /// Iterate over embedded spans for boolean algebra.
    pub fn embedded_spans(&self, sl_idx: usize) -> impl Iterator<Item = EmbeddedBinSpan> + '_ {
        let sld = &self.scanlines[sl_idx];
        let spans = &self.spans[sld.start_span..sld.start_span + sld.num_spans as usize];
        spans.iter().map(|sp| EmbeddedBinSpan {
            x: sp.x,
            len: sp.len,
        })
    }
}

impl Default for ScanlineStorageBin {
    fn default() -> Self {
        Self::new()
    }
}

/// A span from embedded binary scanline iteration.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedBinSpan {
    pub x: i32,
    pub len: i32,
}

impl EmbeddedBinSpan {
    /// Get end X (exclusive).
    pub fn x_end(&self) -> i32 {
        self.x + self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanline_bin::ScanlineBin;

    #[test]
    fn test_empty_storage() {
        let storage = ScanlineStorageBin::new();
        assert_eq!(storage.num_scanlines(), 0);
    }

    #[test]
    fn test_store_and_replay() {
        let mut storage = ScanlineStorageBin::new();
        storage.prepare();

        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_span(10, 5, 255);
        sl.finalize(3);
        storage.render_scanline_bin(&sl);

        assert_eq!(storage.num_scanlines(), 1);
        assert_eq!(storage.min_x(), 10);
        assert_eq!(storage.max_x(), 14);

        assert!(storage.rewind_scanlines());
        let mut sl2 = ScanlineBin::new();
        sl2.reset(0, 100);
        assert!(storage.sweep_scanline(&mut sl2));
        assert_eq!(sl2.y(), 3);
        assert_eq!(sl2.num_spans(), 1);
        let spans = sl2.begin();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 5);
    }

    #[test]
    fn test_multiple_scanlines() {
        let mut storage = ScanlineStorageBin::new();
        storage.prepare();

        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_span(0, 10, 255);
        sl.finalize(0);
        storage.render_scanline_bin(&sl);

        sl.reset_spans();
        sl.add_span(5, 15, 255);
        sl.finalize(1);
        storage.render_scanline_bin(&sl);

        assert_eq!(storage.num_scanlines(), 2);
        assert_eq!(storage.min_x(), 0);
        assert_eq!(storage.max_x(), 19);
        assert_eq!(storage.min_y(), 0);
        assert_eq!(storage.max_y(), 1);
    }

    #[test]
    fn test_embedded_spans() {
        let mut storage = ScanlineStorageBin::new();
        storage.prepare();

        let mut sl = ScanlineBin::new();
        sl.reset(0, 100);
        sl.add_span(5, 10, 255);
        sl.add_span(30, 5, 255);
        sl.finalize(0);
        storage.render_scanline_bin(&sl);

        let spans: Vec<_> = storage.embedded_spans(0).collect();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].x, 5);
        assert_eq!(spans[0].len, 10);
        assert_eq!(spans[1].x, 30);
        assert_eq!(spans[1].len, 5);
    }
}

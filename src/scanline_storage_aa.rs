//! Anti-aliased scanline storage.
//!
//! Port of `agg_scanline_storage_aa.h`.
//! Stores rasterized scanlines in memory for later boolean operations
//! or serialized replay. Each span stores per-pixel coverage data.

use crate::rasterizer_scanline_aa::Scanline;

/// Stored span data: x, len (positive=per-pixel, negative=solid), covers offset.
#[derive(Debug, Clone, Copy, Default)]
struct SpanData {
    x: i32,
    len: i32,
    covers_offset: usize,
}

/// Per-scanline metadata.
#[derive(Debug, Clone, Copy, Default)]
struct ScanlineData {
    y: i32,
    num_spans: u32,
    start_span: usize,
}

/// Anti-aliased scanline storage.
///
/// Port of C++ `scanline_storage_aa<T>`.
/// Stores rendered scanlines for later replay, boolean operations, or serialization.
pub struct ScanlineStorageAa {
    spans: Vec<SpanData>,
    covers: Vec<u8>,
    scanlines: Vec<ScanlineData>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    cur_scanline: usize,
}

impl ScanlineStorageAa {
    pub fn new() -> Self {
        Self {
            spans: Vec::new(),
            covers: Vec::new(),
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
        self.covers.clear();
        self.scanlines.clear();
        self.min_x = i32::MAX;
        self.min_y = i32::MAX;
        self.max_x = i32::MIN;
        self.max_y = i32::MIN;
        self.cur_scanline = 0;
    }

    /// Store a scanline from a ScanlineU8 (unpacked, per-pixel covers).
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
        let covers_buf = sl.covers();

        for sp in spans {
            let x = sp.x;
            let len = sp.len;
            if len > 0 {
                // Per-pixel covers
                let xe = x + len - 1;
                if x < self.min_x {
                    self.min_x = x;
                }
                if xe > self.max_x {
                    self.max_x = xe;
                }
                let covers_offset = self.covers.len();
                self.covers
                    .extend_from_slice(&covers_buf[sp.cover_offset..sp.cover_offset + len as usize]);
                self.spans.push(SpanData {
                    x,
                    len,
                    covers_offset,
                });
            }
            num_spans += 1;
        }

        self.scanlines.push(ScanlineData {
            y,
            num_spans,
            start_span,
        });
    }

    /// Store a scanline from a ScanlineP8 (packed, may have solid spans).
    pub fn render_scanline_p8(&mut self, sl: &crate::scanline_p::ScanlineP8) {
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
        let covers_buf = sl.covers();

        for sp in spans {
            let x = sp.x;
            let len = sp.len;
            let abs_len = len.unsigned_abs() as i32;
            let xe = x + abs_len - 1;
            if x < self.min_x {
                self.min_x = x;
            }
            if xe > self.max_x {
                self.max_x = xe;
            }
            let covers_offset = self.covers.len();
            if len < 0 {
                // Solid span — store single cover value
                self.covers.push(covers_buf[sp.cover_offset]);
            } else {
                // Per-pixel covers
                self.covers
                    .extend_from_slice(&covers_buf[sp.cover_offset..sp.cover_offset + len as usize]);
            }
            self.spans.push(SpanData {
                x,
                len,
                covers_offset,
            });
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
    /// Returns false when all scanlines have been consumed.
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
                if sp.len < 0 {
                    // Solid span
                    sl.add_span(sp.x, (-sp.len) as u32, self.covers[sp.covers_offset] as u32);
                } else if sp.len > 0 {
                    // Per-pixel spans — add cell by cell
                    for j in 0..sp.len as usize {
                        sl.add_cell(sp.x + j as i32, self.covers[sp.covers_offset + j] as u32);
                    }
                }
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

    // ========================================================================
    // Embedded scanline access (for boolean algebra)
    // ========================================================================

    /// Get the Y coordinate for a stored scanline by index.
    pub fn scanline_y(&self, idx: usize) -> i32 {
        self.scanlines[idx].y
    }

    /// Get the number of spans for a stored scanline by index.
    pub fn scanline_num_spans(&self, idx: usize) -> u32 {
        self.scanlines[idx].num_spans
    }

    /// Get the covers slice for a span.
    fn span_covers(&self, sp: &SpanData) -> &[u8] {
        if sp.len < 0 {
            &self.covers[sp.covers_offset..sp.covers_offset + 1]
        } else {
            &self.covers[sp.covers_offset..sp.covers_offset + sp.len as usize]
        }
    }

    /// Iterate over embedded scanline spans for boolean algebra.
    /// Returns an iterator of (x, len, covers_slice) tuples.
    pub fn embedded_spans(
        &self,
        sl_idx: usize,
    ) -> impl Iterator<Item = EmbeddedSpan<'_>> + '_ {
        let sld = &self.scanlines[sl_idx];
        let spans = &self.spans[sld.start_span..sld.start_span + sld.num_spans as usize];
        spans.iter().map(move |sp| EmbeddedSpan {
            x: sp.x,
            len: sp.len,
            covers: self.span_covers(sp),
        })
    }
}

impl Default for ScanlineStorageAa {
    fn default() -> Self {
        Self::new()
    }
}

/// A span reference from embedded scanline iteration.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedSpan<'a> {
    pub x: i32,
    pub len: i32,
    pub covers: &'a [u8],
}

impl<'a> EmbeddedSpan<'a> {
    /// Get the absolute length (number of pixels).
    pub fn abs_len(&self) -> i32 {
        self.len.abs()
    }

    /// Get end X (exclusive).
    pub fn x_end(&self) -> i32 {
        self.x + self.abs_len()
    }

    /// Get the coverage at pixel offset `i` from span start.
    pub fn cover_at(&self, i: usize) -> u8 {
        if self.len < 0 {
            self.covers[0] // solid span
        } else {
            self.covers[i]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanline_u::ScanlineU8;

    #[test]
    fn test_empty_storage() {
        let storage = ScanlineStorageAa::new();
        assert_eq!(storage.num_scanlines(), 0);
    }

    #[test]
    fn test_store_and_replay() {
        let mut storage = ScanlineStorageAa::new();
        storage.prepare();

        // Build a scanline with a few cells
        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        sl.add_cell(11, 255);
        sl.add_cell(12, 64);
        sl.finalize(5);
        storage.render_scanline_u8(&sl);

        assert_eq!(storage.num_scanlines(), 1);
        assert_eq!(storage.min_x(), 10);
        assert_eq!(storage.max_x(), 12);
        assert_eq!(storage.min_y(), 5);
        assert_eq!(storage.max_y(), 5);

        // Replay into a fresh scanline
        assert!(storage.rewind_scanlines());
        let mut sl2 = ScanlineU8::new();
        sl2.reset(0, 100);
        assert!(storage.sweep_scanline(&mut sl2));
        assert_eq!(sl2.y(), 5);
        assert_eq!(sl2.num_spans(), 1);

        // Check coverage values
        let spans = sl2.begin();
        let covers = sl2.covers();
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 3);
        assert_eq!(covers[spans[0].cover_offset], 128);
        assert_eq!(covers[spans[0].cover_offset + 1], 255);
        assert_eq!(covers[spans[0].cover_offset + 2], 64);
    }

    #[test]
    fn test_multiple_scanlines() {
        let mut storage = ScanlineStorageAa::new();
        storage.prepare();

        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(5, 200);
        sl.finalize(0);
        storage.render_scanline_u8(&sl);

        sl.reset_spans();
        sl.add_cell(10, 100);
        sl.finalize(1);
        storage.render_scanline_u8(&sl);

        assert_eq!(storage.num_scanlines(), 2);
        assert_eq!(storage.min_y(), 0);
        assert_eq!(storage.max_y(), 1);

        // Replay
        assert!(storage.rewind_scanlines());
        let mut sl2 = ScanlineU8::new();
        sl2.reset(0, 100);
        assert!(storage.sweep_scanline(&mut sl2));
        assert_eq!(sl2.y(), 0);
        assert!(storage.sweep_scanline(&mut sl2));
        assert_eq!(sl2.y(), 1);
        assert!(!storage.sweep_scanline(&mut sl2));
    }

    #[test]
    fn test_prepare_clears() {
        let mut storage = ScanlineStorageAa::new();

        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(5, 200);
        sl.finalize(0);
        storage.render_scanline_u8(&sl);

        storage.prepare();
        assert_eq!(storage.num_scanlines(), 0);
    }

    #[test]
    fn test_embedded_spans() {
        let mut storage = ScanlineStorageAa::new();
        storage.prepare();

        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        sl.add_cell(10, 128);
        sl.add_cell(11, 255);
        sl.add_cell(20, 64);
        sl.finalize(5);
        storage.render_scanline_u8(&sl);

        let spans: Vec<_> = storage.embedded_spans(0).collect();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].x, 10);
        assert_eq!(spans[0].len, 2);
        assert_eq!(spans[0].cover_at(0), 128);
        assert_eq!(spans[0].cover_at(1), 255);
        assert_eq!(spans[1].x, 20);
        assert_eq!(spans[1].len, 1);
        assert_eq!(spans[1].cover_at(0), 64);
    }
}

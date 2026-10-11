//! agg-rust's original storage-based boolean API, kept for its callers and run through the
//! faithful C++ walkers in `shapes.rs`.
//!
//! `sbool_combine_shapes_aa` / `_bin` here stage both rasterizers into storages (as C++'s
//! `scanline_boolean2` demo does) and combine the storages; `sbool_combine_storages_*`
//! combine two storages read through borrowed cursors, so the inputs stay untouched. Like
//! C++, the result storage is prepared (cleared) by the walk whenever there is anything to
//! combine. The generic C++-signature dispatchers are `shapes::sbool_combine_shapes_aa` /
//! `shapes::sbool_combine_shapes_bin`.

use super::{shapes, SBoolOp, ScanlineGenerator};
use crate::rasterizer_scanline_aa::{RasterizerScanlineAa, Scanline};
use crate::scanline_bin::ScanlineBin;
use crate::scanline_storage_aa::ScanlineStorageAa;
use crate::scanline_storage_bin::ScanlineStorageBin;
use crate::scanline_u::ScanlineU8;

/// Perform a boolean operation on two rasterized shapes (anti-aliased): both are rendered
/// into `storage1` / `storage2`, then combined into `storage_result` through `sl_result`.
#[allow(clippy::too_many_arguments)]
pub fn sbool_combine_shapes_aa(
    op: SBoolOp,
    ras1: &mut RasterizerScanlineAa,
    ras2: &mut RasterizerScanlineAa,
    sl1: &mut ScanlineU8,
    sl2: &mut ScanlineU8,
    sl_result: &mut ScanlineU8,
    storage1: &mut ScanlineStorageAa,
    storage2: &mut ScanlineStorageAa,
    storage_result: &mut ScanlineStorageAa,
) {
    storage1.prepare();
    if ras1.rewind_scanlines() {
        sl1.reset(ras1.min_x(), ras1.max_x());
        while ras1.sweep_scanline(sl1) {
            storage1.render_scanline_u8(sl1);
        }
    }
    storage2.prepare();
    if ras2.rewind_scanlines() {
        sl2.reset(ras2.min_x(), ras2.max_x());
        while ras2.sweep_scanline(sl2) {
            storage2.render_scanline_u8(sl2);
        }
    }
    storage_result.prepare();
    shapes::sbool_combine_shapes_aa(op, storage1, storage2, sl1, sl2, sl_result, storage_result);
}

/// Combine two AA scanline storages into `result`, through the result scanline `sl`.
pub fn sbool_combine_storages_aa(
    op: SBoolOp,
    storage1: &ScanlineStorageAa,
    storage2: &ScanlineStorageAa,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageAa,
) {
    let mut g1 = StorageAaCursor::new(storage1);
    let mut g2 = StorageAaCursor::new(storage2);
    let mut sl1 = ScanlineU8::new();
    let mut sl2 = ScanlineU8::new();
    shapes::sbool_combine_shapes_aa(op, &mut g1, &mut g2, &mut sl1, &mut sl2, sl, result);
}

/// Perform a boolean operation on two rasterized shapes (binary): both are rendered into
/// `storage1` / `storage2`, then combined into `storage_result` through `sl_result`.
#[allow(clippy::too_many_arguments)]
pub fn sbool_combine_shapes_bin(
    op: SBoolOp,
    ras1: &mut RasterizerScanlineAa,
    ras2: &mut RasterizerScanlineAa,
    sl1: &mut ScanlineU8,
    sl2: &mut ScanlineU8,
    sl_result: &mut ScanlineU8,
    storage1: &mut ScanlineStorageBin,
    storage2: &mut ScanlineStorageBin,
    storage_result: &mut ScanlineStorageBin,
) {
    storage1.prepare();
    if ras1.rewind_scanlines() {
        sl1.reset(ras1.min_x(), ras1.max_x());
        while ras1.sweep_scanline(sl1) {
            storage1.render_scanline_u8(sl1);
        }
    }
    storage2.prepare();
    if ras2.rewind_scanlines() {
        sl2.reset(ras2.min_x(), ras2.max_x());
        while ras2.sweep_scanline(sl2) {
            storage2.render_scanline_u8(sl2);
        }
    }
    storage_result.prepare();
    let mut b1 = ScanlineBin::new();
    let mut b2 = ScanlineBin::new();
    shapes::sbool_combine_shapes_bin(
        op,
        storage1,
        storage2,
        &mut b1,
        &mut b2,
        sl_result,
        storage_result,
    );
}

/// Combine two binary scanline storages into `result`, through the result scanline `sl`.
pub fn sbool_combine_storages_bin(
    op: SBoolOp,
    storage1: &ScanlineStorageBin,
    storage2: &ScanlineStorageBin,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageBin,
) {
    let mut g1 = StorageBinCursor::new(storage1);
    let mut g2 = StorageBinCursor::new(storage2);
    let mut sl1 = ScanlineBin::new();
    let mut sl2 = ScanlineBin::new();
    shapes::sbool_combine_shapes_bin(op, &mut g1, &mut g2, &mut sl1, &mut sl2, sl, result);
}

/// Reads a borrowed AA storage as a generator, as its own `sweep_scanline` does.
struct StorageAaCursor<'a> {
    storage: &'a ScanlineStorageAa,
    cur: usize,
}

impl<'a> StorageAaCursor<'a> {
    fn new(storage: &'a ScanlineStorageAa) -> Self {
        Self { storage, cur: 0 }
    }
}

impl ScanlineGenerator for StorageAaCursor<'_> {
    fn rewind_scanlines(&mut self) -> bool {
        self.cur = 0;
        self.storage.num_scanlines() > 0
    }
    fn min_x(&self) -> i32 {
        self.storage.min_x()
    }
    fn min_y(&self) -> i32 {
        self.storage.min_y()
    }
    fn max_x(&self) -> i32 {
        self.storage.max_x()
    }
    fn max_y(&self) -> i32 {
        self.storage.max_y()
    }
    fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool {
        sl.reset_spans();
        while self.cur < self.storage.num_scanlines() {
            let idx = self.cur;
            self.cur += 1;
            if self.storage.scanline_num_spans(idx) == 0 {
                continue;
            }
            for sp in self.storage.embedded_spans(idx) {
                if sp.len < 0 {
                    sl.add_span(sp.x, sp.len.unsigned_abs(), sp.covers[0] as u32);
                } else if sp.len > 0 {
                    sl.add_cells(sp.x, sp.len as u32, sp.covers);
                }
            }
            sl.finalize(self.storage.scanline_y(idx));
            return true;
        }
        false
    }
}

/// Reads a borrowed binary storage as a generator, as its own `sweep_scanline` does.
struct StorageBinCursor<'a> {
    storage: &'a ScanlineStorageBin,
    cur: usize,
}

impl<'a> StorageBinCursor<'a> {
    fn new(storage: &'a ScanlineStorageBin) -> Self {
        Self { storage, cur: 0 }
    }
}

impl ScanlineGenerator for StorageBinCursor<'_> {
    fn rewind_scanlines(&mut self) -> bool {
        self.cur = 0;
        self.storage.num_scanlines() > 0
    }
    fn min_x(&self) -> i32 {
        self.storage.min_x()
    }
    fn min_y(&self) -> i32 {
        self.storage.min_y()
    }
    fn max_x(&self) -> i32 {
        self.storage.max_x()
    }
    fn max_y(&self) -> i32 {
        self.storage.max_y()
    }
    fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool {
        sl.reset_spans();
        while self.cur < self.storage.num_scanlines() {
            let idx = self.cur;
            self.cur += 1;
            if self.storage.scanline_num_spans(idx) == 0 {
                continue;
            }
            for sp in self.storage.embedded_spans(idx) {
                sl.add_span(sp.x, sp.len as u32, 255);
            }
            sl.finalize(self.storage.scanline_y(idx));
            return true;
        }
        false
    }
}

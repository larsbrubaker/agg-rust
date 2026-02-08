//! Scanline boolean algebra.
//!
//! Port of `agg_scanline_boolean_algebra.h`.
//! Provides boolean operations (union, intersect, subtract, XOR) on
//! rasterized shapes stored in scanline storage.

use crate::rasterizer_scanline_aa::{RasterizerScanlineAa, Scanline};
use crate::scanline_storage_aa::ScanlineStorageAa;
use crate::scanline_storage_bin::ScanlineStorageBin;
use crate::scanline_u::ScanlineU8;

/// Boolean operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SBoolOp {
    Or,
    And,
    Xor,
    AMinusB,
    BMinusA,
}

const COVER_FULL: u32 = 255;
const COVER_SHIFT: u32 = 8;

// ============================================================================
// AA Coverage combination functions
// ============================================================================

/// Combine two AA coverage values for intersection (AND).
/// C++: `cover = c1 * c2; if (cover == full*full) full else cover >> shift`
#[inline]
fn intersect_covers(c1: u32, c2: u32) -> u8 {
    let cover = c1 * c2;
    if cover == COVER_FULL * COVER_FULL {
        COVER_FULL as u8
    } else {
        (cover >> COVER_SHIFT) as u8
    }
}

/// Combine two AA coverage values for union (OR).
/// C++: `cover = full*full - (full-c1)*(full-c2); special case for full*full`
#[inline]
fn unite_covers(c1: u32, c2: u32) -> u8 {
    let cover = COVER_FULL * COVER_FULL - (COVER_FULL - c1) * (COVER_FULL - c2);
    if cover == COVER_FULL * COVER_FULL {
        COVER_FULL as u8
    } else {
        (cover >> COVER_SHIFT) as u8
    }
}

/// Combine two AA coverage values for subtraction (A - B).
/// C++: `cover = c1 * (full - c2); special case for full*full`
#[inline]
fn subtract_covers(c1: u32, c2: u32) -> u8 {
    let cover = c1 * (COVER_FULL - c2);
    if cover == COVER_FULL * COVER_FULL {
        COVER_FULL as u8
    } else {
        (cover >> COVER_SHIFT) as u8
    }
}

/// Combine two AA coverage values for XOR (linear formula).
/// C++: `cover = a + b; if (cover > full) cover = full + full - cover`
#[inline]
fn xor_covers(c1: u32, c2: u32) -> u8 {
    let cover = c1 + c2;
    if cover > COVER_FULL {
        (COVER_FULL + COVER_FULL - cover) as u8
    } else {
        cover as u8
    }
}

// ============================================================================
// Shape-level boolean operations (AA)
// ============================================================================

/// Perform a boolean operation on two rasterized shapes.
///
/// Takes two rasterizers, rasterizes both into storage, then combines them.
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
    // Render shape 1 into storage1
    storage1.prepare();
    render_to_storage(ras1, sl1, storage1);

    // Render shape 2 into storage2
    storage2.prepare();
    render_to_storage(ras2, sl2, storage2);

    // Combine
    storage_result.prepare();
    sbool_combine_storages_aa(op, storage1, storage2, sl_result, storage_result);
}

/// Render a rasterizer's output into AA storage.
fn render_to_storage(
    ras: &mut RasterizerScanlineAa,
    sl: &mut ScanlineU8,
    storage: &mut ScanlineStorageAa,
) {
    if ras.rewind_scanlines() {
        sl.reset(ras.min_x(), ras.max_x());
        while ras.sweep_scanline(sl) {
            storage.render_scanline_u8(sl);
        }
    }
}

/// Combine two AA scanline storages.
pub fn sbool_combine_storages_aa(
    op: SBoolOp,
    storage1: &ScanlineStorageAa,
    storage2: &ScanlineStorageAa,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageAa,
) {
    let n1 = storage1.num_scanlines();
    let n2 = storage2.num_scanlines();

    if n1 == 0 && n2 == 0 {
        return;
    }

    // Determine the combined X range for the result scanline
    let min_x = if n1 > 0 && n2 > 0 {
        storage1.min_x().min(storage2.min_x())
    } else if n1 > 0 {
        storage1.min_x()
    } else {
        storage2.min_x()
    };
    let max_x = if n1 > 0 && n2 > 0 {
        storage1.max_x().max(storage2.max_x())
    } else if n1 > 0 {
        storage1.max_x()
    } else {
        storage2.max_x()
    };

    // Handle cases where one storage is empty
    match op {
        SBoolOp::Or | SBoolOp::Xor => {
            // Union/XOR: one empty → result is the other
            if n1 == 0 {
                copy_storage_aa(storage2, sl, result, min_x, max_x);
                return;
            }
            if n2 == 0 {
                copy_storage_aa(storage1, sl, result, min_x, max_x);
                return;
            }
        }
        SBoolOp::And => {
            // Intersection: one empty → empty result
            if n1 == 0 || n2 == 0 {
                return;
            }
        }
        SBoolOp::AMinusB => {
            if n1 == 0 {
                return; // nothing to subtract from
            }
            if n2 == 0 {
                copy_storage_aa(storage1, sl, result, min_x, max_x);
                return;
            }
        }
        SBoolOp::BMinusA => {
            if n2 == 0 {
                return;
            }
            if n1 == 0 {
                copy_storage_aa(storage2, sl, result, min_x, max_x);
                return;
            }
        }
    }

    // Both storages have scanlines — synchronize Y coordinates
    let mut i1 = 0usize;
    let mut i2 = 0usize;

    while i1 < n1 || i2 < n2 {
        if i1 >= n1 {
            // Only storage2 remaining
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::BMinusA => {
                    emit_scanline_from_storage(storage2, i2, sl, result, min_x, max_x);
                }
                _ => {} // And, AMinusB: nothing
            }
            i2 += 1;
            continue;
        }
        if i2 >= n2 {
            // Only storage1 remaining
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::AMinusB => {
                    emit_scanline_from_storage(storage1, i1, sl, result, min_x, max_x);
                }
                _ => {} // And, BMinusA: nothing
            }
            i1 += 1;
            continue;
        }

        let y1 = storage1.scanline_y(i1);
        let y2 = storage2.scanline_y(i2);

        if y1 < y2 {
            // Scanline only in storage1
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::AMinusB => {
                    emit_scanline_from_storage(storage1, i1, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i1 += 1;
        } else if y2 < y1 {
            // Scanline only in storage2
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::BMinusA => {
                    emit_scanline_from_storage(storage2, i2, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i2 += 1;
        } else {
            // Same Y — combine the two scanlines
            combine_scanlines_aa(op, storage1, i1, storage2, i2, sl, result, min_x, max_x);
            i1 += 1;
            i2 += 1;
        }
    }
}

/// Copy all scanlines from a storage into the result.
fn copy_storage_aa(
    src: &ScanlineStorageAa,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageAa,
    min_x: i32,
    max_x: i32,
) {
    for i in 0..src.num_scanlines() {
        emit_scanline_from_storage(src, i, sl, result, min_x, max_x);
    }
}

/// Emit a single scanline from storage into the result.
fn emit_scanline_from_storage(
    src: &ScanlineStorageAa,
    sl_idx: usize,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageAa,
    min_x: i32,
    max_x: i32,
) {
    let y = src.scanline_y(sl_idx);
    sl.reset(min_x, max_x);
    sl.reset_spans();

    for sp in src.embedded_spans(sl_idx) {
        if sp.len < 0 {
            // Solid span
            sl.add_span(sp.x, (-sp.len) as u32, sp.covers[0] as u32);
        } else {
            for j in 0..sp.len as usize {
                sl.add_cell(sp.x + j as i32, sp.covers[j] as u32);
            }
        }
    }
    sl.finalize(y);
    if sl.num_spans() > 0 {
        result.render_scanline_u8(sl);
    }
}

/// Combine two scanlines at the same Y coordinate.
fn combine_scanlines_aa(
    op: SBoolOp,
    storage1: &ScanlineStorageAa,
    sl_idx1: usize,
    storage2: &ScanlineStorageAa,
    sl_idx2: usize,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageAa,
    min_x: i32,
    max_x: i32,
) {
    let y = storage1.scanline_y(sl_idx1);
    sl.reset(min_x, max_x);
    sl.reset_spans();

    // Collect spans from both scanlines into flat coverage arrays, then combine.
    // This is simpler than the C++ span-walking approach and correct for all ops.
    let width = (max_x - min_x + 1) as usize;
    let mut cov1 = vec![0u8; width];
    let mut cov2 = vec![0u8; width];

    // Fill coverage array 1
    for sp in storage1.embedded_spans(sl_idx1) {
        let abs_len = sp.abs_len();
        for j in 0..abs_len {
            let x = sp.x + j;
            if x >= min_x && x <= max_x {
                cov1[(x - min_x) as usize] = sp.cover_at(j as usize);
            }
        }
    }

    // Fill coverage array 2
    for sp in storage2.embedded_spans(sl_idx2) {
        let abs_len = sp.abs_len();
        for j in 0..abs_len {
            let x = sp.x + j;
            if x >= min_x && x <= max_x {
                cov2[(x - min_x) as usize] = sp.cover_at(j as usize);
            }
        }
    }

    // Combine and emit
    for i in 0..width {
        let c1 = cov1[i] as u32;
        let c2 = cov2[i] as u32;
        let result_cover = match op {
            SBoolOp::Or => {
                if c1 > 0 && c2 > 0 {
                    unite_covers(c1, c2)
                } else if c1 > 0 {
                    c1 as u8
                } else {
                    c2 as u8
                }
            }
            SBoolOp::And => intersect_covers(c1, c2),
            SBoolOp::Xor => {
                if c1 > 0 && c2 > 0 {
                    xor_covers(c1, c2)
                } else if c1 > 0 {
                    c1 as u8
                } else {
                    c2 as u8
                }
            }
            SBoolOp::AMinusB => subtract_covers(c1, c2),
            SBoolOp::BMinusA => subtract_covers(c2, c1),
        };
        if result_cover > 0 {
            sl.add_cell(min_x + i as i32, result_cover as u32);
        }
    }

    sl.finalize(y);
    if sl.num_spans() > 0 {
        result.render_scanline_u8(sl);
    }
}

// ============================================================================
// Shape-level boolean operations (Binary)
// ============================================================================

/// Perform a boolean operation on two rasterized shapes (binary/no AA).
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
    // Render shape 1 into storage1
    storage1.prepare();
    render_to_bin_storage(ras1, sl1, storage1);

    // Render shape 2 into storage2
    storage2.prepare();
    render_to_bin_storage(ras2, sl2, storage2);

    // Combine
    storage_result.prepare();
    sbool_combine_storages_bin(op, storage1, storage2, sl_result, storage_result);
}

/// Render a rasterizer's output into binary storage.
fn render_to_bin_storage(
    ras: &mut RasterizerScanlineAa,
    sl: &mut ScanlineU8,
    storage: &mut ScanlineStorageBin,
) {
    if ras.rewind_scanlines() {
        sl.reset(ras.min_x(), ras.max_x());
        while ras.sweep_scanline(sl) {
            storage.render_scanline_u8(sl);
        }
    }
}

/// Combine two binary scanline storages.
pub fn sbool_combine_storages_bin(
    op: SBoolOp,
    storage1: &ScanlineStorageBin,
    storage2: &ScanlineStorageBin,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageBin,
) {
    let n1 = storage1.num_scanlines();
    let n2 = storage2.num_scanlines();

    if n1 == 0 && n2 == 0 {
        return;
    }

    let min_x = if n1 > 0 && n2 > 0 {
        storage1.min_x().min(storage2.min_x())
    } else if n1 > 0 {
        storage1.min_x()
    } else {
        storage2.min_x()
    };
    let max_x = if n1 > 0 && n2 > 0 {
        storage1.max_x().max(storage2.max_x())
    } else if n1 > 0 {
        storage1.max_x()
    } else {
        storage2.max_x()
    };

    match op {
        SBoolOp::Or | SBoolOp::Xor => {
            if n1 == 0 {
                copy_storage_bin(storage2, sl, result, min_x, max_x);
                return;
            }
            if n2 == 0 {
                copy_storage_bin(storage1, sl, result, min_x, max_x);
                return;
            }
        }
        SBoolOp::And => {
            if n1 == 0 || n2 == 0 {
                return;
            }
        }
        SBoolOp::AMinusB => {
            if n1 == 0 {
                return;
            }
            if n2 == 0 {
                copy_storage_bin(storage1, sl, result, min_x, max_x);
                return;
            }
        }
        SBoolOp::BMinusA => {
            if n2 == 0 {
                return;
            }
            if n1 == 0 {
                copy_storage_bin(storage2, sl, result, min_x, max_x);
                return;
            }
        }
    }

    let mut i1 = 0usize;
    let mut i2 = 0usize;

    while i1 < n1 || i2 < n2 {
        if i1 >= n1 {
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::BMinusA => {
                    emit_scanline_from_bin_storage(storage2, i2, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i2 += 1;
            continue;
        }
        if i2 >= n2 {
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::AMinusB => {
                    emit_scanline_from_bin_storage(storage1, i1, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i1 += 1;
            continue;
        }

        let y1 = storage1.scanline_y(i1);
        let y2 = storage2.scanline_y(i2);

        if y1 < y2 {
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::AMinusB => {
                    emit_scanline_from_bin_storage(storage1, i1, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i1 += 1;
        } else if y2 < y1 {
            match op {
                SBoolOp::Or | SBoolOp::Xor | SBoolOp::BMinusA => {
                    emit_scanline_from_bin_storage(storage2, i2, sl, result, min_x, max_x);
                }
                _ => {}
            }
            i2 += 1;
        } else {
            combine_scanlines_bin(op, storage1, i1, storage2, i2, sl, result, min_x, max_x);
            i1 += 1;
            i2 += 1;
        }
    }
}

fn copy_storage_bin(
    src: &ScanlineStorageBin,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageBin,
    min_x: i32,
    max_x: i32,
) {
    // Re-emit via ScanlineBin
    let mut sl_bin = crate::scanline_bin::ScanlineBin::new();
    for i in 0..src.num_scanlines() {
        let y = src.scanline_y(i);
        sl_bin.reset(min_x, max_x);
        sl_bin.reset_spans();
        for sp in src.embedded_spans(i) {
            sl_bin.add_span(sp.x, sp.len as u32, COVER_FULL);
        }
        sl_bin.finalize(y);
        if sl_bin.num_spans() > 0 {
            result.render_scanline_bin(&sl_bin);
        }
    }
    let _ = sl; // unused but kept for API consistency
}

fn emit_scanline_from_bin_storage(
    src: &ScanlineStorageBin,
    sl_idx: usize,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageBin,
    min_x: i32,
    max_x: i32,
) {
    let y = src.scanline_y(sl_idx);
    let mut sl_bin = crate::scanline_bin::ScanlineBin::new();
    sl_bin.reset(min_x, max_x);
    sl_bin.reset_spans();
    for sp in src.embedded_spans(sl_idx) {
        sl_bin.add_span(sp.x, sp.len as u32, COVER_FULL);
    }
    sl_bin.finalize(y);
    if sl_bin.num_spans() > 0 {
        result.render_scanline_bin(&sl_bin);
    }
    let _ = sl; // unused but kept for API consistency
}

fn combine_scanlines_bin(
    op: SBoolOp,
    storage1: &ScanlineStorageBin,
    sl_idx1: usize,
    storage2: &ScanlineStorageBin,
    sl_idx2: usize,
    sl: &mut ScanlineU8,
    result: &mut ScanlineStorageBin,
    min_x: i32,
    max_x: i32,
) {
    let y = storage1.scanline_y(sl_idx1);
    let width = (max_x - min_x + 1) as usize;
    let mut bits1 = vec![false; width];
    let mut bits2 = vec![false; width];

    // Fill bitmask 1
    for sp in storage1.embedded_spans(sl_idx1) {
        for j in 0..sp.len {
            let x = sp.x + j;
            if x >= min_x && x <= max_x {
                bits1[(x - min_x) as usize] = true;
            }
        }
    }

    // Fill bitmask 2
    for sp in storage2.embedded_spans(sl_idx2) {
        for j in 0..sp.len {
            let x = sp.x + j;
            if x >= min_x && x <= max_x {
                bits2[(x - min_x) as usize] = true;
            }
        }
    }

    // Combine and emit
    let mut sl_bin = crate::scanline_bin::ScanlineBin::new();
    sl_bin.reset(min_x, max_x);
    sl_bin.reset_spans();

    for i in 0..width {
        let in1 = bits1[i];
        let in2 = bits2[i];
        let result_on = match op {
            SBoolOp::Or => in1 || in2,
            SBoolOp::And => in1 && in2,
            SBoolOp::Xor => in1 ^ in2,
            SBoolOp::AMinusB => in1 && !in2,
            SBoolOp::BMinusA => in2 && !in1,
        };
        if result_on {
            sl_bin.add_cell(min_x + i as i32, COVER_FULL);
        }
    }

    sl_bin.finalize(y);
    if sl_bin.num_spans() > 0 {
        result.render_scanline_bin(&sl_bin);
    }
    let _ = sl; // API consistency
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create AA storage with a horizontal band from x1..x2 at rows y1..y2
    fn make_aa_rect(
        x1: i32,
        x2: i32,
        y1: i32,
        y2: i32,
        cover: u8,
    ) -> ScanlineStorageAa {
        let mut storage = ScanlineStorageAa::new();
        storage.prepare();
        let mut sl = ScanlineU8::new();
        for y in y1..=y2 {
            sl.reset(x1, x2);
            sl.reset_spans();
            for x in x1..=x2 {
                sl.add_cell(x, cover as u32);
            }
            sl.finalize(y);
            storage.render_scanline_u8(&sl);
        }
        storage
    }

    // Helper: create binary storage with a rect
    fn make_bin_rect(x1: i32, x2: i32, y1: i32, y2: i32) -> ScanlineStorageBin {
        let mut storage = ScanlineStorageBin::new();
        storage.prepare();
        let mut sl = crate::scanline_bin::ScanlineBin::new();
        for y in y1..=y2 {
            sl.reset(x1, x2);
            sl.reset_spans();
            sl.add_span(x1, (x2 - x1 + 1) as u32, 255);
            sl.finalize(y);
            storage.render_scanline_bin(&sl);
        }
        storage
    }

    #[test]
    fn test_aa_intersection() {
        // Two overlapping rectangles
        let s1 = make_aa_rect(0, 19, 0, 19, 255);
        let s2 = make_aa_rect(10, 29, 10, 29, 255);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::And, &s1, &s2, &mut sl, &mut result);

        // Intersection should be [10..19, 10..19]
        assert_eq!(result.num_scanlines(), 10);
        assert_eq!(result.min_x(), 10);
        assert_eq!(result.max_x(), 19);
        assert_eq!(result.min_y(), 10);
        assert_eq!(result.max_y(), 19);
    }

    #[test]
    fn test_aa_union() {
        let s1 = make_aa_rect(0, 9, 0, 4, 255);
        let s2 = make_aa_rect(5, 14, 0, 4, 255);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::Or, &s1, &s2, &mut sl, &mut result);

        // Union should span [0..14, 0..4]
        assert_eq!(result.num_scanlines(), 5);
        assert_eq!(result.min_x(), 0);
        assert_eq!(result.max_x(), 14);
    }

    #[test]
    fn test_aa_subtract() {
        let s1 = make_aa_rect(0, 19, 0, 19, 255);
        let s2 = make_aa_rect(10, 29, 10, 29, 255);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::AMinusB, &s1, &s2, &mut sl, &mut result);

        // A-B: scanlines 0..9 fully present, scanlines 10..19 only x=0..9
        assert_eq!(result.num_scanlines(), 20);
    }

    #[test]
    fn test_aa_xor() {
        let s1 = make_aa_rect(0, 9, 0, 4, 200);
        let s2 = make_aa_rect(5, 14, 0, 4, 200);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::Xor, &s1, &s2, &mut sl, &mut result);

        // XOR: both have coverage → reduced; only one → full
        assert_eq!(result.num_scanlines(), 5);
    }

    #[test]
    fn test_aa_intersect_no_overlap() {
        let s1 = make_aa_rect(0, 9, 0, 4, 255);
        let s2 = make_aa_rect(20, 29, 10, 14, 255);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::And, &s1, &s2, &mut sl, &mut result);

        // No overlap → empty intersection
        assert_eq!(result.num_scanlines(), 0);
    }

    #[test]
    fn test_aa_union_disjoint() {
        let s1 = make_aa_rect(0, 4, 0, 2, 255);
        let s2 = make_aa_rect(10, 14, 5, 7, 255);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::Or, &s1, &s2, &mut sl, &mut result);

        // 3 + 3 scanlines (no overlap in Y)
        assert_eq!(result.num_scanlines(), 6);
    }

    #[test]
    fn test_aa_empty_operand() {
        let s1 = make_aa_rect(0, 9, 0, 4, 255);
        let s2 = ScanlineStorageAa::new();

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();

        // OR with empty → copy of s1
        sbool_combine_storages_aa(SBoolOp::Or, &s1, &s2, &mut sl, &mut result);
        assert_eq!(result.num_scanlines(), 5);

        // AND with empty → empty
        result.prepare();
        sbool_combine_storages_aa(SBoolOp::And, &s1, &s2, &mut sl, &mut result);
        assert_eq!(result.num_scanlines(), 0);
    }

    #[test]
    fn test_aa_semi_transparent_intersection() {
        // Two semi-transparent rectangles
        let s1 = make_aa_rect(0, 9, 0, 0, 128);
        let s2 = make_aa_rect(0, 9, 0, 0, 128);

        let mut result = ScanlineStorageAa::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_aa(SBoolOp::And, &s1, &s2, &mut sl, &mut result);

        // Intersection of two 128-cover spans → (128*128 + 127) >> 8 = 64
        assert_eq!(result.num_scanlines(), 1);

        // Read back the coverage
        assert!(result.rewind_scanlines());
        let mut sl_out = ScanlineU8::new();
        sl_out.reset(0, 9);
        assert!(result.sweep_scanline(&mut sl_out));
        let spans = sl_out.begin();
        let covers = sl_out.covers();
        // Check one pixel's coverage
        let cover = covers[spans[0].cover_offset];
        assert_eq!(cover, 64);
    }

    #[test]
    fn test_bin_intersection() {
        let s1 = make_bin_rect(0, 19, 0, 19);
        let s2 = make_bin_rect(10, 29, 10, 29);

        let mut result = ScanlineStorageBin::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_bin(SBoolOp::And, &s1, &s2, &mut sl, &mut result);

        assert_eq!(result.num_scanlines(), 10);
    }

    #[test]
    fn test_bin_union() {
        let s1 = make_bin_rect(0, 4, 0, 2);
        let s2 = make_bin_rect(10, 14, 5, 7);

        let mut result = ScanlineStorageBin::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_bin(SBoolOp::Or, &s1, &s2, &mut sl, &mut result);

        assert_eq!(result.num_scanlines(), 6);
    }

    #[test]
    fn test_bin_xor() {
        let s1 = make_bin_rect(0, 9, 0, 4);
        let s2 = make_bin_rect(5, 14, 0, 4);

        let mut result = ScanlineStorageBin::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_bin(SBoolOp::Xor, &s1, &s2, &mut sl, &mut result);

        assert_eq!(result.num_scanlines(), 5);
        // Each scanline should have only the non-overlapping parts: [0..4] and [10..14]
    }

    #[test]
    fn test_bin_subtract() {
        let s1 = make_bin_rect(0, 19, 0, 19);
        let s2 = make_bin_rect(10, 29, 10, 29);

        let mut result = ScanlineStorageBin::new();
        let mut sl = ScanlineU8::new();
        sbool_combine_storages_bin(SBoolOp::AMinusB, &s1, &s2, &mut sl, &mut result);

        // All 20 scanlines from s1 should be present (rows 0..19)
        // rows 0..9: full width, rows 10..19: only x=0..9
        assert_eq!(result.num_scanlines(), 20);
    }

    #[test]
    fn test_cover_math_intersect() {
        assert_eq!(intersect_covers(255, 255), 255); // full*full → full
        assert_eq!(intersect_covers(0, 255), 0);
        assert_eq!(intersect_covers(128, 128), 64); // 16384 >> 8 = 64
    }

    #[test]
    fn test_cover_math_unite() {
        assert_eq!(unite_covers(255, 255), 255);
        assert_eq!(unite_covers(0, 0), 0);
        assert_eq!(unite_covers(0, 255), 255); // full*full special case
        // 128 OR 128: 65025 - 127*127 = 48896, >> 8 = 191
        assert_eq!(unite_covers(128, 128), 191);
    }

    #[test]
    fn test_cover_math_subtract() {
        assert_eq!(subtract_covers(255, 0), 255); // 255*255 == full*full → 255
        assert_eq!(subtract_covers(255, 255), 0);
        assert_eq!(subtract_covers(0, 255), 0);
    }

    #[test]
    fn test_cover_math_xor() {
        assert_eq!(xor_covers(0, 0), 0);
        assert_eq!(xor_covers(255, 0), 255);
        assert_eq!(xor_covers(0, 255), 255);
        assert_eq!(xor_covers(255, 255), 0); // 510 > 255, 255+255-510 = 0
    }
}

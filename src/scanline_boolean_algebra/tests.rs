//! Unit tests of the boolean algebra: the storage-based API on rectangles and the cover
//! formulas the span functors use.

use super::spans::{intersect_covers, scale_cover, subtract_covers_raw, unite_covers};
use super::*;
use crate::scanline_storage_aa::ScanlineStorageAa;
use crate::scanline_storage_bin::ScanlineStorageBin;
use crate::scanline_u::ScanlineU8;

/// `sbool_subtract_spans_aa`'s cover for a pixel: the raw product, scaled.
fn subtract_covers(a: u32, b: u32) -> u32 {
    scale_cover(subtract_covers_raw(a, b))
}

// Helper: create AA storage with a horizontal band from x1..x2 at rows y1..y2
fn make_aa_rect(x1: i32, x2: i32, y1: i32, y2: i32, cover: u8) -> ScanlineStorageAa {
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
fn test_cover_math_xor_linear() {
    assert_eq!(SboolXorFormulaLinear::calculate(0, 0), 0);
    assert_eq!(SboolXorFormulaLinear::calculate(255, 0), 255);
    assert_eq!(SboolXorFormulaLinear::calculate(0, 255), 255);
    assert_eq!(SboolXorFormulaLinear::calculate(255, 255), 0); // 510 > 255, 255+255-510 = 0
}

#[test]
fn test_cover_math_xor_saddle() {
    // Note: saddle formula has small fixed-point imprecision at (0,0)
    // returning 3 instead of 0, which is expected and doesn't affect
    // rendering since the combine functor is only called when both covers > 0.
    assert!(SboolXorFormulaSaddle::calculate(0, 0) <= 5); // near-zero
    assert_eq!(SboolXorFormulaSaddle::calculate(255, 255), 0);
    // Saddle formula: 1-((1-a+ab)(1-b+ab)) in [0,255] space
    // With partial coverage, saddle gives different result than linear
    let saddle = SboolXorFormulaSaddle::calculate(128, 128);
    assert!(saddle > 0 && saddle < 255);
    // Verify saddle is different from linear for partial coverage
    let linear = SboolXorFormulaLinear::calculate(128, 128);
    assert_ne!(saddle, linear);
}

#[test]
fn test_cover_math_xor_abs_diff() {
    assert_eq!(SboolXorFormulaAbsDiff::calculate(0, 0), 0);
    assert_eq!(SboolXorFormulaAbsDiff::calculate(255, 255), 0);
    assert_eq!(SboolXorFormulaAbsDiff::calculate(200, 100), 100);
    assert_eq!(SboolXorFormulaAbsDiff::calculate(100, 200), 100);
}

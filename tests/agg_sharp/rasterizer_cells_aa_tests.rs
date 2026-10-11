//! Port of agg-sharp `RasterizerCellsAaTests.cs`: the cell accumulation invariant of
//! `RasterizerCellsAa`. For a single `line()` call the sum of `cover` over every emitted cell
//! must telescope to exactly `y2 - y1` in subpixel units. That one scalar catches a segment
//! traversed more than once (a missing `return` after recursive subdivision) and 32-bit
//! overflow in the subdivision guard or midpoint; agg-rust does this arithmetic in i64.

use agg_rust::rasterizer_cells_aa::RasterizerCellsAa;

/// 16384 << 8: the point at which line() must subdivide rather than traverse.
const DX_LIMIT: i32 = 16384 << 8;

/// Baseline: an ordinary short segment already satisfies the cover-sum invariant, in both y
/// directions. Guards the helper itself, so a failure in the other tests means the rasterizer,
/// not the measurement.
#[test]
fn short_segment_cover_sum() {
    assert_eq!(cover_for_line(0, 0, 100 << 8, 10 << 8), 10 << 8);
    assert_eq!(cover_for_line(0, 10 << 8, 100 << 8, 0), -(10 << 8));
}

/// A segment wider than the subdivision limit must be rendered by the two recursive halves
/// only. Without the `return` after subdividing, the full traversal runs as well and the cover
/// comes out a multiple of the true value.
#[test]
fn long_segment_cover_is_not_doubled() {
    assert_eq!(cover_for_line(0, 0, 3 * DX_LIMIT, 10 << 8), 10 << 8);
}

/// Pins the `>=` in the subdivision guard: a `dx` of exactly the limit subdivides, one subunit
/// less traverses directly, and both land on the same cover sum.
#[test]
fn subdivision_boundary_cover_sum() {
    assert_eq!(cover_for_line(0, 0, DX_LIMIT, 10 << 8), 10 << 8);
    assert_eq!(cover_for_line(0, 0, DX_LIMIT - 1, 10 << 8), 10 << 8);
}

/// The widest representable segment, in both x directions. Computing `dx` or the subdivision
/// midpoint in 32 bits wraps here, which skips subdivision and walks a wrapped cell range (a
/// regression shows up as a hang, not a failed assertion).
#[test]
fn extreme_coordinates_do_not_overflow() {
    assert_eq!(cover_for_line(i32::MIN, 0, i32::MAX, 256), 256);
    assert_eq!(cover_for_line(i32::MAX, 0, i32::MIN, 256), 256);
}

fn cover_for_line(x1: i32, y1: i32, x2: i32, y2: i32) -> i64 {
    let mut ras = RasterizerCellsAa::new();
    ras.line(x1, y1, x2, y2);
    ras.sort_cells();
    total_cover(&ras)
}

/// Sums `cover` over every sorted cell. Must be called after `sort_cells()`, which also flushes
/// the in-progress cell.
fn total_cover(ras: &RasterizerCellsAa) -> i64 {
    if ras.total_cells() == 0 {
        // No cells were emitted, so min_y/max_y are still at their sentinel (min_y > max_y).
        return 0;
    }

    let mut total = 0i64;
    for y in ras.min_y()..=ras.max_y() {
        // scanline_cells takes the row as u32 and casts it back to i32, so a negative row
        // round-trips unchanged.
        for &idx in ras.scanline_cells(y as u32) {
            total += ras.cell(idx).cover as i64;
        }
    }

    total
}

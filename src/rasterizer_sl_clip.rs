//! Rasterizer scanline clipping policies.
//!
//! Port of `agg_rasterizer_sl_clip.h` — coordinate conversion (double → 24.8
//! fixed-point) and optional viewport clipping for the scanline rasterizer.
//!
//! Provides two concrete clipper types:
//! - `RasterizerSlClipInt` — default, with viewport clipping
//! - `RasterizerSlNoClip` — passthrough, no clipping
//!
//! The clip interpolation runs in `i64` where C++ overflows `int` on
//! coordinates far outside the 24.8 range (see `interpolate`).

use crate::basics::{iround, Rect, POLY_SUBPIXEL_SCALE};
use crate::clip_liang_barsky::{clipping_flags, clipping_flags_y};
use crate::rasterizer_cells_aa::RasterizerCellsAa;

// ============================================================================
// Coordinate conversion helpers (port of ras_conv_int)
// ============================================================================

/// Convert double to 24.8 fixed-point (upscale).
#[inline]
fn upscale(v: f64) -> i32 {
    iround(v * POLY_SUBPIXEL_SCALE as f64)
}

/// Integer coordinate to subpixel X (identity for int conv).
#[inline]
fn xi(v: i32) -> i32 {
    v
}

/// Integer coordinate to subpixel Y (identity for int conv).
#[inline]
fn yi(v: i32) -> i32 {
    v
}

/// Mul-div for integer coordinates: round(a * b / c), rounding half away
/// from zero like `iround`, in `i64` (see `interpolate`).
#[inline]
fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    let v = a as f64 * b as f64 / c as f64;
    if v < 0.0 {
        (v - 0.5) as i64
    } else {
        (v + 0.5) as i64
    }
}

/// The coordinate `q` where the segment (a1, q1)-(a2, q2) crosses
/// `a = bound`: C++'s `q1 + mul_div(bound - a1, q2 - q1, a2 - a1)`.
///
/// Divergence from C++: `agg_rasterizer_sl_clip.h` evaluates those
/// differences and the sum in `int`. A coordinate far outside the 24.8
/// fixed-point range (1e12 pixels, say) upscales to a saturated
/// `i32::MIN`/`MAX`, and `x2 - x1` then overflows: undefined behaviour in
/// C++, a panic in an overflow-checked Rust build, and an answer right only
/// by luck in a wrapping one. Everything here is computed in `i64`, where
/// the differences of two `i32`s cannot overflow. For input whose
/// differences fit in `i32` the result is identical to C++'s (the `f64`
/// product and quotient see the same exact integers). The crossing lies
/// between `q1` and `q2`, so it fits in `i32`; the clamp only guards
/// against `f64` rounding at the extremes.
#[inline]
fn interpolate(bound: i32, a1: i32, a2: i32, q1: i32, q2: i32) -> i32 {
    let d = mul_div(
        bound as i64 - a1 as i64,
        q2 as i64 - q1 as i64,
        a2 as i64 - a1 as i64,
    );
    (q1 as i64 + d).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

// ============================================================================
// RasterizerSlClipInt — clipping policy with viewport clipping
// ============================================================================

/// Scanline rasterizer clipping policy that clips line segments against
/// a viewport rectangle, then converts to 24.8 fixed-point coordinates.
///
/// Port of C++ `rasterizer_sl_clip<ras_conv_int>`.
pub struct RasterizerSlClipInt {
    clip_box: Rect<i32>,
    x1: i32,
    y1: i32,
    f1: u32,
    clipping: bool,
}

impl RasterizerSlClipInt {
    pub fn new() -> Self {
        Self {
            clip_box: Rect::new(0, 0, 0, 0),
            x1: 0,
            y1: 0,
            f1: 0,
            clipping: false,
        }
    }

    /// Disable clipping.
    pub fn reset_clipping(&mut self) {
        self.clipping = false;
    }

    /// Set the clipping rectangle in 24.8 fixed-point coordinates.
    pub fn clip_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.clip_box = Rect::new(x1, y1, x2, y2);
        self.clip_box.normalize();
        self.clipping = true;
    }

    /// Record a move_to in 24.8 fixed-point coordinates.
    pub fn move_to(&mut self, x1: i32, y1: i32) {
        self.x1 = x1;
        self.y1 = y1;
        if self.clipping {
            self.f1 = clipping_flags(x1, y1, &self.clip_box);
        }
    }

    /// Record a move_to from double coordinates (upscales to 24.8).
    pub fn move_to_d(&mut self, x: f64, y: f64) {
        self.move_to(upscale(x), upscale(y));
    }

    /// Clip and emit a line segment to the cell rasterizer.
    ///
    /// Implements the 13-case clipping switch from C++ `line_to`.
    pub fn line_to(&mut self, ras: &mut RasterizerCellsAa, x2: i32, y2: i32) {
        if self.clipping {
            let f2 = clipping_flags(x2, y2, &self.clip_box);

            // Both endpoints invisible by Y on the same side → skip
            if (self.f1 & 10) == (f2 & 10) && (self.f1 & 10) != 0 {
                self.x1 = x2;
                self.y1 = y2;
                self.f1 = f2;
                return;
            }

            let x1 = self.x1;
            let y1 = self.y1;
            let f1 = self.f1;

            match ((f1 & 5) << 1) | (f2 & 5) {
                0 => {
                    // Visible by X
                    self.line_clip_y(ras, x1, y1, x2, y2, f1, f2);
                }
                1 => {
                    // x2 > clip.x2
                    let y3 = interpolate(self.clip_box.x2, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, x1, y1, self.clip_box.x2, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x2, y3, self.clip_box.x2, y2, f3, f2);
                }
                2 => {
                    // x1 > clip.x2
                    let y3 = interpolate(self.clip_box.x2, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x2, y1, self.clip_box.x2, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x2, y3, x2, y2, f3, f2);
                }
                3 => {
                    // x1 > clip.x2 && x2 > clip.x2
                    self.line_clip_y(ras, self.clip_box.x2, y1, self.clip_box.x2, y2, f1, f2);
                }
                4 => {
                    // x2 < clip.x1
                    let y3 = interpolate(self.clip_box.x1, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, x1, y1, self.clip_box.x1, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x1, y3, self.clip_box.x1, y2, f3, f2);
                }
                6 => {
                    // x1 > clip.x2 && x2 < clip.x1
                    let y3 = interpolate(self.clip_box.x2, x1, x2, y1, y2);
                    let y4 = interpolate(self.clip_box.x1, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    let f4 = clipping_flags_y(y4, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x2, y1, self.clip_box.x2, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x2, y3, self.clip_box.x1, y4, f3, f4);
                    self.line_clip_y(ras, self.clip_box.x1, y4, self.clip_box.x1, y2, f4, f2);
                }
                8 => {
                    // x1 < clip.x1
                    let y3 = interpolate(self.clip_box.x1, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x1, y1, self.clip_box.x1, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x1, y3, x2, y2, f3, f2);
                }
                9 => {
                    // x1 < clip.x1 && x2 > clip.x2
                    let y3 = interpolate(self.clip_box.x1, x1, x2, y1, y2);
                    let y4 = interpolate(self.clip_box.x2, x1, x2, y1, y2);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    let f4 = clipping_flags_y(y4, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x1, y1, self.clip_box.x1, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x1, y3, self.clip_box.x2, y4, f3, f4);
                    self.line_clip_y(ras, self.clip_box.x2, y4, self.clip_box.x2, y2, f4, f2);
                }
                12 => {
                    // x1 < clip.x1 && x2 < clip.x1
                    self.line_clip_y(ras, self.clip_box.x1, y1, self.clip_box.x1, y2, f1, f2);
                }
                _ => {
                    // cases 5, 7, 10, 11 — cannot happen with valid clipping flags
                }
            }
            self.f1 = f2;
        } else {
            ras.line(xi(self.x1), yi(self.y1), xi(x2), yi(y2));
        }
        self.x1 = x2;
        self.y1 = y2;
    }

    /// Emit a line_to from double coordinates (upscales to 24.8).
    pub fn line_to_d(&mut self, ras: &mut RasterizerCellsAa, x: f64, y: f64) {
        self.line_to(ras, upscale(x), upscale(y));
    }

    /// Clip a line segment in Y and emit to the rasterizer.
    #[allow(clippy::too_many_arguments)]
    fn line_clip_y(
        &self,
        ras: &mut RasterizerCellsAa,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        f1: u32,
        f2: u32,
    ) {
        let f1 = f1 & 10;
        let f2 = f2 & 10;

        if (f1 | f2) == 0 {
            // Fully visible
            ras.line(xi(x1), yi(y1), xi(x2), yi(y2));
        } else if f1 != f2 {
            // Partially visible — clip in Y
            let mut tx1 = x1;
            let mut ty1 = y1;
            let mut tx2 = x2;
            let mut ty2 = y2;

            if f1 & 8 != 0 {
                // y1 < clip.y1
                tx1 = interpolate(self.clip_box.y1, y1, y2, x1, x2);
                ty1 = self.clip_box.y1;
            }

            if f1 & 2 != 0 {
                // y1 > clip.y2
                tx1 = interpolate(self.clip_box.y2, y1, y2, x1, x2);
                ty1 = self.clip_box.y2;
            }

            if f2 & 8 != 0 {
                // y2 < clip.y1
                tx2 = interpolate(self.clip_box.y1, y1, y2, x1, x2);
                ty2 = self.clip_box.y1;
            }

            if f2 & 2 != 0 {
                // y2 > clip.y2
                tx2 = interpolate(self.clip_box.y2, y1, y2, x1, x2);
                ty2 = self.clip_box.y2;
            }

            ras.line(xi(tx1), yi(ty1), xi(tx2), yi(ty2));
        }
        // else: f1 == f2, both invisible by Y on same side → skip
    }
}

impl Default for RasterizerSlClipInt {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RasterizerSlNoClip — passthrough (no clipping)
// ============================================================================

/// Scanline rasterizer policy that performs no clipping, just coordinate
/// conversion (double → 24.8 fixed-point) and direct passthrough to the
/// cell rasterizer.
///
/// Port of C++ `rasterizer_sl_no_clip`.
pub struct RasterizerSlNoClip {
    x1: i32,
    y1: i32,
}

impl RasterizerSlNoClip {
    pub fn new() -> Self {
        Self { x1: 0, y1: 0 }
    }

    pub fn reset_clipping(&mut self) {}

    pub fn clip_box(&mut self, _x1: i32, _y1: i32, _x2: i32, _y2: i32) {}

    pub fn move_to(&mut self, x1: i32, y1: i32) {
        self.x1 = x1;
        self.y1 = y1;
    }

    pub fn move_to_d(&mut self, x: f64, y: f64) {
        self.move_to(upscale(x), upscale(y));
    }

    pub fn line_to(&mut self, ras: &mut RasterizerCellsAa, x2: i32, y2: i32) {
        ras.line(self.x1, self.y1, x2, y2);
        self.x1 = x2;
        self.y1 = y2;
    }

    pub fn line_to_d(&mut self, ras: &mut RasterizerCellsAa, x: f64, y: f64) {
        self.line_to(ras, upscale(x), upscale(y));
    }
}

impl Default for RasterizerSlNoClip {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Public helpers
// ============================================================================

/// Upscale a double coordinate to 24.8 fixed-point.
/// Public wrapper around the internal `upscale` function.
pub fn poly_coord(v: f64) -> i32 {
    upscale(v)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::POLY_SUBPIXEL_SCALE;

    #[test]
    fn test_upscale() {
        assert_eq!(upscale(0.0), 0);
        assert_eq!(upscale(1.0), POLY_SUBPIXEL_SCALE as i32);
        assert_eq!(upscale(10.5), iround(10.5 * POLY_SUBPIXEL_SCALE as f64));
        assert_eq!(upscale(-1.0), -(POLY_SUBPIXEL_SCALE as i32));
    }

    #[test]
    fn test_mul_div() {
        assert_eq!(mul_div(10, 20, 5), 40);
        assert_eq!(mul_div(0, 100, 1), 0);
        assert_eq!(mul_div(7, 3, 2), 11); // round(10.5) = 11
        assert_eq!(mul_div(-7, 3, 2), -11); // half away from zero, as iround
    }

    // ------------------------------------------------------------------
    // RasterizerSlClipInt tests
    // ------------------------------------------------------------------

    #[test]
    fn test_clip_int_new() {
        let clip = RasterizerSlClipInt::new();
        assert!(!clip.clipping);
    }

    #[test]
    fn test_clip_int_no_clip_passthrough() {
        let mut clip = RasterizerSlClipInt::new();
        let mut ras = RasterizerCellsAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;

        clip.move_to(0, 0);
        clip.line_to(&mut ras, 10 * s, 10 * s);
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
    }

    #[test]
    fn test_clip_int_visible_line() {
        let mut clip = RasterizerSlClipInt::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        clip.clip_box(0, 0, 100 * s, 100 * s);

        let mut ras = RasterizerCellsAa::new();
        clip.move_to(10 * s, 10 * s);
        clip.line_to(&mut ras, 50 * s, 50 * s);
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
        assert!(ras.min_x() >= 10);
        assert!(ras.max_x() <= 50);
    }

    #[test]
    fn test_clip_int_fully_clipped_by_y() {
        let mut clip = RasterizerSlClipInt::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        clip.clip_box(0, 10 * s, 100 * s, 90 * s);

        let mut ras = RasterizerCellsAa::new();
        // Line entirely above the clip box
        clip.move_to(10 * s, 0);
        clip.line_to(&mut ras, 50 * s, 5 * s);
        ras.sort_cells();

        assert_eq!(ras.total_cells(), 0);
    }

    #[test]
    fn test_clip_int_clipped_by_x_right() {
        let mut clip = RasterizerSlClipInt::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        clip.clip_box(0, 0, 50 * s, 100 * s);

        let mut ras = RasterizerCellsAa::new();
        clip.move_to(10 * s, 10 * s);
        clip.line_to(&mut ras, 80 * s, 80 * s);
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
        // All cells should be within clip bounds
        for cell in ras.cells() {
            assert!(cell.x <= 50, "Cell x={} exceeds clip x2=50", cell.x);
        }
    }

    #[test]
    fn test_clip_int_reset_clipping() {
        let mut clip = RasterizerSlClipInt::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        clip.clip_box(0, 0, 10 * s, 10 * s);
        assert!(clip.clipping);
        clip.reset_clipping();
        assert!(!clip.clipping);
    }

    #[test]
    fn test_clip_int_move_to_d() {
        let mut clip = RasterizerSlClipInt::new();
        clip.move_to_d(10.5, 20.5);
        assert_eq!(clip.x1, upscale(10.5));
        assert_eq!(clip.y1, upscale(20.5));
    }

    #[test]
    fn test_clip_int_line_to_d() {
        let mut clip = RasterizerSlClipInt::new();
        let mut ras = RasterizerCellsAa::new();
        clip.move_to_d(0.0, 0.0);
        clip.line_to_d(&mut ras, 10.0, 10.0);
        ras.sort_cells();
        assert!(ras.total_cells() > 0);
    }

    // ------------------------------------------------------------------
    // RasterizerSlNoClip tests
    // ------------------------------------------------------------------

    #[test]
    fn test_no_clip_passthrough() {
        let mut clip = RasterizerSlNoClip::new();
        let mut ras = RasterizerCellsAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;

        clip.move_to(0, 0);
        clip.line_to(&mut ras, 10 * s, 10 * s);
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
    }

    #[test]
    fn test_no_clip_double_api() {
        let mut clip = RasterizerSlNoClip::new();
        let mut ras = RasterizerCellsAa::new();

        clip.move_to_d(0.0, 0.0);
        clip.line_to_d(&mut ras, 5.0, 5.0);
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
    }

    /// Sorted (x, y, cover, area) of every cell, for comparing two outlines.
    fn cell_list(ras: &mut RasterizerCellsAa) -> Vec<(i32, i32, i32, i32)> {
        ras.sort_cells();
        let mut cells: Vec<_> = ras
            .cells()
            .iter()
            .map(|c| (c.x, c.y, c.cover, c.area))
            .collect();
        cells.sort_unstable();
        cells
    }

    /// Feeds a closed polygon in double coordinates through a clipper with
    /// a 100x100 pixel clip box and returns its cells.
    fn clipped_polygon_cells(points: &[(f64, f64)]) -> Vec<(i32, i32, i32, i32)> {
        let s = POLY_SUBPIXEL_SCALE as i32;
        let mut clip = RasterizerSlClipInt::new();
        clip.clip_box(0, 0, 100 * s, 100 * s);
        let mut ras = RasterizerCellsAa::new();
        clip.move_to_d(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            clip.line_to_d(&mut ras, x, y);
        }
        clip.line_to_d(&mut ras, points[0].0, points[0].1);
        cell_list(&mut ras)
    }

    /// Divergence from C++: `agg_rasterizer_sl_clip.h` computes `x2 - x1`,
    /// `y2 - y1` and `clip - x1` in `int`. A coordinate far outside the
    /// 24.8 range (1e12 here) saturates to `i32::MIN`/`MAX` on upscale, and
    /// those differences overflow (undefined behaviour in C++, a panic in
    /// an overflow-checked Rust build). The clip computes them in `i64`, so
    /// extreme coordinates clip to the same cells as in-range ones.
    #[test]
    fn test_clip_int_extreme_coordinates_clip_like_in_range_ones() {
        let huge = 1e12;
        // Every clip case with both x endpoints outside (9, 6) and the
        // y clip on both ends: a diagonal through the box, both directions.
        let diag_huge = clipped_polygon_cells(&[(-huge, -huge), (huge, huge), (huge, -huge)]);
        let diag_in_range =
            clipped_polygon_cells(&[(-50.0, -50.0), (150.0, 150.0), (150.0, -50.0)]);
        assert!(!diag_in_range.is_empty());
        assert_eq!(diag_huge, diag_in_range);

        // A rectangle covering the whole box.
        let rect_huge =
            clipped_polygon_cells(&[(-huge, -huge), (huge, -huge), (huge, huge), (-huge, huge)]);
        let rect_in_range =
            clipped_polygon_cells(&[(-8.0, -8.0), (108.0, -8.0), (108.0, 108.0), (-8.0, 108.0)]);
        assert!(!rect_in_range.is_empty());
        assert_eq!(rect_huge, rect_in_range);

        // One edge running far off the right (cases 1 and 2): the clipped
        // y at the box edge is exact for a horizontal band.
        let band_huge =
            clipped_polygon_cells(&[(10.0, 10.0), (huge, 10.0), (huge, 20.0), (10.0, 20.0)]);
        let band_in_range =
            clipped_polygon_cells(&[(10.0, 10.0), (200.0, 10.0), (200.0, 20.0), (10.0, 20.0)]);
        assert!(!band_in_range.is_empty());
        assert_eq!(band_huge, band_in_range);
    }

    /// The interpolated y of a segment from `i32::MIN` to `i32::MAX` in both
    /// axes, clipped at x = 0, is 0 (the midpoint), not an `i32` wrap.
    #[test]
    fn test_clip_int_saturated_segment_interpolates_in_i64() {
        let s = POLY_SUBPIXEL_SCALE as i32;
        let mut clip = RasterizerSlClipInt::new();
        clip.clip_box(0, 0, 100 * s, 100 * s);
        let mut ras = RasterizerCellsAa::new();
        clip.move_to(i32::MIN, i32::MIN);
        clip.line_to(&mut ras, i32::MAX, i32::MAX);
        let mut expected = RasterizerCellsAa::new();
        expected.line(0, 0, 100 * s, 100 * s);
        assert_eq!(cell_list(&mut ras), cell_list(&mut expected));
    }

    #[test]
    fn test_poly_coord() {
        assert_eq!(poly_coord(1.0), POLY_SUBPIXEL_SCALE as i32);
        assert_eq!(poly_coord(0.0), 0);
    }
}

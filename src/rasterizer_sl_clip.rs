//! Rasterizer scanline clipping policies.
//!
//! Port of `agg_rasterizer_sl_clip.h` — coordinate conversion (double → 24.8
//! fixed-point) and optional viewport clipping for the scanline rasterizer.
//!
//! Provides two concrete clipper types:
//! - `RasterizerSlClipInt` — default, with viewport clipping
//! - `RasterizerSlNoClip` — passthrough, no clipping

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

/// Mul-div for integer coordinates: round(a * b / c).
#[inline]
fn mul_div(a: i32, b: i32, c: i32) -> i32 {
    iround(a as f64 * b as f64 / c as f64)
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
                    let y3 = y1 + mul_div(self.clip_box.x2 - x1, y2 - y1, x2 - x1);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, x1, y1, self.clip_box.x2, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x2, y3, self.clip_box.x2, y2, f3, f2);
                }
                2 => {
                    // x1 > clip.x2
                    let y3 = y1 + mul_div(self.clip_box.x2 - x1, y2 - y1, x2 - x1);
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
                    let y3 = y1 + mul_div(self.clip_box.x1 - x1, y2 - y1, x2 - x1);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, x1, y1, self.clip_box.x1, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x1, y3, self.clip_box.x1, y2, f3, f2);
                }
                6 => {
                    // x1 > clip.x2 && x2 < clip.x1
                    let y3 = y1 + mul_div(self.clip_box.x2 - x1, y2 - y1, x2 - x1);
                    let y4 = y1 + mul_div(self.clip_box.x1 - x1, y2 - y1, x2 - x1);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    let f4 = clipping_flags_y(y4, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x2, y1, self.clip_box.x2, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x2, y3, self.clip_box.x1, y4, f3, f4);
                    self.line_clip_y(ras, self.clip_box.x1, y4, self.clip_box.x1, y2, f4, f2);
                }
                8 => {
                    // x1 < clip.x1
                    let y3 = y1 + mul_div(self.clip_box.x1 - x1, y2 - y1, x2 - x1);
                    let f3 = clipping_flags_y(y3, &self.clip_box);
                    self.line_clip_y(ras, self.clip_box.x1, y1, self.clip_box.x1, y3, f1, f3);
                    self.line_clip_y(ras, self.clip_box.x1, y3, x2, y2, f3, f2);
                }
                9 => {
                    // x1 < clip.x1 && x2 > clip.x2
                    let y3 = y1 + mul_div(self.clip_box.x1 - x1, y2 - y1, x2 - x1);
                    let y4 = y1 + mul_div(self.clip_box.x2 - x1, y2 - y1, x2 - x1);
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
                tx1 = x1 + mul_div(self.clip_box.y1 - y1, x2 - x1, y2 - y1);
                ty1 = self.clip_box.y1;
            }

            if f1 & 2 != 0 {
                // y1 > clip.y2
                tx1 = x1 + mul_div(self.clip_box.y2 - y1, x2 - x1, y2 - y1);
                ty1 = self.clip_box.y2;
            }

            if f2 & 8 != 0 {
                // y2 < clip.y1
                tx2 = x1 + mul_div(self.clip_box.y1 - y1, x2 - x1, y2 - y1);
                ty2 = self.clip_box.y1;
            }

            if f2 & 2 != 0 {
                // y2 > clip.y2
                tx2 = x1 + mul_div(self.clip_box.y2 - y1, x2 - x1, y2 - y1);
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

    #[test]
    fn test_poly_coord() {
        assert_eq!(poly_coord(1.0), POLY_SUBPIXEL_SCALE as i32);
        assert_eq!(poly_coord(0.0), 0);
    }
}

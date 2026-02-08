//! High-level polygon scanline rasterizer with anti-aliasing.
//!
//! Port of `agg_rasterizer_scanline_aa_nogamma.h` — the heart of AGG's
//! rendering pipeline. Accepts polygon contours (move_to/line_to/close),
//! rasterizes them into anti-aliased scanlines, and feeds the scanlines
//! to a renderer.
//!
//! This is the "nogamma" variant that returns raw coverage values (0..255).
//! Gamma correction can be applied in the renderer or pixel format layer.

use crate::basics::{
    is_close, is_move_to, is_stop, is_vertex, FillingRule, VertexSource, POLY_SUBPIXEL_SHIFT,
};
use crate::rasterizer_cells_aa::{RasterizerCellsAa, ScanlineHitTest};
use crate::rasterizer_sl_clip::{poly_coord, RasterizerSlClipInt};

// ============================================================================
// AA scale constants
// ============================================================================

const AA_SHIFT: u32 = 8;
const AA_SCALE: u32 = 1 << AA_SHIFT;
const AA_MASK: u32 = AA_SCALE - 1;
const AA_SCALE2: u32 = AA_SCALE * 2;
const AA_MASK2: u32 = AA_SCALE2 - 1;

// ============================================================================
// Scanline trait — the interface that sweep_scanline feeds data into
// ============================================================================

/// Trait for scanline containers that accumulate coverage data.
///
/// Implementations include `ScanlineU8` (unpacked per-pixel coverage),
/// `ScanlineP8` (packed/RLE), and `ScanlineBin` (binary, no coverage).
pub trait Scanline {
    /// Prepare for a new scanline, clearing all span data.
    fn reset_spans(&mut self);

    /// Add a single cell at position `x` with coverage `cover`.
    fn add_cell(&mut self, x: i32, cover: u32);

    /// Add a horizontal span of `len` pixels starting at `x`, all with `cover`.
    fn add_span(&mut self, x: i32, len: u32, cover: u32);

    /// Finalize the scanline at the given Y coordinate.
    fn finalize(&mut self, y: i32);

    /// Number of spans in this scanline (0 means empty).
    fn num_spans(&self) -> u32;

    /// The Y coordinate of this scanline.
    fn y(&self) -> i32;
}

// ============================================================================
// RasterizerScanlineAa — the high-level polygon rasterizer
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    MoveTo,
    LineTo,
    Closed,
}

/// High-level polygon rasterizer with anti-aliased output.
///
/// Port of C++ `rasterizer_scanline_aa_nogamma<rasterizer_sl_clip_int>`.
///
/// Usage:
/// 1. Optionally set `filling_rule()` and `clip_box()`
/// 2. Define contours with `move_to_d()` / `line_to_d()` or `add_path()`
/// 3. Call `rewind_scanlines()` then repeatedly `sweep_scanline()` to extract AA data
pub struct RasterizerScanlineAa {
    outline: RasterizerCellsAa,
    clipper: RasterizerSlClipInt,
    filling_rule: FillingRule,
    auto_close: bool,
    start_x: i32,
    start_y: i32,
    status: Status,
    scan_y: i32,
}

impl RasterizerScanlineAa {
    pub fn new() -> Self {
        Self {
            outline: RasterizerCellsAa::new(),
            clipper: RasterizerSlClipInt::new(),
            filling_rule: FillingRule::NonZero,
            auto_close: true,
            start_x: 0,
            start_y: 0,
            status: Status::Initial,
            scan_y: 0,
        }
    }

    /// Reset the rasterizer, discarding all polygon data.
    pub fn reset(&mut self) {
        self.outline.reset();
        self.status = Status::Initial;
    }

    /// Set the filling rule (non-zero winding or even-odd).
    pub fn filling_rule(&mut self, rule: FillingRule) {
        self.filling_rule = rule;
    }

    /// Enable or disable automatic polygon closing on move_to.
    pub fn auto_close(&mut self, flag: bool) {
        self.auto_close = flag;
    }

    /// Set the clipping rectangle in floating-point coordinates.
    pub fn clip_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.reset();
        self.clipper.clip_box(
            poly_coord(x1),
            poly_coord(y1),
            poly_coord(x2),
            poly_coord(y2),
        );
    }

    /// Disable clipping.
    pub fn reset_clipping(&mut self) {
        self.reset();
        self.clipper.reset_clipping();
    }

    // ========================================================================
    // Path building
    // ========================================================================

    /// Close the current polygon contour.
    pub fn close_polygon(&mut self) {
        if self.status == Status::LineTo {
            self.clipper
                .line_to(&mut self.outline, self.start_x, self.start_y);
            self.status = Status::Closed;
        }
    }

    /// Move to a new position in 24.8 fixed-point coordinates.
    pub fn move_to(&mut self, x: i32, y: i32) {
        if self.outline.sorted() {
            self.reset();
        }
        if self.auto_close {
            self.close_polygon();
        }
        // For ras_conv_int, downscale is identity
        self.start_x = x;
        self.start_y = y;
        self.clipper.move_to(x, y);
        self.status = Status::MoveTo;
    }

    /// Line to in 24.8 fixed-point coordinates.
    pub fn line_to(&mut self, x: i32, y: i32) {
        self.clipper.line_to(&mut self.outline, x, y);
        self.status = Status::LineTo;
    }

    /// Move to a new position in floating-point coordinates.
    pub fn move_to_d(&mut self, x: f64, y: f64) {
        if self.outline.sorted() {
            self.reset();
        }
        if self.auto_close {
            self.close_polygon();
        }
        let sx = poly_coord(x);
        let sy = poly_coord(y);
        self.start_x = sx;
        self.start_y = sy;
        self.clipper.move_to(sx, sy);
        self.status = Status::MoveTo;
    }

    /// Line to in floating-point coordinates.
    pub fn line_to_d(&mut self, x: f64, y: f64) {
        self.clipper
            .line_to(&mut self.outline, poly_coord(x), poly_coord(y));
        self.status = Status::LineTo;
    }

    /// Add a vertex (dispatches to move_to, line_to, or close based on command).
    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        if is_move_to(cmd) {
            self.move_to_d(x, y);
        } else if is_vertex(cmd) {
            self.line_to_d(x, y);
        } else if is_close(cmd) {
            self.close_polygon();
        }
    }

    /// Add a single edge in 24.8 fixed-point coordinates.
    pub fn edge(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if self.outline.sorted() {
            self.reset();
        }
        self.clipper.move_to(x1, y1);
        self.clipper.line_to(&mut self.outline, x2, y2);
        self.status = Status::MoveTo;
    }

    /// Add a single edge in floating-point coordinates.
    pub fn edge_d(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if self.outline.sorted() {
            self.reset();
        }
        self.clipper.move_to(poly_coord(x1), poly_coord(y1));
        self.clipper
            .line_to(&mut self.outline, poly_coord(x2), poly_coord(y2));
        self.status = Status::MoveTo;
    }

    /// Add all vertices from a vertex source.
    pub fn add_path(&mut self, vs: &mut dyn VertexSource, path_id: u32) {
        let mut x = 0.0;
        let mut y = 0.0;

        vs.rewind(path_id);
        if self.outline.sorted() {
            self.reset();
        }
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.add_vertex(x, y, cmd);
        }
    }

    // ========================================================================
    // Bounding box
    // ========================================================================

    pub fn min_x(&self) -> i32 {
        self.outline.min_x()
    }
    pub fn min_y(&self) -> i32 {
        self.outline.min_y()
    }
    pub fn max_x(&self) -> i32 {
        self.outline.max_x()
    }
    pub fn max_y(&self) -> i32 {
        self.outline.max_y()
    }

    // ========================================================================
    // Scanline sweeping
    // ========================================================================

    /// Sort cells and prepare for scanline sweeping.
    /// Returns `false` if there are no cells (nothing to render).
    pub fn rewind_scanlines(&mut self) -> bool {
        if self.auto_close {
            self.close_polygon();
        }
        self.outline.sort_cells();
        if self.outline.total_cells() == 0 {
            return false;
        }
        self.scan_y = self.outline.min_y();
        true
    }

    /// Navigate to a specific scanline Y (for random access).
    pub fn navigate_scanline(&mut self, y: i32) -> bool {
        if self.auto_close {
            self.close_polygon();
        }
        self.outline.sort_cells();
        if self.outline.total_cells() == 0 || y < self.outline.min_y() || y > self.outline.max_y() {
            return false;
        }
        self.scan_y = y;
        true
    }

    /// Sort cells (explicit sort without starting a sweep).
    pub fn sort(&mut self) {
        if self.auto_close {
            self.close_polygon();
        }
        self.outline.sort_cells();
    }

    /// Calculate alpha (coverage) from accumulated area.
    ///
    /// This is the "nogamma" variant — no gamma LUT, raw coverage.
    #[inline]
    pub fn calculate_alpha(&self, area: i32) -> u32 {
        let mut cover = area >> (POLY_SUBPIXEL_SHIFT * 2 + 1 - AA_SHIFT);

        if cover < 0 {
            cover = -cover;
        }
        if self.filling_rule == FillingRule::EvenOdd {
            cover &= AA_MASK2 as i32;
            if cover > AA_SCALE as i32 {
                cover = AA_SCALE2 as i32 - cover;
            }
        }
        if cover > AA_MASK as i32 {
            cover = AA_MASK as i32;
        }
        cover as u32
    }

    /// Extract the next scanline of anti-aliased coverage data.
    ///
    /// This is THE CORE function of the rasterizer. It iterates sorted cells
    /// for the current scanline Y, accumulates coverage, and feeds spans
    /// to the scanline object.
    ///
    /// Returns `false` when all scanlines have been consumed.
    pub fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool {
        loop {
            if self.scan_y > self.outline.max_y() {
                return false;
            }
            sl.reset_spans();

            let cell_indices = self.outline.scanline_cells(self.scan_y as u32);
            let mut num_cells = cell_indices.len();
            let mut idx = 0;
            let mut cover: i32 = 0;

            while num_cells > 0 {
                let cur_idx = cell_indices[idx];
                let cur_cell = self.outline.cell(cur_idx);
                let x = cur_cell.x;
                let mut area = cur_cell.area;

                cover += cur_cell.cover;

                // Accumulate all cells with the same X
                num_cells -= 1;
                idx += 1;
                while num_cells > 0 {
                    let next_cell = self.outline.cell(cell_indices[idx]);
                    if next_cell.x != x {
                        break;
                    }
                    area += next_cell.area;
                    cover += next_cell.cover;
                    num_cells -= 1;
                    idx += 1;
                }

                if area != 0 {
                    let alpha = self.calculate_alpha((cover << (POLY_SUBPIXEL_SHIFT + 1)) - area);
                    if alpha != 0 {
                        sl.add_cell(x, alpha);
                    }
                    // The partial cell at x has been handled; next span starts at x+1
                    let x_next = x + 1;

                    if num_cells > 0 {
                        let next_cell = self.outline.cell(cell_indices[idx]);
                        if next_cell.x > x_next {
                            let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                            if alpha != 0 {
                                sl.add_span(x_next, (next_cell.x - x_next) as u32, alpha);
                            }
                        }
                    }
                } else if num_cells > 0 {
                    let next_cell = self.outline.cell(cell_indices[idx]);
                    if next_cell.x > x {
                        let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                        if alpha != 0 {
                            sl.add_span(x, (next_cell.x - x) as u32, alpha);
                        }
                    }
                }
            }

            if sl.num_spans() > 0 {
                break;
            }
            self.scan_y += 1;
        }

        sl.finalize(self.scan_y);
        self.scan_y += 1;
        true
    }

    /// Test if a specific pixel coordinate is inside the rasterized polygon.
    pub fn hit_test(&mut self, tx: i32, ty: i32) -> bool {
        if !self.navigate_scanline(ty) {
            return false;
        }
        let mut sl = ScanlineHitTest::new(tx);
        self.sweep_scanline_hit_test(&mut sl);
        sl.hit()
    }

    /// Specialized sweep for ScanlineHitTest (avoids trait object overhead).
    fn sweep_scanline_hit_test(&mut self, sl: &mut ScanlineHitTest) -> bool {
        if self.scan_y > self.outline.max_y() {
            return false;
        }
        sl.reset_spans();

        let cell_indices = self.outline.scanline_cells(self.scan_y as u32);
        let mut num_cells = cell_indices.len();
        let mut idx = 0;
        let mut cover: i32 = 0;

        while num_cells > 0 {
            let cur_cell = self.outline.cell(cell_indices[idx]);
            let x = cur_cell.x;
            let mut area = cur_cell.area;

            cover += cur_cell.cover;

            num_cells -= 1;
            idx += 1;
            while num_cells > 0 {
                let next_cell = self.outline.cell(cell_indices[idx]);
                if next_cell.x != x {
                    break;
                }
                area += next_cell.area;
                cover += next_cell.cover;
                num_cells -= 1;
                idx += 1;
            }

            if area != 0 {
                let alpha = self.calculate_alpha((cover << (POLY_SUBPIXEL_SHIFT + 1)) - area);
                if alpha != 0 {
                    sl.add_cell(x, alpha);
                }
                let x_next = x + 1;
                if num_cells > 0 {
                    let next_cell = self.outline.cell(cell_indices[idx]);
                    if next_cell.x > x_next {
                        let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                        if alpha != 0 {
                            sl.add_span(x_next, (next_cell.x - x_next) as u32, alpha);
                        }
                    }
                }
            } else if num_cells > 0 {
                let next_cell = self.outline.cell(cell_indices[idx]);
                if next_cell.x > x {
                    let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                    if alpha != 0 {
                        sl.add_span(x, (next_cell.x - x) as u32, alpha);
                    }
                }
            }
        }

        sl.finalize(self.scan_y);
        self.scan_y += 1;
        true
    }
}

impl Default for RasterizerScanlineAa {
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
    use crate::basics::{PATH_FLAGS_NONE, POLY_SUBPIXEL_SCALE};
    use crate::ellipse::Ellipse;
    use crate::path_storage::PathStorage;

    /// Minimal scanline for testing: just tracks cells and spans.
    struct TestScanline {
        spans: Vec<(i32, u32, u32)>, // (x, len, cover)
        y_val: i32,
    }

    impl TestScanline {
        fn new() -> Self {
            Self {
                spans: Vec::new(),
                y_val: 0,
            }
        }
    }

    impl Scanline for TestScanline {
        fn reset_spans(&mut self) {
            self.spans.clear();
        }
        fn add_cell(&mut self, x: i32, cover: u32) {
            self.spans.push((x, 1, cover));
        }
        fn add_span(&mut self, x: i32, len: u32, cover: u32) {
            self.spans.push((x, len, cover));
        }
        fn finalize(&mut self, y: i32) {
            self.y_val = y;
        }
        fn num_spans(&self) -> u32 {
            self.spans.len() as u32
        }
        fn y(&self) -> i32 {
            self.y_val
        }
    }

    #[test]
    fn test_new_rasterizer() {
        let ras = RasterizerScanlineAa::new();
        assert_eq!(ras.min_x(), i32::MAX);
        assert_eq!(ras.min_y(), i32::MAX);
    }

    #[test]
    fn test_filling_rule() {
        let mut ras = RasterizerScanlineAa::new();
        ras.filling_rule(FillingRule::EvenOdd);
        assert_eq!(ras.filling_rule, FillingRule::EvenOdd);
    }

    #[test]
    fn test_calculate_alpha_nonzero() {
        let ras = RasterizerScanlineAa::new();
        // Full coverage: area = POLY_SUBPIXEL_SCALE^2 * 2 → alpha should be 255
        let full_area = (POLY_SUBPIXEL_SCALE as i32) << (POLY_SUBPIXEL_SHIFT + 1);
        let alpha = ras.calculate_alpha(full_area);
        assert_eq!(alpha, 255);
    }

    #[test]
    fn test_calculate_alpha_zero_area() {
        let ras = RasterizerScanlineAa::new();
        assert_eq!(ras.calculate_alpha(0), 0);
    }

    #[test]
    fn test_calculate_alpha_negative_area() {
        let ras = RasterizerScanlineAa::new();
        // Negative area should give same magnitude as positive
        let area = 256 * 256; // = 65536
        let alpha_pos = ras.calculate_alpha(area);
        let alpha_neg = ras.calculate_alpha(-area);
        assert_eq!(alpha_pos, alpha_neg);
    }

    #[test]
    fn test_calculate_alpha_even_odd() {
        let mut ras = RasterizerScanlineAa::new();
        ras.filling_rule(FillingRule::EvenOdd);
        // With even-odd, double-covered areas should wrap around
        let full_area = (POLY_SUBPIXEL_SCALE as i32) << (POLY_SUBPIXEL_SHIFT + 1);
        let double_area = full_area * 2;
        let alpha = ras.calculate_alpha(double_area);
        // Double coverage with even-odd should give ~0 (covered twice = uncovered)
        assert!(
            alpha < 10,
            "Expected near-zero alpha for double even-odd, got {alpha}"
        );
    }

    #[test]
    fn test_triangle_sweep() {
        let mut ras = RasterizerScanlineAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        // Triangle: (10,10) -> (20,10) -> (15,20) -> close
        ras.move_to(10 * s, 10 * s);
        ras.line_to(20 * s, 10 * s);
        ras.line_to(15 * s, 20 * s);
        ras.close_polygon();

        assert!(ras.rewind_scanlines());

        let mut sl = TestScanline::new();
        let mut scanline_count = 0;
        while ras.sweep_scanline(&mut sl) {
            scanline_count += 1;
            assert!(sl.num_spans() > 0);
        }
        assert!(scanline_count > 0, "Should have at least one scanline");
        assert_eq!(ras.min_y(), 10);
        assert_eq!(ras.max_y(), 20);
    }

    #[test]
    fn test_triangle_hit_test() {
        let mut ras = RasterizerScanlineAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        // Triangle: (10,10) -> (30,10) -> (20,30)
        ras.move_to(10 * s, 10 * s);
        ras.line_to(30 * s, 10 * s);
        ras.line_to(20 * s, 30 * s);

        // Center should be inside
        assert!(ras.hit_test(20, 15));
        // Far outside should not be hit
        assert!(!ras.hit_test(0, 0));
        assert!(!ras.hit_test(100, 100));
    }

    #[test]
    fn test_move_to_d_line_to_d() {
        let mut ras = RasterizerScanlineAa::new();
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(20.0, 10.0);
        ras.line_to_d(15.0, 20.0);

        assert!(ras.rewind_scanlines());
    }

    #[test]
    fn test_edge_d() {
        let mut ras = RasterizerScanlineAa::new();
        ras.edge_d(10.0, 10.0, 20.0, 20.0);
        ras.edge_d(20.0, 20.0, 10.0, 20.0);
        ras.edge_d(10.0, 20.0, 10.0, 10.0);

        assert!(ras.rewind_scanlines());
    }

    #[test]
    fn test_add_path_with_path_storage() {
        let mut ras = RasterizerScanlineAa::new();
        let mut path = PathStorage::new();
        path.move_to(10.0, 10.0);
        path.line_to(50.0, 10.0);
        path.line_to(30.0, 50.0);
        path.close_polygon(PATH_FLAGS_NONE);

        ras.add_path(&mut path, 0);
        assert!(ras.rewind_scanlines());

        let mut sl = TestScanline::new();
        let mut count = 0;
        while ras.sweep_scanline(&mut sl) {
            count += 1;
        }
        assert!(count > 0);
    }

    #[test]
    fn test_add_path_with_ellipse() {
        let mut ras = RasterizerScanlineAa::new();
        let mut ellipse = Ellipse::new(50.0, 50.0, 20.0, 20.0, 32, false);

        ras.add_path(&mut ellipse, 0);
        assert!(ras.rewind_scanlines());

        // Center should be covered
        assert!(ras.hit_test(50, 50));
    }

    #[test]
    fn test_empty_rasterizer_no_scanlines() {
        let mut ras = RasterizerScanlineAa::new();
        assert!(!ras.rewind_scanlines());
    }

    #[test]
    fn test_reset_clears_state() {
        let mut ras = RasterizerScanlineAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        ras.move_to(10 * s, 10 * s);
        ras.line_to(20 * s, 10 * s);
        ras.line_to(15 * s, 20 * s);
        ras.reset();
        assert!(!ras.rewind_scanlines());
    }

    #[test]
    fn test_clip_box() {
        let mut ras = RasterizerScanlineAa::new();
        ras.clip_box(0.0, 0.0, 50.0, 50.0);

        // Triangle extending beyond clip box
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(100.0, 10.0);
        ras.line_to_d(50.0, 100.0);

        assert!(ras.rewind_scanlines());
        // max_y should be clipped
        assert!(ras.max_y() <= 50);
    }

    #[test]
    fn test_navigate_scanline() {
        let mut ras = RasterizerScanlineAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        ras.move_to(10 * s, 10 * s);
        ras.line_to(20 * s, 10 * s);
        ras.line_to(15 * s, 20 * s);

        // Navigate to a scanline in the middle
        assert!(ras.navigate_scanline(15));
        let mut sl = TestScanline::new();
        assert!(ras.sweep_scanline(&mut sl));
        assert_eq!(sl.y(), 15);

        // Navigate outside range should fail
        assert!(!ras.navigate_scanline(0));
        assert!(!ras.navigate_scanline(100));
    }

    #[test]
    fn test_auto_close_on_move_to() {
        let mut ras = RasterizerScanlineAa::new();
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(20.0, 10.0);
        ras.line_to_d(15.0, 20.0);
        // Don't close explicitly — auto_close should handle it on rewind
        assert!(ras.rewind_scanlines());
    }
}

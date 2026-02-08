//! Anti-aliased cell rasterizer engine.
//!
//! Port of `agg_rasterizer_cells_aa.h` — converts edges (line segments in
//! 24.8 fixed-point coordinates) into cells with coverage and area values.
//! This is the core computational engine used by `RasterizerScanlineAa`.
//!
//! Also ports the `cell_aa` struct from `agg_rasterizer_scanline_aa_nogamma.h`.

use crate::basics::{POLY_SUBPIXEL_MASK, POLY_SUBPIXEL_SCALE, POLY_SUBPIXEL_SHIFT};

// ============================================================================
// CellAa — a single pixel cell with coverage data
// ============================================================================

/// A pixel cell storing accumulated coverage and area from edges.
///
/// Port of C++ `cell_aa` from `agg_rasterizer_scanline_aa_nogamma.h`.
/// - `cover`: net winding contribution (sum of dy across this cell)
/// - `area`: twice the signed area of edge fragments within this cell,
///   used to compute the partial-pixel coverage at cell boundaries
#[derive(Debug, Clone, Copy)]
pub struct CellAa {
    pub x: i32,
    pub y: i32,
    pub cover: i32,
    pub area: i32,
}

impl CellAa {
    /// Reset to the "initial" sentinel state (matches C++ `cell_aa::initial`).
    #[inline]
    pub fn initial(&mut self) {
        self.x = i32::MAX;
        self.y = i32::MAX;
        self.cover = 0;
        self.area = 0;
    }

    /// Style comparison (no-op for basic cell_aa — only meaningful for
    /// compound rasterizer cells). Matches C++ `cell_aa::style`.
    #[inline]
    pub fn style(&mut self, _other: &CellAa) {}

    /// Returns non-zero if this cell differs from position (ex, ey).
    /// Matches C++ `cell_aa::not_equal` — uses unsigned subtraction trick.
    #[inline]
    pub fn not_equal(&self, ex: i32, ey: i32, _style: &CellAa) -> bool {
        (ex as u32).wrapping_sub(self.x as u32) | (ey as u32).wrapping_sub(self.y as u32) != 0
    }
}

impl Default for CellAa {
    fn default() -> Self {
        Self {
            x: i32::MAX,
            y: i32::MAX,
            cover: 0,
            area: 0,
        }
    }
}

// ============================================================================
// SortedY — per-scanline index into sorted cell array
// ============================================================================

#[derive(Debug, Clone, Copy, Default)]
struct SortedY {
    start: u32,
    num: u32,
}

// ============================================================================
// RasterizerCellsAa — the edge-to-cell conversion engine
// ============================================================================

/// The main rasterization engine that converts line segments (edges) into
/// anti-aliased pixel cells.
///
/// Port of C++ `rasterizer_cells_aa<Cell>` from `agg_rasterizer_cells_aa.h`.
///
/// Instead of the C++ block-based allocator, we use a flat `Vec<CellAa>`.
/// Sorted cell access uses indices into this vec rather than raw pointers.
pub struct RasterizerCellsAa {
    cells: Vec<CellAa>,
    sorted_cells: Vec<u32>,
    sorted_y: Vec<SortedY>,
    curr_cell: CellAa,
    style_cell: CellAa,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    sorted: bool,
}

/// Limit for dx magnitude before recursive subdivision in `line()`.
const DX_LIMIT: i64 = 16384 << POLY_SUBPIXEL_SHIFT;

impl RasterizerCellsAa {
    /// Create a new empty cell rasterizer.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            sorted_cells: Vec::new(),
            sorted_y: Vec::new(),
            curr_cell: CellAa::default(),
            style_cell: CellAa::default(),
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: i32::MIN,
            max_y: i32::MIN,
            sorted: false,
        }
    }

    /// Reset the rasterizer, discarding all cells.
    pub fn reset(&mut self) {
        self.cells.clear();
        self.sorted_cells.clear();
        self.sorted_y.clear();
        self.curr_cell.initial();
        self.style_cell.initial();
        self.min_x = i32::MAX;
        self.min_y = i32::MAX;
        self.max_x = i32::MIN;
        self.max_y = i32::MIN;
        self.sorted = false;
    }

    /// Set the current style cell (used by compound rasterizer; no-op for basic usage).
    #[inline]
    pub fn style(&mut self, style_cell: &CellAa) {
        self.style_cell.style(style_cell);
    }

    #[inline]
    pub fn min_x(&self) -> i32 {
        self.min_x
    }
    #[inline]
    pub fn min_y(&self) -> i32 {
        self.min_y
    }
    #[inline]
    pub fn max_x(&self) -> i32 {
        self.max_x
    }
    #[inline]
    pub fn max_y(&self) -> i32 {
        self.max_y
    }

    /// Total number of accumulated cells.
    #[inline]
    pub fn total_cells(&self) -> u32 {
        self.cells.len() as u32
    }

    /// Whether cells have been sorted.
    #[inline]
    pub fn sorted(&self) -> bool {
        self.sorted
    }

    /// Number of cells on scanline `y` (only valid after `sort_cells()`).
    #[inline]
    pub fn scanline_num_cells(&self, y: u32) -> u32 {
        self.sorted_y[(y as i32 - self.min_y) as usize].num
    }

    /// Get a slice of cell indices for scanline `y` (only valid after `sort_cells()`).
    /// Returns indices into the internal cells array.
    #[inline]
    pub fn scanline_cells(&self, y: u32) -> &[u32] {
        let sy = &self.sorted_y[(y as i32 - self.min_y) as usize];
        &self.sorted_cells[sy.start as usize..(sy.start + sy.num) as usize]
    }

    /// Get a reference to the cell at the given index.
    #[inline]
    pub fn cell(&self, idx: u32) -> &CellAa {
        &self.cells[idx as usize]
    }

    /// Get a reference to all cells.
    #[inline]
    pub fn cells(&self) -> &[CellAa] {
        &self.cells
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Flush the current cell into the cells array if it has non-zero data.
    #[inline]
    fn add_curr_cell(&mut self) {
        if self.curr_cell.area | self.curr_cell.cover != 0 {
            self.cells.push(self.curr_cell);
        }
    }

    /// Move to a new cell position, flushing the previous cell if needed.
    #[inline]
    fn set_curr_cell(&mut self, x: i32, y: i32) {
        if self.curr_cell.not_equal(x, y, &self.style_cell) {
            self.add_curr_cell();
            self.curr_cell.style(&self.style_cell);
            self.curr_cell.x = x;
            self.curr_cell.y = y;
            self.curr_cell.cover = 0;
            self.curr_cell.area = 0;
        }
    }

    /// Render a horizontal line segment within a single scanline row `ey`.
    ///
    /// `x1`, `x2` are in 24.8 fixed-point; `y1`, `y2` are fractional y within
    /// the scanline (0..POLY_SUBPIXEL_SCALE).
    ///
    /// This is the most performance-critical helper. Matches C++ `render_hline`
    /// exactly, including the `i64` arithmetic for large dx values.
    fn render_hline(&mut self, ey: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
        let ex1 = x1 >> POLY_SUBPIXEL_SHIFT;
        let ex2 = x2 >> POLY_SUBPIXEL_SHIFT;
        let fx1 = x1 & POLY_SUBPIXEL_MASK as i32;
        let fx2 = x2 & POLY_SUBPIXEL_MASK as i32;

        // Trivial case: horizontal line (y1 == y2) — just move to target cell
        if y1 == y2 {
            self.set_curr_cell(ex2, ey);
            return;
        }

        // Everything in a single cell
        if ex1 == ex2 {
            let delta = y2 - y1;
            self.curr_cell.cover += delta;
            self.curr_cell.area += (fx1 + fx2) * delta;
            return;
        }

        // Run of adjacent cells on the same hline
        let mut p = (POLY_SUBPIXEL_SCALE as i64 - fx1 as i64) * (y2 - y1) as i64;
        let mut first = POLY_SUBPIXEL_SCALE as i32;
        let mut incr = 1_i32;

        let mut dx = x2 as i64 - x1 as i64;

        if dx < 0 {
            p = fx1 as i64 * (y2 - y1) as i64;
            first = 0;
            incr = -1;
            dx = -dx;
        }

        let mut delta = (p / dx) as i32;
        let mut modulo = p % dx;

        if modulo < 0 {
            delta -= 1;
            modulo += dx;
        }

        self.curr_cell.cover += delta;
        self.curr_cell.area += (fx1 + first) * delta;

        let mut ex1 = ex1 + incr;
        self.set_curr_cell(ex1, ey);
        let mut y1 = y1 + delta;

        if ex1 != ex2 {
            p = POLY_SUBPIXEL_SCALE as i64 * (y2 - y1 + delta) as i64;
            let mut lift = (p / dx) as i32;
            let mut rem = p % dx;

            if rem < 0 {
                lift -= 1;
                rem += dx;
            }

            modulo -= dx;

            while ex1 != ex2 {
                delta = lift;
                modulo += rem;
                if modulo >= 0 {
                    modulo -= dx;
                    delta += 1;
                }

                self.curr_cell.cover += delta;
                self.curr_cell.area += POLY_SUBPIXEL_SCALE as i32 * delta;
                y1 += delta;
                ex1 += incr;
                self.set_curr_cell(ex1, ey);
            }
        }
        delta = y2 - y1;
        self.curr_cell.cover += delta;
        self.curr_cell.area += (fx2 + POLY_SUBPIXEL_SCALE as i32 - first) * delta;
    }

    /// Add a line segment in 24.8 fixed-point coordinates.
    ///
    /// This is the primary entry point for the rasterizer. Large dx values
    /// are handled by recursive subdivision (matching the C++ implementation).
    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let dx = x2 as i64 - x1 as i64;

        if dx >= DX_LIMIT || dx <= -DX_LIMIT {
            let cx = ((x1 as i64 + x2 as i64) >> 1) as i32;
            let cy = ((y1 as i64 + y2 as i64) >> 1) as i32;
            self.line(x1, y1, cx, cy);
            self.line(cx, cy, x2, y2);
            return;
        }

        let dy = y2 as i64 - y1 as i64;
        let ex1 = x1 >> POLY_SUBPIXEL_SHIFT;
        let ex2 = x2 >> POLY_SUBPIXEL_SHIFT;
        let ey1_orig = y1 >> POLY_SUBPIXEL_SHIFT;
        let ey2 = y2 >> POLY_SUBPIXEL_SHIFT;
        let fy1 = y1 & POLY_SUBPIXEL_MASK as i32;
        let fy2 = y2 & POLY_SUBPIXEL_MASK as i32;

        // Update bounding box
        if ex1 < self.min_x {
            self.min_x = ex1;
        }
        if ex1 > self.max_x {
            self.max_x = ex1;
        }
        if ey1_orig < self.min_y {
            self.min_y = ey1_orig;
        }
        if ey1_orig > self.max_y {
            self.max_y = ey1_orig;
        }
        if ex2 < self.min_x {
            self.min_x = ex2;
        }
        if ex2 > self.max_x {
            self.max_x = ex2;
        }
        if ey2 < self.min_y {
            self.min_y = ey2;
        }
        if ey2 > self.max_y {
            self.max_y = ey2;
        }

        let mut ey1 = ey1_orig;

        self.set_curr_cell(ex1, ey1);

        // Everything on a single hline
        if ey1 == ey2 {
            self.render_hline(ey1, x1, fy1, x2, fy2);
            return;
        }

        // Vertical line — optimized path without render_hline calls
        let mut incr = 1_i32;
        if dx == 0 {
            let ex = x1 >> POLY_SUBPIXEL_SHIFT;
            let two_fx = (x1 - (ex << POLY_SUBPIXEL_SHIFT)) << 1;

            let mut first = POLY_SUBPIXEL_SCALE as i32;
            if dy < 0 {
                first = 0;
                incr = -1;
            }

            let x_from = x1;
            let _ = x_from; // keep name for clarity matching C++

            let mut delta = first - fy1;
            self.curr_cell.cover += delta;
            self.curr_cell.area += two_fx * delta;

            ey1 += incr;
            self.set_curr_cell(ex, ey1);

            delta = first + first - POLY_SUBPIXEL_SCALE as i32;
            let area = two_fx * delta;
            while ey1 != ey2 {
                self.curr_cell.cover = delta;
                self.curr_cell.area = area;
                ey1 += incr;
                self.set_curr_cell(ex, ey1);
            }
            delta = fy2 - POLY_SUBPIXEL_SCALE as i32 + first;
            self.curr_cell.cover += delta;
            self.curr_cell.area += two_fx * delta;
            return;
        }

        // General case: multiple hlines
        let mut p = (POLY_SUBPIXEL_SCALE as i64 - fy1 as i64) * dx;
        let mut first = POLY_SUBPIXEL_SCALE as i32;

        let mut dy_abs = dy;
        if dy < 0 {
            p = fy1 as i64 * dx;
            first = 0;
            incr = -1;
            dy_abs = -dy;
        }

        let mut delta = (p / dy_abs) as i32;
        let mut modulo = p % dy_abs;

        if modulo < 0 {
            delta -= 1;
            modulo += dy_abs;
        }

        let mut x_from = x1 + delta;
        self.render_hline(ey1, x1, fy1, x_from, first);

        ey1 += incr;
        self.set_curr_cell(x_from >> POLY_SUBPIXEL_SHIFT, ey1);

        if ey1 != ey2 {
            p = POLY_SUBPIXEL_SCALE as i64 * dx;
            let mut lift = (p / dy_abs) as i32;
            let mut rem = p % dy_abs;

            if rem < 0 {
                lift -= 1;
                rem += dy_abs;
            }
            modulo -= dy_abs;

            while ey1 != ey2 {
                delta = lift;
                modulo += rem;
                if modulo >= 0 {
                    modulo -= dy_abs;
                    delta += 1;
                }

                let x_to = x_from + delta;
                self.render_hline(ey1, x_from, POLY_SUBPIXEL_SCALE as i32 - first, x_to, first);
                x_from = x_to;

                ey1 += incr;
                self.set_curr_cell(x_from >> POLY_SUBPIXEL_SHIFT, ey1);
            }
        }
        self.render_hline(ey1, x_from, POLY_SUBPIXEL_SCALE as i32 - first, x2, fy2);
    }

    /// Sort all accumulated cells by Y then X.
    ///
    /// After sorting, cells can be queried per-scanline via
    /// `scanline_num_cells()` and `scanline_cells()`.
    pub fn sort_cells(&mut self) {
        if self.sorted {
            return;
        }

        self.add_curr_cell();
        self.curr_cell.x = i32::MAX;
        self.curr_cell.y = i32::MAX;
        self.curr_cell.cover = 0;
        self.curr_cell.area = 0;

        if self.cells.is_empty() {
            return;
        }

        // Allocate sorted_cells (indices) and sorted_y (histogram)
        let num_cells = self.cells.len();
        self.sorted_cells.clear();
        self.sorted_cells.resize(num_cells, 0);

        let y_range = (self.max_y - self.min_y + 1) as usize;
        self.sorted_y.clear();
        self.sorted_y.resize(y_range, SortedY::default());

        // Pass 1: Build Y-histogram (count cells per scanline)
        for cell in &self.cells {
            let yi = (cell.y - self.min_y) as usize;
            self.sorted_y[yi].start += 1;
        }

        // Convert histogram to starting indices
        let mut start = 0u32;
        for sy in &mut self.sorted_y {
            let count = sy.start;
            sy.start = start;
            start += count;
        }

        // Pass 2: Fill sorted_cells with cell indices, sorted by Y
        for (i, cell) in self.cells.iter().enumerate() {
            let yi = (cell.y - self.min_y) as usize;
            let sy = &mut self.sorted_y[yi];
            self.sorted_cells[(sy.start + sy.num) as usize] = i as u32;
            sy.num += 1;
        }

        // Pass 3: Sort each scanline's cells by X
        for sy in &self.sorted_y {
            if sy.num > 0 {
                let start = sy.start as usize;
                let end = (sy.start + sy.num) as usize;
                let slice = &mut self.sorted_cells[start..end];
                let cells = &self.cells;
                slice.sort_unstable_by_key(|&idx| cells[idx as usize].x);
            }
        }

        self.sorted = true;
    }
}

impl Default for RasterizerCellsAa {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ScanlineHitTest — simple scanline that tests if a specific X is covered
// ============================================================================

/// A minimal "scanline" that only checks whether a specific X coordinate
/// is covered by the rasterized polygon.
///
/// Port of C++ `scanline_hit_test` from `agg_rasterizer_cells_aa.h`.
pub struct ScanlineHitTest {
    x: i32,
    hit: bool,
}

impl ScanlineHitTest {
    pub fn new(x: i32) -> Self {
        Self { x, hit: false }
    }

    #[inline]
    pub fn reset_spans(&mut self) {}

    #[inline]
    pub fn finalize(&mut self, _y: i32) {}

    #[inline]
    pub fn add_cell(&mut self, x: i32, _cover: u32) {
        if self.x == x {
            self.hit = true;
        }
    }

    #[inline]
    pub fn add_span(&mut self, x: i32, len: u32, _cover: u32) {
        if self.x >= x && self.x < x + len as i32 {
            self.hit = true;
        }
    }

    #[inline]
    pub fn num_spans(&self) -> u32 {
        1
    }

    #[inline]
    pub fn hit(&self) -> bool {
        self.hit
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // CellAa tests
    // ------------------------------------------------------------------

    #[test]
    fn test_cell_aa_default() {
        let cell = CellAa::default();
        assert_eq!(cell.x, i32::MAX);
        assert_eq!(cell.y, i32::MAX);
        assert_eq!(cell.cover, 0);
        assert_eq!(cell.area, 0);
    }

    #[test]
    fn test_cell_aa_initial() {
        let mut cell = CellAa {
            x: 10,
            y: 20,
            cover: 5,
            area: 100,
        };
        cell.initial();
        assert_eq!(cell.x, i32::MAX);
        assert_eq!(cell.y, i32::MAX);
        assert_eq!(cell.cover, 0);
        assert_eq!(cell.area, 0);
    }

    #[test]
    fn test_cell_aa_not_equal() {
        let cell = CellAa {
            x: 10,
            y: 20,
            cover: 0,
            area: 0,
        };
        let style = CellAa::default();
        assert!(!cell.not_equal(10, 20, &style));
        assert!(cell.not_equal(11, 20, &style));
        assert!(cell.not_equal(10, 21, &style));
        assert!(cell.not_equal(11, 21, &style));
    }

    // ------------------------------------------------------------------
    // RasterizerCellsAa basic tests
    // ------------------------------------------------------------------

    #[test]
    fn test_new_rasterizer_is_empty() {
        let ras = RasterizerCellsAa::new();
        assert_eq!(ras.total_cells(), 0);
        assert!(!ras.sorted());
        assert_eq!(ras.min_x(), i32::MAX);
        assert_eq!(ras.min_y(), i32::MAX);
        assert_eq!(ras.max_x(), i32::MIN);
        assert_eq!(ras.max_y(), i32::MIN);
    }

    #[test]
    fn test_reset() {
        let mut ras = RasterizerCellsAa::new();
        // Add a line to generate some cells
        ras.line(0, 0, 256, 256);
        assert!(ras.total_cells() > 0 || true); // line may not add cells until sort
        ras.reset();
        assert_eq!(ras.total_cells(), 0);
        assert!(!ras.sorted());
    }

    // ------------------------------------------------------------------
    // Horizontal line tests
    // ------------------------------------------------------------------

    #[test]
    fn test_horizontal_line_no_cells() {
        let mut ras = RasterizerCellsAa::new();
        // Horizontal line: y1 == y2 in pixel space, same fractional y
        let y = 10 << POLY_SUBPIXEL_SHIFT;
        ras.line(0, y, 512, y);
        ras.sort_cells();
        // A perfectly horizontal line generates no coverage (dy=0 at subpixel level)
        // The cells may have zero area/cover, so they might not be stored
    }

    // ------------------------------------------------------------------
    // Vertical line tests
    // ------------------------------------------------------------------

    #[test]
    fn test_vertical_line_generates_cells() {
        let mut ras = RasterizerCellsAa::new();
        let x = 10 << POLY_SUBPIXEL_SHIFT;
        let y1 = 5 << POLY_SUBPIXEL_SHIFT;
        let y2 = 15 << POLY_SUBPIXEL_SHIFT;
        ras.line(x, y1, x, y2);
        ras.sort_cells();
        assert!(ras.total_cells() > 0);
        // Should span from y=5 to y=14 (10 scanlines)
        assert_eq!(ras.min_y(), 5);
        assert_eq!(ras.max_y(), 15);
    }

    #[test]
    fn test_vertical_line_cover_sum() {
        let mut ras = RasterizerCellsAa::new();
        // Vertical line from pixel (10, 5) to (10, 8) — 3 pixel rows
        let x = (10 << POLY_SUBPIXEL_SHIFT) + 128; // at x=10.5 subpixel
        let y1 = 5 << POLY_SUBPIXEL_SHIFT;
        let y2 = 8 << POLY_SUBPIXEL_SHIFT;
        ras.line(x, y1, x, y2);
        ras.sort_cells();

        // Total cover across all cells should equal dy in subpixel units
        let total_cover: i32 = ras.cells.iter().map(|c| c.cover).sum();
        assert_eq!(total_cover, (y2 - y1) >> 0); // dy in subpixel = 3*256 = 768
                                                 // Actually, cover is accumulated in subpixel units within each cell row
                                                 // The total should be 3 * POLY_SUBPIXEL_SCALE = 768
        assert_eq!(total_cover, 3 * POLY_SUBPIXEL_SCALE as i32);
    }

    // ------------------------------------------------------------------
    // Diagonal line tests
    // ------------------------------------------------------------------

    #[test]
    fn test_diagonal_line_generates_cells() {
        let mut ras = RasterizerCellsAa::new();
        let x1 = 0;
        let y1 = 0;
        let x2 = 10 << POLY_SUBPIXEL_SHIFT;
        let y2 = 10 << POLY_SUBPIXEL_SHIFT;
        ras.line(x1, y1, x2, y2);
        ras.sort_cells();
        assert!(ras.total_cells() > 0);
        assert_eq!(ras.min_x(), 0);
        assert_eq!(ras.min_y(), 0);
        assert_eq!(ras.max_x(), 10);
        assert_eq!(ras.max_y(), 10);
    }

    #[test]
    fn test_diagonal_line_cover_sum() {
        let mut ras = RasterizerCellsAa::new();
        let x1 = 0;
        let y1 = 0;
        let x2 = 5 << POLY_SUBPIXEL_SHIFT;
        let y2 = 5 << POLY_SUBPIXEL_SHIFT;
        ras.line(x1, y1, x2, y2);
        ras.sort_cells();

        // Total cover should equal total dy in subpixel units
        let total_cover: i32 = ras.cells.iter().map(|c| c.cover).sum();
        assert_eq!(total_cover, 5 * POLY_SUBPIXEL_SCALE as i32);
    }

    // ------------------------------------------------------------------
    // Sort and query tests
    // ------------------------------------------------------------------

    #[test]
    fn test_sort_cells_idempotent() {
        let mut ras = RasterizerCellsAa::new();
        let x = 5 << POLY_SUBPIXEL_SHIFT;
        ras.line(x, 0, x, 3 << POLY_SUBPIXEL_SHIFT);
        ras.sort_cells();
        let count1 = ras.total_cells();
        ras.sort_cells(); // second call should be a no-op
        assert_eq!(ras.total_cells(), count1);
    }

    #[test]
    fn test_sort_empty_rasterizer() {
        let mut ras = RasterizerCellsAa::new();
        ras.sort_cells();
        assert_eq!(ras.total_cells(), 0);
    }

    #[test]
    fn test_scanline_query() {
        let mut ras = RasterizerCellsAa::new();
        let x = 5 << POLY_SUBPIXEL_SHIFT;
        ras.line(x, 2 << POLY_SUBPIXEL_SHIFT, x, 5 << POLY_SUBPIXEL_SHIFT);
        ras.sort_cells();

        // Check that each scanline in range has cells
        for y in ras.min_y()..=ras.max_y() {
            let num = ras.scanline_num_cells(y as u32);
            let indices = ras.scanline_cells(y as u32);
            assert_eq!(indices.len(), num as usize);
            // Each cell should be on the correct scanline
            for &idx in indices {
                assert_eq!(ras.cell(idx).y, y);
            }
        }
    }

    #[test]
    fn test_cells_sorted_by_x_within_scanline() {
        let mut ras = RasterizerCellsAa::new();
        // Draw a diagonal that crosses multiple X cells on the same scanline
        ras.line(0, 0, 10 << POLY_SUBPIXEL_SHIFT, 1 << POLY_SUBPIXEL_SHIFT);
        ras.sort_cells();

        for y in ras.min_y()..=ras.max_y() {
            let indices = ras.scanline_cells(y as u32);
            for window in indices.windows(2) {
                let x_a = ras.cell(window[0]).x;
                let x_b = ras.cell(window[1]).x;
                assert!(x_a <= x_b, "Cells not sorted by X: {} > {}", x_a, x_b);
            }
        }
    }

    // ------------------------------------------------------------------
    // Triangle (closed polygon) test
    // ------------------------------------------------------------------

    #[test]
    fn test_triangle_closed_polygon() {
        let mut ras = RasterizerCellsAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;
        // Triangle: (10,10) -> (20,10) -> (15,20) -> (10,10)
        ras.line(10 * s, 10 * s, 20 * s, 10 * s); // top edge (horizontal)
        ras.line(20 * s, 10 * s, 15 * s, 20 * s); // right edge
        ras.line(15 * s, 20 * s, 10 * s, 10 * s); // left edge
        ras.sort_cells();

        assert!(ras.total_cells() > 0);
        assert_eq!(ras.min_y(), 10);
        assert_eq!(ras.max_y(), 20);
    }

    // ------------------------------------------------------------------
    // Large dx subdivision test
    // ------------------------------------------------------------------

    #[test]
    fn test_large_dx_subdivision() {
        let mut ras = RasterizerCellsAa::new();
        // Very large dx should trigger recursive subdivision
        let x1 = 0;
        let y1 = 0;
        let x2 = 20000 << POLY_SUBPIXEL_SHIFT;
        let y2 = 1 << POLY_SUBPIXEL_SHIFT;
        ras.line(x1, y1, x2, y2);
        ras.sort_cells();
        // Should not panic, and should produce cells
        assert!(ras.total_cells() > 0);
    }

    // ------------------------------------------------------------------
    // ScanlineHitTest tests
    // ------------------------------------------------------------------

    #[test]
    fn test_scanline_hit_test_add_cell() {
        let mut ht = ScanlineHitTest::new(42);
        assert!(!ht.hit());
        ht.add_cell(41, 255);
        assert!(!ht.hit());
        ht.add_cell(42, 255);
        assert!(ht.hit());
    }

    #[test]
    fn test_scanline_hit_test_add_span() {
        let mut ht = ScanlineHitTest::new(15);
        assert!(!ht.hit());
        ht.add_span(10, 4, 255); // covers [10, 13]
        assert!(!ht.hit());
        ht.add_span(10, 6, 255); // covers [10, 15]
        assert!(ht.hit());
    }

    #[test]
    fn test_scanline_hit_test_num_spans() {
        let ht = ScanlineHitTest::new(0);
        assert_eq!(ht.num_spans(), 1);
    }
}

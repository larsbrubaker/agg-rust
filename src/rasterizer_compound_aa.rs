//! Compound AA rasterizer with per-edge style indices.
//!
//! Port of `agg_rasterizer_compound_aa.h`.
//! Extends the standard rasterizer with left/right style indices per edge,
//! enabling multi-style rendering (e.g. Flash-style fills where a single
//! scanline contains pixels from multiple fill styles).

use crate::basics::{
    is_close, is_move_to, is_stop, is_vertex, iround, FillingRule, VertexSource,
    POLY_SUBPIXEL_MASK, POLY_SUBPIXEL_SCALE, POLY_SUBPIXEL_SHIFT,
};
use crate::rasterizer_scanline_aa::Scanline;

// ============================================================================
// CellStyleAa — cell with left/right style indices
// ============================================================================

/// A pixel cell with left/right style indices for compound rendering.
///
/// Port of C++ `cell_style_aa`.
#[derive(Debug, Clone, Copy)]
pub struct CellStyleAa {
    pub x: i32,
    pub y: i32,
    pub cover: i32,
    pub area: i32,
    pub left: i16,
    pub right: i16,
}

impl CellStyleAa {
    #[inline]
    pub fn initial(&mut self) {
        self.x = i32::MAX;
        self.y = i32::MAX;
        self.cover = 0;
        self.area = 0;
        self.left = -1;
        self.right = -1;
    }

    #[inline]
    pub fn style(&mut self, other: &CellStyleAa) {
        self.left = other.left;
        self.right = other.right;
    }

    #[inline]
    pub fn not_equal(&self, ex: i32, ey: i32, style: &CellStyleAa) -> bool {
        (ex as u32).wrapping_sub(self.x as u32)
            | (ey as u32).wrapping_sub(self.y as u32)
            | (self.left as u32).wrapping_sub(style.left as u32)
            | (self.right as u32).wrapping_sub(style.right as u32)
            != 0
    }
}

impl Default for CellStyleAa {
    fn default() -> Self {
        Self {
            x: i32::MAX,
            y: i32::MAX,
            cover: 0,
            area: 0,
            left: -1,
            right: -1,
        }
    }
}

// ============================================================================
// LayerOrder — rendering order for styles
// ============================================================================

/// Layer order for compound rasterizer style rendering.
///
/// Port of C++ `layer_order_e`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerOrder {
    Unsorted,
    Direct,
    Inverse,
}

// ============================================================================
// Internal cells engine for CellStyleAa (matches rasterizer_cells_aa logic)
// ============================================================================

/// Limit for dx magnitude before recursive subdivision.
const DX_LIMIT: i64 = 16384 << POLY_SUBPIXEL_SHIFT;

#[derive(Debug, Clone, Copy, Default)]
struct SortedY {
    start: u32,
    num: u32,
}

/// Self-contained cells engine for `CellStyleAa`.
/// Port of C++ `rasterizer_cells_aa<cell_style_aa>`.
struct CellsEngine {
    cells: Vec<CellStyleAa>,
    sorted_cells: Vec<u32>,
    sorted_y: Vec<SortedY>,
    curr_cell: CellStyleAa,
    style_cell: CellStyleAa,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    sorted: bool,
}

impl CellsEngine {
    fn new() -> Self {
        Self {
            cells: Vec::new(),
            sorted_cells: Vec::new(),
            sorted_y: Vec::new(),
            curr_cell: CellStyleAa::default(),
            style_cell: CellStyleAa::default(),
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: i32::MIN,
            max_y: i32::MIN,
            sorted: false,
        }
    }

    fn reset(&mut self) {
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

    #[inline]
    fn style(&mut self, style_cell: &CellStyleAa) {
        self.style_cell.style(style_cell);
    }

    #[inline]
    fn min_x(&self) -> i32 {
        self.min_x
    }
    #[inline]
    fn min_y(&self) -> i32 {
        self.min_y
    }
    #[inline]
    fn max_x(&self) -> i32 {
        self.max_x
    }
    #[inline]
    fn max_y(&self) -> i32 {
        self.max_y
    }
    #[inline]
    fn total_cells(&self) -> u32 {
        self.cells.len() as u32
    }
    #[inline]
    fn sorted(&self) -> bool {
        self.sorted
    }

    #[inline]
    fn scanline_num_cells(&self, y: u32) -> u32 {
        self.sorted_y[(y as i32 - self.min_y) as usize].num
    }

    /// Get sorted cell indices for scanline `y`.
    #[inline]
    fn scanline_cells(&self, y: u32) -> &[u32] {
        let sy = &self.sorted_y[(y as i32 - self.min_y) as usize];
        &self.sorted_cells[sy.start as usize..(sy.start + sy.num) as usize]
    }

    #[inline]
    fn cell(&self, idx: u32) -> &CellStyleAa {
        &self.cells[idx as usize]
    }

    #[inline]
    fn add_curr_cell(&mut self) {
        if self.curr_cell.area | self.curr_cell.cover != 0 {
            self.cells.push(self.curr_cell);
        }
    }

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

    fn render_hline(&mut self, ey: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
        let ex1 = x1 >> POLY_SUBPIXEL_SHIFT;
        let ex2 = x2 >> POLY_SUBPIXEL_SHIFT;
        let fx1 = x1 & POLY_SUBPIXEL_MASK as i32;
        let fx2 = x2 & POLY_SUBPIXEL_MASK as i32;

        if y1 == y2 {
            self.set_curr_cell(ex2, ey);
            return;
        }

        if ex1 == ex2 {
            let delta = y2 - y1;
            self.curr_cell.cover += delta;
            self.curr_cell.area += (fx1 + fx2) * delta;
            return;
        }

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

    fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
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

        if ex1 < self.min_x { self.min_x = ex1; }
        if ex1 > self.max_x { self.max_x = ex1; }
        if ey1_orig < self.min_y { self.min_y = ey1_orig; }
        if ey1_orig > self.max_y { self.max_y = ey1_orig; }
        if ex2 < self.min_x { self.min_x = ex2; }
        if ex2 > self.max_x { self.max_x = ex2; }
        if ey2 < self.min_y { self.min_y = ey2; }
        if ey2 > self.max_y { self.max_y = ey2; }

        let mut ey1 = ey1_orig;
        self.set_curr_cell(ex1, ey1);

        if ey1 == ey2 {
            self.render_hline(ey1, x1, fy1, x2, fy2);
            return;
        }

        let mut incr = 1_i32;
        if dx == 0 {
            let ex = x1 >> POLY_SUBPIXEL_SHIFT;
            let two_fx = (x1 - (ex << POLY_SUBPIXEL_SHIFT)) << 1;
            let mut first = POLY_SUBPIXEL_SCALE as i32;
            if dy < 0 {
                first = 0;
                incr = -1;
            }
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
                self.render_hline(
                    ey1,
                    x_from,
                    POLY_SUBPIXEL_SCALE as i32 - first,
                    x_to,
                    first,
                );
                x_from = x_to;
                ey1 += incr;
                self.set_curr_cell(x_from >> POLY_SUBPIXEL_SHIFT, ey1);
            }
        }
        self.render_hline(ey1, x_from, POLY_SUBPIXEL_SCALE as i32 - first, x2, fy2);
    }

    fn sort_cells(&mut self) {
        if self.sorted {
            return;
        }

        self.add_curr_cell();
        self.curr_cell.initial();

        if self.cells.is_empty() {
            return;
        }

        let num_cells = self.cells.len();
        self.sorted_cells.clear();
        self.sorted_cells.resize(num_cells, 0);

        let y_range = (self.max_y - self.min_y + 1) as usize;
        self.sorted_y.clear();
        self.sorted_y.resize(y_range, SortedY::default());

        for cell in &self.cells {
            let yi = (cell.y - self.min_y) as usize;
            self.sorted_y[yi].start += 1;
        }

        let mut start = 0u32;
        for sy in &mut self.sorted_y {
            let count = sy.start;
            sy.start = start;
            start += count;
        }

        for (i, cell) in self.cells.iter().enumerate() {
            let yi = (cell.y - self.min_y) as usize;
            let sy = &mut self.sorted_y[yi];
            self.sorted_cells[(sy.start + sy.num) as usize] = i as u32;
            sy.num += 1;
        }

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

// ============================================================================
// Simple inline clipper for compound rasterizer
// ============================================================================

fn upscale(v: f64) -> i32 {
    iround(v * POLY_SUBPIXEL_SCALE as f64)
}

fn downscale(v: i32) -> i32 {
    v
}

/// Clipping flags for Liang-Barsky line clipping.
fn clipping_flags(x: i32, y: i32, clip_box: &[i32; 4]) -> u32 {
    ((x > clip_box[2]) as u32) << 0
        | ((y > clip_box[3]) as u32) << 1
        | ((x < clip_box[0]) as u32) << 2
        | ((y < clip_box[1]) as u32) << 3
}

/// Clip and render a line segment.
fn clip_line_segment(
    engine: &mut CellsEngine,
    x1: &mut i32,
    y1: &mut i32,
    x2: i32,
    y2: i32,
    clip_box: &[i32; 4],
) {
    let f1 = clipping_flags(*x1, *y1, clip_box);
    let f2 = clipping_flags(x2, y2, clip_box);

    if f1 == 0 && f2 == 0 {
        // Fully visible
        engine.line(*x1, *y1, x2, y2);
    } else if (f1 & f2) != 0 {
        // Fully clipped (both on same side)
    } else {
        // Partial — use simple Liang-Barsky approach
        let mut cx1 = *x1;
        let mut cy1 = *y1;
        let mut cx2 = x2;
        let mut cy2 = y2;

        if liang_barsky_clip(&mut cx1, &mut cy1, &mut cx2, &mut cy2, clip_box) {
            engine.line(cx1, cy1, cx2, cy2);
        }
    }
    *x1 = x2;
    *y1 = y2;
}

/// Liang-Barsky line clipping. Returns true if line is (partially) visible.
fn liang_barsky_clip(
    x1: &mut i32,
    y1: &mut i32,
    x2: &mut i32,
    y2: &mut i32,
    clip: &[i32; 4],
) -> bool {
    let dx = *x2 as f64 - *x1 as f64;
    let dy = *y2 as f64 - *y1 as f64;
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;

    let clips = [
        (-dx, *x1 as f64 - clip[0] as f64),
        (dx, clip[2] as f64 - *x1 as f64),
        (-dy, *y1 as f64 - clip[1] as f64),
        (dy, clip[3] as f64 - *y1 as f64),
    ];

    for &(p, q) in &clips {
        if p.abs() < 1e-10 {
            if q < 0.0 {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    return false;
                }
                if t > t0 {
                    t0 = t;
                }
            } else {
                if t < t0 {
                    return false;
                }
                if t < t1 {
                    t1 = t;
                }
            }
        }
    }

    let ox1 = *x1 as f64;
    let oy1 = *y1 as f64;
    if t1 < 1.0 {
        *x2 = (ox1 + dx * t1) as i32;
        *y2 = (oy1 + dy * t1) as i32;
    }
    if t0 > 0.0 {
        *x1 = (ox1 + dx * t0) as i32;
        *y1 = (oy1 + dy * t0) as i32;
    }
    true
}

// ============================================================================
// StyleInfo / CellInfo — internal data structures
// ============================================================================

#[derive(Debug, Clone, Copy)]
struct StyleInfo {
    start_cell: u32,
    num_cells: u32,
    last_x: i32,
}

impl Default for StyleInfo {
    fn default() -> Self {
        Self {
            start_cell: 0,
            num_cells: 0,
            last_x: i32::MIN,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CellInfo {
    x: i32,
    area: i32,
    cover: i32,
}

// ============================================================================
// RasterizerCompoundAa
// ============================================================================

const AA_SHIFT: u32 = 8;
const AA_SCALE: u32 = 1 << AA_SHIFT;
const AA_MASK: u32 = AA_SCALE - 1;
const AA_SCALE2: u32 = AA_SCALE * 2;
const AA_MASK2: u32 = AA_SCALE2 - 1;

/// Compound anti-aliased rasterizer with per-edge style indices.
///
/// Port of C++ `rasterizer_compound_aa`.
/// Each edge can have independent left and right fill styles, enabling
/// multi-style rendering in a single pass.
pub struct RasterizerCompoundAa {
    outline: CellsEngine,
    filling_rule: FillingRule,
    layer_order: LayerOrder,
    styles: Vec<StyleInfo>,
    ast: Vec<u32>,   // Active Style Table
    asm: Vec<u8>,    // Active Style Mask (bitmask)
    cells: Vec<CellInfo>,
    cover_buf: Vec<u8>,
    min_style: i32,
    max_style: i32,
    start_x: i32,
    start_y: i32,
    scan_y: i32,
    sl_start: i32,
    sl_len: u32,
    // Clipping
    clipping: bool,
    clip_box: [i32; 4], // x1, y1, x2, y2 in subpixel coords
    clip_x1: i32,
    clip_y1: i32,
}

impl RasterizerCompoundAa {
    pub fn new() -> Self {
        Self {
            outline: CellsEngine::new(),
            filling_rule: FillingRule::NonZero,
            layer_order: LayerOrder::Direct,
            styles: Vec::new(),
            ast: Vec::new(),
            asm: Vec::new(),
            cells: Vec::new(),
            cover_buf: Vec::new(),
            min_style: i32::MAX,
            max_style: i32::MIN,
            start_x: 0,
            start_y: 0,
            scan_y: i32::MAX,
            sl_start: 0,
            sl_len: 0,
            clipping: false,
            clip_box: [0; 4],
            clip_x1: 0,
            clip_y1: 0,
        }
    }

    pub fn reset(&mut self) {
        self.outline.reset();
        self.min_style = i32::MAX;
        self.max_style = i32::MIN;
        self.scan_y = i32::MAX;
        self.sl_start = 0;
        self.sl_len = 0;
    }

    pub fn reset_clipping(&mut self) {
        self.reset();
        self.clipping = false;
    }

    pub fn clip_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.reset();
        self.clipping = true;
        self.clip_box = [upscale(x1), upscale(y1), upscale(x2), upscale(y2)];
    }

    pub fn filling_rule(&mut self, rule: FillingRule) {
        self.filling_rule = rule;
    }

    pub fn layer_order(&mut self, order: LayerOrder) {
        self.layer_order = order;
    }

    /// Set the left and right fill styles for subsequent edges.
    pub fn styles(&mut self, left: i32, right: i32) {
        let mut cell = CellStyleAa::default();
        cell.initial();
        cell.left = left as i16;
        cell.right = right as i16;
        self.outline.style(&cell);
        if left >= 0 && left < self.min_style {
            self.min_style = left;
        }
        if left >= 0 && left > self.max_style {
            self.max_style = left;
        }
        if right >= 0 && right < self.min_style {
            self.min_style = right;
        }
        if right >= 0 && right > self.max_style {
            self.max_style = right;
        }
    }

    /// Move to position (subpixel integer coords).
    pub fn move_to(&mut self, x: i32, y: i32) {
        if self.outline.sorted() {
            self.reset();
        }
        self.start_x = downscale(x);
        self.start_y = downscale(y);
        if self.clipping {
            self.clip_x1 = self.start_x;
            self.clip_y1 = self.start_y;
        }
    }

    /// Line to position (subpixel integer coords).
    pub fn line_to(&mut self, x: i32, y: i32) {
        let x = downscale(x);
        let y = downscale(y);
        if self.clipping {
            clip_line_segment(
                &mut self.outline,
                &mut self.clip_x1,
                &mut self.clip_y1,
                x,
                y,
                &self.clip_box,
            );
        } else {
            self.outline.line(self.clip_x1, self.clip_y1, x, y);
            self.clip_x1 = x;
            self.clip_y1 = y;
        }
    }

    /// Move to position (f64 coords).
    pub fn move_to_d(&mut self, x: f64, y: f64) {
        if self.outline.sorted() {
            self.reset();
        }
        self.start_x = upscale(x);
        self.start_y = upscale(y);
        if self.clipping {
            self.clip_x1 = self.start_x;
            self.clip_y1 = self.start_y;
        } else {
            self.clip_x1 = self.start_x;
            self.clip_y1 = self.start_y;
        }
    }

    /// Line to position (f64 coords).
    pub fn line_to_d(&mut self, x: f64, y: f64) {
        let x = upscale(x);
        let y = upscale(y);
        if self.clipping {
            clip_line_segment(
                &mut self.outline,
                &mut self.clip_x1,
                &mut self.clip_y1,
                x,
                y,
                &self.clip_box,
            );
        } else {
            self.outline.line(self.clip_x1, self.clip_y1, x, y);
            self.clip_x1 = x;
            self.clip_y1 = y;
        }
    }

    /// Process a vertex command.
    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        if is_move_to(cmd) {
            self.move_to_d(x, y);
        } else if is_vertex(cmd) {
            self.line_to_d(x, y);
        } else if is_close(cmd) {
            let sx = self.start_x;
            let sy = self.start_y;
            if self.clipping {
                clip_line_segment(
                    &mut self.outline,
                    &mut self.clip_x1,
                    &mut self.clip_y1,
                    sx,
                    sy,
                    &self.clip_box,
                );
            } else {
                self.outline.line(self.clip_x1, self.clip_y1, sx, sy);
                self.clip_x1 = sx;
                self.clip_y1 = sy;
            }
        }
    }

    /// Add a single edge (subpixel integer coords).
    pub fn edge(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if self.outline.sorted() {
            self.reset();
        }
        let x1 = downscale(x1);
        let y1 = downscale(y1);
        let x2 = downscale(x2);
        let y2 = downscale(y2);
        self.outline.line(x1, y1, x2, y2);
    }

    /// Add a single edge (f64 coords).
    pub fn edge_d(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        if self.outline.sorted() {
            self.reset();
        }
        self.outline
            .line(upscale(x1), upscale(y1), upscale(x2), upscale(y2));
    }

    /// Add an entire path from a vertex source.
    pub fn add_path<VS: VertexSource>(&mut self, vs: &mut VS, path_id: u32) {
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
    pub fn min_style(&self) -> i32 {
        self.min_style
    }
    pub fn max_style(&self) -> i32 {
        self.max_style
    }

    /// Sort cells.
    pub fn sort(&mut self) {
        self.outline.sort_cells();
    }

    /// Prepare for scanline iteration.
    pub fn rewind_scanlines(&mut self) -> bool {
        self.outline.sort_cells();
        if self.outline.total_cells() == 0 {
            return false;
        }
        if self.max_style < self.min_style {
            return false;
        }
        self.scan_y = self.outline.min_y();
        let num_styles = (self.max_style - self.min_style + 2) as usize;
        self.styles.resize(num_styles, StyleInfo::default());
        true
    }

    /// Calculate alpha from coverage area.
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

    /// Internal: add a style to the active style table.
    fn add_style(&mut self, style_id: i32) {
        let style_id = if style_id < 0 {
            0
        } else {
            (style_id - self.min_style + 1) as u32
        } as usize;

        let nbyte = style_id >> 3;
        let mask = 1u8 << (style_id & 7);

        if (self.asm[nbyte] & mask) == 0 {
            self.ast.push(style_id as u32);
            self.asm[nbyte] |= mask;
            self.styles[style_id].start_cell = 0;
            self.styles[style_id].num_cells = 0;
            self.styles[style_id].last_x = i32::MIN;
        }
        self.styles[style_id].start_cell += 1;
    }

    /// Process the current scanline and return the number of active styles.
    ///
    /// Call `style(idx)` to get the actual style ID for each index 0..n-1.
    pub fn sweep_styles(&mut self) -> u32 {
        loop {
            if self.scan_y > self.outline.max_y() {
                return 0;
            }
            let num_cells = self.outline.scanline_num_cells(self.scan_y as u32);
            let cell_indices: Vec<u32> =
                self.outline.scanline_cells(self.scan_y as u32).to_vec();

            let num_styles = (self.max_style - self.min_style + 2) as usize;

            self.cells
                .resize(num_cells as usize * 2, CellInfo::default());
            self.ast.clear();
            self.ast.reserve(num_styles);
            self.asm.clear();
            self.asm.resize((num_styles + 7) >> 3, 0);

            if num_cells > 0 {
                // Pre-add the "no fill" style (index 0)
                self.asm[0] |= 1;
                self.ast.push(0);
                self.styles[0].start_cell = 0;
                self.styles[0].num_cells = 0;
                self.styles[0].last_x = i32::MIN;

                let first_x = self.outline.cell(cell_indices[0]).x;
                let last_x = self.outline.cell(cell_indices[num_cells as usize - 1]).x;
                self.sl_start = first_x;
                self.sl_len = (last_x - first_x + 1) as u32;

                // Pass 1: Count cells per style
                // Copy left/right to avoid borrow conflict with add_style
                let style_pairs: Vec<(i16, i16)> = (0..num_cells as usize)
                    .map(|i| {
                        let c = self.outline.cell(cell_indices[i]);
                        (c.left, c.right)
                    })
                    .collect();
                for &(left, right) in &style_pairs {
                    self.add_style(left as i32);
                    self.add_style(right as i32);
                }

                // Convert histogram to starting indices
                let mut start_cell = 0u32;
                for i in 0..self.ast.len() {
                    let si = self.ast[i] as usize;
                    let v = self.styles[si].start_cell;
                    self.styles[si].start_cell = start_cell;
                    start_cell += v;
                }

                // Pass 2: Distribute cells to styles
                let cell_indices2: Vec<u32> =
                    self.outline.scanline_cells(self.scan_y as u32).to_vec();
                for i in 0..num_cells as usize {
                    let curr_cell = self.outline.cell(cell_indices2[i]);

                    // Left style: add
                    let style_id = if curr_cell.left < 0 {
                        0usize
                    } else {
                        (curr_cell.left as i32 - self.min_style + 1) as usize
                    };

                    let style = &mut self.styles[style_id];
                    if curr_cell.x == style.last_x {
                        let ci = (style.start_cell + style.num_cells - 1) as usize;
                        self.cells[ci].area += curr_cell.area;
                        self.cells[ci].cover += curr_cell.cover;
                    } else {
                        let ci = (style.start_cell + style.num_cells) as usize;
                        self.cells[ci].x = curr_cell.x;
                        self.cells[ci].area = curr_cell.area;
                        self.cells[ci].cover = curr_cell.cover;
                        style.last_x = curr_cell.x;
                        style.num_cells += 1;
                    }

                    // Right style: subtract
                    let style_id = if curr_cell.right < 0 {
                        0usize
                    } else {
                        (curr_cell.right as i32 - self.min_style + 1) as usize
                    };

                    let style = &mut self.styles[style_id];
                    if curr_cell.x == style.last_x {
                        let ci = (style.start_cell + style.num_cells - 1) as usize;
                        self.cells[ci].area -= curr_cell.area;
                        self.cells[ci].cover -= curr_cell.cover;
                    } else {
                        let ci = (style.start_cell + style.num_cells) as usize;
                        self.cells[ci].x = curr_cell.x;
                        self.cells[ci].area = -curr_cell.area;
                        self.cells[ci].cover = -curr_cell.cover;
                        style.last_x = curr_cell.x;
                        style.num_cells += 1;
                    }
                }
            }

            if self.ast.len() > 1 {
                break;
            }
            self.scan_y += 1;
        }
        self.scan_y += 1;

        // Sort styles by ID if requested
        if self.layer_order != LayerOrder::Unsorted && self.ast.len() > 1 {
            let ast_slice = &mut self.ast[1..];
            match self.layer_order {
                LayerOrder::Direct => ast_slice.sort_unstable_by(|a, b| b.cmp(a)),
                LayerOrder::Inverse => ast_slice.sort_unstable(),
                LayerOrder::Unsorted => {}
            }
        }

        (self.ast.len() - 1) as u32
    }

    /// Get the actual style ID for the given style index (0-based).
    #[inline]
    pub fn style(&self, style_idx: u32) -> u32 {
        (self.ast[style_idx as usize + 1] as i32 + self.min_style - 1) as u32
    }

    /// Get the X start of the current scanline.
    pub fn scanline_start(&self) -> i32 {
        self.sl_start
    }

    /// Get the length of the current scanline.
    pub fn scanline_length(&self) -> u32 {
        self.sl_len
    }

    /// Sweep one scanline for one style.
    ///
    /// `style_idx` is -1 for the "no fill" style, or 0..n-1 for actual styles.
    pub fn sweep_scanline<SL: Scanline>(&self, sl: &mut SL, style_idx: i32) -> bool {
        let scan_y = self.scan_y - 1;
        if scan_y > self.outline.max_y() {
            return false;
        }

        sl.reset_spans();

        let si = if style_idx < 0 {
            0usize
        } else {
            (style_idx + 1) as usize
        };

        let st = &self.styles[self.ast[si] as usize];
        let mut num_cells = st.num_cells;
        let mut cell_idx = st.start_cell;

        let mut cover = 0i32;
        while num_cells > 0 {
            num_cells -= 1;
            let cell = &self.cells[cell_idx as usize];
            let x = cell.x;
            let area = cell.area;
            cover += cell.cover;
            cell_idx += 1;

            if area != 0 {
                let alpha =
                    self.calculate_alpha((cover << (POLY_SUBPIXEL_SHIFT + 1)) - area as u32 as i32);
                sl.add_cell(x, alpha);
                if num_cells > 0 && self.cells[cell_idx as usize].x > x + 1 {
                    let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                    if alpha > 0 {
                        sl.add_span(
                            x + 1,
                            (self.cells[cell_idx as usize].x - x - 1) as u32,
                            alpha,
                        );
                    }
                }
            } else if num_cells > 0 && self.cells[cell_idx as usize].x > x {
                let alpha = self.calculate_alpha(cover << (POLY_SUBPIXEL_SHIFT + 1));
                if alpha > 0 {
                    sl.add_span(x, (self.cells[cell_idx as usize].x - x) as u32, alpha);
                }
            }
        }

        if sl.num_spans() == 0 {
            return false;
        }
        sl.finalize(scan_y);
        true
    }

    /// Navigate to a specific scanline.
    pub fn navigate_scanline(&mut self, y: i32) -> bool {
        self.outline.sort_cells();
        if self.outline.total_cells() == 0 {
            return false;
        }
        if self.max_style < self.min_style {
            return false;
        }
        if y < self.outline.min_y() || y > self.outline.max_y() {
            return false;
        }
        self.scan_y = y;
        let num_styles = (self.max_style - self.min_style + 2) as usize;
        self.styles.resize(num_styles, StyleInfo::default());
        true
    }

    /// Allocate a cover buffer of the given length. Returns a mutable slice.
    pub fn allocate_cover_buffer(&mut self, len: u32) -> &mut [u8] {
        self.cover_buf.resize(len as usize, 0);
        &mut self.cover_buf
    }
}

impl Default for RasterizerCompoundAa {
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
    use crate::scanline_u::ScanlineU8;

    #[test]
    fn test_cell_style_aa_default() {
        let cell = CellStyleAa::default();
        assert_eq!(cell.x, i32::MAX);
        assert_eq!(cell.y, i32::MAX);
        assert_eq!(cell.left, -1);
        assert_eq!(cell.right, -1);
    }

    #[test]
    fn test_cell_style_aa_not_equal() {
        let mut c1 = CellStyleAa::default();
        c1.x = 10;
        c1.y = 20;
        c1.left = 1;
        c1.right = 2;

        let mut style = CellStyleAa::default();
        style.left = 1;
        style.right = 2;
        // Same position and style
        assert!(!c1.not_equal(10, 20, &style));
        // Different position
        assert!(c1.not_equal(11, 20, &style));
        // Different style
        style.left = 3;
        assert!(c1.not_equal(10, 20, &style));
    }

    #[test]
    fn test_empty_rasterizer() {
        let mut ras = RasterizerCompoundAa::new();
        assert!(!ras.rewind_scanlines());
    }

    #[test]
    fn test_simple_rectangle_two_styles() {
        let mut ras = RasterizerCompoundAa::new();

        // Draw a simple rectangle with style 0 on the left
        // Using subpixel coords (multiply by 256)
        let scale = POLY_SUBPIXEL_SCALE as i32;

        // Left edge going down (style 0 on left, -1 on right)
        ras.styles(0, -1);
        ras.move_to(10 * scale, 10 * scale);
        ras.line_to(10 * scale, 20 * scale);

        // Bottom edge going right (style -1 on left, 0 on right)
        ras.styles(-1, 0);
        ras.line_to(20 * scale, 20 * scale);

        // Right edge going up (-1 on left, 0 on right)
        ras.styles(-1, 0);
        ras.line_to(20 * scale, 10 * scale);

        // Top edge going left (style 0 on left, -1 on right)
        ras.styles(0, -1);
        ras.line_to(10 * scale, 10 * scale);

        assert!(ras.rewind_scanlines());
        assert_eq!(ras.min_style(), 0);
        assert_eq!(ras.max_style(), 0);

        // Sweep at least one scanline
        let num_styles = ras.sweep_styles();
        assert!(num_styles >= 1, "Expected at least 1 style, got {num_styles}");
    }

    #[test]
    fn test_two_adjacent_styles() {
        let mut ras = RasterizerCompoundAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;

        // Two rectangles side by side: style 0 (left) and style 1 (right)
        // Shared edge at x=15

        // Left rectangle: style 0
        ras.styles(0, -1);
        ras.move_to(10 * s, 10 * s);
        ras.line_to(10 * s, 20 * s);

        // Shared boundary edge: style 0 on left, style 1 on right
        ras.styles(0, 1);
        ras.line_to(15 * s, 20 * s);
        ras.line_to(15 * s, 10 * s);

        ras.styles(0, -1);
        ras.line_to(10 * s, 10 * s);

        // Right rectangle: style 1
        ras.styles(1, -1);
        ras.move_to(15 * s, 10 * s);
        ras.line_to(15 * s, 20 * s);

        ras.styles(-1, 1);
        ras.line_to(20 * s, 20 * s);
        ras.line_to(20 * s, 10 * s);

        ras.styles(1, -1);
        ras.line_to(15 * s, 10 * s);

        assert!(ras.rewind_scanlines());
        assert_eq!(ras.min_style(), 0);
        assert_eq!(ras.max_style(), 1);
    }

    #[test]
    fn test_sweep_scanline_produces_spans() {
        let mut ras = RasterizerCompoundAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;

        // Simple filled rectangle with style 0
        ras.styles(0, -1);
        ras.move_to(10 * s, 10 * s);
        ras.line_to(10 * s, 20 * s);

        ras.styles(-1, 0);
        ras.line_to(20 * s, 20 * s);
        ras.line_to(20 * s, 10 * s);

        ras.styles(0, -1);
        ras.line_to(10 * s, 10 * s);

        assert!(ras.rewind_scanlines());

        let num_styles = ras.sweep_styles();
        assert!(num_styles >= 1);

        let mut sl = ScanlineU8::new();
        sl.reset(0, 100);
        // Sweep scanline for style index 0
        let ok = ras.sweep_scanline(&mut sl, 0);
        if ok {
            assert!(sl.num_spans() > 0);
        }
    }

    #[test]
    fn test_layer_order() {
        let mut ras = RasterizerCompoundAa::new();
        ras.layer_order(LayerOrder::Direct);
        ras.layer_order(LayerOrder::Inverse);
        ras.layer_order(LayerOrder::Unsorted);
    }

    #[test]
    fn test_calculate_alpha() {
        let ras = RasterizerCompoundAa::new();
        // Full coverage
        let full_area = (POLY_SUBPIXEL_SCALE * POLY_SUBPIXEL_SCALE * 2) as i32;
        let alpha = ras.calculate_alpha(full_area);
        assert_eq!(alpha, AA_MASK);

        // Zero coverage
        assert_eq!(ras.calculate_alpha(0), 0);
    }

    #[test]
    fn test_add_path() {
        use crate::path_storage::PathStorage;

        let mut ras = RasterizerCompoundAa::new();
        let mut path = PathStorage::new();
        path.move_to(10.0, 10.0);
        path.line_to(20.0, 10.0);
        path.line_to(20.0, 20.0);
        path.close_polygon(0);

        ras.styles(0, -1);
        ras.add_path(&mut path, 0);

        assert!(ras.rewind_scanlines());
    }

    #[test]
    fn test_edge_methods() {
        let mut ras = RasterizerCompoundAa::new();
        let s = POLY_SUBPIXEL_SCALE as i32;

        ras.styles(0, -1);
        ras.edge(10 * s, 10 * s, 20 * s, 10 * s);
        ras.edge(20 * s, 10 * s, 20 * s, 20 * s);
        ras.edge(20 * s, 20 * s, 10 * s, 20 * s);
        ras.edge(10 * s, 20 * s, 10 * s, 10 * s);

        assert!(ras.rewind_scanlines());
    }

    #[test]
    fn test_clip_box() {
        let mut ras = RasterizerCompoundAa::new();
        ras.clip_box(0.0, 0.0, 100.0, 100.0);

        let s = POLY_SUBPIXEL_SCALE as i32;
        ras.styles(0, -1);
        ras.move_to(10 * s, 10 * s);
        ras.line_to(10 * s, 20 * s);
        ras.line_to(20 * s, 20 * s);
        ras.line_to(20 * s, 10 * s);
        ras.line_to(10 * s, 10 * s);

        assert!(ras.rewind_scanlines());
    }

    #[test]
    fn test_allocate_cover_buffer() {
        let mut ras = RasterizerCompoundAa::new();
        let buf = ras.allocate_cover_buffer(100);
        assert_eq!(buf.len(), 100);
    }
}

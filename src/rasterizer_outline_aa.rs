//! Anti-aliased outline rasterizer.
//!
//! Port of `agg_rasterizer_outline_aa.h`.
//! Consumes a vertex source and renders anti-aliased outlines using
//! the `RendererOutlineAa` renderer.
//!
//! Copyright 2025.

use crate::basics::{is_close, is_end_poly, is_move_to, is_stop, VertexSource};
use crate::line_aa_basics::*;
use crate::renderer_outline_aa::OutlineAaRenderer;

/// Join type for outline AA lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineAaJoin {
    NoJoin,
    Miter,
    Round,
    MiterAccurate,
}

/// Vertex with distance for AA line rendering.
/// Port of C++ `line_aa_vertex`.
#[derive(Debug, Clone, Copy)]
pub struct LineAaVertex {
    pub x: i32,
    pub y: i32,
    pub len: i32,
}

impl LineAaVertex {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y, len: 0 }
    }

    /// Calculate distance to another vertex and store in self.len.
    /// Returns true if distance > threshold (i.e., not coincident).
    /// This is the equivalent of C++ `operator()`.
    pub fn calc_distance(&mut self, other: &LineAaVertex) -> bool {
        let dx = (other.x - self.x) as f64;
        let dy = (other.y - self.y) as f64;
        self.len = (dx * dx + dy * dy).sqrt().round() as i32;
        self.len > (LINE_SUBPIXEL_SCALE + LINE_SUBPIXEL_SCALE / 2)
    }
}

// ============================================================================
// Vertex Sequence — mirrors C++ vertex_sequence<line_aa_vertex, 6>
// ============================================================================

/// A sequence of vertices that automatically removes coincident points.
/// Port of C++ `vertex_sequence`.
struct VertexSeq {
    data: Vec<LineAaVertex>,
}

impl VertexSeq {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn size(&self) -> usize {
        self.data.len()
    }

    fn remove_all(&mut self) {
        self.data.clear();
    }

    /// Add a vertex, removing the previous last if it was too close to its predecessor.
    /// Port of C++ `vertex_sequence::add`.
    fn add(&mut self, val: LineAaVertex) {
        if self.data.len() > 1 {
            let n = self.data.len();
            let last = self.data[n - 1];
            if !self.data[n - 2].calc_distance(&last) {
                self.data.pop();
            }
        }
        self.data.push(val);
    }

    /// Replace the last element.
    /// Port of C++ `vertex_sequence::modify_last`.
    fn modify_last(&mut self, val: LineAaVertex) {
        if !self.data.is_empty() {
            self.data.pop();
        }
        self.add(val);
    }

    /// Close the sequence, removing coincident vertices.
    /// Port of C++ `vertex_sequence::close`.
    fn close(&mut self, closed: bool) {
        // First: trim coincident tail vertices
        while self.data.len() > 1 {
            let n = self.data.len();
            let last = self.data[n - 1];
            if self.data[n - 2].calc_distance(&last) {
                break;
            }
            let t = self.data.pop().unwrap();
            self.modify_last(t);
        }

        // If closed: remove last vertex if it coincides with first
        if closed {
            while self.data.len() > 1 {
                let n = self.data.len();
                let first = self.data[0];
                if self.data[n - 1].calc_distance(&first) {
                    break;
                }
                self.data.pop();
            }
        }
    }

    fn get(&self, idx: usize) -> &LineAaVertex {
        &self.data[idx]
    }

    fn get_mut(&mut self, idx: usize) -> &mut LineAaVertex {
        &mut self.data[idx]
    }
}

impl std::ops::Index<usize> for VertexSeq {
    type Output = LineAaVertex;
    fn index(&self, idx: usize) -> &LineAaVertex {
        &self.data[idx]
    }
}

// ============================================================================
// Draw Variables — mirrors C++ draw_vars
// ============================================================================

struct DrawVars {
    idx: usize,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    curr: LineParameters,
    next: LineParameters,
    lcurr: i32,
    lnext: i32,
    xb1: i32,
    yb1: i32,
    xb2: i32,
    yb2: i32,
    flags: u32,
}

// ============================================================================
// RasterizerOutlineAa
// ============================================================================

/// Anti-aliased outline rasterizer.
///
/// Port of C++ `rasterizer_outline_aa<Renderer>`.
/// Builds polylines from vertex sources, then dispatches to the renderer
/// for AA line drawing with configurable join types.
pub struct RasterizerOutlineAa {
    src_vertices: VertexSeq,
    line_join: OutlineAaJoin,
    round_cap: bool,
    start_x: i32,
    start_y: i32,
}

impl RasterizerOutlineAa {
    pub fn new() -> Self {
        Self {
            src_vertices: VertexSeq::new(),
            line_join: OutlineAaJoin::NoJoin,
            round_cap: false,
            start_x: 0,
            start_y: 0,
        }
    }

    pub fn set_line_join(&mut self, join: OutlineAaJoin) {
        self.line_join = join;
    }

    pub fn line_join(&self) -> OutlineAaJoin {
        self.line_join
    }

    pub fn set_round_cap(&mut self, v: bool) {
        self.round_cap = v;
    }

    pub fn round_cap(&self) -> bool {
        self.round_cap
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.start_x = x;
        self.start_y = y;
        self.src_vertices.modify_last(LineAaVertex::new(x, y));
    }

    pub fn line_to(&mut self, x: i32, y: i32) {
        self.src_vertices.add(LineAaVertex::new(x, y));
    }

    pub fn move_to_d(&mut self, x: f64, y: f64) {
        self.move_to(line_coord(x), line_coord(y));
    }

    pub fn line_to_d(&mut self, x: f64, y: f64) {
        self.line_to(line_coord(x), line_coord(y));
    }

    /// Process a single vertex command. Port of C++ `add_vertex`.
    fn add_vertex<R: OutlineAaRenderer>(
        &mut self,
        x: f64,
        y: f64,
        cmd: u32,
        ren: &mut R,
    ) {
        if is_move_to(cmd) {
            self.render(ren, false);
            self.move_to_d(x, y);
        } else if is_end_poly(cmd) {
            self.render(ren, is_close(cmd));
            if is_close(cmd) {
                self.move_to(self.start_x, self.start_y);
            }
        } else {
            self.line_to_d(x, y);
        }
    }

    /// Add a path from a vertex source and render it.
    pub fn add_path<VS: VertexSource, R: OutlineAaRenderer>(
        &mut self,
        vs: &mut VS,
        path_id: u32,
        ren: &mut R,
    ) {
        vs.rewind(path_id);
        let (mut x, mut y) = (0.0, 0.0);
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.add_vertex(x, y, cmd, ren);
        }
        // C++ has render(false) at the end to flush any remaining open polyline
        self.render(ren, false);
    }

    // ========================================================================
    // draw() — Port of C++ draw(draw_vars&, unsigned start, unsigned end)
    // ========================================================================

    fn draw<R: OutlineAaRenderer>(
        &self,
        dv: &mut DrawVars,
        start: usize,
        end: usize,
        ren: &mut R,
    ) {
        for _i in start..end {
            if self.line_join == OutlineAaJoin::Round {
                dv.xb1 = dv.curr.x1 + (dv.curr.y2 - dv.curr.y1);
                dv.yb1 = dv.curr.y1 - (dv.curr.x2 - dv.curr.x1);
                dv.xb2 = dv.curr.x2 + (dv.curr.y2 - dv.curr.y1);
                dv.yb2 = dv.curr.y2 - (dv.curr.x2 - dv.curr.x1);
            }

            match dv.flags {
                0 => ren.line3(&dv.curr, dv.xb1, dv.yb1, dv.xb2, dv.yb2),
                1 => ren.line2(&dv.curr, dv.xb2, dv.yb2),
                2 => ren.line1(&dv.curr, dv.xb1, dv.yb1),
                _ => ren.line0(&dv.curr),
            }

            if self.line_join == OutlineAaJoin::Round && (dv.flags & 2) == 0 {
                ren.pie(
                    dv.curr.x2,
                    dv.curr.y2,
                    dv.curr.x2 + (dv.curr.y2 - dv.curr.y1),
                    dv.curr.y2 - (dv.curr.x2 - dv.curr.x1),
                    dv.curr.x2 + (dv.next.y2 - dv.next.y1),
                    dv.curr.y2 - (dv.next.x2 - dv.next.x1),
                );
            }

            dv.x1 = dv.x2;
            dv.y1 = dv.y2;
            dv.lcurr = dv.lnext;
            dv.lnext = self.src_vertices[dv.idx].len;

            dv.idx += 1;
            if dv.idx >= self.src_vertices.size() {
                dv.idx = 0;
            }

            let v = self.src_vertices.get(dv.idx);
            dv.x2 = v.x;
            dv.y2 = v.y;

            dv.curr = dv.next;
            dv.next = LineParameters::new(dv.x1, dv.y1, dv.x2, dv.y2, dv.lnext);
            dv.xb1 = dv.xb2;
            dv.yb1 = dv.yb2;

            match self.line_join {
                OutlineAaJoin::NoJoin => {
                    dv.flags = 3;
                }
                OutlineAaJoin::Miter => {
                    dv.flags >>= 1;
                    dv.flags |= if dv.curr.diagonal_quadrant() == dv.next.diagonal_quadrant() {
                        2
                    } else {
                        0
                    };
                    if (dv.flags & 2) == 0 {
                        bisectrix(&dv.curr, &dv.next, &mut dv.xb2, &mut dv.yb2);
                    }
                }
                OutlineAaJoin::Round => {
                    dv.flags >>= 1;
                    dv.flags |= if dv.curr.diagonal_quadrant() == dv.next.diagonal_quadrant() {
                        2
                    } else {
                        0
                    };
                }
                OutlineAaJoin::MiterAccurate => {
                    dv.flags = 0;
                    bisectrix(&dv.curr, &dv.next, &mut dv.xb2, &mut dv.yb2);
                }
            }
        }
    }

    // ========================================================================
    // render() — Port of C++ render(bool close_polygon)
    // ========================================================================

    pub fn render<R: OutlineAaRenderer>(
        &mut self,
        ren: &mut R,
        close_polygon: bool,
    ) {
        self.src_vertices.close(close_polygon);

        // Match C++ behavior: when the renderer only supports accurate joins
        // (e.g. image pattern renderer), override the join type to MiterAccurate.
        // In C++, this is done in the constructor; here we do it per-call since
        // the Rust rasterizer is not parameterized by renderer type.
        let saved_join = self.line_join;
        if ren.accurate_join_only() {
            self.line_join = OutlineAaJoin::MiterAccurate;
        }

        if close_polygon {
            // ------- Closed polygon -------
            if self.src_vertices.size() >= 3 {
                let mut dv = DrawVars {
                    idx: 2,
                    x1: 0, y1: 0, x2: 0, y2: 0,
                    curr: LineParameters::new(0, 0, 1, 0, 1), // placeholder
                    next: LineParameters::new(0, 0, 1, 0, 1),
                    lcurr: 0, lnext: 0,
                    xb1: 0, yb1: 0, xb2: 0, yb2: 0,
                    flags: 0,
                };

                let n = self.src_vertices.size();

                let v_last = self.src_vertices[n - 1];
                let x1 = v_last.x;
                let y1 = v_last.y;
                let lprev = v_last.len;

                let v0 = self.src_vertices[0];
                let x2 = v0.x;
                let y2 = v0.y;
                dv.lcurr = v0.len;
                let prev = LineParameters::new(x1, y1, x2, y2, lprev);

                let v1 = self.src_vertices[1];
                dv.x1 = v1.x;
                dv.y1 = v1.y;
                dv.lnext = v1.len;
                dv.curr = LineParameters::new(x2, y2, dv.x1, dv.y1, dv.lcurr);

                let v2 = self.src_vertices[dv.idx];
                dv.x2 = v2.x;
                dv.y2 = v2.y;
                dv.next = LineParameters::new(dv.x1, dv.y1, dv.x2, dv.y2, dv.lnext);

                match self.line_join {
                    OutlineAaJoin::NoJoin => {
                        dv.flags = 3;
                    }
                    OutlineAaJoin::Miter | OutlineAaJoin::Round => {
                        let f1 = if prev.diagonal_quadrant() == dv.curr.diagonal_quadrant() { 1 } else { 0 };
                        let f2 = if dv.curr.diagonal_quadrant() == dv.next.diagonal_quadrant() { 2 } else { 0 };
                        dv.flags = f1 | f2;
                    }
                    OutlineAaJoin::MiterAccurate => {
                        dv.flags = 0;
                    }
                }

                if (dv.flags & 1) == 0 && self.line_join != OutlineAaJoin::Round {
                    bisectrix(&prev, &dv.curr, &mut dv.xb1, &mut dv.yb1);
                }
                if (dv.flags & 2) == 0 && self.line_join != OutlineAaJoin::Round {
                    bisectrix(&dv.curr, &dv.next, &mut dv.xb2, &mut dv.yb2);
                }

                self.draw(&mut dv, 0, n, ren);
            }
        } else {
            // ------- Open polyline -------
            let n = self.src_vertices.size();

            match n {
                0 | 1 => {} // nothing to draw
                2 => {
                    let v0 = self.src_vertices[0];
                    let x1 = v0.x;
                    let y1 = v0.y;
                    let lprev = v0.len;
                    let v1 = self.src_vertices[1];
                    let x2 = v1.x;
                    let y2 = v1.y;
                    let lp = LineParameters::new(x1, y1, x2, y2, lprev);

                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_start,
                            x1, y1,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                        );
                    }
                    ren.line3(
                        &lp,
                        x1 + (y2 - y1), y1 - (x2 - x1),
                        x2 + (y2 - y1), y2 - (x2 - x1),
                    );
                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_end,
                            x2, y2,
                            x2 + (y2 - y1), y2 - (x2 - x1),
                        );
                    }
                }
                3 => {
                    let v0 = self.src_vertices[0];
                    let x1 = v0.x;
                    let y1 = v0.y;
                    let lprev = v0.len;
                    let v1 = self.src_vertices[1];
                    let x2 = v1.x;
                    let y2 = v1.y;
                    let lnext = v1.len;
                    let v2 = self.src_vertices[2];
                    let x3 = v2.x;
                    let y3 = v2.y;
                    let lp1 = LineParameters::new(x1, y1, x2, y2, lprev);
                    let lp2 = LineParameters::new(x2, y2, x3, y3, lnext);

                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_start,
                            x1, y1,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                        );
                    }

                    if self.line_join == OutlineAaJoin::Round {
                        ren.line3(
                            &lp1,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                            x2 + (y2 - y1), y2 - (x2 - x1),
                        );
                        ren.pie(
                            x2, y2,
                            x2 + (y2 - y1), y2 - (x2 - x1),
                            x2 + (y3 - y2), y2 - (x3 - x2),
                        );
                        ren.line3(
                            &lp2,
                            x2 + (y3 - y2), y2 - (x3 - x2),
                            x3 + (y3 - y2), y3 - (x3 - x2),
                        );
                    } else {
                        let (mut xb1, mut yb1) = (0i32, 0i32);
                        bisectrix(&lp1, &lp2, &mut xb1, &mut yb1);
                        ren.line3(
                            &lp1,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                            xb1, yb1,
                        );
                        ren.line3(
                            &lp2,
                            xb1, yb1,
                            x3 + (y3 - y2), y3 - (x3 - x2),
                        );
                    }

                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_end,
                            x3, y3,
                            x3 + (y3 - y2), y3 - (x3 - x2),
                        );
                    }
                }
                _ => {
                    // General case: 4+ vertices, open polyline
                    let mut dv = DrawVars {
                        idx: 3,
                        x1: 0, y1: 0, x2: 0, y2: 0,
                        curr: LineParameters::new(0, 0, 1, 0, 1),
                        next: LineParameters::new(0, 0, 1, 0, 1),
                        lcurr: 0, lnext: 0,
                        xb1: 0, yb1: 0, xb2: 0, yb2: 0,
                        flags: 0,
                    };

                    let v0 = self.src_vertices[0];
                    let x1 = v0.x;
                    let y1 = v0.y;
                    let lprev = v0.len;

                    let v1 = self.src_vertices[1];
                    let x2 = v1.x;
                    let y2 = v1.y;
                    dv.lcurr = v1.len;
                    let prev = LineParameters::new(x1, y1, x2, y2, lprev);

                    let v2 = self.src_vertices[2];
                    dv.x1 = v2.x;
                    dv.y1 = v2.y;
                    dv.lnext = v2.len;
                    dv.curr = LineParameters::new(x2, y2, dv.x1, dv.y1, dv.lcurr);

                    let v3 = self.src_vertices[dv.idx];
                    dv.x2 = v3.x;
                    dv.y2 = v3.y;
                    dv.next = LineParameters::new(dv.x1, dv.y1, dv.x2, dv.y2, dv.lnext);

                    match self.line_join {
                        OutlineAaJoin::NoJoin => {
                            dv.flags = 3;
                        }
                        OutlineAaJoin::Miter | OutlineAaJoin::Round => {
                            let f1 = if prev.diagonal_quadrant() == dv.curr.diagonal_quadrant() { 1 } else { 0 };
                            let f2 = if dv.curr.diagonal_quadrant() == dv.next.diagonal_quadrant() { 2 } else { 0 };
                            dv.flags = f1 | f2;
                        }
                        OutlineAaJoin::MiterAccurate => {
                            dv.flags = 0;
                        }
                    }

                    // Start cap
                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_start,
                            x1, y1,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                        );
                    }

                    // First segment
                    if (dv.flags & 1) == 0 {
                        if self.line_join == OutlineAaJoin::Round {
                            ren.line3(
                                &prev,
                                x1 + (y2 - y1), y1 - (x2 - x1),
                                x2 + (y2 - y1), y2 - (x2 - x1),
                            );
                            ren.pie(
                                prev.x2, prev.y2,
                                x2 + (y2 - y1), y2 - (x2 - x1),
                                dv.curr.x1 + (dv.curr.y2 - dv.curr.y1),
                                dv.curr.y1 - (dv.curr.x2 - dv.curr.x1),
                            );
                        } else {
                            bisectrix(&prev, &dv.curr, &mut dv.xb1, &mut dv.yb1);
                            ren.line3(
                                &prev,
                                x1 + (y2 - y1), y1 - (x2 - x1),
                                dv.xb1, dv.yb1,
                            );
                        }
                    } else {
                        ren.line1(
                            &prev,
                            x1 + (y2 - y1), y1 - (x2 - x1),
                        );
                    }

                    if (dv.flags & 2) == 0 && self.line_join != OutlineAaJoin::Round {
                        bisectrix(&dv.curr, &dv.next, &mut dv.xb2, &mut dv.yb2);
                    }

                    // Middle segments
                    self.draw(&mut dv, 1, n - 2, ren);

                    // Last segment
                    if (dv.flags & 1) == 0 {
                        if self.line_join == OutlineAaJoin::Round {
                            ren.line3(
                                &dv.curr,
                                dv.curr.x1 + (dv.curr.y2 - dv.curr.y1),
                                dv.curr.y1 - (dv.curr.x2 - dv.curr.x1),
                                dv.curr.x2 + (dv.curr.y2 - dv.curr.y1),
                                dv.curr.y2 - (dv.curr.x2 - dv.curr.x1),
                            );
                        } else {
                            ren.line3(
                                &dv.curr,
                                dv.xb1, dv.yb1,
                                dv.curr.x2 + (dv.curr.y2 - dv.curr.y1),
                                dv.curr.y2 - (dv.curr.x2 - dv.curr.x1),
                            );
                        }
                    } else {
                        ren.line2(
                            &dv.curr,
                            dv.curr.x2 + (dv.curr.y2 - dv.curr.y1),
                            dv.curr.y2 - (dv.curr.x2 - dv.curr.x1),
                        );
                    }

                    // End cap
                    if self.round_cap {
                        ren.semidot(
                            cmp_dist_end,
                            dv.curr.x2, dv.curr.y2,
                            dv.curr.x2 + (dv.curr.y2 - dv.curr.y1),
                            dv.curr.y2 - (dv.curr.x2 - dv.curr.x1),
                        );
                    }
                }
            }
        }
        self.src_vertices.remove_all();
        self.line_join = saved_join;
    }
}

impl Default for RasterizerOutlineAa {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison function for start caps.
/// Port of C++ `cmp_dist_start` — returns true when d > 0.
fn cmp_dist_start(dist: i32) -> bool {
    dist > 0
}

/// Comparison function for end caps.
/// Port of C++ `cmp_dist_end` — returns true when d <= 0.
fn cmp_dist_end(dist: i32) -> bool {
    dist <= 0
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
    use crate::renderer_base::RendererBase;
    use crate::renderer_outline_aa::{LineProfileAa, RendererOutlineAa};
    use crate::rendering_buffer::RowAccessor;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * 4) as i32;
        let buf = vec![0u8; (h * w * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_rasterizer_creation() {
        let ras = RasterizerOutlineAa::new();
        assert_eq!(ras.line_join(), OutlineAaJoin::NoJoin);
        assert!(!ras.round_cap());
    }

    fn scan_for_color(ren_aa: &RendererOutlineAa<PixfmtRgba32>, cx: i32, cy: i32, radius: i32, channel: &str) -> bool {
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                if x < 0 || y < 0 { continue; }
                let p = ren_aa.ren().pixel(x, y);
                match channel {
                    "r" => { if p.r > 0 { return true; } }
                    "g" => { if p.g > 0 { return true; } }
                    "b" => { if p.b > 0 { return true; } }
                    _ => {}
                }
            }
        }
        false
    }

    #[test]
    fn test_rasterizer_two_points() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);
        let prof = LineProfileAa::with_width(2.0);
        let mut ren_aa = RendererOutlineAa::new(&mut ren, &prof);
        ren_aa.set_color(Rgba8::new(255, 0, 0, 255));

        let mut ras = RasterizerOutlineAa::new();
        ras.move_to_d(10.0, 50.0);
        ras.line_to_d(90.0, 50.0);
        ras.render(&mut ren_aa, false);

        assert!(scan_for_color(&ren_aa, 50, 50, 2, "r"), "Expected red pixels near (50,50)");
    }

    #[test]
    fn test_rasterizer_polyline() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);
        let prof = LineProfileAa::with_width(1.5);
        let mut ren_aa = RendererOutlineAa::new(&mut ren, &prof);
        ren_aa.set_color(Rgba8::new(0, 255, 0, 255));

        let mut ras = RasterizerOutlineAa::new();
        ras.set_line_join(OutlineAaJoin::Miter);
        ras.move_to_d(10.0, 10.0);
        ras.line_to_d(50.0, 50.0);
        ras.line_to_d(90.0, 10.0);
        ras.render(&mut ren_aa, false);

        assert!(scan_for_color(&ren_aa, 50, 50, 2, "g"), "Expected green pixels near (50,50)");
    }

    #[test]
    fn test_rasterizer_closed_polygon() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);
        let prof = LineProfileAa::with_width(1.0);
        let mut ren_aa = RendererOutlineAa::new(&mut ren, &prof);
        ren_aa.set_color(Rgba8::new(0, 0, 255, 255));

        let mut ras = RasterizerOutlineAa::new();
        ras.move_to_d(20.0, 20.0);
        ras.line_to_d(80.0, 20.0);
        ras.line_to_d(80.0, 80.0);
        ras.line_to_d(20.0, 80.0);
        ras.render(&mut ren_aa, true);

        assert!(scan_for_color(&ren_aa, 50, 20, 2, "b"), "Expected blue pixels near (50,20)");
    }
}

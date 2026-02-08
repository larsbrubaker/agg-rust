//! Anti-aliased outline rasterizer.
//!
//! Port of `agg_rasterizer_outline_aa.h`.
//! Consumes a vertex source and renders anti-aliased outlines using
//! the `RendererOutlineAa` renderer.

use crate::basics::{is_close, is_end_poly, is_move_to, is_stop, is_vertex, VertexSource};
use crate::line_aa_basics::*;
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_outline_aa::RendererOutlineAa;

/// Join type for outline AA lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineAaJoin {
    NoJoin,
    Miter,
    Round,
    MiterAccurate,
}

/// Vertex with distance for AA line rendering.
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

    /// Calculate distance to another vertex.
    /// Returns true if distance > threshold (i.e., not coincident).
    pub fn calc_distance(&mut self, other: &LineAaVertex) -> bool {
        let dx = (other.x - self.x) as f64;
        let dy = (other.y - self.y) as f64;
        self.len = (dx * dx + dy * dy).sqrt().round() as i32;
        self.len > (LINE_SUBPIXEL_SCALE + LINE_SUBPIXEL_SCALE / 2)
    }
}

/// Anti-aliased outline rasterizer.
///
/// Port of C++ `rasterizer_outline_aa<Renderer>`.
/// Builds polylines from vertex sources, then dispatches to the renderer
/// for AA line drawing with configurable join types.
pub struct RasterizerOutlineAa {
    src_vertices: Vec<LineAaVertex>,
    line_join: OutlineAaJoin,
    round_cap: bool,
    start_x: i32,
    start_y: i32,
}

impl RasterizerOutlineAa {
    pub fn new() -> Self {
        Self {
            src_vertices: Vec::new(),
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
        if !self.src_vertices.is_empty() {
            self.src_vertices.clear();
        }
        self.src_vertices.push(LineAaVertex::new(x, y));
    }

    pub fn line_to(&mut self, x: i32, y: i32) {
        let v = LineAaVertex::new(x, y);
        // Remove coincident vertices (like VertexSequence does)
        if let Some(last) = self.src_vertices.last_mut() {
            if !last.calc_distance(&v) {
                return; // too close, skip
            }
        }
        self.src_vertices.push(v);
    }

    pub fn move_to_d(&mut self, x: f64, y: f64) {
        self.move_to(line_coord(x), line_coord(y));
    }

    pub fn line_to_d(&mut self, x: f64, y: f64) {
        self.line_to(line_coord(x), line_coord(y));
    }

    /// Render the accumulated path.
    pub fn render<PF: PixelFormat>(
        &mut self,
        ren: &mut RendererOutlineAa<PF>,
        close_polygon: bool,
    ) where
        PF::ColorType: Default + Clone,
    {
        // Close: if closing polygon, compute distance from last to first
        // and remove coincident vertices at the boundary
        if close_polygon && self.src_vertices.len() >= 2 {
            let first = self.src_vertices[0];
            let last_idx = self.src_vertices.len() - 1;
            if !self.src_vertices[last_idx].calc_distance(&first) {
                self.src_vertices.pop();
            }
        }
        // Compute distance for the last vertex (to handle open polylines)
        if self.src_vertices.len() >= 2 {
            let n = self.src_vertices.len();
            let next = if close_polygon {
                self.src_vertices[0]
            } else {
                // For the last vertex in open polyline, len stays as-is from line_to
                self.src_vertices[n - 1]
            };
            let last_idx = n - 1;
            if close_polygon {
                self.src_vertices[last_idx].calc_distance(&next);
            }
        }
        let n = self.src_vertices.len();

        if n < 2 {
            return;
        }

        if n == 2 {
            // Single line segment
            let v0 = self.src_vertices[0];
            let v1 = self.src_vertices[1];
            let lp = LineParameters::new(v0.x, v0.y, v1.x, v1.y, v0.len);
            if close_polygon {
                ren.line0(&lp);
            } else {
                if self.round_cap {
                    ren.semidot(cmp_dist_start, v0.x, v0.y, v0.x + (v0.y - v1.y), v0.y - (v0.x - v1.x));
                }
                ren.line0(&lp);
                if self.round_cap {
                    ren.semidot(cmp_dist_end, v1.x, v1.y, v1.x + (v1.y - v0.y), v1.y - (v1.x - v0.x));
                }
            }
            return;
        }

        if n == 3 && !close_polygon {
            // Two segments, open
            let v0 = self.src_vertices[0];
            let v1 = self.src_vertices[1];
            let v2 = self.src_vertices[2];

            let lp1 = LineParameters::new(v0.x, v0.y, v1.x, v1.y, v0.len);
            let lp2 = LineParameters::new(v1.x, v1.y, v2.x, v2.y, v1.len);

            if self.round_cap {
                ren.semidot(cmp_dist_start, v0.x, v0.y, v0.x + (v0.y - v1.y), v0.y - (v0.x - v1.x));
            }

            if self.line_join == OutlineAaJoin::Round {
                ren.line0(&lp1);
                // Draw round join
                let (mut bx, mut by) = (0, 0);
                bisectrix(&lp1, &lp2, &mut bx, &mut by);
                ren.pie(v1.x, v1.y, v0.x + (v0.y - v1.y), v0.y - (v0.x - v1.x), v1.x + (v1.y - v2.y), v1.y - (v1.x - v2.x));
                ren.line0(&lp2);
            } else {
                let (mut bx, mut by) = (0, 0);
                bisectrix(&lp1, &lp2, &mut bx, &mut by);
                ren.line1(&lp1, bx, by);
                ren.line2(&lp2, bx, by);
            }

            if self.round_cap {
                ren.semidot(cmp_dist_end, v2.x, v2.y, v2.x + (v2.y - v1.y), v2.y - (v2.x - v1.x));
            }
            return;
        }

        // General case: 4+ vertices or closed polygon with 3+ vertices
        self.render_general(ren, close_polygon, n);
    }

    fn render_general<PF: PixelFormat>(
        &mut self,
        ren: &mut RendererOutlineAa<PF>,
        close_polygon: bool,
        n: usize,
    ) where
        PF::ColorType: Default + Clone,
    {
        if close_polygon {
            // Closed polygon rendering
            // Process first segment with joins to last
            let v_last = self.src_vertices[n - 1];
            let v0 = self.src_vertices[0];
            let v1 = self.src_vertices[1];

            let lp_prev = LineParameters::new(v_last.x, v_last.y, v0.x, v0.y, v_last.len);
            let lp_curr = LineParameters::new(v0.x, v0.y, v1.x, v1.y, v0.len);

            let same_diag = lp_prev.same_diagonal_quadrant(&lp_curr);

            let (mut bx1, mut by1) = (0, 0);
            if !same_diag || self.line_join == OutlineAaJoin::MiterAccurate {
                bisectrix(&lp_prev, &lp_curr, &mut bx1, &mut by1);
            }

            // Render each segment
            let mut prev_lp = lp_curr;
            let mut prev_bx = bx1;
            let mut prev_by = by1;

            for i in 1..n {
                let next_idx = if i + 1 < n { i + 1 } else { (i + 1) % n };
                let v_curr = self.src_vertices[i];
                let v_next = self.src_vertices[next_idx];

                let lp_next = LineParameters::new(v_curr.x, v_curr.y, v_next.x, v_next.y, v_curr.len);
                let same_diag2 = prev_lp.same_diagonal_quadrant(&lp_next);

                let (mut bx2, mut by2) = (0, 0);
                if !same_diag2 || self.line_join == OutlineAaJoin::MiterAccurate {
                    bisectrix(&prev_lp, &lp_next, &mut bx2, &mut by2);
                }

                // Determine flags
                let flags = if same_diag { 1 } else { 0 } | if same_diag2 { 2 } else { 0 };

                match flags {
                    0 => ren.line3(&prev_lp, prev_bx, prev_by, bx2, by2),
                    1 => ren.line2(&prev_lp, bx2, by2),
                    2 => ren.line1(&prev_lp, prev_bx, prev_by),
                    _ => ren.line0(&prev_lp),
                }

                if self.line_join == OutlineAaJoin::Round && !same_diag2 {
                    ren.pie(
                        v_curr.x, v_curr.y,
                        self.src_vertices[i - 1].x + (self.src_vertices[i - 1].y - v_curr.y),
                        self.src_vertices[i - 1].y - (self.src_vertices[i - 1].x - v_curr.x),
                        v_curr.x + (v_curr.y - v_next.y),
                        v_curr.y - (v_curr.x - v_next.x),
                    );
                }

                prev_lp = lp_next;
                prev_bx = bx2;
                prev_by = by2;
            }
        } else {
            // Open polyline
            let v0 = self.src_vertices[0];
            let v1 = self.src_vertices[1];

            if self.round_cap {
                ren.semidot(cmp_dist_start, v0.x, v0.y, v0.x + (v0.y - v1.y), v0.y - (v0.x - v1.x));
            }

            // First segment
            let mut prev_lp = LineParameters::new(v0.x, v0.y, v1.x, v1.y, v0.len);

            for i in 1..n - 1 {
                let v_curr = self.src_vertices[i];
                let v_next = self.src_vertices[i + 1];

                let lp_next = LineParameters::new(v_curr.x, v_curr.y, v_next.x, v_next.y, v_curr.len);
                let same_diag = prev_lp.same_diagonal_quadrant(&lp_next);

                let (mut bx, mut by) = (0, 0);
                if !same_diag || self.line_join == OutlineAaJoin::MiterAccurate {
                    bisectrix(&prev_lp, &lp_next, &mut bx, &mut by);
                }

                if i == 1 {
                    // First segment with end join
                    if same_diag {
                        ren.line0(&prev_lp);
                    } else {
                        ren.line2(&prev_lp, bx, by);
                    }
                } else {
                    // Middle segment with start join (from previous bisectrix)
                    if same_diag {
                        ren.line0(&prev_lp);
                    } else {
                        ren.line1(&prev_lp, bx, by);
                    }
                }

                if self.line_join == OutlineAaJoin::Round && !same_diag {
                    ren.pie(
                        v_curr.x, v_curr.y,
                        self.src_vertices[i - 1].x + (self.src_vertices[i - 1].y - v_curr.y),
                        self.src_vertices[i - 1].y - (self.src_vertices[i - 1].x - v_curr.x),
                        v_curr.x + (v_curr.y - v_next.y),
                        v_curr.y - (v_curr.x - v_next.x),
                    );
                }

                prev_lp = lp_next;
            }

            // Last segment
            ren.line0(&prev_lp);

            // End cap
            if self.round_cap {
                let v_last = self.src_vertices[n - 1];
                let v_prev = self.src_vertices[n - 2];
                ren.semidot(cmp_dist_end, v_last.x, v_last.y, v_last.x + (v_last.y - v_prev.y), v_last.y - (v_last.x - v_prev.x));
            }
        }
    }

    /// Add a path from a vertex source and render it.
    pub fn add_path<VS: VertexSource, PF: PixelFormat>(
        &mut self,
        vs: &mut VS,
        path_id: u32,
        ren: &mut RendererOutlineAa<PF>,
    ) where
        PF::ColorType: Default + Clone,
    {
        vs.rewind(path_id);
        let (mut x, mut y) = (0.0, 0.0);
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_move_to(cmd) {
                self.render(ren, false);
                self.src_vertices.clear();
                self.move_to_d(x, y);
            } else if is_end_poly(cmd) {
                self.render(ren, is_close(cmd));
                self.src_vertices.clear();
            } else if is_vertex(cmd) {
                self.line_to_d(x, y);
            }
        }
        self.render(ren, false);
        self.src_vertices.clear();
    }
}

impl Default for RasterizerOutlineAa {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparison function for start caps.
fn cmp_dist_start(dist: i32) -> bool {
    dist <= 0
}

/// Comparison function for end caps.
fn cmp_dist_end(dist: i32) -> bool {
    dist > 0
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
    use crate::renderer_base::RendererBase;
    use crate::renderer_outline_aa::LineProfileAa;
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

//! Contour vertex generator.
//!
//! Port of `agg_vcgen_contour.h` / `agg_vcgen_contour.cpp` — generates
//! an offset contour from a closed polygon path using `MathStroke`.

use crate::array::{VertexDist, VertexSequence};
use crate::basics::{
    get_close_flag, get_orientation, is_ccw, is_end_poly, is_move_to, is_oriented, is_vertex,
    PointD, PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PATH_FLAGS_CCW,
    PATH_FLAGS_CLOSE, PATH_FLAGS_NONE,
};
use crate::math::calc_polygon_area_vd;
use crate::math_stroke::{InnerJoin, LineCap, LineJoin, MathStroke};

// ============================================================================
// VcgenContour
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    Ready,
    Outline,
    OutVertices,
    EndPoly,
    Stop,
}

/// Contour vertex generator.
///
/// Generates an offset contour (inset or outset) from a closed polygon.
/// Uses `MathStroke` for join calculations.
///
/// Port of C++ `vcgen_contour`.
pub struct VcgenContour {
    stroker: MathStroke,
    width: f64,
    src_vertices: VertexSequence,
    out_vertices: Vec<PointD>,
    status: Status,
    src_vertex: usize,
    out_vertex: usize,
    closed: u32,
    orientation: u32,
    auto_detect: bool,
}

impl VcgenContour {
    pub fn new() -> Self {
        Self {
            stroker: MathStroke::new(),
            width: 1.0,
            src_vertices: VertexSequence::new(),
            out_vertices: Vec::new(),
            status: Status::Initial,
            src_vertex: 0,
            out_vertex: 0,
            closed: 0,
            orientation: 0,
            auto_detect: false,
        }
    }

    // Parameter forwarding to MathStroke
    pub fn set_line_cap(&mut self, lc: LineCap) {
        self.stroker.set_line_cap(lc);
    }
    pub fn line_cap(&self) -> LineCap {
        self.stroker.line_cap()
    }

    pub fn set_line_join(&mut self, lj: LineJoin) {
        self.stroker.set_line_join(lj);
    }
    pub fn line_join(&self) -> LineJoin {
        self.stroker.line_join()
    }

    pub fn set_inner_join(&mut self, ij: InnerJoin) {
        self.stroker.set_inner_join(ij);
    }
    pub fn inner_join(&self) -> InnerJoin {
        self.stroker.inner_join()
    }

    pub fn set_width(&mut self, w: f64) {
        self.width = w;
        self.stroker.set_width(w);
    }
    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn set_miter_limit(&mut self, ml: f64) {
        self.stroker.set_miter_limit(ml);
    }
    pub fn miter_limit(&self) -> f64 {
        self.stroker.miter_limit()
    }

    pub fn set_miter_limit_theta(&mut self, t: f64) {
        self.stroker.set_miter_limit_theta(t);
    }

    pub fn set_inner_miter_limit(&mut self, ml: f64) {
        self.stroker.set_inner_miter_limit(ml);
    }
    pub fn inner_miter_limit(&self) -> f64 {
        self.stroker.inner_miter_limit()
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.stroker.set_approximation_scale(s);
    }
    pub fn approximation_scale(&self) -> f64 {
        self.stroker.approximation_scale()
    }

    pub fn set_auto_detect_orientation(&mut self, v: bool) {
        self.auto_detect = v;
    }
    pub fn auto_detect_orientation(&self) -> bool {
        self.auto_detect
    }

    // Generator interface
    pub fn remove_all(&mut self) {
        self.src_vertices.remove_all();
        self.closed = 0;
        self.orientation = 0;
        self.status = Status::Initial;
    }

    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        self.status = Status::Initial;
        if is_move_to(cmd) {
            self.src_vertices.modify_last(VertexDist::new(x, y));
        } else if is_vertex(cmd) {
            self.src_vertices.add(VertexDist::new(x, y));
        } else if is_end_poly(cmd) {
            self.closed = get_close_flag(cmd);
            if self.orientation == PATH_FLAGS_NONE {
                self.orientation = get_orientation(cmd);
            }
        }
    }

    // Vertex Source Interface
    pub fn rewind(&mut self, _path_id: u32) {
        if self.status == Status::Initial {
            self.src_vertices.close(true);
            if self.auto_detect && !is_oriented(self.orientation) {
                let verts: Vec<VertexDist> = (0..self.src_vertices.size())
                    .map(|i| self.src_vertices[i])
                    .collect();
                self.orientation = if calc_polygon_area_vd(&verts) > 0.0 {
                    PATH_FLAGS_CCW
                } else {
                    crate::basics::PATH_FLAGS_CW
                };
            }
            if is_oriented(self.orientation) {
                self.stroker.set_width(if is_ccw(self.orientation) {
                    self.width
                } else {
                    -self.width
                });
            }
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
    }

    pub fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        // C++ uses while(!is_stop(cmd)) with switch/case fallthrough.
        // Rust: loop{match} where arms fall through or return.
        let mut cmd = PATH_CMD_LINE_TO;
        loop {
            match self.status {
                Status::Initial => {
                    self.rewind(0);
                    // fall through to Ready
                }
                Status::Ready => {
                    if self.src_vertices.size() < 2 + (self.closed != 0) as usize {
                        return PATH_CMD_STOP;
                    }
                    self.status = Status::Outline;
                    cmd = PATH_CMD_MOVE_TO;
                    self.src_vertex = 0;
                    self.out_vertex = 0;
                    // fall through to Outline
                }
                Status::Outline => {
                    if self.src_vertex >= self.src_vertices.size() {
                        self.status = Status::EndPoly;
                        continue;
                    }
                    // Copy to locals to avoid borrow conflicts
                    let v_prev = *self.src_vertices.prev(self.src_vertex);
                    let v_curr = *self.src_vertices.curr(self.src_vertex);
                    let v_next = *self.src_vertices.next(self.src_vertex);
                    self.stroker.calc_join(
                        &mut self.out_vertices,
                        &v_prev,
                        &v_curr,
                        &v_next,
                        v_prev.dist,
                        v_curr.dist,
                    );
                    self.src_vertex += 1;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                    // fall through to OutVertices
                }
                Status::OutVertices => {
                    if self.out_vertex >= self.out_vertices.len() {
                        self.status = Status::Outline;
                        // continue loop back to Outline
                    } else {
                        let c = self.out_vertices[self.out_vertex];
                        self.out_vertex += 1;
                        *x = c.x;
                        *y = c.y;
                        return cmd;
                    }
                }
                Status::EndPoly => {
                    if self.closed == 0 {
                        return PATH_CMD_STOP;
                    }
                    self.status = Status::Stop;
                    return PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
                }
                Status::Stop => {
                    return PATH_CMD_STOP;
                }
            }
        }
    }
}

impl Default for VcgenContour {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::conv_adaptor_vcgen::VcgenGenerator for VcgenContour {
    fn remove_all(&mut self) {
        self.remove_all();
    }
    fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        self.add_vertex(x, y, cmd);
    }
    fn rewind(&mut self, path_id: u32) {
        self.rewind(path_id);
    }
    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.vertex(x, y)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_stop, PATH_FLAGS_CLOSE};

    fn collect_gen_vertices(gen: &mut VcgenContour) -> Vec<(f64, f64, u32)> {
        gen.rewind(0);
        let mut result = Vec::new();
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gen.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            result.push((x, y, cmd));
        }
        result
    }

    #[test]
    fn test_new_defaults() {
        let gen = VcgenContour::new();
        assert!((gen.width() - 1.0).abs() < 1e-10);
        assert!(!gen.auto_detect_orientation());
    }

    #[test]
    fn test_empty_produces_stop() {
        let mut gen = VcgenContour::new();
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_closed_square_contour() {
        let mut gen = VcgenContour::new();
        gen.set_width(5.0);
        gen.set_auto_detect_orientation(true);

        // CCW square
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.add_vertex(100.0, 100.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 100.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);

        let verts = collect_gen_vertices(&mut gen);
        assert!(
            verts.len() >= 4,
            "Expected at least 4 contour vertices, got {}",
            verts.len()
        );
        // First should be move_to
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_contour_expands_ccw_polygon() {
        let mut gen = VcgenContour::new();
        gen.set_width(10.0);
        gen.set_auto_detect_orientation(true);

        // CCW triangle
        gen.add_vertex(50.0, 10.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(90.0, 90.0, PATH_CMD_LINE_TO);
        gen.add_vertex(10.0, 90.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);

        let verts = collect_gen_vertices(&mut gen);

        // Contour should be larger than the original — check bounds
        let max_x = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.0)
            .fold(f64::MIN, f64::max);
        let min_x = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.0)
            .fold(f64::MAX, f64::min);

        assert!(max_x > 90.0, "Max x={} should exceed original 90", max_x);
        assert!(
            min_x < 10.0,
            "Min x={} should be less than original 10",
            min_x
        );
    }

    #[test]
    fn test_width_setter() {
        let mut gen = VcgenContour::new();
        gen.set_width(7.5);
        assert!((gen.width() - 7.5).abs() < 1e-10);
    }

    #[test]
    fn test_remove_all() {
        let mut gen = VcgenContour::new();
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.add_vertex(100.0, 100.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);
        gen.remove_all();
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_rewind_replay() {
        let mut gen = VcgenContour::new();
        gen.set_width(5.0);
        gen.set_auto_detect_orientation(true);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.add_vertex(50.0, 80.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);

        let v1 = collect_gen_vertices(&mut gen);
        let v2 = collect_gen_vertices(&mut gen);
        assert_eq!(v1.len(), v2.len());
    }

    #[test]
    fn test_auto_detect_orientation() {
        let mut gen = VcgenContour::new();
        gen.set_auto_detect_orientation(true);
        assert!(gen.auto_detect_orientation());
        gen.set_auto_detect_orientation(false);
        assert!(!gen.auto_detect_orientation());
    }

    #[test]
    fn test_line_join_setter() {
        let mut gen = VcgenContour::new();
        gen.set_line_join(LineJoin::Round);
        assert_eq!(gen.line_join(), LineJoin::Round);
    }

    #[test]
    fn test_open_path_no_contour() {
        let mut gen = VcgenContour::new();
        gen.set_width(5.0);
        // Open path (no close flag)
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.add_vertex(100.0, 100.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        // Contour generator needs a closed polygon; open path should produce
        // vertices but end_poly check will return stop if not closed
        // (the C++ code returns stop if !closed in EndPoly state)
        // Just verify it doesn't crash
        let _ = verts;
    }

    #[test]
    fn test_end_poly_emitted_for_closed() {
        let mut gen = VcgenContour::new();
        gen.set_width(5.0);
        gen.set_auto_detect_orientation(true);

        // CCW triangle
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.add_vertex(50.0, 80.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);

        gen.rewind(0);
        let mut found_end_poly = false;
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gen.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if (cmd & PATH_CMD_END_POLY) == PATH_CMD_END_POLY {
                found_end_poly = true;
                assert_ne!(cmd & PATH_FLAGS_CLOSE, 0, "end_poly should have close flag");
            }
        }
        assert!(found_end_poly, "Closed contour should emit end_poly");
    }
}

//! Stroke vertex generator.
//!
//! Port of `agg_vcgen_stroke.h` / `agg_vcgen_stroke.cpp` — generates
//! a stroked outline from a center-line path using `MathStroke`.

use crate::array::{shorten_path, VertexDist, VertexSequence};
use crate::basics::{
    get_close_flag, is_move_to, is_vertex, PointD, PATH_CMD_END_POLY, PATH_CMD_LINE_TO,
    PATH_CMD_MOVE_TO, PATH_CMD_STOP, PATH_FLAGS_CCW, PATH_FLAGS_CLOSE, PATH_FLAGS_CW,
};
use crate::math_stroke::{InnerJoin, LineCap, LineJoin, MathStroke};

// ============================================================================
// VcgenStroke
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    Ready,
    Cap1,
    Cap2,
    Outline1,
    CloseFirst,
    Outline2,
    OutVertices,
    EndPoly1,
    EndPoly2,
    Stop,
}

/// Stroke vertex generator.
///
/// Accumulates source path vertices, then generates a stroked outline
/// using `MathStroke` for join/cap calculations.
///
/// Port of C++ `vcgen_stroke`.
pub struct VcgenStroke {
    stroker: MathStroke,
    src_vertices: VertexSequence,
    out_vertices: Vec<PointD>,
    shorten: f64,
    closed: u32,
    status: Status,
    prev_status: Status,
    src_vertex: usize,
    out_vertex: usize,
}

impl VcgenStroke {
    pub fn new() -> Self {
        Self {
            stroker: MathStroke::new(),
            src_vertices: VertexSequence::new(),
            out_vertices: Vec::new(),
            shorten: 0.0,
            closed: 0,
            status: Status::Initial,
            prev_status: Status::Initial,
            src_vertex: 0,
            out_vertex: 0,
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
        self.stroker.set_width(w);
    }
    pub fn width(&self) -> f64 {
        self.stroker.width()
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

    pub fn set_shorten(&mut self, s: f64) {
        self.shorten = s;
    }
    pub fn shorten(&self) -> f64 {
        self.shorten
    }

    // Vertex Generator Interface
    pub fn remove_all(&mut self) {
        self.src_vertices.remove_all();
        self.closed = 0;
        self.status = Status::Initial;
    }

    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        self.status = Status::Initial;
        if is_move_to(cmd) {
            self.src_vertices.modify_last(VertexDist::new(x, y));
        } else if is_vertex(cmd) {
            self.src_vertices.add(VertexDist::new(x, y));
        } else {
            self.closed = get_close_flag(cmd);
        }
    }

    // Vertex Source Interface
    pub fn rewind(&mut self, _path_id: u32) {
        if self.status == Status::Initial {
            self.src_vertices.close(self.closed != 0);
            shorten_path(&mut self.src_vertices, self.shorten, self.closed);
            if self.src_vertices.size() < 3 {
                self.closed = 0;
            }
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
        self.out_vertex = 0;
    }

    pub fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        // cmd persists across states within one call; initialized per the C++ pattern.
        // In C++, break exits the switch (not the while), so the while loop continues.
        // We use loop{match} where each arm either returns or falls through to the
        // next iteration.
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
                    self.status = if self.closed != 0 {
                        Status::Outline1
                    } else {
                        Status::Cap1
                    };
                    cmd = PATH_CMD_MOVE_TO;
                    self.src_vertex = 0;
                    self.out_vertex = 0;
                    // continue loop to Cap1/Outline1
                }
                Status::Cap1 => {
                    let v0 = *self.src_vertices.curr(0);
                    let v1 = *self.src_vertices.curr(1);
                    self.stroker
                        .calc_cap(&mut self.out_vertices, &v0, &v1, v0.dist);
                    self.src_vertex = 1;
                    self.prev_status = Status::Outline1;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                    // continue loop to OutVertices
                }
                Status::Cap2 => {
                    let n = self.src_vertices.size();
                    let v0 = *self.src_vertices.curr(n - 1);
                    let v1 = *self.src_vertices.curr(n - 2);
                    self.stroker
                        .calc_cap(&mut self.out_vertices, &v0, &v1, v1.dist);
                    self.prev_status = Status::Outline2;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                    // continue loop to OutVertices
                }
                Status::Outline1 => {
                    if self.closed != 0 {
                        if self.src_vertex >= self.src_vertices.size() {
                            self.prev_status = Status::CloseFirst;
                            self.status = Status::EndPoly1;
                            continue; // to EndPoly1
                        }
                    } else if self.src_vertex >= self.src_vertices.size() - 1 {
                        self.status = Status::Cap2;
                        continue; // to Cap2
                    }
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
                    self.prev_status = self.status;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                    // continue loop to OutVertices
                }
                Status::CloseFirst => {
                    self.status = Status::Outline2;
                    cmd = PATH_CMD_MOVE_TO;
                    // fall through to Outline2
                }
                Status::Outline2 => {
                    if self.src_vertex <= (self.closed == 0) as usize {
                        self.status = Status::EndPoly2;
                        self.prev_status = Status::Stop;
                        continue; // to EndPoly2
                    }
                    self.src_vertex -= 1;
                    let v_next = *self.src_vertices.next(self.src_vertex);
                    let v_curr = *self.src_vertices.curr(self.src_vertex);
                    let v_prev = *self.src_vertices.prev(self.src_vertex);
                    self.stroker.calc_join(
                        &mut self.out_vertices,
                        &v_next,
                        &v_curr,
                        &v_prev,
                        v_curr.dist,
                        v_prev.dist,
                    );
                    self.prev_status = self.status;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                    // continue loop to OutVertices
                }
                Status::OutVertices => {
                    if self.out_vertex >= self.out_vertices.len() {
                        self.status = self.prev_status;
                        // continue loop to prev_status
                    } else {
                        let c = self.out_vertices[self.out_vertex];
                        self.out_vertex += 1;
                        *x = c.x;
                        *y = c.y;
                        return cmd;
                    }
                }
                Status::EndPoly1 => {
                    self.status = self.prev_status;
                    return PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
                }
                Status::EndPoly2 => {
                    self.status = self.prev_status;
                    return PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CW;
                }
                Status::Stop => {
                    return PATH_CMD_STOP;
                }
            }
        }
    }
}

impl Default for VcgenStroke {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::conv_adaptor_vcgen::VcgenGenerator for VcgenStroke {
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
    use crate::basics::is_stop;

    fn collect_gen_vertices(gen: &mut VcgenStroke) -> Vec<(f64, f64, u32)> {
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
        let gen = VcgenStroke::new();
        assert!((gen.width() - 1.0).abs() < 1e-10);
        assert_eq!(gen.line_cap(), LineCap::Butt);
        assert_eq!(gen.line_join(), LineJoin::Miter);
    }

    #[test]
    fn test_empty_produces_stop() {
        let mut gen = VcgenStroke::new();
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_single_segment_open() {
        let mut gen = VcgenStroke::new();
        gen.set_width(10.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        // Should produce a stroked rectangle (open path with two caps)
        assert!(
            verts.len() >= 4,
            "Expected at least 4 vertices, got {}",
            verts.len()
        );
        // First vertex should be move_to
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_closed_triangle() {
        let mut gen = VcgenStroke::new();
        gen.set_width(4.0);
        gen.add_vertex(10.0, 10.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(50.0, 10.0, PATH_CMD_LINE_TO);
        gen.add_vertex(30.0, 40.0, PATH_CMD_LINE_TO);
        gen.add_vertex(0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);

        let verts = collect_gen_vertices(&mut gen);
        // Closed polygon produces two outline loops (inner + outer)
        assert!(
            verts.len() >= 6,
            "Expected at least 6 vertices for closed triangle stroke, got {}",
            verts.len()
        );
    }

    #[test]
    fn test_width_setter() {
        let mut gen = VcgenStroke::new();
        gen.set_width(5.0);
        assert!((gen.width() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_line_cap_setter() {
        let mut gen = VcgenStroke::new();
        gen.set_line_cap(LineCap::Round);
        assert_eq!(gen.line_cap(), LineCap::Round);
    }

    #[test]
    fn test_line_join_setter() {
        let mut gen = VcgenStroke::new();
        gen.set_line_join(LineJoin::Round);
        assert_eq!(gen.line_join(), LineJoin::Round);
    }

    #[test]
    fn test_shorten() {
        let mut gen = VcgenStroke::new();
        gen.set_shorten(5.0);
        assert!((gen.shorten() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_rewind_resets() {
        let mut gen = VcgenStroke::new();
        gen.set_width(10.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let verts1 = collect_gen_vertices(&mut gen);
        // Rewind and collect again — should be the same
        gen.rewind(0);
        let mut verts2 = Vec::new();
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = gen.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            verts2.push((x, y, cmd));
        }
        assert_eq!(verts1.len(), verts2.len());
    }

    #[test]
    fn test_remove_all() {
        let mut gen = VcgenStroke::new();
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.remove_all();
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_round_cap_produces_more_vertices() {
        let mut butt = VcgenStroke::new();
        butt.set_width(20.0);
        butt.set_line_cap(LineCap::Butt);
        butt.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        butt.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let butt_verts = collect_gen_vertices(&mut butt);

        let mut round = VcgenStroke::new();
        round.set_width(20.0);
        round.set_line_cap(LineCap::Round);
        round.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        round.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let round_verts = collect_gen_vertices(&mut round);

        // Round caps produce more vertices than butt caps
        assert!(
            round_verts.len() > butt_verts.len(),
            "Round ({}) should have more vertices than butt ({})",
            round_verts.len(),
            butt_verts.len()
        );
    }

    #[test]
    fn test_horizontal_line_stroke_y_extent() {
        let mut gen = VcgenStroke::new();
        gen.set_width(10.0); // half-width = 5
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        let max_y = verts.iter().map(|v| v.1).fold(f64::MIN, f64::max);
        let min_y = verts.iter().map(|v| v.1).fold(f64::MAX, f64::min);

        // Stroke should extend ~5 units above and below
        assert!(max_y >= 4.5, "Max y={} should be >= 4.5", max_y);
        assert!(min_y <= -4.5, "Min y={} should be <= -4.5", min_y);
    }
}

//! Dash vertex generator.
//!
//! Port of `agg_vcgen_dash.h` / `agg_vcgen_dash.cpp` — generates
//! dashed lines from a continuous center-line path.

use crate::array::{shorten_path, VertexDist, VertexSequence};
use crate::basics::{
    get_close_flag, is_move_to, is_vertex, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
};

const MAX_DASHES: usize = 32;

// ============================================================================
// VcgenDash
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    Ready,
    Polyline,
    Stop,
}

/// Dash vertex generator.
///
/// Maintains a dash pattern (up to 16 dash/gap pairs) and generates
/// dashed segments from a continuous path.
///
/// Port of C++ `vcgen_dash`.
pub struct VcgenDash {
    dashes: [f64; MAX_DASHES],
    total_dash_len: f64,
    num_dashes: usize,
    dash_start: f64,
    shorten: f64,
    curr_dash_start: f64,
    curr_dash: usize,
    curr_rest: f64,
    v1_idx: usize,
    v2_idx: usize,
    src_vertices: VertexSequence,
    closed: u32,
    status: Status,
    src_vertex: usize,
}

impl VcgenDash {
    pub fn new() -> Self {
        Self {
            dashes: [0.0; MAX_DASHES],
            total_dash_len: 0.0,
            num_dashes: 0,
            dash_start: 0.0,
            shorten: 0.0,
            curr_dash_start: 0.0,
            curr_dash: 0,
            curr_rest: 0.0,
            v1_idx: 0,
            v2_idx: 0,
            src_vertices: VertexSequence::new(),
            closed: 0,
            status: Status::Initial,
            src_vertex: 0,
        }
    }

    pub fn remove_all_dashes(&mut self) {
        self.total_dash_len = 0.0;
        self.num_dashes = 0;
        self.curr_dash_start = 0.0;
        self.curr_dash = 0;
    }

    pub fn add_dash(&mut self, dash_len: f64, gap_len: f64) {
        if self.num_dashes < MAX_DASHES {
            self.total_dash_len += dash_len + gap_len;
            self.dashes[self.num_dashes] = dash_len;
            self.num_dashes += 1;
            self.dashes[self.num_dashes] = gap_len;
            self.num_dashes += 1;
        }
    }

    pub fn dash_start(&mut self, ds: f64) {
        self.dash_start = ds;
        self.calc_dash_start(ds.abs());
    }

    fn calc_dash_start(&mut self, mut ds: f64) {
        self.curr_dash = 0;
        self.curr_dash_start = 0.0;
        while ds > 0.0 {
            if ds > self.dashes[self.curr_dash] {
                ds -= self.dashes[self.curr_dash];
                self.curr_dash += 1;
                self.curr_dash_start = 0.0;
                if self.curr_dash >= self.num_dashes {
                    self.curr_dash = 0;
                }
            } else {
                self.curr_dash_start = ds;
                ds = 0.0;
            }
        }
    }

    pub fn set_shorten(&mut self, s: f64) {
        self.shorten = s;
    }

    pub fn shorten(&self) -> f64 {
        self.shorten
    }

    // Vertex Generator Interface
    pub fn remove_all(&mut self) {
        self.status = Status::Initial;
        self.src_vertices.remove_all();
        self.closed = 0;
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
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
    }

    pub fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        // C++ uses while(!is_stop(cmd)) with switch/case fallthrough.
        // We use loop{match} with the same semantics: states either return
        // or fall through to the next iteration.
        let mut cmd = PATH_CMD_MOVE_TO;
        loop {
            if crate::basics::is_stop(cmd) {
                break;
            }
            match self.status {
                Status::Initial => {
                    self.rewind(0);
                    // fall through to Ready
                }
                Status::Ready => {
                    if self.num_dashes < 2 || self.src_vertices.size() < 2 {
                        cmd = PATH_CMD_STOP;
                        continue; // re-check while condition (is_stop → break)
                    }
                    self.status = Status::Polyline;
                    self.src_vertex = 1;
                    self.v1_idx = 0;
                    self.v2_idx = 1;
                    self.curr_rest = self.src_vertices[0].dist;
                    *x = self.src_vertices[0].x;
                    *y = self.src_vertices[0].y;
                    if self.dash_start >= 0.0 {
                        self.calc_dash_start(self.dash_start);
                    }
                    return PATH_CMD_MOVE_TO;
                }
                Status::Polyline => {
                    let dash_rest = self.dashes[self.curr_dash] - self.curr_dash_start;

                    cmd = if (self.curr_dash & 1) != 0 {
                        PATH_CMD_MOVE_TO
                    } else {
                        PATH_CMD_LINE_TO
                    };

                    let v1 = self.src_vertices[self.v1_idx];
                    let v2 = self.src_vertices[self.v2_idx];

                    if self.curr_rest > dash_rest {
                        self.curr_rest -= dash_rest;
                        self.curr_dash += 1;
                        if self.curr_dash >= self.num_dashes {
                            self.curr_dash = 0;
                        }
                        self.curr_dash_start = 0.0;
                        *x = v2.x - (v2.x - v1.x) * self.curr_rest / v1.dist;
                        *y = v2.y - (v2.y - v1.y) * self.curr_rest / v1.dist;
                    } else {
                        self.curr_dash_start += self.curr_rest;
                        *x = v2.x;
                        *y = v2.y;
                        self.src_vertex += 1;
                        self.v1_idx = self.v2_idx;
                        self.curr_rest = self.src_vertices[self.v1_idx].dist;
                        if self.closed != 0 {
                            if self.src_vertex > self.src_vertices.size() {
                                self.status = Status::Stop;
                            } else {
                                self.v2_idx = if self.src_vertex >= self.src_vertices.size() {
                                    0
                                } else {
                                    self.src_vertex
                                };
                            }
                        } else if self.src_vertex >= self.src_vertices.size() {
                            self.status = Status::Stop;
                        } else {
                            self.v2_idx = self.src_vertex;
                        }
                    }
                    return cmd;
                }
                Status::Stop => {
                    cmd = PATH_CMD_STOP;
                    continue; // re-check while condition (is_stop → break)
                }
            }
        }
        PATH_CMD_STOP
    }
}

impl Default for VcgenDash {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::conv_adaptor_vcgen::VcgenGenerator for VcgenDash {
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
    use crate::basics::{is_stop, is_vertex, PATH_CMD_MOVE_TO};

    fn collect_gen_vertices(gen: &mut VcgenDash) -> Vec<(f64, f64, u32)> {
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
        let gen = VcgenDash::new();
        assert!((gen.shorten() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_empty_produces_stop() {
        let mut gen = VcgenDash::new();
        gen.add_dash(10.0, 5.0);
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_no_dashes_produces_stop() {
        let mut gen = VcgenDash::new();
        // No dashes added
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty(), "No dashes → no output");
    }

    #[test]
    fn test_basic_dash_pattern() {
        let mut gen = VcgenDash::new();
        gen.add_dash(20.0, 10.0); // 20px dash, 10px gap
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        assert!(!verts.is_empty(), "Should produce dash vertices");

        // First vertex should be a move_to at the start
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        assert!((verts[0].0 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_dash_has_gaps() {
        let mut gen = VcgenDash::new();
        gen.add_dash(20.0, 10.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);

        // Should have multiple move_to commands (one per dash segment)
        let move_count = verts.iter().filter(|v| v.2 == PATH_CMD_MOVE_TO).count();
        assert!(
            move_count >= 2,
            "Expected multiple dash segments, got {} move_to",
            move_count
        );
    }

    #[test]
    fn test_dash_vertices_on_line() {
        let mut gen = VcgenDash::new();
        gen.add_dash(25.0, 10.0);
        gen.add_vertex(0.0, 50.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 50.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);

        // All vertices should be on y=50 for a horizontal line
        for v in &verts {
            if is_vertex(v.2) {
                assert!((v.1 - 50.0).abs() < 1e-10, "y={} should be 50", v.1);
            }
        }
    }

    #[test]
    fn test_dash_rewind_replay() {
        let mut gen = VcgenDash::new();
        gen.add_dash(15.0, 5.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);

        let v1 = collect_gen_vertices(&mut gen);
        let v2 = collect_gen_vertices(&mut gen);
        assert_eq!(v1.len(), v2.len());
    }

    #[test]
    fn test_remove_all() {
        let mut gen = VcgenDash::new();
        gen.add_dash(10.0, 5.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        gen.remove_all();
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_remove_all_dashes() {
        let mut gen = VcgenDash::new();
        gen.add_dash(10.0, 5.0);
        gen.remove_all_dashes();
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let verts = collect_gen_vertices(&mut gen);
        assert!(verts.is_empty(), "Removed dashes → no output");
    }

    #[test]
    fn test_dash_start_offset() {
        let mut gen1 = VcgenDash::new();
        gen1.add_dash(20.0, 10.0);
        gen1.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen1.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let v1 = collect_gen_vertices(&mut gen1);

        let mut gen2 = VcgenDash::new();
        gen2.add_dash(20.0, 10.0);
        gen2.dash_start(15.0); // offset by 15
        gen2.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen2.add_vertex(100.0, 0.0, PATH_CMD_LINE_TO);
        let v2 = collect_gen_vertices(&mut gen2);

        // Different dash start should produce different vertices
        assert_ne!(
            v1.len(),
            v2.len(),
            "Different dash start should change vertex count"
        );
    }

    #[test]
    fn test_multiple_dash_gaps() {
        let mut gen = VcgenDash::new();
        gen.add_dash(10.0, 5.0);
        gen.add_dash(20.0, 5.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(200.0, 0.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        assert!(!verts.is_empty());

        // Count line_to segments to verify pattern is applied
        let line_count = verts.iter().filter(|v| v.2 == PATH_CMD_LINE_TO).count();
        assert!(
            line_count >= 3,
            "Expected multiple line segments, got {}",
            line_count
        );
    }

    #[test]
    fn test_shorten_setter() {
        let mut gen = VcgenDash::new();
        gen.set_shorten(5.0);
        assert!((gen.shorten() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_diagonal_dash() {
        let mut gen = VcgenDash::new();
        gen.add_dash(10.0, 5.0);
        gen.add_vertex(0.0, 0.0, PATH_CMD_MOVE_TO);
        gen.add_vertex(100.0, 100.0, PATH_CMD_LINE_TO);

        let verts = collect_gen_vertices(&mut gen);
        assert!(!verts.is_empty());

        // Vertices should lie on the diagonal line y=x
        for v in &verts {
            if is_vertex(v.2) {
                assert!(
                    (v.0 - v.1).abs() < 1e-10,
                    "Point ({}, {}) should be on y=x diagonal",
                    v.0,
                    v.1
                );
            }
        }
    }
}

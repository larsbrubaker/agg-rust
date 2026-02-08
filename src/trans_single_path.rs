//! Single-path coordinate transformation.
//!
//! Port of `agg_trans_single_path.h` + `agg_trans_single_path.cpp`.
//! Maps coordinates along a path: x → distance along path, y → perpendicular offset.

use crate::array::{VertexDist, VertexSequence};
use crate::basics::{is_move_to, is_stop, is_vertex, VertexSource};
use crate::span_interpolator_linear::Transformer;

/// Status of the path building state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Initial,
    MakingPath,
    Ready,
}

/// Single-path coordinate transformation.
///
/// Stores a path as a sequence of vertices with cumulative distances.
/// `transform()` maps x → distance-along-path, y → perpendicular offset.
/// Used by `ConvTransform` with `ConvSegmentator` for text-on-path effects.
pub struct TransSinglePath {
    src_vertices: VertexSequence,
    base_length: f64,
    kindex: f64,
    status: Status,
    preserve_x_scale: bool,
}

impl TransSinglePath {
    pub fn new() -> Self {
        Self {
            src_vertices: VertexSequence::new(),
            base_length: 0.0,
            kindex: 0.0,
            status: Status::Initial,
            preserve_x_scale: true,
        }
    }

    pub fn base_length(&self) -> f64 {
        self.base_length
    }

    pub fn set_base_length(&mut self, v: f64) {
        self.base_length = v;
    }

    pub fn preserve_x_scale(&self) -> bool {
        self.preserve_x_scale
    }

    pub fn set_preserve_x_scale(&mut self, f: bool) {
        self.preserve_x_scale = f;
    }

    pub fn reset(&mut self) {
        self.src_vertices.remove_all();
        self.kindex = 0.0;
        self.status = Status::Initial;
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        if self.status == Status::Initial {
            self.src_vertices.modify_last(VertexDist::new(x, y));
            self.status = Status::MakingPath;
        } else {
            self.line_to(x, y);
        }
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        if self.status == Status::MakingPath {
            self.src_vertices.add(VertexDist::new(x, y));
        }
    }

    /// Build the path from a VertexSource.
    pub fn add_path<VS: VertexSource>(&mut self, vs: &mut VS, path_id: u32) {
        let mut x = 0.0;
        let mut y = 0.0;

        vs.rewind(path_id);
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_move_to(cmd) {
                self.move_to(x, y);
            } else if is_vertex(cmd) {
                self.line_to(x, y);
            }
        }
        self.finalize_path();
    }

    /// Finalize the path — compute cumulative distances and prepare for transform.
    pub fn finalize_path(&mut self) {
        if self.status != Status::MakingPath || self.src_vertices.size() <= 1 {
            return;
        }

        self.src_vertices.close(false);

        if self.src_vertices.size() > 2 {
            let n = self.src_vertices.size();
            // If the second-to-last segment is very short compared to the one before it,
            // merge the last two vertices.
            if self.src_vertices[n - 2].dist * 10.0 < self.src_vertices[n - 3].dist {
                let d = self.src_vertices[n - 3].dist + self.src_vertices[n - 2].dist;
                let last = self.src_vertices[n - 1];
                self.src_vertices[n - 2] = last;
                self.src_vertices.remove_last();
                let idx = self.src_vertices.size() - 2;
                self.src_vertices[idx].dist = d;
            }
        }

        // Convert per-segment distances to cumulative distances.
        let mut dist = 0.0;
        for i in 0..self.src_vertices.size() {
            let d = self.src_vertices[i].dist;
            self.src_vertices[i].dist = dist;
            dist += d;
        }

        self.kindex = (self.src_vertices.size() - 1) as f64 / dist;
        self.status = Status::Ready;
    }

    /// Total length of the path (or base_length if set).
    pub fn total_length(&self) -> f64 {
        if self.base_length >= 1e-10 {
            return self.base_length;
        }
        if self.status == Status::Ready {
            self.src_vertices[self.src_vertices.size() - 1].dist
        } else {
            0.0
        }
    }
}

impl Transformer for TransSinglePath {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        if self.status != Status::Ready {
            return;
        }

        let n = self.src_vertices.size();
        let total_dist = self.src_vertices[n - 1].dist;

        if self.base_length > 1e-10 {
            *x *= total_dist / self.base_length;
        }

        let x1;
        let y1;
        let dx;
        let dy;
        let d;
        let dd;

        if *x < 0.0 {
            // Extrapolation on the left
            x1 = self.src_vertices[0].x;
            y1 = self.src_vertices[0].y;
            dx = self.src_vertices[1].x - x1;
            dy = self.src_vertices[1].y - y1;
            dd = self.src_vertices[1].dist - self.src_vertices[0].dist;
            d = *x;
        } else if *x > total_dist {
            // Extrapolation on the right
            let i = n - 2;
            let j = n - 1;
            x1 = self.src_vertices[j].x;
            y1 = self.src_vertices[j].y;
            dx = x1 - self.src_vertices[i].x;
            dy = y1 - self.src_vertices[i].y;
            dd = self.src_vertices[j].dist - self.src_vertices[i].dist;
            d = *x - self.src_vertices[j].dist;
        } else {
            // Interpolation — binary search for segment
            let mut i = 0usize;
            let mut j = n - 1;

            if self.preserve_x_scale {
                loop {
                    if j - i <= 1 {
                        break;
                    }
                    let k = (i + j) >> 1;
                    if *x < self.src_vertices[k].dist {
                        j = k;
                    } else {
                        i = k;
                    }
                }
                dd = self.src_vertices[j].dist - self.src_vertices[i].dist;
                d = *x - self.src_vertices[i].dist;
            } else {
                let fi = *x * self.kindex;
                i = fi as usize;
                j = i + 1;
                dd = self.src_vertices[j].dist - self.src_vertices[i].dist;
                d = (fi - i as f64) * dd;
            }

            x1 = self.src_vertices[i].x;
            y1 = self.src_vertices[i].y;
            dx = self.src_vertices[j].x - x1;
            dy = self.src_vertices[j].y - y1;
        }

        let x2 = x1 + dx * d / dd;
        let y2 = y1 + dy * d / dd;
        *x = x2 - *y * dy / dd;
        *y = y2 + *y * dx / dd;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_path() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(0.0, 0.0);
        tsp.line_to(100.0, 0.0);
        tsp.finalize_path();

        assert!((tsp.total_length() - 100.0).abs() < 1e-10);

        // Midpoint of path, zero offset
        let (mut x, mut y) = (50.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);

        // Midpoint with perpendicular offset
        let (mut x, mut y) = (50.0, 10.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_diagonal_path() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(0.0, 0.0);
        tsp.line_to(100.0, 100.0);
        tsp.finalize_path();

        let expected_len = (100.0_f64 * 100.0 + 100.0 * 100.0).sqrt();
        assert!((tsp.total_length() - expected_len).abs() < 1e-10);
    }

    #[test]
    fn test_base_length_scaling() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(0.0, 0.0);
        tsp.line_to(200.0, 0.0);
        tsp.finalize_path();
        tsp.set_base_length(100.0);

        // x=50 with base_length=100 maps to x=100 on the 200-unit path
        let (mut x, mut y) = (50.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_extrapolation_left() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(10.0, 0.0);
        tsp.line_to(110.0, 0.0);
        tsp.finalize_path();

        // x=-10 is before the path start
        let (mut x, mut y) = (-10.0, 0.0);
        tsp.transform(&mut x, &mut y);
        // Should extrapolate backwards along first segment direction
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_extrapolation_right() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(0.0, 0.0);
        tsp.line_to(100.0, 0.0);
        tsp.finalize_path();

        // x=110 is past the path end
        let (mut x, mut y) = (110.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 110.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_multi_segment_path() {
        let mut tsp = TransSinglePath::new();
        tsp.move_to(0.0, 0.0);
        tsp.line_to(50.0, 0.0);
        tsp.line_to(50.0, 50.0);
        tsp.finalize_path();

        // Total length = 50 + 50 = 100
        assert!((tsp.total_length() - 100.0).abs() < 1e-10);

        // x=25 is in first segment (horizontal)
        let (mut x, mut y) = (25.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 25.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);

        // x=75 is in second segment (vertical)
        let (mut x, mut y) = (75.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_no_preserve_x_scale() {
        let mut tsp = TransSinglePath::new();
        tsp.set_preserve_x_scale(false);
        tsp.move_to(0.0, 0.0);
        tsp.line_to(100.0, 0.0);
        tsp.finalize_path();

        let (mut x, mut y) = (50.0, 0.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_not_ready_is_noop() {
        let tsp = TransSinglePath::new();
        let (mut x, mut y) = (50.0, 25.0);
        tsp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 25.0).abs() < 1e-10);
    }
}

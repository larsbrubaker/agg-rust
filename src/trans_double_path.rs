//! Double-path coordinate transformation.
//!
//! Port of `agg_trans_double_path.h` + `agg_trans_double_path.cpp`.
//! Maps coordinates between two paths: x → distance along paths,
//! y → interpolation between path1 and path2.
//!
//! Copyright (c) 2025. BSD-3-Clause License.

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

/// Double-path coordinate transformation.
///
/// Stores two paths as sequences of vertices with cumulative distances.
/// `transform()` maps x → distance along path1 (and path2 scaled proportionally),
/// y → linear interpolation between the two paths based on `base_height`.
/// Used for text-between-two-curves effects.
pub struct TransDoublePath {
    src_vertices1: VertexSequence,
    src_vertices2: VertexSequence,
    base_length: f64,
    base_height: f64,
    kindex1: f64,
    kindex2: f64,
    status1: Status,
    status2: Status,
    preserve_x_scale: bool,
}

impl TransDoublePath {
    pub fn new() -> Self {
        Self {
            src_vertices1: VertexSequence::new(),
            src_vertices2: VertexSequence::new(),
            base_length: 0.0,
            base_height: 1.0,
            kindex1: 0.0,
            kindex2: 0.0,
            status1: Status::Initial,
            status2: Status::Initial,
            preserve_x_scale: true,
        }
    }

    pub fn set_base_length(&mut self, v: f64) {
        self.base_length = v;
    }

    pub fn base_length(&self) -> f64 {
        self.base_length
    }

    pub fn set_base_height(&mut self, v: f64) {
        self.base_height = v;
    }

    pub fn base_height(&self) -> f64 {
        self.base_height
    }

    pub fn set_preserve_x_scale(&mut self, f: bool) {
        self.preserve_x_scale = f;
    }

    pub fn preserve_x_scale(&self) -> bool {
        self.preserve_x_scale
    }

    pub fn reset(&mut self) {
        self.src_vertices1.remove_all();
        self.src_vertices2.remove_all();
        self.kindex1 = 0.0;
        self.kindex2 = 0.0;
        self.status1 = Status::Initial;
        self.status2 = Status::Initial;
    }

    pub fn move_to1(&mut self, x: f64, y: f64) {
        if self.status1 == Status::Initial {
            self.src_vertices1.modify_last(VertexDist::new(x, y));
            self.status1 = Status::MakingPath;
        } else {
            self.line_to1(x, y);
        }
    }

    pub fn line_to1(&mut self, x: f64, y: f64) {
        if self.status1 == Status::MakingPath {
            self.src_vertices1.add(VertexDist::new(x, y));
        }
    }

    pub fn move_to2(&mut self, x: f64, y: f64) {
        if self.status2 == Status::Initial {
            self.src_vertices2.modify_last(VertexDist::new(x, y));
            self.status2 = Status::MakingPath;
        } else {
            self.line_to2(x, y);
        }
    }

    pub fn line_to2(&mut self, x: f64, y: f64) {
        if self.status2 == Status::MakingPath {
            self.src_vertices2.add(VertexDist::new(x, y));
        }
    }

    /// Build both paths from two VertexSources and finalize.
    pub fn add_paths<VS1: VertexSource, VS2: VertexSource>(
        &mut self,
        vs1: &mut VS1,
        vs2: &mut VS2,
        path1_id: u32,
        path2_id: u32,
    ) {
        let (mut x, mut y) = (0.0, 0.0);

        vs1.rewind(path1_id);
        loop {
            let cmd = vs1.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_move_to(cmd) {
                self.move_to1(x, y);
            } else if is_vertex(cmd) {
                self.line_to1(x, y);
            }
        }

        vs2.rewind(path2_id);
        loop {
            let cmd = vs2.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_move_to(cmd) {
                self.move_to2(x, y);
            } else if is_vertex(cmd) {
                self.line_to2(x, y);
            }
        }

        self.finalize_paths();
    }

    /// Finalize both paths — compute cumulative distances and prepare for transform.
    pub fn finalize_paths(&mut self) {
        if self.status1 == Status::MakingPath
            && self.src_vertices1.size() > 1
            && self.status2 == Status::MakingPath
            && self.src_vertices2.size() > 1
        {
            self.kindex1 = Self::finalize_path(&mut self.src_vertices1);
            self.kindex2 = Self::finalize_path(&mut self.src_vertices2);
            self.status1 = Status::Ready;
            self.status2 = Status::Ready;
        }
    }

    /// Total length of path 1 (or base_length if set).
    pub fn total_length1(&self) -> f64 {
        if self.base_length >= 1e-10 {
            return self.base_length;
        }
        if self.status1 == Status::Ready {
            self.src_vertices1[self.src_vertices1.size() - 1].dist
        } else {
            0.0
        }
    }

    /// Total length of path 2 (or base_length if set).
    pub fn total_length2(&self) -> f64 {
        if self.base_length >= 1e-10 {
            return self.base_length;
        }
        if self.status2 == Status::Ready {
            self.src_vertices2[self.src_vertices2.size() - 1].dist
        } else {
            0.0
        }
    }

    // -- Internal helpers --

    /// Finalize a single path: merge tiny trailing segments, convert to cumulative distances.
    fn finalize_path(vertices: &mut VertexSequence) -> f64 {
        vertices.close(false);

        if vertices.size() > 2 {
            let n = vertices.size();
            if vertices[n - 2].dist * 10.0 < vertices[n - 3].dist {
                let d = vertices[n - 3].dist + vertices[n - 2].dist;
                let last = vertices[n - 1];
                vertices[n - 2] = last;
                vertices.remove_last();
                let idx = vertices.size() - 2;
                vertices[idx].dist = d;
            }
        }

        let mut dist = 0.0;
        for i in 0..vertices.size() {
            let d = vertices[i].dist;
            vertices[i].dist = dist;
            dist += d;
        }

        (vertices.size() - 1) as f64 / dist
    }

    /// Transform a point along a single path with a given kx scaling factor.
    /// This is the inner helper used by both path1 and path2 transforms.
    fn transform1(
        &self,
        vertices: &VertexSequence,
        kindex: f64,
        kx: f64,
        x: &mut f64,
        y: &mut f64,
    ) {
        let x1;
        let y1;
        let dx;
        let dy;
        let d;
        let dd;

        *x *= kx;

        if *x < 0.0 {
            // Extrapolation on the left
            x1 = vertices[0].x;
            y1 = vertices[0].y;
            dx = vertices[1].x - x1;
            dy = vertices[1].y - y1;
            dd = vertices[1].dist - vertices[0].dist;
            d = *x;
        } else if *x > vertices[vertices.size() - 1].dist {
            // Extrapolation on the right
            let i = vertices.size() - 2;
            let j = vertices.size() - 1;
            x1 = vertices[j].x;
            y1 = vertices[j].y;
            dx = x1 - vertices[i].x;
            dy = y1 - vertices[i].y;
            dd = vertices[j].dist - vertices[i].dist;
            d = *x - vertices[j].dist;
        } else {
            // Interpolation
            let mut i = 0usize;
            let mut j = vertices.size() - 1;

            if self.preserve_x_scale {
                loop {
                    if j - i <= 1 {
                        break;
                    }
                    let k = (i + j) >> 1;
                    if *x < vertices[k].dist {
                        j = k;
                    } else {
                        i = k;
                    }
                }
                dd = vertices[j].dist - vertices[i].dist;
                d = *x - vertices[i].dist;
            } else {
                let fi = *x * kindex;
                i = fi as usize;
                j = i + 1;
                dd = vertices[j].dist - vertices[i].dist;
                d = (fi - i as f64) * dd;
            }

            x1 = vertices[i].x;
            y1 = vertices[i].y;
            dx = vertices[j].x - x1;
            dy = vertices[j].y - y1;
        }

        *x = x1 + dx * d / dd;
        *y = y1 + dy * d / dd;
    }
}

impl Default for TransDoublePath {
    fn default() -> Self {
        Self::new()
    }
}

impl Transformer for TransDoublePath {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        if self.status1 != Status::Ready || self.status2 != Status::Ready {
            return;
        }

        if self.base_length > 1e-10 {
            *x *= self.src_vertices1[self.src_vertices1.size() - 1].dist / self.base_length;
        }

        let mut x1 = *x;
        let mut y1 = *y;
        let mut x2 = *x;
        let mut y2 = *y;

        let dd = self.src_vertices2[self.src_vertices2.size() - 1].dist
            / self.src_vertices1[self.src_vertices1.size() - 1].dist;

        self.transform1(&self.src_vertices1, self.kindex1, 1.0, &mut x1, &mut y1);
        self.transform1(&self.src_vertices2, self.kindex2, dd, &mut x2, &mut y2);

        *x = x1 + *y * (x2 - x1) / self.base_height;
        *y = y1 + *y * (y2 - y1) / self.base_height;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_horizontal_paths() {
        let mut tdp = TransDoublePath::new();
        tdp.set_base_height(20.0);

        // Path 1: y=0
        tdp.move_to1(0.0, 0.0);
        tdp.line_to1(100.0, 0.0);

        // Path 2: y=20
        tdp.move_to2(0.0, 20.0);
        tdp.line_to2(100.0, 20.0);

        tdp.finalize_paths();

        assert!((tdp.total_length1() - 100.0).abs() < 1e-10);
        assert!((tdp.total_length2() - 100.0).abs() < 1e-10);

        // Point on path1 (y=0): should map to path1
        let (mut x, mut y) = (50.0, 0.0);
        tdp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);

        // Point halfway between (y=base_height/2=10): should interpolate midway
        let (mut x, mut y) = (50.0, 10.0);
        tdp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);

        // Point on path2 (y=base_height=20): should map to path2
        let (mut x, mut y) = (50.0, 20.0);
        tdp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_diverging_paths() {
        let mut tdp = TransDoublePath::new();
        tdp.set_base_height(10.0);

        // Path 1: y=0
        tdp.move_to1(0.0, 0.0);
        tdp.line_to1(100.0, 0.0);

        // Path 2: y=0 to y=50 (diverging)
        tdp.move_to2(0.0, 0.0);
        tdp.line_to2(100.0, 50.0);

        tdp.finalize_paths();

        // At x=50, y=0 → on path1 → (50, 0)
        let (mut x, mut y) = (50.0, 0.0);
        tdp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-6);
        assert!((y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_base_length() {
        let mut tdp = TransDoublePath::new();
        tdp.set_base_height(10.0);
        tdp.set_base_length(50.0);

        tdp.move_to1(0.0, 0.0);
        tdp.line_to1(100.0, 0.0);
        tdp.move_to2(0.0, 10.0);
        tdp.line_to2(100.0, 10.0);
        tdp.finalize_paths();

        assert!((tdp.total_length1() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_not_ready_is_noop() {
        let tdp = TransDoublePath::new();
        let (mut x, mut y) = (50.0, 25.0);
        tdp.transform(&mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_paths_from_vertex_sources() {
        use crate::path_storage::PathStorage;

        let mut tdp = TransDoublePath::new();
        tdp.set_base_height(20.0);

        let mut p1 = PathStorage::new();
        p1.move_to(0.0, 0.0);
        p1.line_to(100.0, 0.0);

        let mut p2 = PathStorage::new();
        p2.move_to(0.0, 20.0);
        p2.line_to(100.0, 20.0);

        tdp.add_paths(&mut p1, &mut p2, 0, 0);

        assert!((tdp.total_length1() - 100.0).abs() < 1e-10);
        assert!((tdp.total_length2() - 100.0).abs() < 1e-10);
    }
}

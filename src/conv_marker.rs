//! Marker placement converter.
//!
//! Port of `agg_conv_marker.h` — places marker shapes at positions along a
//! path, rotating each marker to align with the edge direction.

use crate::basics::{is_stop, VertexSource, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
use crate::trans_affine::TransAffine;

// ============================================================================
// ConvMarker
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum Status {
    Initial,
    Markers,
    Polygon,
    Stop,
}

/// Place marker shapes at positions along a path.
///
/// Takes a marker locator (which provides edge start/end vertex pairs)
/// and marker shapes (which define the shape to place). Each marker is
/// rotated to match the edge direction and optionally transformed.
///
/// Port of C++ `conv_marker<MarkerLocator, MarkerShapes>`.
pub struct ConvMarker<'a, ML: VertexSource, MS: VertexSource> {
    marker_locator: &'a mut ML,
    marker_shapes: &'a mut MS,
    transform: TransAffine,
    mtx: TransAffine,
    status: Status,
    marker: u32,
    num_markers: u32,
}

impl<'a, ML: VertexSource, MS: VertexSource> ConvMarker<'a, ML, MS> {
    pub fn new(marker_locator: &'a mut ML, marker_shapes: &'a mut MS) -> Self {
        Self {
            marker_locator,
            marker_shapes,
            transform: TransAffine::new(),
            mtx: TransAffine::new(),
            status: Status::Initial,
            marker: 0,
            num_markers: 1,
        }
    }

    pub fn transform(&self) -> &TransAffine {
        &self.transform
    }

    pub fn transform_mut(&mut self) -> &mut TransAffine {
        &mut self.transform
    }
}

impl<ML: VertexSource, MS: VertexSource> VertexSource for ConvMarker<'_, ML, MS> {
    fn rewind(&mut self, _path_id: u32) {
        self.status = Status::Initial;
        self.marker = 0;
        self.num_markers = 1;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        let mut cmd = PATH_CMD_MOVE_TO;
        let mut x1: f64 = 0.0;
        let mut y1: f64 = 0.0;
        let mut x2: f64 = 0.0;
        let mut y2: f64 = 0.0;

        // C++ uses switch with fallthrough: initial → markers → polygon.
        // Rust: loop { match { ... continue for fallthrough } }
        loop {
            if is_stop(cmd) {
                return cmd;
            }
            match self.status {
                Status::Initial => {
                    if self.num_markers == 0 {
                        cmd = PATH_CMD_STOP;
                        continue;
                    }
                    self.marker_locator.rewind(self.marker);
                    self.marker += 1;
                    self.num_markers = 0;
                    self.status = Status::Markers;
                    // fallthrough to Markers
                    continue;
                }
                Status::Markers => {
                    if is_stop(self.marker_locator.vertex(&mut x1, &mut y1)) {
                        self.status = Status::Initial;
                        continue;
                    }
                    if is_stop(self.marker_locator.vertex(&mut x2, &mut y2)) {
                        self.status = Status::Initial;
                        continue;
                    }
                    self.num_markers += 1;
                    self.mtx = self.transform;
                    self.mtx
                        .multiply(&TransAffine::new_rotation((y2 - y1).atan2(x2 - x1)));
                    self.mtx.multiply(&TransAffine::new_translation(x1, y1));
                    self.marker_shapes.rewind(self.marker - 1);
                    self.status = Status::Polygon;
                    // fallthrough to Polygon
                    continue;
                }
                Status::Polygon => {
                    cmd = self.marker_shapes.vertex(x, y);
                    if is_stop(cmd) {
                        cmd = PATH_CMD_MOVE_TO;
                        self.status = Status::Markers;
                        continue;
                    }
                    self.mtx.transform(x, y);
                    return cmd;
                }
                Status::Stop => {
                    cmd = PATH_CMD_STOP;
                    continue;
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_move_to, is_vertex, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};

    /// Simple vertex source that emits edge vertices for path_id 0 only.
    struct SimpleLocator {
        vertices: Vec<(f64, f64, u32)>,
        pos: usize,
        active: bool,
    }

    impl SimpleLocator {
        fn new(vertices: Vec<(f64, f64, u32)>) -> Self {
            Self {
                vertices,
                pos: 0,
                active: false,
            }
        }
    }

    impl VertexSource for SimpleLocator {
        fn rewind(&mut self, path_id: u32) {
            if path_id == 0 {
                self.pos = 0;
                self.active = true;
            } else {
                self.active = false;
            }
        }
        fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
            if !self.active || self.pos >= self.vertices.len() {
                return PATH_CMD_STOP;
            }
            let (vx, vy, cmd) = self.vertices[self.pos];
            *x = vx;
            *y = vy;
            self.pos += 1;
            cmd
        }
    }

    /// Simple triangle marker shape.
    struct TriangleMarker {
        vertices: [(f64, f64, u32); 4],
        pos: usize,
    }

    impl TriangleMarker {
        fn new() -> Self {
            Self {
                vertices: [
                    (0.0, -5.0, PATH_CMD_MOVE_TO),
                    (5.0, 5.0, PATH_CMD_LINE_TO),
                    (-5.0, 5.0, PATH_CMD_LINE_TO),
                    (0.0, 0.0, PATH_CMD_STOP),
                ],
                pos: 0,
            }
        }
    }

    impl VertexSource for TriangleMarker {
        fn rewind(&mut self, _path_id: u32) {
            self.pos = 0;
        }
        fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
            if self.pos >= self.vertices.len() {
                return PATH_CMD_STOP;
            }
            let (vx, vy, cmd) = self.vertices[self.pos];
            *x = vx;
            *y = vy;
            self.pos += 1;
            cmd
        }
    }

    #[test]
    fn test_single_marker_horizontal() {
        // Locator with one horizontal edge: (0,0) → (10,0)
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (10.0, 0.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        // First vertex should be a move_to
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_move_to(cmd));

        // Should get 2 more line_to vertices
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_vertex(cmd));
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_vertex(cmd));
    }

    #[test]
    fn test_marker_at_origin_no_rotation() {
        // Edge along x-axis: rotation = 0, translation = (0,0)
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (10.0, 0.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_move_to(cmd));
        // First vertex of triangle (0, -5) rotated by 0 + translated to (0,0) = (0, -5)
        assert!((x - 0.0).abs() < 1e-8);
        assert!((y - (-5.0)).abs() < 1e-8);
    }

    #[test]
    fn test_marker_terminates() {
        // Single edge, after 3 triangle vertices we should get stop
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (10.0, 0.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        // Read all 3 vertices of the triangle shape
        marker.vertex(&mut x, &mut y);
        marker.vertex(&mut x, &mut y);
        marker.vertex(&mut x, &mut y);
        // Next should be stop (no more edges from locator)
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_user_transform() {
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (10.0, 0.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        // Apply a scale to the user transform
        *marker.transform_mut() = TransAffine::new_scaling_uniform(2.0);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        marker.vertex(&mut x, &mut y);
        // First vertex: (0, -5) scaled by 2 = (0, -10), then rotated 0, translated to (0,0)
        assert!((x - 0.0).abs() < 1e-8);
        assert!((y - (-10.0)).abs() < 1e-8);
    }

    #[test]
    fn test_empty_locator() {
        let mut locator = SimpleLocator::new(vec![]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_single_vertex_locator() {
        // Only one vertex (need 2 for an edge) → should stop
        let mut locator = SimpleLocator::new(vec![(0.0, 0.0, PATH_CMD_MOVE_TO)]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = marker.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_marker_vertical_edge() {
        // Vertical edge: rotation = π/2
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (0.0, 10.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);
        marker.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        marker.vertex(&mut x, &mut y);
        // First vertex: (0, -5) rotated by π/2 → (5, 0), translated to (0,0) → (5, 0)
        assert!((x - 5.0).abs() < 1e-8);
        assert!((y - 0.0).abs() < 1e-8);
    }

    #[test]
    fn test_rewind_resets() {
        let mut locator = SimpleLocator::new(vec![
            (0.0, 0.0, PATH_CMD_MOVE_TO),
            (10.0, 0.0, PATH_CMD_LINE_TO),
        ]);
        let mut shape = TriangleMarker::new();
        let mut marker = ConvMarker::new(&mut locator, &mut shape);

        marker.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;
        marker.vertex(&mut x, &mut y);

        // Rewind and try again — should produce same result
        marker.rewind(0);
        let mut x2 = 0.0;
        let mut y2 = 0.0;
        marker.vertex(&mut x2, &mut y2);
        assert_eq!(x, x2);
        assert_eq!(y, y2);
    }
}

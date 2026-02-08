//! Curve flattening converter.
//!
//! Port of `agg_conv_curve.h` — converts `PATH_CMD_CURVE3` and `PATH_CMD_CURVE4`
//! commands into sequences of `line_to` vertices by approximating the curves
//! with straight line segments.

use crate::basics::{is_stop, VertexSource, PATH_CMD_CURVE3, PATH_CMD_CURVE4, PATH_CMD_LINE_TO};
use crate::curves::{Curve3, Curve4};

// ============================================================================
// ConvCurve
// ============================================================================

/// Curve flattening converter.
///
/// Wraps a `VertexSource` and replaces `curve3`/`curve4` commands with
/// sequences of `line_to` vertices computed by the `Curve3`/`Curve4` classes.
///
/// Port of C++ `conv_curve<VertexSource>`.
pub struct ConvCurve<VS: VertexSource> {
    source: VS,
    last_x: f64,
    last_y: f64,
    curve3: Curve3,
    curve4: Curve4,
}

impl<VS: VertexSource> ConvCurve<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            source,
            last_x: 0.0,
            last_y: 0.0,
            curve3: Curve3::new(),
            curve4: Curve4::new(),
        }
    }

    pub fn source(&self) -> &VS {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut VS {
        &mut self.source
    }

    /// Set the approximation scale for both curve types.
    pub fn set_approximation_scale(&mut self, s: f64) {
        self.curve3.set_approximation_scale(s);
        self.curve4.set_approximation_scale(s);
    }

    pub fn approximation_scale(&self) -> f64 {
        self.curve4.approximation_scale()
    }

    /// Set the angle tolerance for both curve types.
    pub fn set_angle_tolerance(&mut self, v: f64) {
        self.curve3.set_angle_tolerance(v);
        self.curve4.set_angle_tolerance(v);
    }

    pub fn angle_tolerance(&self) -> f64 {
        self.curve4.angle_tolerance()
    }

    /// Set the cusp limit (curve4 only — curve3 does not support cusp limit).
    pub fn set_cusp_limit(&mut self, v: f64) {
        self.curve4.set_cusp_limit(v);
    }

    pub fn cusp_limit(&self) -> f64 {
        self.curve4.cusp_limit()
    }
}

impl<VS: VertexSource> VertexSource for ConvCurve<VS> {
    fn rewind(&mut self, path_id: u32) {
        self.source.rewind(path_id);
        self.last_x = 0.0;
        self.last_y = 0.0;
        self.curve3.reset();
        self.curve4.reset();
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        // First check if curve3 has pending vertices
        if !is_stop(self.curve3.vertex(x, y)) {
            self.last_x = *x;
            self.last_y = *y;
            return PATH_CMD_LINE_TO;
        }

        // Then check if curve4 has pending vertices
        if !is_stop(self.curve4.vertex(x, y)) {
            self.last_x = *x;
            self.last_y = *y;
            return PATH_CMD_LINE_TO;
        }

        // Read next source vertex
        let mut cmd = self.source.vertex(x, y);

        match cmd {
            PATH_CMD_CURVE3 => {
                // Read the endpoint (control point is in x,y)
                let (mut end_x, mut end_y) = (0.0, 0.0);
                self.source.vertex(&mut end_x, &mut end_y);

                self.curve3
                    .init(self.last_x, self.last_y, *x, *y, end_x, end_y);

                // First vertex() call returns move_to (skip it)
                self.curve3.vertex(x, y);
                // Second call is the first curve vertex
                self.curve3.vertex(x, y);
                cmd = PATH_CMD_LINE_TO;
            }
            PATH_CMD_CURVE4 => {
                // Read the second control point and endpoint
                let (mut ct2_x, mut ct2_y) = (0.0, 0.0);
                let (mut end_x, mut end_y) = (0.0, 0.0);
                self.source.vertex(&mut ct2_x, &mut ct2_y);
                self.source.vertex(&mut end_x, &mut end_y);

                self.curve4
                    .init(self.last_x, self.last_y, *x, *y, ct2_x, ct2_y, end_x, end_y);

                // First vertex() call returns move_to (skip it)
                self.curve4.vertex(x, y);
                // Second call is the first curve vertex
                self.curve4.vertex(x, y);
                cmd = PATH_CMD_LINE_TO;
            }
            _ => {}
        }

        self.last_x = *x;
        self.last_y = *y;
        cmd
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::PATH_CMD_MOVE_TO;
    use crate::path_storage::PathStorage;

    fn collect_vertices<VS: VertexSource>(vs: &mut VS) -> Vec<(f64, f64, u32)> {
        let mut result = Vec::new();
        vs.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            result.push((x, y, cmd));
        }
        result
    }

    #[test]
    fn test_no_curves_passthrough() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 20.0);
        path.line_to(30.0, 40.0);
        path.line_to(50.0, 60.0);

        let mut cc = ConvCurve::new(path);
        let verts = collect_vertices(&mut cc);
        assert_eq!(verts.len(), 3);
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        assert_eq!(verts[1].2, PATH_CMD_LINE_TO);
        assert!((verts[0].0 - 10.0).abs() < 1e-10);
        assert!((verts[2].0 - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_curve3_flattening() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.curve3(50.0, 100.0, 100.0, 0.0);

        let mut cc = ConvCurve::new(path);
        let verts = collect_vertices(&mut cc);

        // Should have move_to + multiple line_to vertices
        assert!(
            verts.len() > 2,
            "Expected multiple vertices, got {}",
            verts.len()
        );
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        // All subsequent should be line_to
        for v in &verts[1..] {
            assert_eq!(v.2, PATH_CMD_LINE_TO);
        }
        // First vertex is the start point
        assert!((verts[0].0).abs() < 1e-10);
        assert!((verts[0].1).abs() < 1e-10);
        // Last vertex should be near the endpoint
        let last = verts.last().unwrap();
        assert!((last.0 - 100.0).abs() < 1.0, "End x={}", last.0);
        assert!((last.1).abs() < 1.0, "End y={}", last.1);
    }

    #[test]
    fn test_curve4_flattening() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.curve4(33.0, 100.0, 66.0, 100.0, 100.0, 0.0);

        let mut cc = ConvCurve::new(path);
        let verts = collect_vertices(&mut cc);

        assert!(
            verts.len() > 2,
            "Expected multiple vertices, got {}",
            verts.len()
        );
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        for v in &verts[1..] {
            assert_eq!(v.2, PATH_CMD_LINE_TO);
        }
        // Last vertex near endpoint
        let last = verts.last().unwrap();
        assert!((last.0 - 100.0).abs() < 1.0, "End x={}", last.0);
    }

    #[test]
    fn test_mixed_lines_and_curves() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(50.0, 0.0);
        path.curve3(75.0, 50.0, 100.0, 0.0);
        path.line_to(150.0, 0.0);

        let mut cc = ConvCurve::new(path);
        let verts = collect_vertices(&mut cc);

        // Should have: move_to, line_to(50,0), curve3 flattened, line_to(150,0)
        assert!(
            verts.len() > 4,
            "Expected > 4 vertices, got {}",
            verts.len()
        );
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_approximation_scale() {
        let path = PathStorage::new();
        let mut cc = ConvCurve::new(path);
        cc.set_approximation_scale(2.0);
        assert!((cc.approximation_scale() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_rewind_resets() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.curve3(50.0, 100.0, 100.0, 0.0);

        let mut cc = ConvCurve::new(path);
        let verts1 = collect_vertices(&mut cc);
        let verts2 = collect_vertices(&mut cc);
        assert_eq!(verts1.len(), verts2.len());
    }

    #[test]
    fn test_empty_path() {
        let path = PathStorage::new();
        let mut cc = ConvCurve::new(path);
        let verts = collect_vertices(&mut cc);
        assert_eq!(verts.len(), 0);
    }

    #[test]
    fn test_source_access() {
        let path = PathStorage::new();
        let cc = ConvCurve::new(path);
        let _ = cc.source();
    }

    #[test]
    fn test_angle_tolerance_and_cusp_limit() {
        let path = PathStorage::new();
        let mut cc = ConvCurve::new(path);
        cc.set_angle_tolerance(0.5);
        assert!((cc.angle_tolerance() - 0.5).abs() < 1e-10);
        cc.set_cusp_limit(2.0);
        assert!((cc.cusp_limit() - 2.0).abs() < 1e-10);
    }
}

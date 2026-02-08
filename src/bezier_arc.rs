//! Bezier arc generator.
//!
//! Port of `agg_bezier_arc.h` / `agg_bezier_arc.cpp` — converts elliptical
//! arcs into sequences of cubic Bezier curves. Produces at most 4 consecutive
//! cubic Bezier curves (4, 7, 10, or 13 vertices).

use crate::basics::{
    VertexSource, PATH_CMD_CURVE4, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PI,
};
use crate::trans_affine::TransAffine;

/// Epsilon to prevent adding degenerate curves.
const BEZIER_ARC_ANGLE_EPSILON: f64 = 0.01;

/// Convert an arc segment to a single cubic Bezier curve (4 control points).
///
/// Writes 8 values to `curve`: `[x0, y0, x1, y1, x2, y2, x3, y3]`.
pub fn arc_to_bezier(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    start_angle: f64,
    sweep_angle: f64,
    curve: &mut [f64],
) {
    let x0 = (sweep_angle / 2.0).cos();
    let y0 = (sweep_angle / 2.0).sin();
    let tx = (1.0 - x0) * 4.0 / 3.0;
    let ty = y0 - tx * x0 / y0;

    let px = [x0, x0 + tx, x0 + tx, x0];
    let py = [-y0, -ty, ty, y0];

    let sn = (start_angle + sweep_angle / 2.0).sin();
    let cs = (start_angle + sweep_angle / 2.0).cos();

    for i in 0..4 {
        curve[i * 2] = cx + rx * (px[i] * cs - py[i] * sn);
        curve[i * 2 + 1] = cy + ry * (px[i] * sn + py[i] * cs);
    }
}

/// Bezier arc generator.
///
/// Generates up to 4 consecutive cubic Bezier curves from an elliptical arc.
///
/// Port of C++ `agg::bezier_arc`.
pub struct BezierArc {
    vertex: usize,
    num_vertices: usize,
    vertices: [f64; 26],
    cmd: u32,
}

impl BezierArc {
    /// Create an uninitialized bezier arc.
    pub fn new() -> Self {
        Self {
            vertex: 26,
            num_vertices: 0,
            vertices: [0.0; 26],
            cmd: PATH_CMD_LINE_TO,
        }
    }

    /// Create and initialize a bezier arc.
    pub fn new_with_params(
        x: f64,
        y: f64,
        rx: f64,
        ry: f64,
        start_angle: f64,
        sweep_angle: f64,
    ) -> Self {
        let mut arc = Self::new();
        arc.init(x, y, rx, ry, start_angle, sweep_angle);
        arc
    }

    /// Initialize the arc with center, radii, and angle parameters.
    pub fn init(&mut self, x: f64, y: f64, rx: f64, ry: f64, start_angle: f64, sweep_angle: f64) {
        let mut start_angle = start_angle % (2.0 * PI);
        let mut sweep_angle = sweep_angle;

        if sweep_angle >= 2.0 * PI {
            sweep_angle = 2.0 * PI;
        }
        if sweep_angle <= -2.0 * PI {
            sweep_angle = -2.0 * PI;
        }

        if sweep_angle.abs() < 1e-10 {
            self.num_vertices = 4;
            self.cmd = PATH_CMD_LINE_TO;
            self.vertices[0] = x + rx * start_angle.cos();
            self.vertices[1] = y + ry * start_angle.sin();
            self.vertices[2] = x + rx * (start_angle + sweep_angle).cos();
            self.vertices[3] = y + ry * (start_angle + sweep_angle).sin();
            return;
        }

        let mut total_sweep = 0.0;
        let mut local_sweep;
        self.num_vertices = 2;
        self.cmd = PATH_CMD_CURVE4;
        let mut done = false;

        loop {
            if sweep_angle < 0.0 {
                let prev_sweep = total_sweep;
                local_sweep = -PI * 0.5;
                total_sweep -= PI * 0.5;
                if total_sweep <= sweep_angle + BEZIER_ARC_ANGLE_EPSILON {
                    local_sweep = sweep_angle - prev_sweep;
                    done = true;
                }
            } else {
                let prev_sweep = total_sweep;
                local_sweep = PI * 0.5;
                total_sweep += PI * 0.5;
                if total_sweep >= sweep_angle - BEZIER_ARC_ANGLE_EPSILON {
                    local_sweep = sweep_angle - prev_sweep;
                    done = true;
                }
            }

            arc_to_bezier(
                x,
                y,
                rx,
                ry,
                start_angle,
                local_sweep,
                &mut self.vertices[self.num_vertices - 2..],
            );

            self.num_vertices += 6;
            start_angle += local_sweep;

            if done || self.num_vertices >= 26 {
                break;
            }
        }
    }

    /// Number of coordinate values (doubled number of vertices).
    pub fn num_vertices(&self) -> usize {
        self.num_vertices
    }

    /// Access the vertex coordinate array.
    pub fn vertices(&self) -> &[f64; 26] {
        &self.vertices
    }

    /// Mutable access to the vertex coordinate array.
    pub fn vertices_mut(&mut self) -> &mut [f64; 26] {
        &mut self.vertices
    }
}

impl Default for BezierArc {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for BezierArc {
    fn rewind(&mut self, _path_id: u32) {
        self.vertex = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex >= self.num_vertices {
            return PATH_CMD_STOP;
        }
        *x = self.vertices[self.vertex];
        *y = self.vertices[self.vertex + 1];
        self.vertex += 2;
        if self.vertex == 2 {
            PATH_CMD_MOVE_TO
        } else {
            self.cmd
        }
    }
}

/// SVG-style bezier arc generator.
///
/// Computes an elliptical arc from `(x1, y1)` to `(x2, y2)` using SVG
/// endpoint parameterization (radii, rotation, flags).
///
/// Port of C++ `agg::bezier_arc_svg`.
pub struct BezierArcSvg {
    arc: BezierArc,
    radii_ok: bool,
}

impl BezierArcSvg {
    /// Create an uninitialized SVG bezier arc.
    pub fn new() -> Self {
        Self {
            arc: BezierArc::new(),
            radii_ok: false,
        }
    }

    /// Create and initialize an SVG bezier arc.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_params(
        x1: f64,
        y1: f64,
        rx: f64,
        ry: f64,
        angle: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        x2: f64,
        y2: f64,
    ) -> Self {
        let mut svg = Self::new();
        svg.init(x1, y1, rx, ry, angle, large_arc_flag, sweep_flag, x2, y2);
        svg
    }

    /// Initialize with SVG arc parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        &mut self,
        x0: f64,
        y0: f64,
        rx: f64,
        ry: f64,
        angle: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        x2: f64,
        y2: f64,
    ) {
        self.radii_ok = true;

        let mut rx = rx.abs();
        let mut ry = ry.abs();

        // Calculate midpoint
        let dx2 = (x0 - x2) / 2.0;
        let dy2 = (y0 - y2) / 2.0;

        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Rotate to align with axes
        let x1 = cos_a * dx2 + sin_a * dy2;
        let y1 = -sin_a * dx2 + cos_a * dy2;

        // Ensure radii are large enough
        let mut prx = rx * rx;
        let mut pry = ry * ry;
        let px1 = x1 * x1;
        let py1 = y1 * y1;

        let radii_check = px1 / prx + py1 / pry;
        if radii_check > 1.0 {
            rx *= radii_check.sqrt();
            ry *= radii_check.sqrt();
            prx = rx * rx;
            pry = ry * ry;
            if radii_check > 10.0 {
                self.radii_ok = false;
            }
        }

        // Calculate center
        let sign = if large_arc_flag == sweep_flag {
            -1.0
        } else {
            1.0
        };
        let sq = (prx * pry - prx * py1 - pry * px1) / (prx * py1 + pry * px1);
        let coef = sign * (if sq < 0.0 { 0.0 } else { sq }).sqrt();
        let cx1 = coef * ((rx * y1) / ry);
        let cy1 = coef * -((ry * x1) / rx);

        // Transform center back
        let sx2 = (x0 + x2) / 2.0;
        let sy2 = (y0 + y2) / 2.0;
        let cx = sx2 + (cos_a * cx1 - sin_a * cy1);
        let cy = sy2 + (sin_a * cx1 + cos_a * cy1);

        // Calculate angles
        let ux = (x1 - cx1) / rx;
        let uy = (y1 - cy1) / ry;
        let vx = (-x1 - cx1) / rx;
        let vy = (-y1 - cy1) / ry;

        // Start angle
        let n = (ux * ux + uy * uy).sqrt();
        let p = ux;
        let sign = if uy < 0.0 { -1.0 } else { 1.0 };
        let v = (p / n).clamp(-1.0, 1.0);
        let start_angle = sign * v.acos();

        // Sweep angle
        let n = ((ux * ux + uy * uy) * (vx * vx + vy * vy)).sqrt();
        let p = ux * vx + uy * vy;
        let sign = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        let v = (p / n).clamp(-1.0, 1.0);
        let mut sweep_angle = sign * v.acos();

        if !sweep_flag && sweep_angle > 0.0 {
            sweep_angle -= PI * 2.0;
        } else if sweep_flag && sweep_angle < 0.0 {
            sweep_angle += PI * 2.0;
        }

        // Build and transform the arc
        self.arc.init(0.0, 0.0, rx, ry, start_angle, sweep_angle);

        let mut mtx = TransAffine::new_rotation(angle);
        mtx.multiply(&TransAffine::new_translation(cx, cy));

        let nv = self.arc.num_vertices();
        if nv > 4 {
            let verts = self.arc.vertices_mut();
            for i in (2..nv - 2).step_by(2) {
                let mut tx = verts[i];
                let mut ty = verts[i + 1];
                mtx.transform(&mut tx, &mut ty);
                verts[i] = tx;
                verts[i + 1] = ty;
            }
        }

        // Ensure exact start and end points
        let verts = self.arc.vertices_mut();
        verts[0] = x0;
        verts[1] = y0;
        if nv > 2 {
            verts[nv - 2] = x2;
            verts[nv - 1] = y2;
        }
    }

    /// Whether the radii were sufficient (not enlarged).
    pub fn radii_ok(&self) -> bool {
        self.radii_ok
    }

    /// Number of coordinate values.
    pub fn num_vertices(&self) -> usize {
        self.arc.num_vertices()
    }

    /// Access the vertex coordinate array.
    pub fn vertices(&self) -> &[f64; 26] {
        self.arc.vertices()
    }
}

impl Default for BezierArcSvg {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for BezierArcSvg {
    fn rewind(&mut self, _path_id: u32) {
        self.arc.rewind(0);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.arc.vertex(x, y)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::is_stop;

    #[test]
    fn test_bezier_arc_quarter() {
        let mut arc = BezierArc::new_with_params(0.0, 0.0, 10.0, 10.0, 0.0, PI / 2.0);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // Should start with move_to
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Next 3 vertices are curve4 control points
        for _ in 0..3 {
            let cmd = arc.vertex(&mut x, &mut y);
            assert_eq!(cmd, PATH_CMD_CURVE4);
        }

        // Should stop
        let cmd = arc.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_bezier_arc_full_circle() {
        let arc = BezierArc::new_with_params(0.0, 0.0, 10.0, 10.0, 0.0, 2.0 * PI);
        // Full circle: 4 quadrants × 3 control points + 1 start + 1 end = 14 values
        // Actually: num_vertices = 2 + 4*6 = 26
        assert_eq!(arc.num_vertices(), 26);
    }

    #[test]
    fn test_bezier_arc_half_circle() {
        let arc = BezierArc::new_with_params(0.0, 0.0, 10.0, 10.0, 0.0, PI);
        // Half circle: 2 quadrants × 6 + 2 = 14
        assert_eq!(arc.num_vertices(), 14);
    }

    #[test]
    fn test_bezier_arc_tiny_sweep() {
        let arc = BezierArc::new_with_params(0.0, 0.0, 10.0, 10.0, 0.0, 1e-15);
        // Tiny sweep: degenerate case, just a line
        assert_eq!(arc.num_vertices(), 4);
        assert_eq!(arc.cmd, PATH_CMD_LINE_TO);
    }

    #[test]
    fn test_bezier_arc_negative_sweep() {
        let arc = BezierArc::new_with_params(0.0, 0.0, 10.0, 10.0, 0.0, -PI / 2.0);
        // Negative quarter arc
        assert_eq!(arc.num_vertices(), 8);
    }

    #[test]
    fn test_bezier_arc_svg_basic() {
        let mut svg = BezierArcSvg::new_with_params(
            0.0, 10.0, // start
            10.0, 10.0, // radii
            0.0,  // angle
            false, true, // flags
            10.0, 0.0, // end
        );
        assert!(svg.radii_ok());

        svg.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex should be the start point
        let cmd = svg.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 0.0).abs() < 1e-6);
        assert!((y - 10.0).abs() < 1e-6);

        // Consume remaining vertices
        let mut count = 1;
        while !is_stop(svg.vertex(&mut x, &mut y)) {
            count += 1;
        }
        assert!(count >= 4);

        // Last vertex should be near the end point
        assert!((x - 10.0).abs() < 1e-6);
        assert!((y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_arc_to_bezier_endpoints() {
        let mut curve = [0.0; 8];
        arc_to_bezier(0.0, 0.0, 10.0, 10.0, 0.0, PI / 2.0, &mut curve);

        // Start point should be at angle 0 → (10, 0)
        let start_x = curve[0];
        let start_y = curve[1];
        assert!((start_x - 10.0).abs() < 1e-6);
        assert!(start_y.abs() < 1e-6);

        // End point should be at angle π/2 → (0, 10)
        let end_x = curve[6];
        let end_y = curve[7];
        assert!(end_x.abs() < 1e-6);
        assert!((end_y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_bezier_arc_default() {
        let arc = BezierArc::new();
        assert_eq!(arc.num_vertices(), 0);
    }

    #[test]
    fn test_bezier_arc_svg_small_radii() {
        // Radii too small - should be enlarged
        let svg = BezierArcSvg::new_with_params(
            0.0, 0.0, // start
            1.0, 1.0, // tiny radii
            0.0, // angle
            false, true, // flags
            100.0, 100.0, // end far away
        );
        // Radii were enlarged significantly
        assert!(!svg.radii_ok());
    }
}

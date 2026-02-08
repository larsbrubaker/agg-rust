//! Ellipse vertex generator.
//!
//! Port of `agg_ellipse.h` — generates vertices approximating an ellipse
//! as a regular polygon, suitable for use as a VertexSource.

use crate::basics::{
    uround, VertexSource, PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
    PATH_FLAGS_CCW, PATH_FLAGS_CLOSE, PI,
};

/// Ellipse vertex generator.
///
/// Generates a closed polygon approximating an ellipse. The number of
/// steps is either specified explicitly or calculated automatically from
/// the approximation scale.
///
/// Port of C++ `agg::ellipse`.
pub struct Ellipse {
    x: f64,
    y: f64,
    rx: f64,
    ry: f64,
    scale: f64,
    num: u32,
    step: u32,
    cw: bool,
}

impl Ellipse {
    /// Create a new ellipse with automatic step calculation.
    pub fn new(x: f64, y: f64, rx: f64, ry: f64, num_steps: u32, cw: bool) -> Self {
        let mut e = Self {
            x,
            y,
            rx,
            ry,
            scale: 1.0,
            num: num_steps,
            step: 0,
            cw,
        };
        if e.num == 0 {
            e.calc_num_steps();
        }
        e
    }

    /// Create a default ellipse (unit circle at origin).
    pub fn default_new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rx: 1.0,
            ry: 1.0,
            scale: 1.0,
            num: 4,
            step: 0,
            cw: false,
        }
    }

    /// Re-initialize with new parameters.
    pub fn init(&mut self, x: f64, y: f64, rx: f64, ry: f64, num_steps: u32, cw: bool) {
        self.x = x;
        self.y = y;
        self.rx = rx;
        self.ry = ry;
        self.num = num_steps;
        self.step = 0;
        self.cw = cw;
        if self.num == 0 {
            self.calc_num_steps();
        }
    }

    /// Set approximation scale (affects automatic step count).
    pub fn set_approximation_scale(&mut self, scale: f64) {
        self.scale = scale;
        self.calc_num_steps();
    }

    /// Calculate step count from radii and approximation scale.
    fn calc_num_steps(&mut self) {
        let ra = (self.rx.abs() + self.ry.abs()) / 2.0;
        let da = (ra / (ra + 0.125 / self.scale)).acos() * 2.0;
        self.num = uround(2.0 * PI / da);
    }
}

impl VertexSource for Ellipse {
    fn rewind(&mut self, _path_id: u32) {
        self.step = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.step == self.num {
            self.step += 1;
            return PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
        }
        if self.step > self.num {
            return PATH_CMD_STOP;
        }
        let mut angle = self.step as f64 / self.num as f64 * 2.0 * PI;
        if self.cw {
            angle = 2.0 * PI - angle;
        }
        *x = self.x + angle.cos() * self.rx;
        *y = self.y + angle.sin() * self.ry;
        self.step += 1;
        if self.step == 1 {
            PATH_CMD_MOVE_TO
        } else {
            PATH_CMD_LINE_TO
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_end_poly, is_stop};

    #[test]
    fn test_ellipse_basic() {
        let mut e = Ellipse::new(0.0, 0.0, 10.0, 10.0, 8, false);
        e.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex = move_to at angle 0
        let cmd = e.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Next 7 vertices = line_to
        for _ in 1..8 {
            let cmd = e.vertex(&mut x, &mut y);
            assert_eq!(cmd, PATH_CMD_LINE_TO);
        }

        // Close polygon
        let cmd = e.vertex(&mut x, &mut y);
        assert!(is_end_poly(cmd));

        // Stop
        let cmd = e.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_ellipse_vertices_on_circle() {
        let mut e = Ellipse::new(0.0, 0.0, 10.0, 10.0, 4, false);
        e.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // Vertex 0: (10, 0)
        e.vertex(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Vertex 1: (0, 10)
        e.vertex(&mut x, &mut y);
        assert!(x.abs() < 1e-6);
        assert!((y - 10.0).abs() < 1e-6);

        // Vertex 2: (-10, 0)
        e.vertex(&mut x, &mut y);
        assert!((x + 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Vertex 3: (0, -10)
        e.vertex(&mut x, &mut y);
        assert!(x.abs() < 1e-6);
        assert!((y + 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_ellipse_cw() {
        let mut e = Ellipse::new(0.0, 0.0, 10.0, 10.0, 4, true);
        e.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // Vertex 0: still (10, 0) — angle 0
        e.vertex(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-6);

        // Vertex 1: CW = (0, -10) instead of (0, 10)
        e.vertex(&mut x, &mut y);
        assert!(x.abs() < 1e-6);
        assert!((y + 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_ellipse_center_offset() {
        let mut e = Ellipse::new(5.0, 3.0, 10.0, 10.0, 4, false);
        e.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        e.vertex(&mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-6);
        assert!((y - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_ellipse_auto_steps() {
        let e = Ellipse::new(0.0, 0.0, 100.0, 100.0, 0, false);
        // Auto-calculated steps should be reasonable for r=100
        assert!(e.num > 20);
    }

    #[test]
    fn test_ellipse_rewind_restarts() {
        let mut e = Ellipse::new(0.0, 0.0, 10.0, 10.0, 4, false);
        let mut x = 0.0;
        let mut y = 0.0;

        // Consume some vertices
        e.rewind(0);
        e.vertex(&mut x, &mut y);
        e.vertex(&mut x, &mut y);

        // Rewind should restart
        e.rewind(0);
        let cmd = e.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_ellipse_different_radii() {
        let mut e = Ellipse::new(0.0, 0.0, 20.0, 10.0, 4, false);
        e.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // Vertex 0: (20, 0) — rx
        e.vertex(&mut x, &mut y);
        assert!((x - 20.0).abs() < 1e-6);

        // Vertex 1: (0, 10) — ry
        e.vertex(&mut x, &mut y);
        assert!(x.abs() < 1e-6);
        assert!((y - 10.0).abs() < 1e-6);
    }
}

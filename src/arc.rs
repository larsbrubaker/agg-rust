//! Arc vertex generator.
//!
//! Port of `agg_arc.h` / `agg_arc.cpp` — generates vertices along an
//! elliptical arc, suitable for use as a VertexSource in the rendering
//! pipeline.

use crate::basics::{is_stop, VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PI};

/// Arc vertex generator.
///
/// Generates vertices along an elliptical arc defined by center, radii,
/// start angle, end angle, and direction (CW/CCW).
///
/// Port of C++ `agg::arc`.
pub struct Arc {
    x: f64,
    y: f64,
    rx: f64,
    ry: f64,
    angle: f64,
    start: f64,
    end: f64,
    scale: f64,
    da: f64,
    ccw: bool,
    initialized: bool,
    path_cmd: u32,
}

impl Arc {
    /// Create a new arc.
    #[allow(clippy::too_many_arguments)]
    pub fn new(x: f64, y: f64, rx: f64, ry: f64, a1: f64, a2: f64, ccw: bool) -> Self {
        let mut arc = Self {
            x,
            y,
            rx,
            ry,
            angle: 0.0,
            start: 0.0,
            end: 0.0,
            scale: 1.0,
            da: 0.0,
            ccw: false,
            initialized: false,
            path_cmd: PATH_CMD_STOP,
        };
        arc.normalize(a1, a2, ccw);
        arc
    }

    /// Create a default (uninitialized) arc.
    pub fn default_new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rx: 0.0,
            ry: 0.0,
            angle: 0.0,
            start: 0.0,
            end: 0.0,
            scale: 1.0,
            da: 0.0,
            ccw: false,
            initialized: false,
            path_cmd: PATH_CMD_STOP,
        }
    }

    /// Re-initialize with new parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, x: f64, y: f64, rx: f64, ry: f64, a1: f64, a2: f64, ccw: bool) {
        self.x = x;
        self.y = y;
        self.rx = rx;
        self.ry = ry;
        self.normalize(a1, a2, ccw);
    }

    /// Set approximation scale (affects step size).
    pub fn set_approximation_scale(&mut self, s: f64) {
        self.scale = s;
        if self.initialized {
            self.normalize(self.start, self.end, self.ccw);
        }
    }

    /// Get current approximation scale.
    pub fn approximation_scale(&self) -> f64 {
        self.scale
    }

    /// Normalize angles and compute step size.
    fn normalize(&mut self, a1: f64, a2: f64, ccw: bool) {
        let ra = (self.rx.abs() + self.ry.abs()) / 2.0;
        self.da = (ra / (ra + 0.125 / self.scale)).acos() * 2.0;

        let mut a1 = a1;
        let mut a2 = a2;

        if ccw {
            while a2 < a1 {
                a2 += PI * 2.0;
            }
        } else {
            while a1 < a2 {
                a1 += PI * 2.0;
            }
            self.da = -self.da;
        }

        self.ccw = ccw;
        self.start = a1;
        self.end = a2;
        self.initialized = true;
    }
}

impl VertexSource for Arc {
    fn rewind(&mut self, _path_id: u32) {
        self.path_cmd = PATH_CMD_MOVE_TO;
        self.angle = self.start;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if is_stop(self.path_cmd) {
            return PATH_CMD_STOP;
        }

        if (self.angle < self.end - self.da / 4.0) != self.ccw {
            *x = self.x + self.end.cos() * self.rx;
            *y = self.y + self.end.sin() * self.ry;
            self.path_cmd = PATH_CMD_STOP;
            return PATH_CMD_LINE_TO;
        }

        *x = self.x + self.angle.cos() * self.rx;
        *y = self.y + self.angle.sin() * self.ry;

        self.angle += self.da;

        let pf = self.path_cmd;
        self.path_cmd = PATH_CMD_LINE_TO;
        pf
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_full_circle_ccw() {
        let mut arc = Arc::new(0.0, 0.0, 10.0, 10.0, 0.0, PI * 2.0, true);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex should be move_to
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Subsequent vertices should be line_to
        let mut count = 1;
        loop {
            let cmd = arc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            assert_eq!(cmd, PATH_CMD_LINE_TO);
            count += 1;
        }
        // Full circle should generate multiple vertices
        assert!(count > 4);
    }

    #[test]
    fn test_arc_quarter_circle() {
        let mut arc = Arc::new(0.0, 0.0, 10.0, 10.0, 0.0, PI / 2.0, true);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex at angle 0
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);

        // Collect last vertex (should be near angle PI/2)
        let mut last_x = x;
        let mut last_y = y;
        loop {
            let cmd = arc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            last_x = x;
            last_y = y;
        }
        // Last vertex should be at (0, 10) approximately
        assert!(last_x.abs() < 1e-6);
        assert!((last_y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_arc_cw_direction() {
        let mut arc = Arc::new(0.0, 0.0, 10.0, 10.0, PI / 2.0, 0.0, false);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex should be at angle PI/2 (= 0, 10)
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!(x.abs() < 1e-6);
        assert!((y - 10.0).abs() < 1e-6);

        // Collect all vertices, last should be near (10, 0)
        let mut last_x = x;
        let mut last_y = y;
        loop {
            let cmd = arc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            last_x = x;
            last_y = y;
        }
        assert!((last_x - 10.0).abs() < 1e-6);
        assert!(last_y.abs() < 1e-6);
    }

    #[test]
    fn test_arc_elliptical() {
        let mut arc = Arc::new(5.0, 5.0, 20.0, 10.0, 0.0, PI / 2.0, true);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // First vertex at center + (rx, 0)
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 25.0).abs() < 1e-6); // 5 + 20
        assert!((y - 5.0).abs() < 1e-6);

        // Last vertex at center + (0, ry)
        let mut last_x = x;
        let mut last_y = y;
        loop {
            let cmd = arc.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            last_x = x;
            last_y = y;
        }
        assert!((last_x - 5.0).abs() < 1e-6); // center x
        assert!((last_y - 15.0).abs() < 1e-6); // 5 + 10
    }

    #[test]
    fn test_arc_rewind_restarts() {
        let mut arc = Arc::new(0.0, 0.0, 10.0, 10.0, 0.0, PI, true);
        let mut x = 0.0;
        let mut y = 0.0;

        // Consume all vertices
        arc.rewind(0);
        while !is_stop(arc.vertex(&mut x, &mut y)) {}

        // After rewind, should start over
        arc.rewind(0);
        let cmd = arc.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_arc_approximation_scale() {
        let mut arc = Arc::new(0.0, 0.0, 100.0, 100.0, 0.0, PI * 2.0, true);
        arc.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;
        let mut count1 = 0;
        while !is_stop(arc.vertex(&mut x, &mut y)) {
            count1 += 1;
        }

        // Higher scale = more vertices
        arc.set_approximation_scale(4.0);
        arc.rewind(0);
        let mut count2 = 0;
        while !is_stop(arc.vertex(&mut x, &mut y)) {
            count2 += 1;
        }
        assert!(count2 > count1);
    }

    #[test]
    fn test_arc_default_new() {
        let arc = Arc::default_new();
        assert_eq!(arc.approximation_scale(), 1.0);
        assert!(!arc.initialized);
    }
}

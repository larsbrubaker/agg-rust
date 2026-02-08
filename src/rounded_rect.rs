//! Rounded rectangle vertex generator.
//!
//! Port of `agg_rounded_rect.h` / `agg_rounded_rect.cpp` — generates vertices
//! for a rectangle with independently controllable corner radii.

use crate::arc::Arc;
use crate::basics::{
    is_stop, VertexSource, PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_STOP, PATH_FLAGS_CCW,
    PATH_FLAGS_CLOSE, PI,
};

/// Rounded rectangle vertex source.
///
/// Generates vertices for a rectangle where each corner can have its own
/// elliptical radius. The corners are emitted as arcs (bottom-left, bottom-right,
/// top-right, top-left) connected by line segments, forming a closed polygon.
///
/// Port of C++ `agg::rounded_rect`.
pub struct RoundedRect {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    rx1: f64,
    ry1: f64,
    rx2: f64,
    ry2: f64,
    rx3: f64,
    ry3: f64,
    rx4: f64,
    ry4: f64,
    status: u32,
    arc: Arc,
}

impl RoundedRect {
    /// Create a new rounded rectangle with uniform corner radius.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64, r: f64) -> Self {
        let (x1, x2) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
        let (y1, y2) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
        Self {
            x1,
            y1,
            x2,
            y2,
            rx1: r,
            ry1: r,
            rx2: r,
            ry2: r,
            rx3: r,
            ry3: r,
            rx4: r,
            ry4: r,
            status: 0,
            arc: Arc::default_new(),
        }
    }

    /// Create a default (zero-sized) rounded rectangle.
    pub fn default_new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
            rx1: 0.0,
            ry1: 0.0,
            rx2: 0.0,
            ry2: 0.0,
            rx3: 0.0,
            ry3: 0.0,
            rx4: 0.0,
            ry4: 0.0,
            status: 0,
            arc: Arc::default_new(),
        }
    }

    /// Set the rectangle coordinates.
    pub fn rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;
        if x1 > x2 {
            self.x1 = x2;
            self.x2 = x1;
        }
        if y1 > y2 {
            self.y1 = y2;
            self.y2 = y1;
        }
    }

    /// Set uniform corner radius.
    pub fn radius(&mut self, r: f64) {
        self.rx1 = r;
        self.ry1 = r;
        self.rx2 = r;
        self.ry2 = r;
        self.rx3 = r;
        self.ry3 = r;
        self.rx4 = r;
        self.ry4 = r;
    }

    /// Set corner radii with separate x/y values (uniform across corners).
    pub fn radius_xy(&mut self, rx: f64, ry: f64) {
        self.rx1 = rx;
        self.rx2 = rx;
        self.rx3 = rx;
        self.rx4 = rx;
        self.ry1 = ry;
        self.ry2 = ry;
        self.ry3 = ry;
        self.ry4 = ry;
    }

    /// Set corner radii for bottom and top edges.
    pub fn radius_bottom_top(&mut self, rx_bottom: f64, ry_bottom: f64, rx_top: f64, ry_top: f64) {
        self.rx1 = rx_bottom;
        self.rx2 = rx_bottom;
        self.rx3 = rx_top;
        self.rx4 = rx_top;
        self.ry1 = ry_bottom;
        self.ry2 = ry_bottom;
        self.ry3 = ry_top;
        self.ry4 = ry_top;
    }

    /// Set each corner radius individually.
    ///
    /// Corners are numbered 1-4 starting from bottom-left going clockwise:
    /// 1 = bottom-left, 2 = bottom-right, 3 = top-right, 4 = top-left.
    #[allow(clippy::too_many_arguments)]
    pub fn radius_all(
        &mut self,
        rx1: f64,
        ry1: f64,
        rx2: f64,
        ry2: f64,
        rx3: f64,
        ry3: f64,
        rx4: f64,
        ry4: f64,
    ) {
        self.rx1 = rx1;
        self.ry1 = ry1;
        self.rx2 = rx2;
        self.ry2 = ry2;
        self.rx3 = rx3;
        self.ry3 = ry3;
        self.rx4 = rx4;
        self.ry4 = ry4;
    }

    /// Normalize radii so they don't exceed rectangle dimensions.
    ///
    /// If the sum of adjacent corner radii exceeds the corresponding
    /// dimension, all radii are uniformly scaled down.
    pub fn normalize_radius(&mut self) {
        let dx = (self.y2 - self.y1).abs();
        let dy = (self.x2 - self.x1).abs();

        let mut k = 1.0_f64;
        let t = dx / (self.rx1 + self.rx2);
        if t < k {
            k = t;
        }
        let t = dx / (self.rx3 + self.rx4);
        if t < k {
            k = t;
        }
        let t = dy / (self.ry1 + self.ry2);
        if t < k {
            k = t;
        }
        let t = dy / (self.ry3 + self.ry4);
        if t < k {
            k = t;
        }

        if k < 1.0 {
            self.rx1 *= k;
            self.ry1 *= k;
            self.rx2 *= k;
            self.ry2 *= k;
            self.rx3 *= k;
            self.ry3 *= k;
            self.rx4 *= k;
            self.ry4 *= k;
        }
    }

    /// Set the approximation scale for arc generation.
    pub fn set_approximation_scale(&mut self, s: f64) {
        self.arc.set_approximation_scale(s);
    }

    /// Get the current approximation scale.
    pub fn approximation_scale(&self) -> f64 {
        self.arc.approximation_scale()
    }
}

impl VertexSource for RoundedRect {
    fn rewind(&mut self, _path_id: u32) {
        self.status = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        let mut cmd;
        loop {
            match self.status {
                0 => {
                    // Bottom-left corner arc (PI to 3PI/2)
                    self.arc.init(
                        self.x1 + self.rx1,
                        self.y1 + self.ry1,
                        self.rx1,
                        self.ry1,
                        PI,
                        PI + PI * 0.5,
                        true,
                    );
                    self.arc.rewind(0);
                    self.status += 1;
                }
                1 => {
                    cmd = self.arc.vertex(x, y);
                    if is_stop(cmd) {
                        self.status += 1;
                    } else {
                        return cmd;
                    }
                }
                2 => {
                    // Bottom-right corner arc (3PI/2 to 2PI)
                    self.arc.init(
                        self.x2 - self.rx2,
                        self.y1 + self.ry2,
                        self.rx2,
                        self.ry2,
                        PI + PI * 0.5,
                        0.0,
                        true,
                    );
                    self.arc.rewind(0);
                    self.status += 1;
                }
                3 => {
                    cmd = self.arc.vertex(x, y);
                    if is_stop(cmd) {
                        self.status += 1;
                    } else {
                        return PATH_CMD_LINE_TO;
                    }
                }
                4 => {
                    // Top-right corner arc (0 to PI/2)
                    self.arc.init(
                        self.x2 - self.rx3,
                        self.y2 - self.ry3,
                        self.rx3,
                        self.ry3,
                        0.0,
                        PI * 0.5,
                        true,
                    );
                    self.arc.rewind(0);
                    self.status += 1;
                }
                5 => {
                    cmd = self.arc.vertex(x, y);
                    if is_stop(cmd) {
                        self.status += 1;
                    } else {
                        return PATH_CMD_LINE_TO;
                    }
                }
                6 => {
                    // Top-left corner arc (PI/2 to PI)
                    self.arc.init(
                        self.x1 + self.rx4,
                        self.y2 - self.ry4,
                        self.rx4,
                        self.ry4,
                        PI * 0.5,
                        PI,
                        true,
                    );
                    self.arc.rewind(0);
                    self.status += 1;
                }
                7 => {
                    cmd = self.arc.vertex(x, y);
                    if is_stop(cmd) {
                        self.status += 1;
                    } else {
                        return PATH_CMD_LINE_TO;
                    }
                }
                8 => {
                    cmd = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
                    self.status += 1;
                    return cmd;
                }
                _ => {
                    return PATH_CMD_STOP;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_close, is_end_poly, is_move_to, is_vertex, PATH_CMD_MOVE_TO};

    #[test]
    fn test_new_normalizes_coords() {
        let rr = RoundedRect::new(100.0, 200.0, 50.0, 30.0, 5.0);
        assert_eq!(rr.x1, 50.0);
        assert_eq!(rr.y1, 30.0);
        assert_eq!(rr.x2, 100.0);
        assert_eq!(rr.y2, 200.0);
    }

    #[test]
    fn test_uniform_radius() {
        let mut rr = RoundedRect::default_new();
        rr.radius(10.0);
        assert_eq!(rr.rx1, 10.0);
        assert_eq!(rr.ry1, 10.0);
        assert_eq!(rr.rx2, 10.0);
        assert_eq!(rr.ry2, 10.0);
        assert_eq!(rr.rx3, 10.0);
        assert_eq!(rr.ry3, 10.0);
        assert_eq!(rr.rx4, 10.0);
        assert_eq!(rr.ry4, 10.0);
    }

    #[test]
    fn test_radius_xy() {
        let mut rr = RoundedRect::default_new();
        rr.radius_xy(10.0, 5.0);
        assert_eq!(rr.rx1, 10.0);
        assert_eq!(rr.ry1, 5.0);
        assert_eq!(rr.rx2, 10.0);
        assert_eq!(rr.ry2, 5.0);
    }

    #[test]
    fn test_radius_bottom_top() {
        let mut rr = RoundedRect::default_new();
        rr.radius_bottom_top(3.0, 4.0, 5.0, 6.0);
        assert_eq!(rr.rx1, 3.0);
        assert_eq!(rr.ry1, 4.0);
        assert_eq!(rr.rx2, 3.0);
        assert_eq!(rr.ry2, 4.0);
        assert_eq!(rr.rx3, 5.0);
        assert_eq!(rr.ry3, 6.0);
        assert_eq!(rr.rx4, 5.0);
        assert_eq!(rr.ry4, 6.0);
    }

    #[test]
    fn test_radius_all() {
        let mut rr = RoundedRect::default_new();
        rr.radius_all(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        assert_eq!(rr.rx1, 1.0);
        assert_eq!(rr.ry1, 2.0);
        assert_eq!(rr.rx2, 3.0);
        assert_eq!(rr.ry2, 4.0);
        assert_eq!(rr.rx3, 5.0);
        assert_eq!(rr.ry3, 6.0);
        assert_eq!(rr.rx4, 7.0);
        assert_eq!(rr.ry4, 8.0);
    }

    #[test]
    fn test_normalize_radius_no_change() {
        // Radii small enough — no scaling needed
        let mut rr = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 5.0);
        rr.normalize_radius();
        assert!((rr.rx1 - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_radius_scales_down() {
        // Radii too large — must scale down
        let mut rr = RoundedRect::new(0.0, 0.0, 20.0, 20.0, 15.0);
        rr.normalize_radius();
        // rx1 + rx2 = 30 > width 20, so k = 20/30 = 2/3
        // Note: C++ uses dx = |y2-y1| and dy = |x2-x1| (swapped labels)
        let expected = 15.0 * (20.0 / 30.0);
        assert!((rr.rx1 - expected).abs() < 1e-10);
    }

    #[test]
    fn test_vertex_generation_produces_closed_shape() {
        let mut rr = RoundedRect::new(10.0, 10.0, 90.0, 90.0, 10.0);
        rr.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let mut vertex_count = 0;
        let mut has_move_to = false;
        let mut has_end_poly = false;

        loop {
            let cmd = rr.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_move_to(cmd) {
                has_move_to = true;
            }
            if is_end_poly(cmd) {
                has_end_poly = true;
            }
            if is_vertex(cmd) {
                vertex_count += 1;
            }
        }

        assert!(has_move_to, "Should start with move_to");
        assert!(has_end_poly, "Should end with end_poly");
        assert!(vertex_count > 4, "Should have more than 4 vertices (arcs)");
    }

    #[test]
    fn test_end_poly_has_close_and_ccw_flags() {
        let mut rr = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 10.0);
        rr.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;

        loop {
            let cmd = rr.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                panic!("Should have end_poly before stop");
            }
            if is_end_poly(cmd) {
                assert!(is_close(cmd), "end_poly should have close flag");
                assert!((cmd & PATH_FLAGS_CCW) != 0, "Should have CCW flag");
                break;
            }
        }
    }

    #[test]
    fn test_first_vertex_is_on_bottom_left_arc() {
        let mut rr = RoundedRect::new(10.0, 20.0, 90.0, 80.0, 10.0);
        rr.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = rr.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);

        // First vertex should be near the bottom-left corner.
        // Arc center is (20, 30), radius 10, starting at PI.
        // At angle PI: x = 20 + 10*cos(PI) = 10, y = 30 + 10*sin(PI) = 30
        assert!((x - 10.0).abs() < 0.5, "x={x}, expected near 10");
        assert!((y - 30.0).abs() < 0.5, "y={y}, expected near 30");
    }

    #[test]
    fn test_approximation_scale() {
        let mut rr = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 10.0);
        rr.set_approximation_scale(2.0);
        assert!((rr.approximation_scale() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_rect_method() {
        let mut rr = RoundedRect::default_new();
        rr.rect(100.0, 200.0, 50.0, 30.0);
        assert_eq!(rr.x1, 50.0);
        assert_eq!(rr.y1, 30.0);
        assert_eq!(rr.x2, 100.0);
        assert_eq!(rr.y2, 200.0);
    }

    #[test]
    fn test_zero_radius_produces_rectangle() {
        let mut rr = RoundedRect::new(0.0, 0.0, 100.0, 100.0, 0.0);
        rr.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;
        let mut vertex_count = 0;

        loop {
            let cmd = rr.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                vertex_count += 1;
            }
        }

        // With zero radius, arcs degenerate — should still produce a valid shape
        assert!(vertex_count >= 4);
    }
}

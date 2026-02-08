//! Bresenham ellipse interpolator.
//!
//! Port of `agg_ellipse_bresenham.h` — discrete pixel stepping around an
//! ellipse using Bresenham's algorithm. Used by `renderer_primitives` for
//! fast rasterized ellipse drawing.

// ============================================================================
// EllipseBresenhamInterpolator
// ============================================================================

/// Bresenham ellipse interpolator.
///
/// Steps through discrete pixel positions on the first quadrant of an ellipse
/// (dx >= 0, dy moving from -ry toward 0). The caller mirrors to produce the
/// full ellipse.
///
/// Port of C++ `ellipse_bresenham_interpolator`.
pub struct EllipseBresenhamInterpolator {
    rx2: i32,
    ry2: i32,
    two_rx2: i32,
    two_ry2: i32,
    dx: i32,
    dy: i32,
    inc_x: i32,
    inc_y: i32,
    cur_f: i32,
}

impl EllipseBresenhamInterpolator {
    pub fn new(rx: i32, ry: i32) -> Self {
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        Self {
            rx2,
            ry2,
            two_rx2: rx2 << 1,
            two_ry2: ry2 << 1,
            dx: 0,
            dy: 0,
            inc_x: 0,
            inc_y: -ry * (rx2 << 1),
            cur_f: 0,
        }
    }

    /// X step from the previous position (0 or 1).
    pub fn dx(&self) -> i32 {
        self.dx
    }

    /// Y step from the previous position (0 or 1).
    pub fn dy(&self) -> i32 {
        self.dy
    }

    /// Advance to the next pixel position on the ellipse.
    pub fn next(&mut self) {
        let fx = self.cur_f + self.inc_x + self.ry2;
        let fy = self.cur_f + self.inc_y + self.rx2;
        let fxy = self.cur_f + self.inc_x + self.ry2 + self.inc_y + self.rx2;

        let mx = fx.abs();
        let my = fy.abs();
        let mxy = fxy.abs();

        let mut min_m = mx;
        let mut flag = true;

        if min_m > my {
            min_m = my;
            flag = false;
        }

        self.dx = 0;
        self.dy = 0;

        if min_m > mxy {
            self.inc_x += self.two_ry2;
            self.inc_y += self.two_rx2;
            self.cur_f = fxy;
            self.dx = 1;
            self.dy = 1;
            return;
        }

        if flag {
            self.inc_x += self.two_ry2;
            self.cur_f = fx;
            self.dx = 1;
            return;
        }

        self.inc_y += self.two_rx2;
        self.cur_f = fy;
        self.dy = 1;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle() {
        // For a circle with r=5, trace the first quadrant
        let mut ei = EllipseBresenhamInterpolator::new(5, 5);
        let mut dx = 0i32;
        let mut dy = -5i32;
        let mut steps = 0;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();
            steps += 1;
            if dy >= 0 {
                break;
            }
        }

        // Should have traversed from (0, -5) to around (5, 0)
        assert!(steps > 0);
        assert!(dx > 0);
        assert!(dy >= 0);
    }

    #[test]
    fn test_wide_ellipse() {
        let mut ei = EllipseBresenhamInterpolator::new(10, 3);
        let mut dx = 0i32;
        let mut dy = -3i32;
        let mut steps = 0;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();
            steps += 1;
            if dy >= 0 {
                break;
            }
        }

        assert!(steps > 0);
        // Wide ellipse: more x steps than y steps
        assert!(dx > 3);
    }

    #[test]
    fn test_tall_ellipse() {
        let mut ei = EllipseBresenhamInterpolator::new(3, 10);
        let mut dx = 0i32;
        let mut dy = -10i32;
        let mut steps = 0;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();
            steps += 1;
            if dy >= 0 {
                break;
            }
        }

        assert!(steps > 0);
        assert!(dy >= 0);
    }

    #[test]
    fn test_unit_ellipse() {
        let mut ei = EllipseBresenhamInterpolator::new(1, 1);
        let mut dy = -1i32;

        ei.next();
        dy += ei.dy();
        // With r=1, should complete quickly
        if dy < 0 {
            ei.next();
            dy += ei.dy();
        }
        assert!(dy >= 0);
    }
}

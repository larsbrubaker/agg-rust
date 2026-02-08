//! Vertex processor generator: line segment subdivider.
//!
//! Port of `agg_vpgen_segmentator.h` + `agg_vpgen_segmentator.cpp`.
//! Subdivides long line segments into shorter ones for better curve
//! approximation when applying non-linear transforms.

use crate::basics::{PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
use crate::conv_adaptor_vpgen::VpgenProcessor;

/// Subdivides line segments based on approximation scale.
///
/// Used by `ConvAdaptorVpgen` / `ConvSegmentator` to ensure no segment
/// exceeds `1/approximation_scale` in length, which is needed for
/// accurate rendering of non-linear transforms.
pub struct VpgenSegmentator {
    approximation_scale: f64,
    x1: f64,
    y1: f64,
    dx: f64,
    dy: f64,
    dl: f64,
    ddl: f64,
    cmd: u32,
}

impl VpgenSegmentator {
    pub fn new() -> Self {
        Self {
            approximation_scale: 1.0,
            x1: 0.0,
            y1: 0.0,
            dx: 0.0,
            dy: 0.0,
            dl: 2.0,
            ddl: 2.0,
            cmd: PATH_CMD_STOP,
        }
    }

    pub fn approximation_scale(&self) -> f64 {
        self.approximation_scale
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.approximation_scale = s;
    }

    pub fn auto_close() -> bool {
        false
    }

    pub fn auto_unclose() -> bool {
        false
    }

    pub fn reset(&mut self) {
        self.cmd = PATH_CMD_STOP;
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.x1 = x;
        self.y1 = y;
        self.dx = 0.0;
        self.dy = 0.0;
        self.dl = 2.0;
        self.ddl = 2.0;
        self.cmd = PATH_CMD_MOVE_TO;
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.x1 += self.dx;
        self.y1 += self.dy;
        self.dx = x - self.x1;
        self.dy = y - self.y1;
        let mut len = (self.dx * self.dx + self.dy * self.dy).sqrt() * self.approximation_scale;
        if len < 1e-30 {
            len = 1e-30;
        }
        self.ddl = 1.0 / len;
        self.dl = if self.cmd == PATH_CMD_MOVE_TO {
            0.0
        } else {
            self.ddl
        };
        if self.cmd == PATH_CMD_STOP {
            self.cmd = PATH_CMD_LINE_TO;
        }
    }

    pub fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.cmd == PATH_CMD_STOP {
            return PATH_CMD_STOP;
        }

        let cmd = self.cmd;
        self.cmd = PATH_CMD_LINE_TO;
        if self.dl >= 1.0 - self.ddl {
            self.dl = 1.0;
            self.cmd = PATH_CMD_STOP;
            *x = self.x1 + self.dx;
            *y = self.y1 + self.dy;
            return cmd;
        }
        *x = self.x1 + self.dx * self.dl;
        *y = self.y1 + self.dy * self.dl;
        self.dl += self.ddl;
        cmd
    }
}

impl VpgenProcessor for VpgenSegmentator {
    fn reset(&mut self) {
        self.reset();
    }

    fn move_to(&mut self, x: f64, y: f64) {
        self.move_to(x, y);
    }

    fn line_to(&mut self, x: f64, y: f64) {
        self.line_to(x, y);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.vertex(x, y)
    }

    fn auto_close() -> bool {
        VpgenSegmentator::auto_close()
    }

    fn auto_unclose() -> bool {
        VpgenSegmentator::auto_unclose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_segment_no_subdivision() {
        let mut vpgen = VpgenSegmentator::new();
        vpgen.set_approximation_scale(1.0);
        vpgen.move_to(0.0, 0.0);
        vpgen.line_to(0.5, 0.0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd1 = vpgen.vertex(&mut x, &mut y);
        assert_eq!(cmd1, PATH_CMD_MOVE_TO);
        // Short segment (len*scale < 1): entire segment consumed in one step
        assert!((x - 0.5).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);

        // No more vertices — segment already complete
        let cmd2 = vpgen.vertex(&mut x, &mut y);
        assert_eq!(cmd2, PATH_CMD_STOP);
    }

    #[test]
    fn test_long_segment_subdivision() {
        let mut vpgen = VpgenSegmentator::new();
        vpgen.set_approximation_scale(10.0); // force many subdivisions
        vpgen.move_to(0.0, 0.0);
        vpgen.line_to(100.0, 0.0);

        let (mut x, mut y) = (0.0, 0.0);
        let mut count = 0;
        loop {
            let cmd = vpgen.vertex(&mut x, &mut y);
            if cmd == PATH_CMD_STOP {
                break;
            }
            count += 1;
        }
        assert!(count > 2, "Long segment should be subdivided: count={count}");
    }
}

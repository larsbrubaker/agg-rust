//! Arrowhead / arrowtail vertex generator.
//!
//! Port of `agg_arrowhead.h` / `agg_arrowhead.cpp` — generates arrow marker
//! geometry as a VertexSource. Used in conjunction with `conv_marker` to
//! place arrowheads at line endpoints.

use crate::basics::{
    VertexSource, PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
    PATH_FLAGS_CCW, PATH_FLAGS_CLOSE,
};

/// Arrowhead / arrowtail vertex generator.
///
/// Generates arrow marker polygons. Path ID 0 = tail, path ID 1 = head.
/// The arrow shape is defined by four parameters (d1-d4) for each end.
///
/// Port of C++ `agg::arrowhead`.
pub struct Arrowhead {
    head_d1: f64,
    head_d2: f64,
    head_d3: f64,
    head_d4: f64,
    tail_d1: f64,
    tail_d2: f64,
    tail_d3: f64,
    tail_d4: f64,
    head_flag: bool,
    tail_flag: bool,
    coord: [f64; 16],
    cmd: [u32; 8],
    curr_id: u32,
    curr_coord: u32,
}

impl Arrowhead {
    /// Create a new arrowhead with default dimensions.
    pub fn new() -> Self {
        Self {
            head_d1: 1.0,
            head_d2: 1.0,
            head_d3: 1.0,
            head_d4: 0.0,
            tail_d1: 1.0,
            tail_d2: 1.0,
            tail_d3: 1.0,
            tail_d4: 0.0,
            head_flag: false,
            tail_flag: false,
            coord: [0.0; 16],
            cmd: [0; 8],
            curr_id: 0,
            curr_coord: 0,
        }
    }

    /// Set head arrow dimensions and enable head.
    pub fn head(&mut self, d1: f64, d2: f64, d3: f64, d4: f64) {
        self.head_d1 = d1;
        self.head_d2 = d2;
        self.head_d3 = d3;
        self.head_d4 = d4;
        self.head_flag = true;
    }

    /// Enable head arrow (uses current dimensions).
    pub fn enable_head(&mut self) {
        self.head_flag = true;
    }

    /// Disable head arrow.
    pub fn no_head(&mut self) {
        self.head_flag = false;
    }

    /// Set tail arrow dimensions and enable tail.
    pub fn tail(&mut self, d1: f64, d2: f64, d3: f64, d4: f64) {
        self.tail_d1 = d1;
        self.tail_d2 = d2;
        self.tail_d3 = d3;
        self.tail_d4 = d4;
        self.tail_flag = true;
    }

    /// Enable tail arrow (uses current dimensions).
    pub fn enable_tail(&mut self) {
        self.tail_flag = true;
    }

    /// Disable tail arrow.
    pub fn no_tail(&mut self) {
        self.tail_flag = false;
    }
}

impl Default for Arrowhead {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Arrowhead {
    fn rewind(&mut self, path_id: u32) {
        self.curr_id = path_id;
        self.curr_coord = 0;

        if path_id == 0 {
            if !self.tail_flag {
                self.cmd[0] = PATH_CMD_STOP;
                return;
            }
            self.coord[0] = self.tail_d1;
            self.coord[1] = 0.0;
            self.coord[2] = self.tail_d1 - self.tail_d4;
            self.coord[3] = self.tail_d3;
            self.coord[4] = -self.tail_d2 - self.tail_d4;
            self.coord[5] = self.tail_d3;
            self.coord[6] = -self.tail_d2;
            self.coord[7] = 0.0;
            self.coord[8] = -self.tail_d2 - self.tail_d4;
            self.coord[9] = -self.tail_d3;
            self.coord[10] = self.tail_d1 - self.tail_d4;
            self.coord[11] = -self.tail_d3;

            self.cmd[0] = PATH_CMD_MOVE_TO;
            self.cmd[1] = PATH_CMD_LINE_TO;
            self.cmd[2] = PATH_CMD_LINE_TO;
            self.cmd[3] = PATH_CMD_LINE_TO;
            self.cmd[4] = PATH_CMD_LINE_TO;
            self.cmd[5] = PATH_CMD_LINE_TO;
            self.cmd[7] = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
            self.cmd[6] = PATH_CMD_STOP;
        } else if path_id == 1 {
            if !self.head_flag {
                self.cmd[0] = PATH_CMD_STOP;
                return;
            }
            self.coord[0] = -self.head_d1;
            self.coord[1] = 0.0;
            self.coord[2] = self.head_d2 + self.head_d4;
            self.coord[3] = -self.head_d3;
            self.coord[4] = self.head_d2;
            self.coord[5] = 0.0;
            self.coord[6] = self.head_d2 + self.head_d4;
            self.coord[7] = self.head_d3;

            self.cmd[0] = PATH_CMD_MOVE_TO;
            self.cmd[1] = PATH_CMD_LINE_TO;
            self.cmd[2] = PATH_CMD_LINE_TO;
            self.cmd[3] = PATH_CMD_LINE_TO;
            self.cmd[4] = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW;
            self.cmd[5] = PATH_CMD_STOP;
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.curr_id < 2 {
            let curr_idx = self.curr_coord as usize * 2;
            *x = self.coord[curr_idx];
            *y = self.coord[curr_idx + 1];
            let cmd = self.cmd[self.curr_coord as usize];
            self.curr_coord += 1;
            return cmd;
        }
        PATH_CMD_STOP
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
    fn test_arrowhead_default() {
        let ah = Arrowhead::new();
        assert!(!ah.head_flag);
        assert!(!ah.tail_flag);
    }

    #[test]
    fn test_arrowhead_tail_disabled() {
        let mut ah = Arrowhead::new();
        ah.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_arrowhead_head_disabled() {
        let mut ah = Arrowhead::new();
        ah.rewind(1);
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_arrowhead_tail_polygon() {
        let mut ah = Arrowhead::new();
        ah.tail(5.0, 5.0, 3.0, 1.0);
        ah.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // 6 vertices: move_to + 5 line_to
        let cmd = ah.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 5.0).abs() < 1e-10); // tail_d1
        assert!(y.abs() < 1e-10);

        for _ in 0..5 {
            let cmd = ah.vertex(&mut x, &mut y);
            assert_eq!(cmd, PATH_CMD_LINE_TO);
        }

        // Then stop (cmd[6])
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_arrowhead_head_polygon() {
        let mut ah = Arrowhead::new();
        ah.head(5.0, 5.0, 3.0, 1.0);
        ah.rewind(1);
        let mut x = 0.0;
        let mut y = 0.0;

        // 4 vertices: move_to + 3 line_to
        let cmd = ah.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x + 5.0).abs() < 1e-10); // -head_d1

        for _ in 0..3 {
            let cmd = ah.vertex(&mut x, &mut y);
            assert_eq!(cmd, PATH_CMD_LINE_TO);
        }

        // Close polygon
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_end_poly(cmd));

        // Stop
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_arrowhead_invalid_path_id() {
        let mut ah = Arrowhead::new();
        ah.rewind(5); // Invalid path ID
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = ah.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_arrowhead_enable_disable() {
        let mut ah = Arrowhead::new();
        ah.head(5.0, 5.0, 3.0, 1.0);
        assert!(ah.head_flag);

        ah.no_head();
        assert!(!ah.head_flag);

        ah.enable_head();
        assert!(ah.head_flag);

        ah.tail(5.0, 5.0, 3.0, 1.0);
        assert!(ah.tail_flag);

        ah.no_tail();
        assert!(!ah.tail_flag);

        ah.enable_tail();
        assert!(ah.tail_flag);
    }

    #[test]
    fn test_arrowhead_tail_vertex_coords() {
        let mut ah = Arrowhead::new();
        ah.tail(5.0, 5.0, 3.0, 1.0);
        ah.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;

        // Vertex 0: (d1, 0) = (5, 0)
        ah.vertex(&mut x, &mut y);
        assert!((x - 5.0).abs() < 1e-10);
        assert!(y.abs() < 1e-10);

        // Vertex 1: (d1 - d4, d3) = (4, 3)
        ah.vertex(&mut x, &mut y);
        assert!((x - 4.0).abs() < 1e-10);
        assert!((y - 3.0).abs() < 1e-10);

        // Vertex 2: (-d2 - d4, d3) = (-6, 3)
        ah.vertex(&mut x, &mut y);
        assert!((x + 6.0).abs() < 1e-10);
        assert!((y - 3.0).abs() < 1e-10);

        // Vertex 3: (-d2, 0) = (-5, 0)
        ah.vertex(&mut x, &mut y);
        assert!((x + 5.0).abs() < 1e-10);
        assert!(y.abs() < 1e-10);

        // Vertex 4: (-d2 - d4, -d3) = (-6, -3)
        ah.vertex(&mut x, &mut y);
        assert!((x + 6.0).abs() < 1e-10);
        assert!((y + 3.0).abs() < 1e-10);

        // Vertex 5: (d1 - d4, -d3) = (4, -3)
        ah.vertex(&mut x, &mut y);
        assert!((x - 4.0).abs() < 1e-10);
        assert!((y + 3.0).abs() < 1e-10);
    }
}

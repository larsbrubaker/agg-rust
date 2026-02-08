//! Path storage — the primary vertex container for AGG.
//!
//! Port of `agg_path_storage.h` — stores vertices with path commands.
//! Uses `Vec<VertexD>` instead of C++'s block-based `vertex_block_storage`
//! since Rust's `Vec` already provides amortized O(1) push.

use crate::basics::{
    is_curve, is_drawing, is_end_poly, is_equal_eps, is_move_to, is_next_poly, is_stop, is_vertex,
    set_orientation, VertexD, VertexSource, PATH_CMD_CURVE3, PATH_CMD_CURVE4, PATH_CMD_END_POLY,
    PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PATH_FLAGS_CCW, PATH_FLAGS_CLOSE,
    PATH_FLAGS_CW, PATH_FLAGS_NONE,
};
use crate::bezier_arc::BezierArcSvg;
use crate::math::{calc_distance, VERTEX_DIST_EPSILON};

/// Path storage — the main vertex container.
///
/// Stores an ordered sequence of vertices, each with an (x, y) coordinate and
/// a path command. Supports multiple sub-paths separated by `move_to` or `stop`
/// commands. Implements `VertexSource` for use in the rendering pipeline.
///
/// Port of C++ `agg::path_storage` (typedef for `path_base<vertex_block_storage<double>>`).
pub struct PathStorage {
    vertices: Vec<VertexD>,
    iterator: usize,
}

impl PathStorage {
    /// Create an empty path storage.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            iterator: 0,
        }
    }

    /// Remove all vertices (keeps allocated memory).
    pub fn remove_all(&mut self) {
        self.vertices.clear();
        self.iterator = 0;
    }

    /// Remove all vertices and free memory.
    pub fn free_all(&mut self) {
        self.vertices = Vec::new();
        self.iterator = 0;
    }

    // ---------------------------------------------------------------
    // Path construction
    // ---------------------------------------------------------------

    /// Begin a new sub-path. If the last command is not `stop`,
    /// inserts a stop command first. Returns the index where the
    /// new path will start.
    pub fn start_new_path(&mut self) -> usize {
        if !is_stop(self.last_command()) {
            self.vertices.push(VertexD::new(0.0, 0.0, PATH_CMD_STOP));
        }
        self.vertices.len()
    }

    /// Add a move_to command.
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.vertices.push(VertexD::new(x, y, PATH_CMD_MOVE_TO));
    }

    /// Add a relative move_to command.
    pub fn move_rel(&mut self, dx: f64, dy: f64) {
        let (mut x, mut y) = (dx, dy);
        self.rel_to_abs(&mut x, &mut y);
        self.vertices.push(VertexD::new(x, y, PATH_CMD_MOVE_TO));
    }

    /// Add a line_to command.
    pub fn line_to(&mut self, x: f64, y: f64) {
        self.vertices.push(VertexD::new(x, y, PATH_CMD_LINE_TO));
    }

    /// Add a relative line_to command.
    pub fn line_rel(&mut self, dx: f64, dy: f64) {
        let (mut x, mut y) = (dx, dy);
        self.rel_to_abs(&mut x, &mut y);
        self.vertices.push(VertexD::new(x, y, PATH_CMD_LINE_TO));
    }

    /// Add a horizontal line_to command.
    pub fn hline_to(&mut self, x: f64) {
        self.vertices
            .push(VertexD::new(x, self.last_y(), PATH_CMD_LINE_TO));
    }

    /// Add a relative horizontal line_to command.
    pub fn hline_rel(&mut self, dx: f64) {
        let (mut x, mut y) = (dx, 0.0);
        self.rel_to_abs(&mut x, &mut y);
        self.vertices.push(VertexD::new(x, y, PATH_CMD_LINE_TO));
    }

    /// Add a vertical line_to command.
    pub fn vline_to(&mut self, y: f64) {
        self.vertices
            .push(VertexD::new(self.last_x(), y, PATH_CMD_LINE_TO));
    }

    /// Add a relative vertical line_to command.
    pub fn vline_rel(&mut self, dy: f64) {
        let (mut x, mut y) = (0.0, dy);
        self.rel_to_abs(&mut x, &mut y);
        self.vertices.push(VertexD::new(x, y, PATH_CMD_LINE_TO));
    }

    /// Add an SVG-style arc_to command.
    #[allow(clippy::too_many_arguments)]
    pub fn arc_to(
        &mut self,
        rx: f64,
        ry: f64,
        angle: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        x: f64,
        y: f64,
    ) {
        if self.total_vertices() > 0 && is_vertex(self.last_command()) {
            let epsilon = 1e-30;
            let mut x0 = 0.0;
            let mut y0 = 0.0;
            self.last_vertex_xy(&mut x0, &mut y0);

            let rx = rx.abs();
            let ry = ry.abs();

            if rx < epsilon || ry < epsilon {
                self.line_to(x, y);
                return;
            }

            if calc_distance(x0, y0, x, y) < epsilon {
                return;
            }

            let mut a = BezierArcSvg::new_with_params(
                x0,
                y0,
                rx,
                ry,
                angle,
                large_arc_flag,
                sweep_flag,
                x,
                y,
            );
            if a.radii_ok() {
                self.join_path(&mut a, 0);
            } else {
                self.line_to(x, y);
            }
        } else {
            self.move_to(x, y);
        }
    }

    /// Add a relative SVG-style arc_to command.
    #[allow(clippy::too_many_arguments)]
    pub fn arc_rel(
        &mut self,
        rx: f64,
        ry: f64,
        angle: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        dx: f64,
        dy: f64,
    ) {
        let (mut x, mut y) = (dx, dy);
        self.rel_to_abs(&mut x, &mut y);
        self.arc_to(rx, ry, angle, large_arc_flag, sweep_flag, x, y);
    }

    /// Add a quadratic Bezier curve (curve3) with explicit control point.
    pub fn curve3(&mut self, x_ctrl: f64, y_ctrl: f64, x_to: f64, y_to: f64) {
        self.vertices
            .push(VertexD::new(x_ctrl, y_ctrl, PATH_CMD_CURVE3));
        self.vertices
            .push(VertexD::new(x_to, y_to, PATH_CMD_CURVE3));
    }

    /// Add a relative quadratic Bezier curve with explicit control point.
    pub fn curve3_rel(&mut self, dx_ctrl: f64, dy_ctrl: f64, dx_to: f64, dy_to: f64) {
        let (mut x_ctrl, mut y_ctrl) = (dx_ctrl, dy_ctrl);
        self.rel_to_abs(&mut x_ctrl, &mut y_ctrl);
        let (mut x_to, mut y_to) = (dx_to, dy_to);
        self.rel_to_abs(&mut x_to, &mut y_to);
        self.vertices
            .push(VertexD::new(x_ctrl, y_ctrl, PATH_CMD_CURVE3));
        self.vertices
            .push(VertexD::new(x_to, y_to, PATH_CMD_CURVE3));
    }

    /// Add a smooth quadratic Bezier curve (reflected control point).
    pub fn curve3_smooth(&mut self, x_to: f64, y_to: f64) {
        let mut x0 = 0.0;
        let mut y0 = 0.0;
        if is_vertex(self.last_vertex_xy(&mut x0, &mut y0)) {
            let mut x_ctrl = 0.0;
            let mut y_ctrl = 0.0;
            let cmd = self.prev_vertex_xy(&mut x_ctrl, &mut y_ctrl);
            if is_curve(cmd) {
                x_ctrl = x0 + x0 - x_ctrl;
                y_ctrl = y0 + y0 - y_ctrl;
            } else {
                x_ctrl = x0;
                y_ctrl = y0;
            }
            self.curve3(x_ctrl, y_ctrl, x_to, y_to);
        }
    }

    /// Add a relative smooth quadratic Bezier curve.
    pub fn curve3_smooth_rel(&mut self, dx_to: f64, dy_to: f64) {
        let (mut x_to, mut y_to) = (dx_to, dy_to);
        self.rel_to_abs(&mut x_to, &mut y_to);
        self.curve3_smooth(x_to, y_to);
    }

    /// Add a cubic Bezier curve (curve4) with two explicit control points.
    #[allow(clippy::too_many_arguments)]
    pub fn curve4(
        &mut self,
        x_ctrl1: f64,
        y_ctrl1: f64,
        x_ctrl2: f64,
        y_ctrl2: f64,
        x_to: f64,
        y_to: f64,
    ) {
        self.vertices
            .push(VertexD::new(x_ctrl1, y_ctrl1, PATH_CMD_CURVE4));
        self.vertices
            .push(VertexD::new(x_ctrl2, y_ctrl2, PATH_CMD_CURVE4));
        self.vertices
            .push(VertexD::new(x_to, y_to, PATH_CMD_CURVE4));
    }

    /// Add a relative cubic Bezier curve with two explicit control points.
    #[allow(clippy::too_many_arguments)]
    pub fn curve4_rel(
        &mut self,
        dx_ctrl1: f64,
        dy_ctrl1: f64,
        dx_ctrl2: f64,
        dy_ctrl2: f64,
        dx_to: f64,
        dy_to: f64,
    ) {
        let (mut x_ctrl1, mut y_ctrl1) = (dx_ctrl1, dy_ctrl1);
        self.rel_to_abs(&mut x_ctrl1, &mut y_ctrl1);
        let (mut x_ctrl2, mut y_ctrl2) = (dx_ctrl2, dy_ctrl2);
        self.rel_to_abs(&mut x_ctrl2, &mut y_ctrl2);
        let (mut x_to, mut y_to) = (dx_to, dy_to);
        self.rel_to_abs(&mut x_to, &mut y_to);
        self.vertices
            .push(VertexD::new(x_ctrl1, y_ctrl1, PATH_CMD_CURVE4));
        self.vertices
            .push(VertexD::new(x_ctrl2, y_ctrl2, PATH_CMD_CURVE4));
        self.vertices
            .push(VertexD::new(x_to, y_to, PATH_CMD_CURVE4));
    }

    /// Add a smooth cubic Bezier curve (reflected first control point).
    pub fn curve4_smooth(&mut self, x_ctrl2: f64, y_ctrl2: f64, x_to: f64, y_to: f64) {
        let mut x0 = 0.0;
        let mut y0 = 0.0;
        if is_vertex(self.last_vertex_xy(&mut x0, &mut y0)) {
            let mut x_ctrl1 = 0.0;
            let mut y_ctrl1 = 0.0;
            let cmd = self.prev_vertex_xy(&mut x_ctrl1, &mut y_ctrl1);
            if is_curve(cmd) {
                x_ctrl1 = x0 + x0 - x_ctrl1;
                y_ctrl1 = y0 + y0 - y_ctrl1;
            } else {
                x_ctrl1 = x0;
                y_ctrl1 = y0;
            }
            self.curve4(x_ctrl1, y_ctrl1, x_ctrl2, y_ctrl2, x_to, y_to);
        }
    }

    /// Add a relative smooth cubic Bezier curve.
    pub fn curve4_smooth_rel(&mut self, dx_ctrl2: f64, dy_ctrl2: f64, dx_to: f64, dy_to: f64) {
        let (mut x_ctrl2, mut y_ctrl2) = (dx_ctrl2, dy_ctrl2);
        self.rel_to_abs(&mut x_ctrl2, &mut y_ctrl2);
        let (mut x_to, mut y_to) = (dx_to, dy_to);
        self.rel_to_abs(&mut x_to, &mut y_to);
        self.curve4_smooth(x_ctrl2, y_ctrl2, x_to, y_to);
    }

    /// Add an end_poly command with optional flags.
    pub fn end_poly(&mut self, flags: u32) {
        if is_vertex(self.last_command()) {
            self.vertices
                .push(VertexD::new(0.0, 0.0, PATH_CMD_END_POLY | flags));
        }
    }

    /// Close the current polygon.
    pub fn close_polygon(&mut self, flags: u32) {
        self.end_poly(PATH_FLAGS_CLOSE | flags);
    }

    // ---------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------

    /// Total number of vertices stored.
    pub fn total_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Convert relative coordinates to absolute by adding last vertex position.
    pub fn rel_to_abs(&self, x: &mut f64, y: &mut f64) {
        if !self.vertices.is_empty() {
            let last = &self.vertices[self.vertices.len() - 1];
            if is_vertex(last.cmd) {
                *x += last.x;
                *y += last.y;
            }
        }
    }

    /// Get the last vertex's (x, y) and command. Returns `PATH_CMD_STOP` if empty.
    pub fn last_vertex_xy(&self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertices.is_empty() {
            *x = 0.0;
            *y = 0.0;
            return PATH_CMD_STOP;
        }
        let v = &self.vertices[self.vertices.len() - 1];
        *x = v.x;
        *y = v.y;
        v.cmd
    }

    /// Get the second-to-last vertex's (x, y) and command.
    pub fn prev_vertex_xy(&self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertices.len() < 2 {
            *x = 0.0;
            *y = 0.0;
            return PATH_CMD_STOP;
        }
        let v = &self.vertices[self.vertices.len() - 2];
        *x = v.x;
        *y = v.y;
        v.cmd
    }

    /// Get the last command (or `PATH_CMD_STOP` if empty).
    pub fn last_command(&self) -> u32 {
        if self.vertices.is_empty() {
            PATH_CMD_STOP
        } else {
            self.vertices[self.vertices.len() - 1].cmd
        }
    }

    /// Get the X coordinate of the last vertex (or 0.0 if empty).
    pub fn last_x(&self) -> f64 {
        if self.vertices.is_empty() {
            0.0
        } else {
            self.vertices[self.vertices.len() - 1].x
        }
    }

    /// Get the Y coordinate of the last vertex (or 0.0 if empty).
    pub fn last_y(&self) -> f64 {
        if self.vertices.is_empty() {
            0.0
        } else {
            self.vertices[self.vertices.len() - 1].y
        }
    }

    /// Get a vertex by index. Returns the command.
    pub fn vertex_idx(&self, idx: usize, x: &mut f64, y: &mut f64) -> u32 {
        let v = &self.vertices[idx];
        *x = v.x;
        *y = v.y;
        v.cmd
    }

    /// Get a command by index.
    pub fn command(&self, idx: usize) -> u32 {
        self.vertices[idx].cmd
    }

    /// Modify a vertex's coordinates.
    pub fn modify_vertex(&mut self, idx: usize, x: f64, y: f64) {
        self.vertices[idx].x = x;
        self.vertices[idx].y = y;
    }

    /// Modify a vertex's coordinates and command.
    pub fn modify_vertex_cmd(&mut self, idx: usize, x: f64, y: f64, cmd: u32) {
        self.vertices[idx].x = x;
        self.vertices[idx].y = y;
        self.vertices[idx].cmd = cmd;
    }

    /// Modify only a vertex's command.
    pub fn modify_command(&mut self, idx: usize, cmd: u32) {
        self.vertices[idx].cmd = cmd;
    }

    /// Swap two vertices (coordinates and commands).
    pub fn swap_vertices(&mut self, v1: usize, v2: usize) {
        self.vertices.swap(v1, v2);
    }

    // ---------------------------------------------------------------
    // Concatenation and joining
    // ---------------------------------------------------------------

    /// Concatenate all vertices from a vertex source as-is.
    pub fn concat_path(&mut self, vs: &mut dyn VertexSource, path_id: u32) {
        let mut x = 0.0;
        let mut y = 0.0;
        vs.rewind(path_id);
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.vertices.push(VertexD::new(x, y, cmd));
        }
    }

    /// Join a vertex source to the existing path (pen stays down).
    ///
    /// The first move_to of the joined path is converted to line_to
    /// if the current path already has a vertex endpoint.
    pub fn join_path(&mut self, vs: &mut dyn VertexSource, path_id: u32) {
        let mut x = 0.0;
        let mut y = 0.0;
        vs.rewind(path_id);
        let mut cmd = vs.vertex(&mut x, &mut y);
        if !is_stop(cmd) {
            if is_vertex(cmd) {
                let mut x0 = 0.0;
                let mut y0 = 0.0;
                let cmd0 = self.last_vertex_xy(&mut x0, &mut y0);
                if is_vertex(cmd0) {
                    if calc_distance(x, y, x0, y0) > VERTEX_DIST_EPSILON {
                        if is_move_to(cmd) {
                            cmd = PATH_CMD_LINE_TO;
                        }
                        self.vertices.push(VertexD::new(x, y, cmd));
                    }
                } else {
                    if is_stop(cmd0) {
                        cmd = PATH_CMD_MOVE_TO;
                    } else if is_move_to(cmd) {
                        cmd = PATH_CMD_LINE_TO;
                    }
                    self.vertices.push(VertexD::new(x, y, cmd));
                }
            }
            loop {
                cmd = vs.vertex(&mut x, &mut y);
                if is_stop(cmd) {
                    break;
                }
                let actual_cmd = if is_move_to(cmd) {
                    PATH_CMD_LINE_TO
                } else {
                    cmd
                };
                self.vertices.push(VertexD::new(x, y, actual_cmd));
            }
        }
    }

    /// Concatenate a polygon from flat coordinate data.
    pub fn concat_poly(&mut self, data: &[f64], closed: bool) {
        let mut adaptor = PolyPlainAdaptor::new(data, closed);
        self.concat_path(&mut adaptor, 0);
    }

    /// Join a polygon from flat coordinate data.
    pub fn join_poly(&mut self, data: &[f64], closed: bool) {
        let mut adaptor = PolyPlainAdaptor::new(data, closed);
        self.join_path(&mut adaptor, 0);
    }

    // ---------------------------------------------------------------
    // Polygon manipulation
    // ---------------------------------------------------------------

    /// Detect the orientation of a polygon between `start` and `end` (exclusive).
    fn perceive_polygon_orientation(&self, start: usize, end: usize) -> u32 {
        let np = end - start;
        let mut area = 0.0;
        for i in 0..np {
            let v1 = &self.vertices[start + i];
            let v2 = &self.vertices[start + (i + 1) % np];
            area += v1.x * v2.y - v1.y * v2.x;
        }
        if area < 0.0 {
            PATH_FLAGS_CW
        } else {
            PATH_FLAGS_CCW
        }
    }

    /// Invert a polygon between `start` and `end` (exclusive).
    fn invert_polygon_range(&mut self, start: usize, end: usize) {
        let tmp_cmd = self.vertices[start].cmd;
        let end = end - 1; // Make end inclusive

        // Shift all commands one position
        for i in start..end {
            let next_cmd = self.vertices[i + 1].cmd;
            self.vertices[i].cmd = next_cmd;
        }

        // Assign starting command to the ending command
        self.vertices[end].cmd = tmp_cmd;

        // Reverse the polygon vertices
        let (mut lo, mut hi) = (start, end);
        while hi > lo {
            self.vertices.swap(lo, hi);
            lo += 1;
            hi -= 1;
        }
    }

    /// Invert the polygon starting at `start`.
    pub fn invert_polygon(&mut self, start: usize) {
        let mut start = start;
        let total = self.vertices.len();

        // Skip all non-vertices at the beginning
        while start < total && !is_vertex(self.vertices[start].cmd) {
            start += 1;
        }

        // Skip all insignificant move_to
        while start + 1 < total
            && is_move_to(self.vertices[start].cmd)
            && is_move_to(self.vertices[start + 1].cmd)
        {
            start += 1;
        }

        // Find the last vertex
        let mut end = start + 1;
        while end < total && !is_next_poly(self.vertices[end].cmd) {
            end += 1;
        }

        self.invert_polygon_range(start, end);
    }

    /// Arrange polygon orientation for a single polygon starting at `start`.
    /// Returns the index past the end of the polygon.
    pub fn arrange_polygon_orientation(&mut self, start: usize, orientation: u32) -> usize {
        if orientation == PATH_FLAGS_NONE {
            return start;
        }

        let mut start = start;
        let total = self.vertices.len();

        // Skip non-vertices
        while start < total && !is_vertex(self.vertices[start].cmd) {
            start += 1;
        }

        // Skip insignificant move_to
        while start + 1 < total
            && is_move_to(self.vertices[start].cmd)
            && is_move_to(self.vertices[start + 1].cmd)
        {
            start += 1;
        }

        // Find end
        let mut end = start + 1;
        while end < total && !is_next_poly(self.vertices[end].cmd) {
            end += 1;
        }

        if end - start > 2 && self.perceive_polygon_orientation(start, end) != orientation {
            self.invert_polygon_range(start, end);
            let mut idx = end;
            while idx < total && is_end_poly(self.vertices[idx].cmd) {
                let cmd = self.vertices[idx].cmd;
                self.vertices[idx].cmd = set_orientation(cmd, orientation);
                idx += 1;
            }
            return idx;
        }
        end
    }

    /// Arrange orientations of all polygons in a sub-path.
    pub fn arrange_orientations(&mut self, start: usize, orientation: u32) -> usize {
        let mut start = start;
        if orientation != PATH_FLAGS_NONE {
            while start < self.vertices.len() {
                start = self.arrange_polygon_orientation(start, orientation);
                if is_stop(self.vertices.get(start).map_or(PATH_CMD_STOP, |v| v.cmd)) {
                    start += 1;
                    break;
                }
            }
        }
        start
    }

    /// Arrange orientations of all polygons in all paths.
    pub fn arrange_orientations_all_paths(&mut self, orientation: u32) {
        if orientation != PATH_FLAGS_NONE {
            let mut start = 0;
            while start < self.vertices.len() {
                start = self.arrange_orientations(start, orientation);
            }
        }
    }

    /// Flip all vertices horizontally between x1 and x2.
    pub fn flip_x(&mut self, x1: f64, x2: f64) {
        for v in &mut self.vertices {
            if is_vertex(v.cmd) {
                v.x = x2 - v.x + x1;
            }
        }
    }

    /// Flip all vertices vertically between y1 and y2.
    pub fn flip_y(&mut self, y1: f64, y2: f64) {
        for v in &mut self.vertices {
            if is_vertex(v.cmd) {
                v.y = y2 - v.y + y1;
            }
        }
    }

    /// Translate vertices starting from `path_id` until a stop command.
    pub fn translate(&mut self, dx: f64, dy: f64, path_id: usize) {
        let total = self.vertices.len();
        let mut idx = path_id;
        while idx < total {
            let cmd = self.vertices[idx].cmd;
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                self.vertices[idx].x += dx;
                self.vertices[idx].y += dy;
            }
            idx += 1;
        }
    }

    /// Translate all vertices in all paths.
    pub fn translate_all_paths(&mut self, dx: f64, dy: f64) {
        for v in &mut self.vertices {
            if is_vertex(v.cmd) {
                v.x += dx;
                v.y += dy;
            }
        }
    }

    /// Transform vertices starting from `path_id` using a closure.
    pub fn transform<F: Fn(f64, f64) -> (f64, f64)>(&mut self, trans: &F, path_id: usize) {
        let total = self.vertices.len();
        let mut idx = path_id;
        while idx < total {
            let cmd = self.vertices[idx].cmd;
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                let (nx, ny) = trans(self.vertices[idx].x, self.vertices[idx].y);
                self.vertices[idx].x = nx;
                self.vertices[idx].y = ny;
            }
            idx += 1;
        }
    }

    /// Transform all vertices in all paths using a closure.
    pub fn transform_all_paths<F: Fn(f64, f64) -> (f64, f64)>(&mut self, trans: &F) {
        for v in &mut self.vertices {
            if is_vertex(v.cmd) {
                let (nx, ny) = trans(v.x, v.y);
                v.x = nx;
                v.y = ny;
            }
        }
    }

    /// Align a single path so that nearly-equal start/end points become exact.
    /// Returns the index past the end of this path.
    pub fn align_path(&mut self, idx: usize) -> usize {
        let total = self.total_vertices();
        let mut idx = idx;

        if idx >= total || !is_move_to(self.command(idx)) {
            return total;
        }

        let mut start_x = 0.0;
        let mut start_y = 0.0;
        while idx < total && is_move_to(self.command(idx)) {
            self.vertex_idx(idx, &mut start_x, &mut start_y);
            idx += 1;
        }
        while idx < total && is_drawing(self.command(idx)) {
            idx += 1;
        }

        let mut x = 0.0;
        let mut y = 0.0;
        if is_drawing(self.vertex_idx(idx - 1, &mut x, &mut y))
            && is_equal_eps(x, start_x, 1e-8)
            && is_equal_eps(y, start_y, 1e-8)
        {
            self.modify_vertex(idx - 1, start_x, start_y);
        }

        while idx < total && !is_move_to(self.command(idx)) {
            idx += 1;
        }
        idx
    }

    /// Align all paths.
    pub fn align_all_paths(&mut self) {
        let mut i = 0;
        while i < self.total_vertices() {
            i = self.align_path(i);
        }
    }
}

impl Default for PathStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for PathStorage {
    fn rewind(&mut self, path_id: u32) {
        self.iterator = path_id as usize;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.iterator >= self.vertices.len() {
            return PATH_CMD_STOP;
        }
        let v = &self.vertices[self.iterator];
        *x = v.x;
        *y = v.y;
        self.iterator += 1;
        v.cmd
    }
}

// ===================================================================
// Adaptors
// ===================================================================

/// Adaptor that wraps a flat slice of coordinates `[x0, y0, x1, y1, ...]`
/// as a `VertexSource`.
///
/// Port of C++ `agg::poly_plain_adaptor<double>`.
pub struct PolyPlainAdaptor<'a> {
    data: &'a [f64],
    index: usize,
    closed: bool,
    stop: bool,
}

impl<'a> PolyPlainAdaptor<'a> {
    /// Create a new adaptor. `data` must contain pairs of (x, y) coordinates.
    pub fn new(data: &'a [f64], closed: bool) -> Self {
        Self {
            data,
            index: 0,
            closed,
            stop: false,
        }
    }
}

impl VertexSource for PolyPlainAdaptor<'_> {
    fn rewind(&mut self, _path_id: u32) {
        self.index = 0;
        self.stop = false;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.index + 1 < self.data.len() {
            let first = self.index == 0;
            *x = self.data[self.index];
            *y = self.data[self.index + 1];
            self.index += 2;
            return if first {
                PATH_CMD_MOVE_TO
            } else {
                PATH_CMD_LINE_TO
            };
        }
        *x = 0.0;
        *y = 0.0;
        if self.closed && !self.stop {
            self.stop = true;
            return PATH_CMD_END_POLY | PATH_FLAGS_CLOSE;
        }
        PATH_CMD_STOP
    }
}

/// Adaptor for a single line segment as a `VertexSource`.
///
/// Port of C++ `agg::line_adaptor`.
pub struct LineAdaptor {
    coords: [f64; 4],
    index: usize,
}

impl LineAdaptor {
    /// Create a new line adaptor.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self {
            coords: [x1, y1, x2, y2],
            index: 0,
        }
    }

    /// Re-initialize with new coordinates.
    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.coords = [x1, y1, x2, y2];
        self.index = 0;
    }
}

impl VertexSource for LineAdaptor {
    fn rewind(&mut self, _path_id: u32) {
        self.index = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.index < 4 {
            let first = self.index == 0;
            *x = self.coords[self.index];
            *y = self.coords[self.index + 1];
            self.index += 2;
            return if first {
                PATH_CMD_MOVE_TO
            } else {
                PATH_CMD_LINE_TO
            };
        }
        *x = 0.0;
        *y = 0.0;
        PATH_CMD_STOP
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::is_close;

    #[test]
    fn test_new_empty() {
        let ps = PathStorage::new();
        assert_eq!(ps.total_vertices(), 0);
        assert_eq!(ps.last_command(), PATH_CMD_STOP);
        assert_eq!(ps.last_x(), 0.0);
        assert_eq!(ps.last_y(), 0.0);
    }

    #[test]
    fn test_move_to_line_to() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.line_to(50.0, 60.0);

        assert_eq!(ps.total_vertices(), 3);
        assert_eq!(ps.command(0), PATH_CMD_MOVE_TO);
        assert_eq!(ps.command(1), PATH_CMD_LINE_TO);
        assert_eq!(ps.command(2), PATH_CMD_LINE_TO);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 30.0).abs() < 1e-10);
        assert!((y - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_relative_commands() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_rel(5.0, 5.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_hline_vline() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.hline_to(50.0);
        ps.vline_to(80.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);

        ps.vertex_idx(2, &mut x, &mut y);
        assert!((x - 50.0).abs() < 1e-10);
        assert!((y - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_hline_rel_vline_rel() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.hline_rel(5.0);
        ps.vline_rel(10.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);

        ps.vertex_idx(2, &mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_curve3() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.curve3(50.0, 100.0, 100.0, 0.0);

        assert_eq!(ps.total_vertices(), 3);
        assert_eq!(ps.command(1), PATH_CMD_CURVE3);
        assert_eq!(ps.command(2), PATH_CMD_CURVE3);
    }

    #[test]
    fn test_curve4() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.curve4(25.0, 100.0, 75.0, 100.0, 100.0, 0.0);

        assert_eq!(ps.total_vertices(), 4);
        assert_eq!(ps.command(1), PATH_CMD_CURVE4);
        assert_eq!(ps.command(2), PATH_CMD_CURVE4);
        assert_eq!(ps.command(3), PATH_CMD_CURVE4);
    }

    #[test]
    fn test_close_polygon() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.line_to(100.0, 0.0);
        ps.line_to(100.0, 100.0);
        ps.close_polygon(PATH_FLAGS_NONE);

        assert_eq!(ps.total_vertices(), 4);
        assert!(is_end_poly(ps.command(3)));
        assert!(is_close(ps.command(3)));
    }

    #[test]
    fn test_vertex_source_iteration() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);

        ps.rewind(0);
        let (mut x, mut y) = (0.0, 0.0);

        let cmd = ps.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-10);

        let cmd = ps.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 30.0).abs() < 1e-10);

        let cmd = ps.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    #[test]
    fn test_start_new_path() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.line_to(10.0, 10.0);

        let id = ps.start_new_path();
        assert_eq!(id, 3); // 2 vertices + 1 stop

        ps.move_to(50.0, 50.0);
        ps.line_to(60.0, 60.0);

        // Iterate second path
        ps.rewind(id as u32);
        let (mut x, mut y) = (0.0, 0.0);
        let cmd = ps.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_modify_vertex() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.modify_vertex(0, 30.0, 40.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 30.0).abs() < 1e-10);
        assert!((y - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_swap_vertices() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);

        ps.swap_vertices(0, 1);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 30.0).abs() < 1e-10);
        assert_eq!(ps.command(0), PATH_CMD_LINE_TO);

        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert_eq!(ps.command(1), PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_flip_x() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.flip_x(0.0, 100.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 90.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);

        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 70.0).abs() < 1e-10);
    }

    #[test]
    fn test_flip_y() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.flip_y(0.0, 100.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_translate() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.translate(5.0, 10.0, 0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_translate_all_paths() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.translate_all_paths(100.0, 200.0);

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 110.0).abs() < 1e-10);
        assert!((y - 220.0).abs() < 1e-10);
    }

    #[test]
    fn test_concat_path() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);

        let mut ps2 = PathStorage::new();
        ps2.move_to(10.0, 20.0);
        ps2.line_to(30.0, 40.0);

        ps.concat_path(&mut ps2, 0);

        assert_eq!(ps.total_vertices(), 3);
        assert_eq!(ps.command(1), PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_join_path() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.line_to(10.0, 10.0);

        let mut ps2 = PathStorage::new();
        ps2.move_to(20.0, 20.0);
        ps2.line_to(30.0, 30.0);

        ps.join_path(&mut ps2, 0);

        // move_to(20,20) should become line_to(20,20)
        assert_eq!(ps.command(2), PATH_CMD_LINE_TO);
        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(2, &mut x, &mut y);
        assert!((x - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_remove_all() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);
        ps.remove_all();

        assert_eq!(ps.total_vertices(), 0);
    }

    #[test]
    fn test_poly_plain_adaptor() {
        let data = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
        let mut adaptor = PolyPlainAdaptor::new(&data, true);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-10);

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 30.0).abs() < 1e-10);

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 50.0).abs() < 1e-10);

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_end_poly(cmd));
        assert!(is_close(cmd));

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    #[test]
    fn test_poly_plain_adaptor_open() {
        let data = [10.0, 20.0, 30.0, 40.0];
        let mut adaptor = PolyPlainAdaptor::new(&data, false);

        let (mut x, mut y) = (0.0, 0.0);
        adaptor.vertex(&mut x, &mut y); // move_to
        adaptor.vertex(&mut x, &mut y); // line_to

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP); // no close
    }

    #[test]
    fn test_line_adaptor() {
        let mut la = LineAdaptor::new(10.0, 20.0, 30.0, 40.0);
        let (mut x, mut y) = (0.0, 0.0);

        let cmd = la.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);

        let cmd = la.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 30.0).abs() < 1e-10);
        assert!((y - 40.0).abs() < 1e-10);

        let cmd = la.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    #[test]
    fn test_line_adaptor_rewind() {
        let mut la = LineAdaptor::new(10.0, 20.0, 30.0, 40.0);
        let (mut x, mut y) = (0.0, 0.0);

        la.vertex(&mut x, &mut y);
        la.vertex(&mut x, &mut y);
        la.rewind(0);

        let cmd = la.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_curve3_smooth() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.curve3(50.0, 100.0, 100.0, 0.0);
        ps.curve3_smooth(200.0, 0.0);

        // Reflected control point: (100,0) + (100,0) - (50,100) = (150, -100)
        assert_eq!(ps.total_vertices(), 5);
        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(3, &mut x, &mut y);
        assert!((x - 150.0).abs() < 1e-10);
        assert!((y - (-100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_curve4_smooth() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.curve4(25.0, 100.0, 75.0, 100.0, 100.0, 0.0);
        ps.curve4_smooth(175.0, 100.0, 200.0, 0.0);

        // Reflected ctrl1: (100,0) + (100,0) - (75,100) = (125, -100)
        assert_eq!(ps.total_vertices(), 7);
        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(4, &mut x, &mut y);
        assert!((x - 125.0).abs() < 1e-10);
        assert!((y - (-100.0)).abs() < 1e-10);
    }

    #[test]
    fn test_invert_polygon() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        ps.line_to(100.0, 0.0);
        ps.line_to(100.0, 100.0);

        ps.invert_polygon(0);

        // After inversion, vertices are reversed with commands shifted
        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_perceive_orientation_ccw() {
        let mut ps = PathStorage::new();
        // CCW triangle
        ps.move_to(0.0, 0.0);
        ps.line_to(100.0, 0.0);
        ps.line_to(100.0, 100.0);

        let ori = ps.perceive_polygon_orientation(0, 3);
        assert_eq!(ori, PATH_FLAGS_CCW);
    }

    #[test]
    fn test_perceive_orientation_cw() {
        let mut ps = PathStorage::new();
        // CW triangle
        ps.move_to(0.0, 0.0);
        ps.line_to(100.0, 100.0);
        ps.line_to(100.0, 0.0);

        let ori = ps.perceive_polygon_orientation(0, 3);
        assert_eq!(ori, PATH_FLAGS_CW);
    }

    #[test]
    fn test_arrange_polygon_orientation() {
        let mut ps = PathStorage::new();
        // CW triangle
        ps.move_to(0.0, 0.0);
        ps.line_to(100.0, 100.0);
        ps.line_to(100.0, 0.0);

        // Force CCW
        ps.arrange_polygon_orientation(0, PATH_FLAGS_CCW);

        let ori = ps.perceive_polygon_orientation(0, 3);
        assert_eq!(ori, PATH_FLAGS_CCW);
    }

    #[test]
    fn test_concat_poly() {
        let mut ps = PathStorage::new();
        let coords = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0];
        ps.concat_poly(&coords, true);

        assert_eq!(ps.total_vertices(), 4); // 3 vertices + end_poly
        assert_eq!(ps.command(0), PATH_CMD_MOVE_TO);
        assert_eq!(ps.command(1), PATH_CMD_LINE_TO);
        assert_eq!(ps.command(2), PATH_CMD_LINE_TO);
        assert!(is_end_poly(ps.command(3)));
    }

    #[test]
    fn test_transform_all_paths() {
        let mut ps = PathStorage::new();
        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);

        ps.transform_all_paths(&|x, y| (x * 2.0, y * 3.0));

        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 20.0).abs() < 1e-10);
        assert!((y - 60.0).abs() < 1e-10);

        ps.vertex_idx(1, &mut x, &mut y);
        assert!((x - 60.0).abs() < 1e-10);
        assert!((y - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_default() {
        let ps = PathStorage::default();
        assert_eq!(ps.total_vertices(), 0);
    }

    #[test]
    fn test_move_rel_from_empty() {
        let mut ps = PathStorage::new();
        // When empty, rel_to_abs doesn't add anything (no prior vertex)
        ps.move_rel(10.0, 20.0);
        let (mut x, mut y) = (0.0, 0.0);
        ps.vertex_idx(0, &mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_end_poly_only_after_vertex() {
        let mut ps = PathStorage::new();
        // end_poly on empty should do nothing
        ps.end_poly(PATH_FLAGS_CLOSE);
        assert_eq!(ps.total_vertices(), 0);

        ps.move_to(0.0, 0.0);
        ps.end_poly(PATH_FLAGS_CLOSE);
        assert_eq!(ps.total_vertices(), 2);
    }

    #[test]
    fn test_arc_to() {
        let mut ps = PathStorage::new();
        ps.move_to(0.0, 0.0);
        // Small arc — should add several vertices via BezierArcSvg
        ps.arc_to(50.0, 50.0, 0.0, false, true, 100.0, 0.0);

        assert!(ps.total_vertices() > 2);
    }

    #[test]
    fn test_arc_to_no_prior_vertex() {
        let mut ps = PathStorage::new();
        // No prior vertex → should become move_to
        ps.arc_to(50.0, 50.0, 0.0, false, true, 100.0, 0.0);
        assert_eq!(ps.command(0), PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_last_prev_vertex() {
        let mut ps = PathStorage::new();
        let (mut x, mut y) = (0.0, 0.0);
        assert_eq!(ps.last_vertex_xy(&mut x, &mut y), PATH_CMD_STOP);
        assert_eq!(ps.prev_vertex_xy(&mut x, &mut y), PATH_CMD_STOP);

        ps.move_to(10.0, 20.0);
        ps.line_to(30.0, 40.0);

        let cmd = ps.last_vertex_xy(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 30.0).abs() < 1e-10);

        let cmd = ps.prev_vertex_xy(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-10);
    }
}

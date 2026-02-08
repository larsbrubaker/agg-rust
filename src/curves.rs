//! Bezier curve generators (quadratic and cubic).
//!
//! Port of `agg_curves.h` / `agg_curves.cpp` — provides two algorithms for
//! flattening Bezier curves into line segments:
//!
//! - **Incremental** (`Curve3Inc`, `Curve4Inc`): forward-differencing, fast but
//!   less precise for extreme curvatures.
//! - **Subdivision** (`Curve3Div`, `Curve4Div`): recursive de Casteljau
//!   subdivision, adaptive and high-quality.
//!
//! The facade types `Curve3` and `Curve4` delegate to either algorithm.
//!
//! Also provides conversion functions: `catrom_to_bezier`,
//! `ubspline_to_bezier`, `hermite_to_bezier`.

use crate::basics::{PointD, VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PI};
use crate::math::calc_sq_distance;

// ============================================================================
// Constants
// ============================================================================

const CURVE_COLLINEARITY_EPSILON: f64 = 1e-30;
const CURVE_ANGLE_TOLERANCE_EPSILON: f64 = 0.01;
const CURVE_RECURSION_LIMIT: u32 = 32;

// ============================================================================
// Curve approximation method
// ============================================================================

/// Algorithm selection for curve flattening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveApproximationMethod {
    Inc,
    Div,
}

// ============================================================================
// Curve4Points
// ============================================================================

/// Eight control-point coordinates for a cubic Bezier curve.
#[derive(Debug, Clone, Copy)]
pub struct Curve4Points {
    pub cp: [f64; 8],
}

impl Curve4Points {
    #[allow(clippy::too_many_arguments)]
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) -> Self {
        Self {
            cp: [x1, y1, x2, y2, x3, y3, x4, y4],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) {
        self.cp = [x1, y1, x2, y2, x3, y3, x4, y4];
    }
}

impl std::ops::Index<usize> for Curve4Points {
    type Output = f64;
    fn index(&self, i: usize) -> &f64 {
        &self.cp[i]
    }
}

impl std::ops::IndexMut<usize> for Curve4Points {
    fn index_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.cp[i]
    }
}

// ============================================================================
// Curve conversion functions
// ============================================================================

/// Convert Catmull-Rom spline segment to cubic Bezier control points.
#[allow(clippy::too_many_arguments)]
pub fn catrom_to_bezier(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> Curve4Points {
    Curve4Points::new(
        x2,
        y2,
        (-x1 + 6.0 * x2 + x3) / 6.0,
        (-y1 + 6.0 * y2 + y3) / 6.0,
        (x2 + 6.0 * x3 - x4) / 6.0,
        (y2 + 6.0 * y3 - y4) / 6.0,
        x3,
        y3,
    )
}

/// Convert uniform B-spline segment to cubic Bezier control points.
#[allow(clippy::too_many_arguments)]
pub fn ubspline_to_bezier(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> Curve4Points {
    Curve4Points::new(
        (x1 + 4.0 * x2 + x3) / 6.0,
        (y1 + 4.0 * y2 + y3) / 6.0,
        (4.0 * x2 + 2.0 * x3) / 6.0,
        (4.0 * y2 + 2.0 * y3) / 6.0,
        (2.0 * x2 + 4.0 * x3) / 6.0,
        (2.0 * y2 + 4.0 * y3) / 6.0,
        (x2 + 4.0 * x3 + x4) / 6.0,
        (y2 + 4.0 * y3 + y4) / 6.0,
    )
}

/// Convert Hermite spline segment to cubic Bezier control points.
#[allow(clippy::too_many_arguments)]
pub fn hermite_to_bezier(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> Curve4Points {
    Curve4Points::new(
        x1,
        y1,
        (3.0 * x1 + x3) / 3.0,
        (3.0 * y1 + y3) / 3.0,
        (3.0 * x2 - x4) / 3.0,
        (3.0 * y2 - y4) / 3.0,
        x2,
        y2,
    )
}

// ============================================================================
// Curve3Inc — incremental (forward differences) quadratic Bezier
// ============================================================================

/// Incremental quadratic Bezier curve flattener using forward differences.
///
/// Port of C++ `agg::curve3_inc`.
pub struct Curve3Inc {
    num_steps: i32,
    step: i32,
    scale: f64,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    fx: f64,
    fy: f64,
    dfx: f64,
    dfy: f64,
    ddfx: f64,
    ddfy: f64,
    saved_fx: f64,
    saved_fy: f64,
    saved_dfx: f64,
    saved_dfy: f64,
}

impl Curve3Inc {
    pub fn new() -> Self {
        Self {
            num_steps: 0,
            step: 0,
            scale: 1.0,
            start_x: 0.0,
            start_y: 0.0,
            end_x: 0.0,
            end_y: 0.0,
            fx: 0.0,
            fy: 0.0,
            dfx: 0.0,
            dfy: 0.0,
            ddfx: 0.0,
            ddfy: 0.0,
            saved_fx: 0.0,
            saved_fy: 0.0,
            saved_dfx: 0.0,
            saved_dfy: 0.0,
        }
    }

    pub fn new_with_points(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3);
        c
    }

    pub fn reset(&mut self) {
        self.num_steps = 0;
        self.step = -1;
    }

    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.start_x = x1;
        self.start_y = y1;
        self.end_x = x3;
        self.end_y = y3;

        let dx1 = x2 - x1;
        let dy1 = y2 - y1;
        let dx2 = x3 - x2;
        let dy2 = y3 - y2;

        let len = (dx1 * dx1 + dy1 * dy1).sqrt() + (dx2 * dx2 + dy2 * dy2).sqrt();

        self.num_steps = crate::basics::uround(len * 0.25 * self.scale) as i32;

        if self.num_steps < 4 {
            self.num_steps = 4;
        }

        let subdivide_step = 1.0 / self.num_steps as f64;
        let subdivide_step2 = subdivide_step * subdivide_step;

        let tmpx = (x1 - x2 * 2.0 + x3) * subdivide_step2;
        let tmpy = (y1 - y2 * 2.0 + y3) * subdivide_step2;

        self.fx = x1;
        self.saved_fx = x1;
        self.fy = y1;
        self.saved_fy = y1;

        self.dfx = tmpx + (x2 - x1) * (2.0 * subdivide_step);
        self.saved_dfx = self.dfx;
        self.dfy = tmpy + (y2 - y1) * (2.0 * subdivide_step);
        self.saved_dfy = self.dfy;

        self.ddfx = tmpx * 2.0;
        self.ddfy = tmpy * 2.0;

        self.step = self.num_steps;
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.scale = s;
    }

    pub fn approximation_scale(&self) -> f64 {
        self.scale
    }
}

impl Default for Curve3Inc {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve3Inc {
    fn rewind(&mut self, _path_id: u32) {
        if self.num_steps == 0 {
            self.step = -1;
            return;
        }
        self.step = self.num_steps;
        self.fx = self.saved_fx;
        self.fy = self.saved_fy;
        self.dfx = self.saved_dfx;
        self.dfy = self.saved_dfy;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.step < 0 {
            return PATH_CMD_STOP;
        }
        if self.step == self.num_steps {
            *x = self.start_x;
            *y = self.start_y;
            self.step -= 1;
            return PATH_CMD_MOVE_TO;
        }
        if self.step == 0 {
            *x = self.end_x;
            *y = self.end_y;
            self.step -= 1;
            return PATH_CMD_LINE_TO;
        }
        self.fx += self.dfx;
        self.fy += self.dfy;
        self.dfx += self.ddfx;
        self.dfy += self.ddfy;
        *x = self.fx;
        *y = self.fy;
        self.step -= 1;
        PATH_CMD_LINE_TO
    }
}

// ============================================================================
// Curve3Div — recursive subdivision quadratic Bezier
// ============================================================================

/// Recursive subdivision quadratic Bezier curve flattener.
///
/// Port of C++ `agg::curve3_div`.
pub struct Curve3Div {
    approximation_scale: f64,
    distance_tolerance_square: f64,
    angle_tolerance: f64,
    count: usize,
    points: Vec<PointD>,
}

impl Curve3Div {
    pub fn new() -> Self {
        Self {
            approximation_scale: 1.0,
            distance_tolerance_square: 0.0,
            angle_tolerance: 0.0,
            count: 0,
            points: Vec::new(),
        }
    }

    pub fn new_with_points(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3);
        c
    }

    pub fn reset(&mut self) {
        self.points.clear();
        self.count = 0;
    }

    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.points.clear();
        self.distance_tolerance_square = 0.5 / self.approximation_scale;
        self.distance_tolerance_square *= self.distance_tolerance_square;
        self.bezier(x1, y1, x2, y2, x3, y3);
        self.count = 0;
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.approximation_scale = s;
    }

    pub fn approximation_scale(&self) -> f64 {
        self.approximation_scale
    }

    pub fn set_angle_tolerance(&mut self, a: f64) {
        self.angle_tolerance = a;
    }

    pub fn angle_tolerance(&self) -> f64 {
        self.angle_tolerance
    }

    fn bezier(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        self.points.push(PointD { x: x1, y: y1 });
        self.recursive_bezier(x1, y1, x2, y2, x3, y3, 0);
        self.points.push(PointD { x: x3, y: y3 });
    }

    #[allow(clippy::too_many_arguments)]
    fn recursive_bezier(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        level: u32,
    ) {
        if level > CURVE_RECURSION_LIMIT {
            return;
        }

        // Calculate midpoints
        let x12 = (x1 + x2) / 2.0;
        let y12 = (y1 + y2) / 2.0;
        let x23 = (x2 + x3) / 2.0;
        let y23 = (y2 + y3) / 2.0;
        let x123 = (x12 + x23) / 2.0;
        let y123 = (y12 + y23) / 2.0;

        let dx = x3 - x1;
        let dy = y3 - y1;
        let d = ((x2 - x3) * dy - (y2 - y3) * dx).abs();

        if d > CURVE_COLLINEARITY_EPSILON {
            // Regular case
            if d * d <= self.distance_tolerance_square * (dx * dx + dy * dy) {
                if self.angle_tolerance < CURVE_ANGLE_TOLERANCE_EPSILON {
                    self.points.push(PointD { x: x123, y: y123 });
                    return;
                }

                // Angle & Cusp Condition
                let mut da = ((y3 - y2).atan2(x3 - x2) - (y2 - y1).atan2(x2 - x1)).abs();
                if da >= PI {
                    da = 2.0 * PI - da;
                }

                if da < self.angle_tolerance {
                    self.points.push(PointD { x: x123, y: y123 });
                    return;
                }
            }
        } else {
            // Collinear case
            let da = dx * dx + dy * dy;
            let d_val = if da == 0.0 {
                calc_sq_distance(x1, y1, x2, y2)
            } else {
                let d_param = ((x2 - x1) * dx + (y2 - y1) * dy) / da;
                if d_param > 0.0 && d_param < 1.0 {
                    // Simple collinear case, 1---2---3
                    return;
                }
                if d_param <= 0.0 {
                    calc_sq_distance(x2, y2, x1, y1)
                } else if d_param >= 1.0 {
                    calc_sq_distance(x2, y2, x3, y3)
                } else {
                    calc_sq_distance(x2, y2, x1 + d_param * dx, y1 + d_param * dy)
                }
            };
            if d_val < self.distance_tolerance_square {
                self.points.push(PointD { x: x2, y: y2 });
                return;
            }
        }

        // Continue subdivision
        self.recursive_bezier(x1, y1, x12, y12, x123, y123, level + 1);
        self.recursive_bezier(x123, y123, x23, y23, x3, y3, level + 1);
    }
}

impl Default for Curve3Div {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve3Div {
    fn rewind(&mut self, _path_id: u32) {
        self.count = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.count >= self.points.len() {
            return PATH_CMD_STOP;
        }
        let p = &self.points[self.count];
        *x = p.x;
        *y = p.y;
        self.count += 1;
        if self.count == 1 {
            PATH_CMD_MOVE_TO
        } else {
            PATH_CMD_LINE_TO
        }
    }
}

// ============================================================================
// Curve4Inc — incremental (forward differences) cubic Bezier
// ============================================================================

/// Incremental cubic Bezier curve flattener using forward differences.
///
/// Port of C++ `agg::curve4_inc`.
pub struct Curve4Inc {
    num_steps: i32,
    step: i32,
    scale: f64,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    fx: f64,
    fy: f64,
    dfx: f64,
    dfy: f64,
    ddfx: f64,
    ddfy: f64,
    dddfx: f64,
    dddfy: f64,
    saved_fx: f64,
    saved_fy: f64,
    saved_dfx: f64,
    saved_dfy: f64,
    saved_ddfx: f64,
    saved_ddfy: f64,
}

impl Curve4Inc {
    pub fn new() -> Self {
        Self {
            num_steps: 0,
            step: 0,
            scale: 1.0,
            start_x: 0.0,
            start_y: 0.0,
            end_x: 0.0,
            end_y: 0.0,
            fx: 0.0,
            fy: 0.0,
            dfx: 0.0,
            dfy: 0.0,
            ddfx: 0.0,
            ddfy: 0.0,
            dddfx: 0.0,
            dddfy: 0.0,
            saved_fx: 0.0,
            saved_fy: 0.0,
            saved_dfx: 0.0,
            saved_dfy: 0.0,
            saved_ddfx: 0.0,
            saved_ddfy: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_points(
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
    ) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3, x4, y4);
        c
    }

    pub fn new_with_curve4_points(cp: &Curve4Points) -> Self {
        let mut c = Self::new();
        c.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
        c
    }

    pub fn reset(&mut self) {
        self.num_steps = 0;
        self.step = -1;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) {
        self.start_x = x1;
        self.start_y = y1;
        self.end_x = x4;
        self.end_y = y4;

        let dx1 = x2 - x1;
        let dy1 = y2 - y1;
        let dx2 = x3 - x2;
        let dy2 = y3 - y2;
        let dx3 = x4 - x3;
        let dy3 = y4 - y3;

        let len = ((dx1 * dx1 + dy1 * dy1).sqrt()
            + (dx2 * dx2 + dy2 * dy2).sqrt()
            + (dx3 * dx3 + dy3 * dy3).sqrt())
            * 0.25
            * self.scale;

        self.num_steps = crate::basics::uround(len) as i32;

        if self.num_steps < 4 {
            self.num_steps = 4;
        }

        let subdivide_step = 1.0 / self.num_steps as f64;
        let subdivide_step2 = subdivide_step * subdivide_step;
        let subdivide_step3 = subdivide_step * subdivide_step * subdivide_step;

        let pre1 = 3.0 * subdivide_step;
        let pre2 = 3.0 * subdivide_step2;
        let pre4 = 6.0 * subdivide_step2;
        let pre5 = 6.0 * subdivide_step3;

        let tmp1x = x1 - x2 * 2.0 + x3;
        let tmp1y = y1 - y2 * 2.0 + y3;

        let tmp2x = (x2 - x3) * 3.0 - x1 + x4;
        let tmp2y = (y2 - y3) * 3.0 - y1 + y4;

        self.saved_fx = x1;
        self.fx = x1;
        self.saved_fy = y1;
        self.fy = y1;

        self.saved_dfx = (x2 - x1) * pre1 + tmp1x * pre2 + tmp2x * subdivide_step3;
        self.dfx = self.saved_dfx;
        self.saved_dfy = (y2 - y1) * pre1 + tmp1y * pre2 + tmp2y * subdivide_step3;
        self.dfy = self.saved_dfy;

        self.saved_ddfx = tmp1x * pre4 + tmp2x * pre5;
        self.ddfx = self.saved_ddfx;
        self.saved_ddfy = tmp1y * pre4 + tmp2y * pre5;
        self.ddfy = self.saved_ddfy;

        self.dddfx = tmp2x * pre5;
        self.dddfy = tmp2y * pre5;

        self.step = self.num_steps;
    }

    pub fn init_with_curve4_points(&mut self, cp: &Curve4Points) {
        self.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.scale = s;
    }

    pub fn approximation_scale(&self) -> f64 {
        self.scale
    }
}

impl Default for Curve4Inc {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve4Inc {
    fn rewind(&mut self, _path_id: u32) {
        if self.num_steps == 0 {
            self.step = -1;
            return;
        }
        self.step = self.num_steps;
        self.fx = self.saved_fx;
        self.fy = self.saved_fy;
        self.dfx = self.saved_dfx;
        self.dfy = self.saved_dfy;
        self.ddfx = self.saved_ddfx;
        self.ddfy = self.saved_ddfy;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.step < 0 {
            return PATH_CMD_STOP;
        }
        if self.step == self.num_steps {
            *x = self.start_x;
            *y = self.start_y;
            self.step -= 1;
            return PATH_CMD_MOVE_TO;
        }
        if self.step == 0 {
            *x = self.end_x;
            *y = self.end_y;
            self.step -= 1;
            return PATH_CMD_LINE_TO;
        }

        self.fx += self.dfx;
        self.fy += self.dfy;
        self.dfx += self.ddfx;
        self.dfy += self.ddfy;
        self.ddfx += self.dddfx;
        self.ddfy += self.dddfy;

        *x = self.fx;
        *y = self.fy;
        self.step -= 1;
        PATH_CMD_LINE_TO
    }
}

// ============================================================================
// Curve4Div — recursive subdivision cubic Bezier
// ============================================================================

/// Recursive subdivision cubic Bezier curve flattener.
///
/// Port of C++ `agg::curve4_div`.
pub struct Curve4Div {
    approximation_scale: f64,
    distance_tolerance_square: f64,
    angle_tolerance: f64,
    cusp_limit: f64,
    count: usize,
    points: Vec<PointD>,
}

impl Curve4Div {
    pub fn new() -> Self {
        Self {
            approximation_scale: 1.0,
            distance_tolerance_square: 0.0,
            angle_tolerance: 0.0,
            cusp_limit: 0.0,
            count: 0,
            points: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_points(
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
    ) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3, x4, y4);
        c
    }

    pub fn new_with_curve4_points(cp: &Curve4Points) -> Self {
        let mut c = Self::new();
        c.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
        c
    }

    pub fn reset(&mut self) {
        self.points.clear();
        self.count = 0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) {
        self.points.clear();
        self.distance_tolerance_square = 0.5 / self.approximation_scale;
        self.distance_tolerance_square *= self.distance_tolerance_square;
        self.bezier(x1, y1, x2, y2, x3, y3, x4, y4);
        self.count = 0;
    }

    pub fn init_with_curve4_points(&mut self, cp: &Curve4Points) {
        self.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.approximation_scale = s;
    }

    pub fn approximation_scale(&self) -> f64 {
        self.approximation_scale
    }

    pub fn set_angle_tolerance(&mut self, a: f64) {
        self.angle_tolerance = a;
    }

    pub fn angle_tolerance(&self) -> f64 {
        self.angle_tolerance
    }

    pub fn set_cusp_limit(&mut self, v: f64) {
        self.cusp_limit = if v == 0.0 { 0.0 } else { PI - v };
    }

    pub fn cusp_limit(&self) -> f64 {
        if self.cusp_limit == 0.0 {
            0.0
        } else {
            PI - self.cusp_limit
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn bezier(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) {
        self.points.push(PointD { x: x1, y: y1 });
        self.recursive_bezier(x1, y1, x2, y2, x3, y3, x4, y4, 0);
        self.points.push(PointD { x: x4, y: y4 });
    }

    #[allow(clippy::too_many_arguments)]
    fn recursive_bezier(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
        level: u32,
    ) {
        if level > CURVE_RECURSION_LIMIT {
            return;
        }

        // Calculate all the mid-points of the line segments
        let x12 = (x1 + x2) / 2.0;
        let y12 = (y1 + y2) / 2.0;
        let x23 = (x2 + x3) / 2.0;
        let y23 = (y2 + y3) / 2.0;
        let x34 = (x3 + x4) / 2.0;
        let y34 = (y3 + y4) / 2.0;
        let x123 = (x12 + x23) / 2.0;
        let y123 = (y12 + y23) / 2.0;
        let x234 = (x23 + x34) / 2.0;
        let y234 = (y23 + y34) / 2.0;
        let x1234 = (x123 + x234) / 2.0;
        let y1234 = (y123 + y234) / 2.0;

        // Try to approximate the full cubic curve by a single straight line
        let dx = x4 - x1;
        let dy = y4 - y1;

        let mut d2 = ((x2 - x4) * dy - (y2 - y4) * dx).abs();
        let mut d3 = ((x3 - x4) * dy - (y3 - y4) * dx).abs();

        let case = ((d2 > CURVE_COLLINEARITY_EPSILON) as u32) << 1
            | (d3 > CURVE_COLLINEARITY_EPSILON) as u32;

        match case {
            0 => {
                // All collinear OR p1==p4
                let k = dx * dx + dy * dy;
                if k == 0.0 {
                    d2 = calc_sq_distance(x1, y1, x2, y2);
                    d3 = calc_sq_distance(x4, y4, x3, y3);
                } else {
                    let k = 1.0 / k;
                    let da1 = x2 - x1;
                    let da2 = y2 - y1;
                    d2 = k * (da1 * dx + da2 * dy);
                    let da1 = x3 - x1;
                    let da2 = y3 - y1;
                    d3 = k * (da1 * dx + da2 * dy);
                    if d2 > 0.0 && d2 < 1.0 && d3 > 0.0 && d3 < 1.0 {
                        // Simple collinear case, 1---2---3---4
                        return;
                    }
                    if d2 <= 0.0 {
                        d2 = calc_sq_distance(x2, y2, x1, y1);
                    } else if d2 >= 1.0 {
                        d2 = calc_sq_distance(x2, y2, x4, y4);
                    } else {
                        d2 = calc_sq_distance(x2, y2, x1 + d2 * dx, y1 + d2 * dy);
                    }

                    if d3 <= 0.0 {
                        d3 = calc_sq_distance(x3, y3, x1, y1);
                    } else if d3 >= 1.0 {
                        d3 = calc_sq_distance(x3, y3, x4, y4);
                    } else {
                        d3 = calc_sq_distance(x3, y3, x1 + d3 * dx, y1 + d3 * dy);
                    }
                }
                if d2 > d3 {
                    if d2 < self.distance_tolerance_square {
                        self.points.push(PointD { x: x2, y: y2 });
                        return;
                    }
                } else if d3 < self.distance_tolerance_square {
                    self.points.push(PointD { x: x3, y: y3 });
                    return;
                }
            }

            1 => {
                // p1,p2,p4 are collinear, p3 is significant
                if d3 * d3 <= self.distance_tolerance_square * (dx * dx + dy * dy) {
                    if self.angle_tolerance < CURVE_ANGLE_TOLERANCE_EPSILON {
                        self.points.push(PointD { x: x23, y: y23 });
                        return;
                    }

                    // Angle Condition
                    let mut da1 = ((y4 - y3).atan2(x4 - x3) - (y3 - y2).atan2(x3 - x2)).abs();
                    if da1 >= PI {
                        da1 = 2.0 * PI - da1;
                    }

                    if da1 < self.angle_tolerance {
                        self.points.push(PointD { x: x2, y: y2 });
                        self.points.push(PointD { x: x3, y: y3 });
                        return;
                    }

                    if self.cusp_limit != 0.0 && da1 > self.cusp_limit {
                        self.points.push(PointD { x: x3, y: y3 });
                        return;
                    }
                }
            }

            2 => {
                // p1,p3,p4 are collinear, p2 is significant
                if d2 * d2 <= self.distance_tolerance_square * (dx * dx + dy * dy) {
                    if self.angle_tolerance < CURVE_ANGLE_TOLERANCE_EPSILON {
                        self.points.push(PointD { x: x23, y: y23 });
                        return;
                    }

                    // Angle Condition
                    let mut da1 = ((y3 - y2).atan2(x3 - x2) - (y2 - y1).atan2(x2 - x1)).abs();
                    if da1 >= PI {
                        da1 = 2.0 * PI - da1;
                    }

                    if da1 < self.angle_tolerance {
                        self.points.push(PointD { x: x2, y: y2 });
                        self.points.push(PointD { x: x3, y: y3 });
                        return;
                    }

                    if self.cusp_limit != 0.0 && da1 > self.cusp_limit {
                        self.points.push(PointD { x: x2, y: y2 });
                        return;
                    }
                }
            }

            3 => {
                // Regular case
                if (d2 + d3) * (d2 + d3) <= self.distance_tolerance_square * (dx * dx + dy * dy) {
                    if self.angle_tolerance < CURVE_ANGLE_TOLERANCE_EPSILON {
                        self.points.push(PointD { x: x23, y: y23 });
                        return;
                    }

                    // Angle & Cusp Condition
                    let k = (y3 - y2).atan2(x3 - x2);
                    let mut da1 = (k - (y2 - y1).atan2(x2 - x1)).abs();
                    let mut da2 = ((y4 - y3).atan2(x4 - x3) - k).abs();
                    if da1 >= PI {
                        da1 = 2.0 * PI - da1;
                    }
                    if da2 >= PI {
                        da2 = 2.0 * PI - da2;
                    }

                    if da1 + da2 < self.angle_tolerance {
                        self.points.push(PointD { x: x23, y: y23 });
                        return;
                    }

                    if self.cusp_limit != 0.0 {
                        if da1 > self.cusp_limit {
                            self.points.push(PointD { x: x2, y: y2 });
                            return;
                        }

                        if da2 > self.cusp_limit {
                            self.points.push(PointD { x: x3, y: y3 });
                            return;
                        }
                    }
                }
            }

            _ => unreachable!(),
        }

        // Continue subdivision
        self.recursive_bezier(x1, y1, x12, y12, x123, y123, x1234, y1234, level + 1);
        self.recursive_bezier(x1234, y1234, x234, y234, x34, y34, x4, y4, level + 1);
    }
}

impl Default for Curve4Div {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve4Div {
    fn rewind(&mut self, _path_id: u32) {
        self.count = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.count >= self.points.len() {
            return PATH_CMD_STOP;
        }
        let p = &self.points[self.count];
        *x = p.x;
        *y = p.y;
        self.count += 1;
        if self.count == 1 {
            PATH_CMD_MOVE_TO
        } else {
            PATH_CMD_LINE_TO
        }
    }
}

// ============================================================================
// Curve3 — facade
// ============================================================================

/// Quadratic Bezier curve with selectable algorithm.
///
/// Defaults to subdivision (`Div`). Delegates to `Curve3Inc` or `Curve3Div`.
///
/// Port of C++ `agg::curve3`.
pub struct Curve3 {
    curve_inc: Curve3Inc,
    curve_div: Curve3Div,
    approximation_method: CurveApproximationMethod,
}

impl Curve3 {
    pub fn new() -> Self {
        Self {
            curve_inc: Curve3Inc::new(),
            curve_div: Curve3Div::new(),
            approximation_method: CurveApproximationMethod::Div,
        }
    }

    pub fn new_with_points(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3);
        c
    }

    pub fn reset(&mut self) {
        self.curve_inc.reset();
        self.curve_div.reset();
    }

    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.init(x1, y1, x2, y2, x3, y3);
        } else {
            self.curve_div.init(x1, y1, x2, y2, x3, y3);
        }
    }

    pub fn set_approximation_method(&mut self, v: CurveApproximationMethod) {
        self.approximation_method = v;
    }

    pub fn approximation_method(&self) -> CurveApproximationMethod {
        self.approximation_method
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.curve_inc.set_approximation_scale(s);
        self.curve_div.set_approximation_scale(s);
    }

    pub fn approximation_scale(&self) -> f64 {
        self.curve_inc.approximation_scale()
    }

    pub fn set_angle_tolerance(&mut self, a: f64) {
        self.curve_div.set_angle_tolerance(a);
    }

    pub fn angle_tolerance(&self) -> f64 {
        self.curve_div.angle_tolerance()
    }
}

impl Default for Curve3 {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve3 {
    fn rewind(&mut self, path_id: u32) {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.rewind(path_id);
        } else {
            self.curve_div.rewind(path_id);
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.vertex(x, y)
        } else {
            self.curve_div.vertex(x, y)
        }
    }
}

// ============================================================================
// Curve4 — facade
// ============================================================================

/// Cubic Bezier curve with selectable algorithm.
///
/// Defaults to subdivision (`Div`). Delegates to `Curve4Inc` or `Curve4Div`.
///
/// Port of C++ `agg::curve4`.
pub struct Curve4 {
    curve_inc: Curve4Inc,
    curve_div: Curve4Div,
    approximation_method: CurveApproximationMethod,
}

impl Curve4 {
    pub fn new() -> Self {
        Self {
            curve_inc: Curve4Inc::new(),
            curve_div: Curve4Div::new(),
            approximation_method: CurveApproximationMethod::Div,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_points(
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
    ) -> Self {
        let mut c = Self::new();
        c.init(x1, y1, x2, y2, x3, y3, x4, y4);
        c
    }

    pub fn new_with_curve4_points(cp: &Curve4Points) -> Self {
        let mut c = Self::new();
        c.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
        c
    }

    pub fn reset(&mut self) {
        self.curve_inc.reset();
        self.curve_div.reset();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.init(x1, y1, x2, y2, x3, y3, x4, y4);
        } else {
            self.curve_div.init(x1, y1, x2, y2, x3, y3, x4, y4);
        }
    }

    pub fn init_with_curve4_points(&mut self, cp: &Curve4Points) {
        self.init(cp[0], cp[1], cp[2], cp[3], cp[4], cp[5], cp[6], cp[7]);
    }

    pub fn set_approximation_method(&mut self, v: CurveApproximationMethod) {
        self.approximation_method = v;
    }

    pub fn approximation_method(&self) -> CurveApproximationMethod {
        self.approximation_method
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.curve_inc.set_approximation_scale(s);
        self.curve_div.set_approximation_scale(s);
    }

    pub fn approximation_scale(&self) -> f64 {
        self.curve_inc.approximation_scale()
    }

    pub fn set_angle_tolerance(&mut self, v: f64) {
        self.curve_div.set_angle_tolerance(v);
    }

    pub fn angle_tolerance(&self) -> f64 {
        self.curve_div.angle_tolerance()
    }

    pub fn set_cusp_limit(&mut self, v: f64) {
        self.curve_div.set_cusp_limit(v);
    }

    pub fn cusp_limit(&self) -> f64 {
        self.curve_div.cusp_limit()
    }
}

impl Default for Curve4 {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for Curve4 {
    fn rewind(&mut self, path_id: u32) {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.rewind(path_id);
        } else {
            self.curve_div.rewind(path_id);
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.approximation_method == CurveApproximationMethod::Inc {
            self.curve_inc.vertex(x, y)
        } else {
            self.curve_div.vertex(x, y)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::is_stop;

    /// Collect all vertices from a vertex source.
    fn collect_vertices(vs: &mut dyn VertexSource) -> Vec<(f64, f64, u32)> {
        vs.rewind(0);
        let mut result = Vec::new();
        loop {
            let mut x = 0.0;
            let mut y = 0.0;
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            result.push((x, y, cmd));
        }
        result
    }

    // --- Curve3Inc tests ---

    #[test]
    fn test_curve3_inc_basic() {
        let mut c = Curve3Inc::new_with_points(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 4);
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        assert!((verts[0].0).abs() < 1e-6);
        assert!((verts[0].1).abs() < 1e-6);
        let last = &verts[verts.len() - 1];
        assert!((last.0 - 100.0).abs() < 1e-6);
        assert!((last.1).abs() < 1e-6);
    }

    #[test]
    fn test_curve3_inc_reset() {
        let mut c = Curve3Inc::new();
        c.reset();
        let mut x = 0.0;
        let mut y = 0.0;
        c.rewind(0);
        let cmd = c.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_curve3_inc_rewind_replays() {
        let mut c = Curve3Inc::new_with_points(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let verts1 = collect_vertices(&mut c);
        let verts2 = collect_vertices(&mut c);
        assert_eq!(verts1.len(), verts2.len());
        for (a, b) in verts1.iter().zip(verts2.iter()) {
            assert!((a.0 - b.0).abs() < 1e-10);
            assert!((a.1 - b.1).abs() < 1e-10);
        }
    }

    #[test]
    fn test_curve3_inc_scale() {
        let mut c1 = Curve3Inc::new();
        c1.set_approximation_scale(1.0);
        c1.init(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let v1 = collect_vertices(&mut c1);

        let mut c2 = Curve3Inc::new();
        c2.set_approximation_scale(4.0);
        c2.init(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let v2 = collect_vertices(&mut c2);

        assert!(v2.len() > v1.len());
    }

    // --- Curve3Div tests ---

    #[test]
    fn test_curve3_div_basic() {
        let mut c = Curve3Div::new_with_points(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 3);
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        assert!((verts[0].0).abs() < 1e-6);
        let last = &verts[verts.len() - 1];
        assert!((last.0 - 100.0).abs() < 1e-6);
        assert!((last.1).abs() < 1e-6);
    }

    #[test]
    fn test_curve3_div_straight_line() {
        // Control point on the line: should produce few points
        let mut c = Curve3Div::new_with_points(0.0, 0.0, 50.0, 0.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        // All y should be 0
        for v in &verts {
            assert!(v.1.abs() < 1e-6);
        }
    }

    #[test]
    fn test_curve3_div_angle_tolerance() {
        let mut c = Curve3Div::new();
        c.set_angle_tolerance(0.1);
        assert!((c.angle_tolerance() - 0.1).abs() < 1e-10);
        c.init(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 3);
    }

    // --- Curve4Inc tests ---

    #[test]
    fn test_curve4_inc_basic() {
        let mut c = Curve4Inc::new_with_points(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 4);
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        assert!((verts[0].0).abs() < 1e-6);
        let last = &verts[verts.len() - 1];
        assert!((last.0 - 100.0).abs() < 1e-6);
        assert!((last.1).abs() < 1e-6);
    }

    #[test]
    fn test_curve4_inc_reset() {
        let mut c = Curve4Inc::new();
        c.reset();
        c.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = c.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_curve4_inc_curve4_points() {
        let cp = Curve4Points::new(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let mut c = Curve4Inc::new_with_curve4_points(&cp);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 4);
    }

    #[test]
    fn test_curve4_inc_scale() {
        let mut c1 = Curve4Inc::new();
        c1.set_approximation_scale(1.0);
        c1.init(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let v1 = collect_vertices(&mut c1);

        let mut c2 = Curve4Inc::new();
        c2.set_approximation_scale(4.0);
        c2.init(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let v2 = collect_vertices(&mut c2);

        assert!(v2.len() > v1.len());
    }

    // --- Curve4Div tests ---

    #[test]
    fn test_curve4_div_basic() {
        let mut c = Curve4Div::new_with_points(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 3);
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
        let last = &verts[verts.len() - 1];
        assert!((last.0 - 100.0).abs() < 1e-6);
        assert!((last.1).abs() < 1e-6);
    }

    #[test]
    fn test_curve4_div_straight_line() {
        let mut c = Curve4Div::new_with_points(0.0, 0.0, 33.0, 0.0, 66.0, 0.0, 100.0, 0.0);
        let verts = collect_vertices(&mut c);
        for v in &verts {
            assert!(v.1.abs() < 1e-6);
        }
    }

    #[test]
    fn test_curve4_div_cusp_limit() {
        let mut c = Curve4Div::new();
        c.set_cusp_limit(0.0);
        assert!((c.cusp_limit() - 0.0).abs() < 1e-10);
        c.set_cusp_limit(0.5);
        assert!((c.cusp_limit() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_curve4_div_curve4_points() {
        let cp = Curve4Points::new(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let mut c = Curve4Div::new_with_curve4_points(&cp);
        let verts = collect_vertices(&mut c);
        assert!(verts.len() >= 3);
    }

    // --- Curve3 facade tests ---

    #[test]
    fn test_curve3_facade_default_div() {
        let c = Curve3::new();
        assert_eq!(c.approximation_method(), CurveApproximationMethod::Div);
    }

    #[test]
    fn test_curve3_facade_switch_method() {
        let mut c = Curve3::new();
        c.set_approximation_method(CurveApproximationMethod::Inc);
        c.init(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let v_inc = collect_vertices(&mut c);

        c.set_approximation_method(CurveApproximationMethod::Div);
        c.init(0.0, 0.0, 50.0, 100.0, 100.0, 0.0);
        let v_div = collect_vertices(&mut c);

        // Both should produce valid output
        assert!(v_inc.len() >= 3);
        assert!(v_div.len() >= 3);
        // Same start point
        assert!((v_inc[0].0).abs() < 1e-6);
        assert!((v_div[0].0).abs() < 1e-6);
    }

    // --- Curve4 facade tests ---

    #[test]
    fn test_curve4_facade_default_div() {
        let c = Curve4::new();
        assert_eq!(c.approximation_method(), CurveApproximationMethod::Div);
    }

    #[test]
    fn test_curve4_facade_switch_method() {
        let mut c = Curve4::new();
        c.set_approximation_method(CurveApproximationMethod::Inc);
        c.init(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let v_inc = collect_vertices(&mut c);

        c.set_approximation_method(CurveApproximationMethod::Div);
        c.init(0.0, 0.0, 33.0, 100.0, 66.0, 100.0, 100.0, 0.0);
        let v_div = collect_vertices(&mut c);

        assert!(v_inc.len() >= 4);
        assert!(v_div.len() >= 3);
    }

    // --- Curve4Points tests ---

    #[test]
    fn test_curve4_points_index() {
        let cp = Curve4Points::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        assert_eq!(cp[0], 1.0);
        assert_eq!(cp[1], 2.0);
        assert_eq!(cp[6], 7.0);
        assert_eq!(cp[7], 8.0);
    }

    #[test]
    fn test_curve4_points_init() {
        let mut cp = Curve4Points::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        cp.init(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0);
        assert_eq!(cp[4], 5.0);
    }

    // --- Conversion function tests ---

    #[test]
    fn test_catrom_to_bezier() {
        let cp = catrom_to_bezier(0.0, 0.0, 10.0, 0.0, 20.0, 0.0, 30.0, 0.0);
        // First point = p2
        assert!((cp[0] - 10.0).abs() < 1e-6);
        assert!(cp[1].abs() < 1e-6);
        // Last point = p3
        assert!((cp[6] - 20.0).abs() < 1e-6);
        assert!(cp[7].abs() < 1e-6);
    }

    #[test]
    fn test_ubspline_to_bezier() {
        let cp = ubspline_to_bezier(0.0, 0.0, 10.0, 0.0, 20.0, 0.0, 30.0, 0.0);
        // Points should be between the input control points
        assert!(cp[0] > 0.0 && cp[0] < 30.0);
        assert!(cp[6] > 0.0 && cp[6] < 30.0);
    }

    #[test]
    fn test_hermite_to_bezier() {
        let cp = hermite_to_bezier(0.0, 0.0, 100.0, 0.0, 30.0, 0.0, 30.0, 0.0);
        // First point = p1
        assert!(cp[0].abs() < 1e-6);
        assert!(cp[1].abs() < 1e-6);
        // Last point = p2
        assert!((cp[6] - 100.0).abs() < 1e-6);
        assert!(cp[7].abs() < 1e-6);
    }

    #[test]
    fn test_curve3_inc_and_div_same_endpoints() {
        // Both algorithms should hit the same start and end points
        let mut inc = Curve3Inc::new_with_points(10.0, 20.0, 50.0, 80.0, 90.0, 20.0);
        let mut div = Curve3Div::new_with_points(10.0, 20.0, 50.0, 80.0, 90.0, 20.0);
        let vi = collect_vertices(&mut inc);
        let vd = collect_vertices(&mut div);

        // Same start
        assert!((vi[0].0 - 10.0).abs() < 1e-6);
        assert!((vi[0].1 - 20.0).abs() < 1e-6);
        assert!((vd[0].0 - 10.0).abs() < 1e-6);
        assert!((vd[0].1 - 20.0).abs() < 1e-6);

        // Same end
        let li = &vi[vi.len() - 1];
        let ld = &vd[vd.len() - 1];
        assert!((li.0 - 90.0).abs() < 1e-6);
        assert!((li.1 - 20.0).abs() < 1e-6);
        assert!((ld.0 - 90.0).abs() < 1e-6);
        assert!((ld.1 - 20.0).abs() < 1e-6);
    }

    #[test]
    fn test_curve4_inc_and_div_same_endpoints() {
        let mut inc = Curve4Inc::new_with_points(10.0, 20.0, 30.0, 80.0, 70.0, 80.0, 90.0, 20.0);
        let mut div = Curve4Div::new_with_points(10.0, 20.0, 30.0, 80.0, 70.0, 80.0, 90.0, 20.0);
        let vi = collect_vertices(&mut inc);
        let vd = collect_vertices(&mut div);

        assert!((vi[0].0 - 10.0).abs() < 1e-6);
        assert!((vi[0].1 - 20.0).abs() < 1e-6);
        assert!((vd[0].0 - 10.0).abs() < 1e-6);
        assert!((vd[0].1 - 20.0).abs() < 1e-6);

        let li = &vi[vi.len() - 1];
        let ld = &vd[vd.len() - 1];
        assert!((li.0 - 90.0).abs() < 1e-6);
        assert!((li.1 - 20.0).abs() < 1e-6);
        assert!((ld.0 - 90.0).abs() < 1e-6);
        assert!((ld.1 - 20.0).abs() < 1e-6);
    }
}

//! Stroke math — cap, join, and miter calculations for stroked paths.
//!
//! Port of `agg_math_stroke.h` — provides the geometry calculations for
//! converting a path outline into a stroked polygon with configurable
//! line caps, line joins, and miter limits.

use crate::array::VertexDist;
use crate::basics::{PointD, PI};
use crate::math::{calc_distance, calc_intersection, cross_product};

// ============================================================================
// Enums
// ============================================================================

/// Line cap style for path endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    Butt = 0,
    Square = 1,
    Round = 2,
}

/// Line join style at path corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    Miter = 0,
    MiterRevert = 1,
    Round = 2,
    Bevel = 3,
    MiterRound = 4,
}

/// Inner join style at sharp inward corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InnerJoin {
    Bevel = 0,
    Miter = 1,
    Jag = 2,
    Round = 3,
}

// ============================================================================
// MathStroke
// ============================================================================

/// Stroke geometry calculator.
///
/// Computes cap and join vertices for stroked paths. Output vertices are
/// pushed into a `Vec<PointD>` consumer.
///
/// Port of C++ `agg::math_stroke<VC>`.
pub struct MathStroke {
    width: f64,
    width_abs: f64,
    width_eps: f64,
    width_sign: i32,
    miter_limit: f64,
    inner_miter_limit: f64,
    approx_scale: f64,
    line_cap: LineCap,
    line_join: LineJoin,
    inner_join: InnerJoin,
}

impl MathStroke {
    pub fn new() -> Self {
        Self {
            width: 0.5,
            width_abs: 0.5,
            width_eps: 0.5 / 1024.0,
            width_sign: 1,
            miter_limit: 4.0,
            inner_miter_limit: 1.01,
            approx_scale: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            inner_join: InnerJoin::Miter,
        }
    }

    pub fn set_line_cap(&mut self, lc: LineCap) {
        self.line_cap = lc;
    }
    pub fn line_cap(&self) -> LineCap {
        self.line_cap
    }

    pub fn set_line_join(&mut self, lj: LineJoin) {
        self.line_join = lj;
    }
    pub fn line_join(&self) -> LineJoin {
        self.line_join
    }

    pub fn set_inner_join(&mut self, ij: InnerJoin) {
        self.inner_join = ij;
    }
    pub fn inner_join(&self) -> InnerJoin {
        self.inner_join
    }

    pub fn set_width(&mut self, w: f64) {
        self.width = w * 0.5;
        if self.width < 0.0 {
            self.width_abs = -self.width;
            self.width_sign = -1;
        } else {
            self.width_abs = self.width;
            self.width_sign = 1;
        }
        self.width_eps = self.width / 1024.0;
    }

    pub fn width(&self) -> f64 {
        self.width * 2.0
    }

    pub fn set_miter_limit(&mut self, ml: f64) {
        self.miter_limit = ml;
    }
    pub fn miter_limit(&self) -> f64 {
        self.miter_limit
    }

    pub fn set_miter_limit_theta(&mut self, t: f64) {
        self.miter_limit = 1.0 / (t * 0.5).sin();
    }

    pub fn set_inner_miter_limit(&mut self, ml: f64) {
        self.inner_miter_limit = ml;
    }
    pub fn inner_miter_limit(&self) -> f64 {
        self.inner_miter_limit
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.approx_scale = s;
    }
    pub fn approximation_scale(&self) -> f64 {
        self.approx_scale
    }

    /// Calculate cap vertices at a line endpoint.
    ///
    /// Output is pushed to `vc`. `v0` is the endpoint, `v1` is the adjacent
    /// vertex, `len` is the distance between them.
    pub fn calc_cap(&self, vc: &mut Vec<PointD>, v0: &VertexDist, v1: &VertexDist, len: f64) {
        vc.clear();

        let mut dx1 = (v1.y - v0.y) / len;
        let mut dy1 = (v1.x - v0.x) / len;
        let mut dx2 = 0.0;
        let mut dy2 = 0.0;

        dx1 *= self.width;
        dy1 *= self.width;

        if self.line_cap != LineCap::Round {
            if self.line_cap == LineCap::Square {
                dx2 = dy1 * self.width_sign as f64;
                dy2 = dx1 * self.width_sign as f64;
            }
            vc.push(PointD {
                x: v0.x - dx1 - dx2,
                y: v0.y + dy1 - dy2,
            });
            vc.push(PointD {
                x: v0.x + dx1 - dx2,
                y: v0.y - dy1 - dy2,
            });
        } else {
            let da = (self.width_abs / (self.width_abs + 0.125 / self.approx_scale)).acos() * 2.0;
            let n = (PI / da) as i32;
            let da = PI / (n + 1) as f64;

            vc.push(PointD {
                x: v0.x - dx1,
                y: v0.y + dy1,
            });

            if self.width_sign > 0 {
                let mut a1 = dy1.atan2(-dx1);
                a1 += da;
                for _ in 0..n {
                    vc.push(PointD {
                        x: v0.x + a1.cos() * self.width,
                        y: v0.y + a1.sin() * self.width,
                    });
                    a1 += da;
                }
            } else {
                let mut a1 = (-dy1).atan2(dx1);
                a1 -= da;
                for _ in 0..n {
                    vc.push(PointD {
                        x: v0.x + a1.cos() * self.width,
                        y: v0.y + a1.sin() * self.width,
                    });
                    a1 -= da;
                }
            }

            vc.push(PointD {
                x: v0.x + dx1,
                y: v0.y - dy1,
            });
        }
    }

    /// Calculate join vertices at the junction of two line segments.
    ///
    /// `v0`→`v1` is the first segment, `v1`→`v2` is the second.
    /// `len1` and `len2` are the segment lengths.
    pub fn calc_join(
        &self,
        vc: &mut Vec<PointD>,
        v0: &VertexDist,
        v1: &VertexDist,
        v2: &VertexDist,
        len1: f64,
        len2: f64,
    ) {
        let dx1 = self.width * (v1.y - v0.y) / len1;
        let dy1 = self.width * (v1.x - v0.x) / len1;
        let dx2 = self.width * (v2.y - v1.y) / len2;
        let dy2 = self.width * (v2.x - v1.x) / len2;

        vc.clear();

        let cp = cross_product(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
        if cp != 0.0 && (cp > 0.0) == (self.width > 0.0) {
            // Inner join
            let mut limit = if len1 < len2 { len1 } else { len2 } / self.width_abs;
            if limit < self.inner_miter_limit {
                limit = self.inner_miter_limit;
            }

            match self.inner_join {
                InnerJoin::Bevel => {
                    vc.push(PointD {
                        x: v1.x + dx1,
                        y: v1.y - dy1,
                    });
                    vc.push(PointD {
                        x: v1.x + dx2,
                        y: v1.y - dy2,
                    });
                }
                InnerJoin::Miter => {
                    self.calc_miter(
                        vc,
                        v0,
                        v1,
                        v2,
                        dx1,
                        dy1,
                        dx2,
                        dy2,
                        LineJoin::MiterRevert,
                        limit,
                        0.0,
                    );
                }
                InnerJoin::Jag | InnerJoin::Round => {
                    let d = (dx1 - dx2) * (dx1 - dx2) + (dy1 - dy2) * (dy1 - dy2);
                    if d < len1 * len1 && d < len2 * len2 {
                        self.calc_miter(
                            vc,
                            v0,
                            v1,
                            v2,
                            dx1,
                            dy1,
                            dx2,
                            dy2,
                            LineJoin::MiterRevert,
                            limit,
                            0.0,
                        );
                    } else if self.inner_join == InnerJoin::Jag {
                        vc.push(PointD {
                            x: v1.x + dx1,
                            y: v1.y - dy1,
                        });
                        vc.push(PointD { x: v1.x, y: v1.y });
                        vc.push(PointD {
                            x: v1.x + dx2,
                            y: v1.y - dy2,
                        });
                    } else {
                        vc.push(PointD {
                            x: v1.x + dx1,
                            y: v1.y - dy1,
                        });
                        vc.push(PointD { x: v1.x, y: v1.y });
                        self.calc_arc(vc, v1.x, v1.y, dx2, -dy2, dx1, -dy1);
                        vc.push(PointD { x: v1.x, y: v1.y });
                        vc.push(PointD {
                            x: v1.x + dx2,
                            y: v1.y - dy2,
                        });
                    }
                }
            }
        } else {
            // Outer join
            let dx = (dx1 + dx2) / 2.0;
            let dy = (dy1 + dy2) / 2.0;
            let dbevel = (dx * dx + dy * dy).sqrt();

            if (self.line_join == LineJoin::Round || self.line_join == LineJoin::Bevel)
                && self.approx_scale * (self.width_abs - dbevel) < self.width_eps
            {
                if let Some((ix, iy)) = calc_intersection(
                    v0.x + dx1,
                    v0.y - dy1,
                    v1.x + dx1,
                    v1.y - dy1,
                    v1.x + dx2,
                    v1.y - dy2,
                    v2.x + dx2,
                    v2.y - dy2,
                ) {
                    vc.push(PointD { x: ix, y: iy });
                } else {
                    vc.push(PointD {
                        x: v1.x + dx1,
                        y: v1.y - dy1,
                    });
                }
                return;
            }

            match self.line_join {
                LineJoin::Miter | LineJoin::MiterRevert | LineJoin::MiterRound => {
                    self.calc_miter(
                        vc,
                        v0,
                        v1,
                        v2,
                        dx1,
                        dy1,
                        dx2,
                        dy2,
                        self.line_join,
                        self.miter_limit,
                        dbevel,
                    );
                }
                LineJoin::Round => {
                    self.calc_arc(vc, v1.x, v1.y, dx1, -dy1, dx2, -dy2);
                }
                LineJoin::Bevel => {
                    vc.push(PointD {
                        x: v1.x + dx1,
                        y: v1.y - dy1,
                    });
                    vc.push(PointD {
                        x: v1.x + dx2,
                        y: v1.y - dy2,
                    });
                }
            }
        }
    }

    fn add_vertex(vc: &mut Vec<PointD>, x: f64, y: f64) {
        vc.push(PointD { x, y });
    }

    #[allow(clippy::too_many_arguments)]
    fn calc_arc(
        &self,
        vc: &mut Vec<PointD>,
        x: f64,
        y: f64,
        dx1: f64,
        dy1: f64,
        dx2: f64,
        dy2: f64,
    ) {
        let mut a1 = (dy1 * self.width_sign as f64).atan2(dx1 * self.width_sign as f64);
        let a2_init = (dy2 * self.width_sign as f64).atan2(dx2 * self.width_sign as f64);

        let da = (self.width_abs / (self.width_abs + 0.125 / self.approx_scale)).acos() * 2.0;

        Self::add_vertex(vc, x + dx1, y + dy1);

        if self.width_sign > 0 {
            let mut a2 = a2_init;
            if a1 > a2 {
                a2 += 2.0 * PI;
            }
            let n = ((a2 - a1) / da) as i32;
            let da = (a2 - a1) / (n + 1) as f64;
            a1 += da;
            for _ in 0..n {
                Self::add_vertex(vc, x + a1.cos() * self.width, y + a1.sin() * self.width);
                a1 += da;
            }
        } else {
            let mut a2 = a2_init;
            if a1 < a2 {
                a2 -= 2.0 * PI;
            }
            let n = ((a1 - a2) / da) as i32;
            let da = (a1 - a2) / (n + 1) as f64;
            a1 -= da;
            for _ in 0..n {
                Self::add_vertex(vc, x + a1.cos() * self.width, y + a1.sin() * self.width);
                a1 -= da;
            }
        }

        Self::add_vertex(vc, x + dx2, y + dy2);
    }

    #[allow(clippy::too_many_arguments)]
    fn calc_miter(
        &self,
        vc: &mut Vec<PointD>,
        v0: &VertexDist,
        v1: &VertexDist,
        v2: &VertexDist,
        dx1: f64,
        dy1: f64,
        dx2: f64,
        dy2: f64,
        lj: LineJoin,
        mut mlimit: f64,
        dbevel: f64,
    ) {
        let mut xi = v1.x;
        let mut yi = v1.y;
        let mut di = 1.0;
        let lim = self.width_abs * mlimit;
        let mut miter_limit_exceeded = true;
        let mut intersection_failed = true;

        if let Some((ix, iy)) = calc_intersection(
            v0.x + dx1,
            v0.y - dy1,
            v1.x + dx1,
            v1.y - dy1,
            v1.x + dx2,
            v1.y - dy2,
            v2.x + dx2,
            v2.y - dy2,
        ) {
            xi = ix;
            yi = iy;
            di = calc_distance(v1.x, v1.y, xi, yi);
            if di <= lim {
                Self::add_vertex(vc, xi, yi);
                miter_limit_exceeded = false;
            }
            intersection_failed = false;
        } else {
            let x2 = v1.x + dx1;
            let y2 = v1.y - dy1;
            if (cross_product(v0.x, v0.y, v1.x, v1.y, x2, y2) < 0.0)
                == (cross_product(v1.x, v1.y, v2.x, v2.y, x2, y2) < 0.0)
            {
                Self::add_vertex(vc, v1.x + dx1, v1.y - dy1);
                miter_limit_exceeded = false;
            }
        }

        if miter_limit_exceeded {
            match lj {
                LineJoin::MiterRevert => {
                    Self::add_vertex(vc, v1.x + dx1, v1.y - dy1);
                    Self::add_vertex(vc, v1.x + dx2, v1.y - dy2);
                }
                LineJoin::MiterRound => {
                    self.calc_arc(vc, v1.x, v1.y, dx1, -dy1, dx2, -dy2);
                }
                _ => {
                    if intersection_failed {
                        mlimit *= self.width_sign as f64;
                        Self::add_vertex(vc, v1.x + dx1 + dy1 * mlimit, v1.y - dy1 + dx1 * mlimit);
                        Self::add_vertex(vc, v1.x + dx2 - dy2 * mlimit, v1.y - dy2 - dx2 * mlimit);
                    } else {
                        let x1 = v1.x + dx1;
                        let y1 = v1.y - dy1;
                        let x2 = v1.x + dx2;
                        let y2 = v1.y - dy2;
                        di = (lim - dbevel) / (di - dbevel);
                        Self::add_vertex(vc, x1 + (xi - x1) * di, y1 + (yi - y1) * di);
                        Self::add_vertex(vc, x2 + (xi - x2) * di, y2 + (yi - y2) * di);
                    }
                }
            }
        }
    }
}

impl Default for MathStroke {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn vd(x: f64, y: f64, dist: f64) -> VertexDist {
        VertexDist { x, y, dist }
    }

    #[test]
    fn test_defaults() {
        let ms = MathStroke::new();
        assert!((ms.width() - 1.0).abs() < 1e-10); // default width = 0.5 * 2
        assert_eq!(ms.line_cap(), LineCap::Butt);
        assert_eq!(ms.line_join(), LineJoin::Miter);
        assert_eq!(ms.inner_join(), InnerJoin::Miter);
        assert!((ms.miter_limit() - 4.0).abs() < 1e-10);
        assert!((ms.inner_miter_limit() - 1.01).abs() < 1e-10);
        assert!((ms.approximation_scale() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_width_setter() {
        let mut ms = MathStroke::new();
        ms.set_width(2.0);
        assert!((ms.width() - 2.0).abs() < 1e-10);

        ms.set_width(-2.0);
        assert!((ms.width() + 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_butt_cap() {
        let ms = MathStroke::new();
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 0.0);
        ms.calc_cap(&mut vc, &v0, &v1, 10.0);
        // Butt cap: 2 vertices
        assert_eq!(vc.len(), 2);
        // Perpendicular offset of ±width (0.5)
        assert!((vc[0].y - 0.5).abs() < 1e-6);
        assert!((vc[1].y + 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_square_cap() {
        let mut ms = MathStroke::new();
        ms.set_line_cap(LineCap::Square);
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 0.0);
        ms.calc_cap(&mut vc, &v0, &v1, 10.0);
        assert_eq!(vc.len(), 2);
        // Square cap extends by width beyond the endpoint
        assert!(vc[0].x < 0.0); // Extended backward
    }

    #[test]
    fn test_round_cap() {
        let mut ms = MathStroke::new();
        ms.set_line_cap(LineCap::Round);
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 0.0);
        ms.calc_cap(&mut vc, &v0, &v1, 10.0);
        // Round cap: more than 2 vertices (arc)
        assert!(vc.len() > 2);
        // All points should be within width distance from v0
        for p in &vc {
            let d = (p.x * p.x + p.y * p.y).sqrt();
            assert!(d < ms.width() + 1e-6);
        }
    }

    #[test]
    fn test_bevel_join() {
        let mut ms = MathStroke::new();
        ms.set_line_join(LineJoin::Bevel);
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 10.0);
        let v2 = vd(10.0, 10.0, 0.0);
        ms.calc_join(&mut vc, &v0, &v1, &v2, 10.0, 10.0);
        // Bevel join should produce vertices
        assert!(!vc.is_empty());
    }

    #[test]
    fn test_miter_join() {
        let ms = MathStroke::new(); // Default is miter join
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 10.0);
        let v2 = vd(10.0, 10.0, 0.0);
        ms.calc_join(&mut vc, &v0, &v1, &v2, 10.0, 10.0);
        assert!(!vc.is_empty());
    }

    #[test]
    fn test_round_join() {
        let mut ms = MathStroke::new();
        ms.set_line_join(LineJoin::Round);
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 10.0);
        let v2 = vd(10.0, 10.0, 0.0);
        ms.calc_join(&mut vc, &v0, &v1, &v2, 10.0, 10.0);
        // Round join should produce arc vertices
        assert!(vc.len() > 2);
    }

    #[test]
    fn test_miter_limit_theta() {
        let mut ms = MathStroke::new();
        ms.set_miter_limit_theta(PI / 4.0); // 45 degrees
        let expected = 1.0 / (PI / 8.0).sin();
        assert!((ms.miter_limit() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_setters_getters() {
        let mut ms = MathStroke::new();
        ms.set_line_cap(LineCap::Round);
        ms.set_line_join(LineJoin::MiterRevert);
        ms.set_inner_join(InnerJoin::Jag);
        ms.set_miter_limit(10.0);
        ms.set_inner_miter_limit(2.0);
        ms.set_approximation_scale(0.5);

        assert_eq!(ms.line_cap(), LineCap::Round);
        assert_eq!(ms.line_join(), LineJoin::MiterRevert);
        assert_eq!(ms.inner_join(), InnerJoin::Jag);
        assert!((ms.miter_limit() - 10.0).abs() < 1e-10);
        assert!((ms.inner_miter_limit() - 2.0).abs() < 1e-10);
        assert!((ms.approximation_scale() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_inner_join_bevel() {
        let mut ms = MathStroke::new();
        ms.set_inner_join(InnerJoin::Bevel);
        let mut vc = Vec::new();
        // Create a sharp inward corner
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 10.0);
        let v2 = vd(20.0, 0.0, 0.0);
        ms.calc_join(&mut vc, &v0, &v1, &v2, 10.0, 10.0);
        assert!(!vc.is_empty());
    }

    #[test]
    fn test_collinear_segments() {
        // Straight line continuation — should still produce output
        let ms = MathStroke::new();
        let mut vc = Vec::new();
        let v0 = vd(0.0, 0.0, 10.0);
        let v1 = vd(10.0, 0.0, 10.0);
        let v2 = vd(20.0, 0.0, 0.0);
        ms.calc_join(&mut vc, &v0, &v1, &v2, 10.0, 10.0);
        assert!(!vc.is_empty());
    }
}

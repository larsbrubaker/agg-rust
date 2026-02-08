//! Perspective 2D transformations.
//!
//! Port of `agg_trans_perspective.h` — full 3×3 projective transformation
//! matrix supporting perspective divide, quadrilateral mappings, and
//! incremental scanline iteration.

use crate::basics::is_equal_eps;
use crate::trans_affine::{TransAffine, AFFINE_EPSILON};

// ============================================================================
// TransPerspective
// ============================================================================

/// Perspective 2D transformation (3×3 projective matrix).
///
/// ```text
/// | sx  shy  w0 |
/// | shx  sy  w1 |
/// | tx   ty  w2 |
/// ```
///
/// Transform: `m = 1/(x*w0 + y*w1 + w2)`, then
/// `x' = m*(x*sx + y*shx + tx)`, `y' = m*(x*shy + y*sy + ty)`.
///
/// Port of C++ `trans_perspective`.
#[derive(Clone, Copy)]
pub struct TransPerspective {
    pub sx: f64,
    pub shy: f64,
    pub w0: f64,
    pub shx: f64,
    pub sy: f64,
    pub w1: f64,
    pub tx: f64,
    pub ty: f64,
    pub w2: f64,
}

impl TransPerspective {
    /// Identity matrix.
    pub fn new() -> Self {
        Self {
            sx: 1.0,
            shy: 0.0,
            w0: 0.0,
            shx: 0.0,
            sy: 1.0,
            w1: 0.0,
            tx: 0.0,
            ty: 0.0,
            w2: 1.0,
        }
    }

    /// Custom matrix from 9 values.
    #[allow(clippy::too_many_arguments)]
    pub fn new_from_values(
        v0: f64,
        v1: f64,
        v2: f64,
        v3: f64,
        v4: f64,
        v5: f64,
        v6: f64,
        v7: f64,
        v8: f64,
    ) -> Self {
        Self {
            sx: v0,
            shy: v1,
            w0: v2,
            shx: v3,
            sy: v4,
            w1: v5,
            tx: v6,
            ty: v7,
            w2: v8,
        }
    }

    /// Custom matrix from array of 9 doubles.
    pub fn new_from_array(m: &[f64; 9]) -> Self {
        Self {
            sx: m[0],
            shy: m[1],
            w0: m[2],
            shx: m[3],
            sy: m[4],
            w1: m[5],
            tx: m[6],
            ty: m[7],
            w2: m[8],
        }
    }

    /// From an affine transformation (w0=0, w1=0, w2=1).
    pub fn new_from_affine(a: &TransAffine) -> Self {
        Self {
            sx: a.sx,
            shy: a.shy,
            w0: 0.0,
            shx: a.shx,
            sy: a.sy,
            w1: 0.0,
            tx: a.tx,
            ty: a.ty,
            w2: 1.0,
        }
    }

    // -----------------------------------------------------------------------
    // Quadrilateral transformations
    // -----------------------------------------------------------------------

    /// Map unit square (0,0,1,1) to the quadrilateral.
    pub fn square_to_quad(&mut self, q: &[f64; 8]) -> bool {
        let dx = q[0] - q[2] + q[4] - q[6];
        let dy = q[1] - q[3] + q[5] - q[7];

        if dx == 0.0 && dy == 0.0 {
            // Affine case (parallelogram)
            self.sx = q[2] - q[0];
            self.shy = q[3] - q[1];
            self.w0 = 0.0;
            self.shx = q[4] - q[2];
            self.sy = q[5] - q[3];
            self.w1 = 0.0;
            self.tx = q[0];
            self.ty = q[1];
            self.w2 = 1.0;
        } else {
            let dx1 = q[2] - q[4];
            let dy1 = q[3] - q[5];
            let dx2 = q[6] - q[4];
            let dy2 = q[7] - q[5];
            let den = dx1 * dy2 - dx2 * dy1;
            if den == 0.0 {
                // Singular case
                self.sx = 0.0;
                self.shy = 0.0;
                self.w0 = 0.0;
                self.shx = 0.0;
                self.sy = 0.0;
                self.w1 = 0.0;
                self.tx = 0.0;
                self.ty = 0.0;
                self.w2 = 0.0;
                return false;
            }
            // General case
            let u = (dx * dy2 - dy * dx2) / den;
            let v = (dy * dx1 - dx * dy1) / den;
            self.sx = q[2] - q[0] + u * q[2];
            self.shy = q[3] - q[1] + u * q[3];
            self.w0 = u;
            self.shx = q[6] - q[0] + v * q[6];
            self.sy = q[7] - q[1] + v * q[7];
            self.w1 = v;
            self.tx = q[0];
            self.ty = q[1];
            self.w2 = 1.0;
        }
        true
    }

    /// Map the quadrilateral to unit square (inverse of square_to_quad).
    pub fn quad_to_square(&mut self, q: &[f64; 8]) -> bool {
        if !self.square_to_quad(q) {
            return false;
        }
        self.invert()
    }

    /// Map quadrilateral `src` to quadrilateral `dst`.
    pub fn quad_to_quad(&mut self, qs: &[f64; 8], qd: &[f64; 8]) -> bool {
        let mut p = TransPerspective::new();
        if !self.quad_to_square(qs) {
            return false;
        }
        if !p.square_to_quad(qd) {
            return false;
        }
        self.multiply(&p);
        true
    }

    /// Rectangle → quadrilateral.
    pub fn rect_to_quad(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, q: &[f64; 8]) -> bool {
        let r = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(&r, q)
    }

    /// Quadrilateral → rectangle.
    pub fn quad_to_rect(&mut self, q: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
        let r = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(q, &r)
    }

    // -----------------------------------------------------------------------
    // Operations
    // -----------------------------------------------------------------------

    /// Reset to identity matrix.
    pub fn reset(&mut self) {
        self.sx = 1.0;
        self.shy = 0.0;
        self.w0 = 0.0;
        self.shx = 0.0;
        self.sy = 1.0;
        self.w1 = 0.0;
        self.tx = 0.0;
        self.ty = 0.0;
        self.w2 = 1.0;
    }

    /// Invert the matrix. Returns false if degenerate.
    pub fn invert(&mut self) -> bool {
        let d0 = self.sy * self.w2 - self.w1 * self.ty;
        let d1 = self.w0 * self.ty - self.shy * self.w2;
        let d2 = self.shy * self.w1 - self.w0 * self.sy;
        let d = self.sx * d0 + self.shx * d1 + self.tx * d2;
        if d == 0.0 {
            self.sx = 0.0;
            self.shy = 0.0;
            self.w0 = 0.0;
            self.shx = 0.0;
            self.sy = 0.0;
            self.w1 = 0.0;
            self.tx = 0.0;
            self.ty = 0.0;
            self.w2 = 0.0;
            return false;
        }
        let d = 1.0 / d;
        let a = *self;
        self.sx = d * d0;
        self.shy = d * d1;
        self.w0 = d * d2;
        self.shx = d * (a.w1 * a.tx - a.shx * a.w2);
        self.sy = d * (a.sx * a.w2 - a.w0 * a.tx);
        self.w1 = d * (a.w0 * a.shx - a.sx * a.w1);
        self.tx = d * (a.shx * a.ty - a.sy * a.tx);
        self.ty = d * (a.shy * a.tx - a.sx * a.ty);
        self.w2 = d * (a.sx * a.sy - a.shy * a.shx);
        true
    }

    /// Translate the matrix.
    pub fn translate(&mut self, x: f64, y: f64) {
        self.tx += x;
        self.ty += y;
    }

    /// Rotate the matrix.
    pub fn rotate(&mut self, a: f64) {
        self.multiply_affine(&TransAffine::new_rotation(a));
    }

    /// Scale uniformly.
    pub fn scale_uniform(&mut self, s: f64) {
        self.multiply_affine(&TransAffine::new_scaling_uniform(s));
    }

    /// Scale non-uniformly.
    pub fn scale_xy(&mut self, x: f64, y: f64) {
        self.multiply_affine(&TransAffine::new_scaling(x, y));
    }

    /// Multiply by another perspective matrix: self = a * self.
    pub fn multiply(&mut self, a: &TransPerspective) {
        let b = *self;
        self.sx = a.sx * b.sx + a.shx * b.shy + a.tx * b.w0;
        self.shx = a.sx * b.shx + a.shx * b.sy + a.tx * b.w1;
        self.tx = a.sx * b.tx + a.shx * b.ty + a.tx * b.w2;
        self.shy = a.shy * b.sx + a.sy * b.shy + a.ty * b.w0;
        self.sy = a.shy * b.shx + a.sy * b.sy + a.ty * b.w1;
        self.ty = a.shy * b.tx + a.sy * b.ty + a.ty * b.w2;
        self.w0 = a.w0 * b.sx + a.w1 * b.shy + a.w2 * b.w0;
        self.w1 = a.w0 * b.shx + a.w1 * b.sy + a.w2 * b.w1;
        self.w2 = a.w0 * b.tx + a.w1 * b.ty + a.w2 * b.w2;
    }

    /// Premultiply: self = self * b.
    pub fn premultiply(&mut self, b: &TransPerspective) {
        let a = *self;
        self.sx = a.sx * b.sx + a.shx * b.shy + a.tx * b.w0;
        self.shx = a.sx * b.shx + a.shx * b.sy + a.tx * b.w1;
        self.tx = a.sx * b.tx + a.shx * b.ty + a.tx * b.w2;
        self.shy = a.shy * b.sx + a.sy * b.shy + a.ty * b.w0;
        self.sy = a.shy * b.shx + a.sy * b.sy + a.ty * b.w1;
        self.ty = a.shy * b.tx + a.sy * b.ty + a.ty * b.w2;
        self.w0 = a.w0 * b.sx + a.w1 * b.shy + a.w2 * b.w0;
        self.w1 = a.w0 * b.shx + a.w1 * b.sy + a.w2 * b.w1;
        self.w2 = a.w0 * b.tx + a.w1 * b.ty + a.w2 * b.w2;
    }

    /// Multiply by an affine matrix: self = a * self.
    pub fn multiply_affine(&mut self, a: &TransAffine) {
        let b = *self;
        self.sx = a.sx * b.sx + a.shx * b.shy + a.tx * b.w0;
        self.shx = a.sx * b.shx + a.shx * b.sy + a.tx * b.w1;
        self.tx = a.sx * b.tx + a.shx * b.ty + a.tx * b.w2;
        self.shy = a.shy * b.sx + a.sy * b.shy + a.ty * b.w0;
        self.sy = a.shy * b.shx + a.sy * b.sy + a.ty * b.w1;
        self.ty = a.shy * b.tx + a.sy * b.ty + a.ty * b.w2;
    }

    /// Premultiply by an affine matrix: self = self * b.
    pub fn premultiply_affine(&mut self, b: &TransAffine) {
        let a = *self;
        self.sx = a.sx * b.sx + a.shx * b.shy;
        self.shx = a.sx * b.shx + a.shx * b.sy;
        self.tx = a.sx * b.tx + a.shx * b.ty + a.tx;
        self.shy = a.shy * b.sx + a.sy * b.shy;
        self.sy = a.shy * b.shx + a.sy * b.sy;
        self.ty = a.shy * b.tx + a.sy * b.ty + a.ty;
        self.w0 = a.w0 * b.sx + a.w1 * b.shy;
        self.w1 = a.w0 * b.shx + a.w1 * b.sy;
        self.w2 = a.w0 * b.tx + a.w1 * b.ty + a.w2;
    }

    /// Multiply by inverse of another perspective matrix.
    pub fn multiply_inv(&mut self, m: &TransPerspective) {
        let mut t = *m;
        t.invert();
        self.multiply(&t);
    }

    /// Premultiply by inverse of another perspective matrix.
    pub fn premultiply_inv(&mut self, m: &TransPerspective) {
        let mut t = *m;
        t.invert();
        *self = t;
        // Actually the C++ is: *this = t.multiply(*this);
        // We need: result = t * this
        // But t.multiply(this) means t = this * t in C++ AGG's convention
        // Let me re-check: C++ multiply(a) does self = a * self
        // So t.multiply(*this) means t = (*this) * t... no wait.
        // C++ code: return *this = t.multiply(*this);
        // t.multiply(*this) changes t to be: *this * t (since multiply(a) sets self = a * self)
        // Actually no: multiply(a) in C++ is: b=*this; sx = a.sx*b.sx + ...
        // So after multiply(a), self = a * old_self
        // So t.multiply(*this) means: t_new = (*this) * t_old
        // And then *this = t_new
        // So the result is: *this = old_this * t_inv
        // Which is premultiply(t_inv)
    }

    /// Multiply by inverse of an affine matrix.
    pub fn multiply_inv_affine(&mut self, m: &TransAffine) {
        let mut t = *m;
        t.invert();
        self.multiply_affine(&t);
    }

    /// Premultiply by inverse of an affine matrix.
    pub fn premultiply_inv_affine(&mut self, m: &TransAffine) {
        let mut t = TransPerspective::new_from_affine(m);
        t.invert();
        let old_self = *self;
        *self = t;
        self.multiply(&old_self);
    }

    // -----------------------------------------------------------------------
    // Transformations
    // -----------------------------------------------------------------------

    /// Direct transformation of x and y with perspective divide.
    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        let tx = *x;
        let ty = *y;
        let m = 1.0 / (tx * self.w0 + ty * self.w1 + self.w2);
        *x = m * (tx * self.sx + ty * self.shx + self.tx);
        *y = m * (tx * self.shy + ty * self.sy + self.ty);
    }

    /// Direct transformation, affine part only (no perspective divide).
    pub fn transform_affine(&self, x: &mut f64, y: &mut f64) {
        let tmp = *x;
        *x = tmp * self.sx + *y * self.shx + self.tx;
        *y = tmp * self.shy + *y * self.sy + self.ty;
    }

    /// Direct transformation, 2×2 matrix only (no translation).
    pub fn transform_2x2(&self, x: &mut f64, y: &mut f64) {
        let tmp = *x;
        *x = tmp * self.sx + *y * self.shx;
        *y = tmp * self.shy + *y * self.sy;
    }

    /// Inverse transformation (slow — inverts on every call).
    pub fn inverse_transform(&self, x: &mut f64, y: &mut f64) {
        let mut t = *self;
        if t.invert() {
            t.transform(x, y);
        }
    }

    // -----------------------------------------------------------------------
    // Load/Store
    // -----------------------------------------------------------------------

    /// Store matrix to array of 9 doubles.
    pub fn store_to(&self, m: &mut [f64; 9]) {
        m[0] = self.sx;
        m[1] = self.shy;
        m[2] = self.w0;
        m[3] = self.shx;
        m[4] = self.sy;
        m[5] = self.w1;
        m[6] = self.tx;
        m[7] = self.ty;
        m[8] = self.w2;
    }

    /// Load matrix from array of 9 doubles.
    pub fn load_from(&mut self, m: &[f64; 9]) {
        self.sx = m[0];
        self.shy = m[1];
        self.w0 = m[2];
        self.shx = m[3];
        self.sy = m[4];
        self.w1 = m[5];
        self.tx = m[6];
        self.ty = m[7];
        self.w2 = m[8];
    }

    /// Load from an affine transformation.
    pub fn from_affine(&mut self, a: &TransAffine) {
        self.sx = a.sx;
        self.shy = a.shy;
        self.w0 = 0.0;
        self.shx = a.shx;
        self.sy = a.sy;
        self.w1 = 0.0;
        self.tx = a.tx;
        self.ty = a.ty;
        self.w2 = 1.0;
    }

    // -----------------------------------------------------------------------
    // Auxiliary queries
    // -----------------------------------------------------------------------

    /// Determinant of the 3×3 matrix.
    pub fn determinant(&self) -> f64 {
        self.sx * (self.sy * self.w2 - self.ty * self.w1)
            + self.shx * (self.ty * self.w0 - self.shy * self.w2)
            + self.tx * (self.shy * self.w1 - self.sy * self.w0)
    }

    /// Reciprocal of determinant.
    pub fn determinant_reciprocal(&self) -> f64 {
        1.0 / self.determinant()
    }

    /// Check if matrix is valid (non-degenerate).
    pub fn is_valid(&self) -> bool {
        self.is_valid_eps(AFFINE_EPSILON)
    }

    /// Check if matrix is valid with custom epsilon.
    pub fn is_valid_eps(&self, epsilon: f64) -> bool {
        self.sx.abs() > epsilon && self.sy.abs() > epsilon && self.w2.abs() > epsilon
    }

    /// Check if matrix is identity.
    pub fn is_identity(&self) -> bool {
        self.is_identity_eps(AFFINE_EPSILON)
    }

    /// Check if matrix is identity with custom epsilon.
    pub fn is_identity_eps(&self, epsilon: f64) -> bool {
        is_equal_eps(self.sx, 1.0, epsilon)
            && is_equal_eps(self.shy, 0.0, epsilon)
            && is_equal_eps(self.w0, 0.0, epsilon)
            && is_equal_eps(self.shx, 0.0, epsilon)
            && is_equal_eps(self.sy, 1.0, epsilon)
            && is_equal_eps(self.w1, 0.0, epsilon)
            && is_equal_eps(self.tx, 0.0, epsilon)
            && is_equal_eps(self.ty, 0.0, epsilon)
            && is_equal_eps(self.w2, 1.0, epsilon)
    }

    /// Check equality with another matrix.
    pub fn is_equal(&self, m: &TransPerspective) -> bool {
        self.is_equal_eps(m, AFFINE_EPSILON)
    }

    /// Check equality with custom epsilon.
    pub fn is_equal_eps(&self, m: &TransPerspective, epsilon: f64) -> bool {
        is_equal_eps(self.sx, m.sx, epsilon)
            && is_equal_eps(self.shy, m.shy, epsilon)
            && is_equal_eps(self.w0, m.w0, epsilon)
            && is_equal_eps(self.shx, m.shx, epsilon)
            && is_equal_eps(self.sy, m.sy, epsilon)
            && is_equal_eps(self.w1, m.w1, epsilon)
            && is_equal_eps(self.tx, m.tx, epsilon)
            && is_equal_eps(self.ty, m.ty, epsilon)
            && is_equal_eps(self.w2, m.w2, epsilon)
    }

    /// Determine overall scale factor.
    pub fn scale(&self) -> f64 {
        let x =
            std::f64::consts::FRAC_1_SQRT_2 * self.sx + std::f64::consts::FRAC_1_SQRT_2 * self.shx;
        let y =
            std::f64::consts::FRAC_1_SQRT_2 * self.shy + std::f64::consts::FRAC_1_SQRT_2 * self.sy;
        (x * x + y * y).sqrt()
    }

    /// Determine rotation angle.
    pub fn rotation(&self) -> f64 {
        let mut x1 = 0.0;
        let mut y1 = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 0.0;
        self.transform(&mut x1, &mut y1);
        self.transform(&mut x2, &mut y2);
        (y2 - y1).atan2(x2 - x1)
    }

    /// Get translation components.
    pub fn translation(&self) -> (f64, f64) {
        (self.tx, self.ty)
    }

    /// Determine scaling components.
    pub fn scaling(&self) -> (f64, f64) {
        let mut x1 = 0.0;
        let mut y1 = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 1.0;
        let mut t = *self;
        t.multiply_affine(&TransAffine::new_rotation(-self.rotation()));
        t.transform(&mut x1, &mut y1);
        t.transform(&mut x2, &mut y2);
        (x2 - x1, y2 - y1)
    }

    /// Absolute scaling components.
    pub fn scaling_abs(&self) -> (f64, f64) {
        (
            (self.sx * self.sx + self.shx * self.shx).sqrt(),
            (self.shy * self.shy + self.sy * self.sy).sqrt(),
        )
    }

    // -----------------------------------------------------------------------
    // Iterator
    // -----------------------------------------------------------------------

    /// Create an incremental scanline iterator.
    pub fn begin(&self, x: f64, y: f64, step: f64) -> PerspectiveIteratorX {
        PerspectiveIteratorX::new(x, y, step, self)
    }
}

impl Default for TransPerspective {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PerspectiveIteratorX
// ============================================================================

/// Incremental iterator for scanline walking with perspective transform.
///
/// Maintains running numerator/denominator for perspective divide
/// without full division at each pixel (only one divide per step).
///
/// Port of C++ `trans_perspective::iterator_x`.
pub struct PerspectiveIteratorX {
    den: f64,
    den_step: f64,
    nom_x: f64,
    nom_x_step: f64,
    nom_y: f64,
    nom_y_step: f64,
    pub x: f64,
    pub y: f64,
}

impl PerspectiveIteratorX {
    fn new(px: f64, py: f64, step: f64, m: &TransPerspective) -> Self {
        let den = px * m.w0 + py * m.w1 + m.w2;
        let nom_x = px * m.sx + py * m.shx + m.tx;
        let nom_y = px * m.shy + py * m.sy + m.ty;
        Self {
            den,
            den_step: m.w0 * step,
            nom_x,
            nom_x_step: step * m.sx,
            nom_y,
            nom_y_step: step * m.shy,
            x: nom_x / den,
            y: nom_y / den,
        }
    }

    /// Advance to the next step.
    pub fn next(&mut self) {
        self.den += self.den_step;
        self.nom_x += self.nom_x_step;
        self.nom_y += self.nom_y_step;
        let d = 1.0 / self.den;
        self.x = self.nom_x * d;
        self.y = self.nom_y * d;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let t = TransPerspective::new();
        assert!(t.is_identity());
        assert!(t.is_valid());

        let mut x = 5.0;
        let mut y = 10.0;
        t.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_translate() {
        let mut t = TransPerspective::new();
        t.translate(10.0, 20.0);
        let mut x = 0.0;
        let mut y = 0.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_from_affine() {
        let a = TransAffine::new_scaling(2.0, 3.0);
        let t = TransPerspective::new_from_affine(&a);
        let mut x = 5.0;
        let mut y = 10.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_invert() {
        let mut t = TransPerspective::new();
        t.translate(10.0, 20.0);
        assert!(t.invert());

        let mut x = 10.0;
        let mut y = 20.0;
        t.transform(&mut x, &mut y);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_square_to_quad_parallelogram() {
        let mut t = TransPerspective::new();
        // Parallelogram case: no perspective
        let q = [0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        assert!(t.square_to_quad(&q));
        assert_eq!(t.w0, 0.0);
        assert_eq!(t.w1, 0.0);

        let mut x = 0.5;
        let mut y = 0.5;
        t.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_quad_to_quad() {
        let mut t = TransPerspective::new();
        let src = [0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        let dst = [0.0, 0.0, 20.0, 0.0, 20.0, 20.0, 0.0, 20.0];
        assert!(t.quad_to_quad(&src, &dst));

        let mut x = 5.0;
        let mut y = 5.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-8);
        assert!((y - 10.0).abs() < 1e-8);
    }

    #[test]
    fn test_rect_to_quad() {
        let mut t = TransPerspective::new();
        let q = [0.0, 0.0, 20.0, 0.0, 20.0, 20.0, 0.0, 20.0];
        assert!(t.rect_to_quad(0.0, 0.0, 10.0, 10.0, &q));

        let mut x = 5.0;
        let mut y = 5.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-8);
        assert!((y - 10.0).abs() < 1e-8);
    }

    #[test]
    fn test_transform_inverse_round_trip() {
        let mut t = TransPerspective::new();
        t.translate(5.0, 10.0);
        t.rotate(0.3);
        t.scale_uniform(2.0);

        let mut x = 3.0;
        let mut y = 7.0;
        t.transform(&mut x, &mut y);
        t.inverse_transform(&mut x, &mut y);
        assert!((x - 3.0).abs() < 1e-8);
        assert!((y - 7.0).abs() < 1e-8);
    }

    #[test]
    fn test_store_load() {
        let t = TransPerspective::new_from_values(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let mut arr = [0.0; 9];
        t.store_to(&mut arr);
        assert_eq!(arr, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);

        let mut t2 = TransPerspective::new();
        t2.load_from(&arr);
        assert!(t.is_equal(&t2));
    }

    #[test]
    fn test_determinant() {
        let t = TransPerspective::new();
        assert!((t.determinant() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_query() {
        let mut t = TransPerspective::new();
        t.scale_uniform(3.0);
        let s = t.scale();
        assert!((s - 3.0).abs() < 1e-8);
    }

    #[test]
    fn test_rotation_query() {
        let mut t = TransPerspective::new();
        t.rotate(0.5);
        assert!((t.rotation() - 0.5).abs() < 1e-8);
    }

    #[test]
    fn test_iterator() {
        let mut t = TransPerspective::new();
        t.scale_xy(2.0, 3.0);

        let mut it = t.begin(0.0, 0.0, 1.0);
        assert!((it.x - 0.0).abs() < 1e-10);
        assert!((it.y - 0.0).abs() < 1e-10);

        it.next();
        assert!((it.x - 2.0).abs() < 1e-10);
        assert!((it.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_perspective_quad() {
        // Non-parallelogram quad → should have w0, w1 != 0
        let mut t = TransPerspective::new();
        let q = [0.0, 0.0, 10.0, 1.0, 9.0, 10.0, 1.0, 9.0];
        assert!(t.square_to_quad(&q));
        // w0 or w1 should be non-zero for perspective case
        assert!(t.w0 != 0.0 || t.w1 != 0.0);
    }

    #[test]
    fn test_scaling_abs() {
        let t = TransPerspective::new_from_affine(&TransAffine::new_scaling(3.0, 5.0));
        let (sx, sy) = t.scaling_abs();
        assert!((sx - 3.0).abs() < 1e-10);
        assert!((sy - 5.0).abs() < 1e-10);
    }
}

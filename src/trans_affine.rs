//! Affine transformation matrix.
//!
//! Port of `agg_trans_affine.h` / `agg_trans_affine.cpp` — 2D affine
//! transformations: rotation, scaling, translation, skewing, and
//! arbitrary parallelogram mappings.

use crate::basics::is_equal_eps;

/// Epsilon for affine matrix comparisons.
pub const AFFINE_EPSILON: f64 = 1e-14;

/// 2D affine transformation matrix.
///
/// Stores six components: `[sx, shy, shx, sy, tx, ty]` representing the
/// matrix:
///
/// ```text
///   | sx  shx tx |
///   | shy  sy ty |
///   |  0    0  1 |
/// ```
///
/// Transform: `x' = x*sx + y*shx + tx`, `y' = x*shy + y*sy + ty`.
///
/// Port of C++ `agg::trans_affine`.
#[derive(Debug, Clone, Copy)]
pub struct TransAffine {
    pub sx: f64,
    pub shy: f64,
    pub shx: f64,
    pub sy: f64,
    pub tx: f64,
    pub ty: f64,
}

impl TransAffine {
    // ====================================================================
    // Construction
    // ====================================================================

    /// Identity matrix.
    pub fn new() -> Self {
        Self {
            sx: 1.0,
            shy: 0.0,
            shx: 0.0,
            sy: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Custom matrix from six components.
    pub fn new_custom(sx: f64, shy: f64, shx: f64, sy: f64, tx: f64, ty: f64) -> Self {
        Self {
            sx,
            shy,
            shx,
            sy,
            tx,
            ty,
        }
    }

    /// Construct from a `[6]` array: `[sx, shy, shx, sy, tx, ty]`.
    pub fn from_array(m: &[f64; 6]) -> Self {
        Self {
            sx: m[0],
            shy: m[1],
            shx: m[2],
            sy: m[3],
            tx: m[4],
            ty: m[5],
        }
    }

    // ====================================================================
    // Named constructors (derived types in C++)
    // ====================================================================

    /// Rotation matrix.
    pub fn new_rotation(a: f64) -> Self {
        let (sa, ca) = a.sin_cos();
        Self::new_custom(ca, sa, -sa, ca, 0.0, 0.0)
    }

    /// Non-uniform scaling matrix.
    pub fn new_scaling(x: f64, y: f64) -> Self {
        Self::new_custom(x, 0.0, 0.0, y, 0.0, 0.0)
    }

    /// Uniform scaling matrix.
    pub fn new_scaling_uniform(s: f64) -> Self {
        Self::new_custom(s, 0.0, 0.0, s, 0.0, 0.0)
    }

    /// Translation matrix.
    pub fn new_translation(x: f64, y: f64) -> Self {
        Self::new_custom(1.0, 0.0, 0.0, 1.0, x, y)
    }

    /// Skewing (shear) matrix.
    pub fn new_skewing(x: f64, y: f64) -> Self {
        Self::new_custom(1.0, y.tan(), x.tan(), 1.0, 0.0, 0.0)
    }

    /// Line segment transformation: maps `0..dist` to the segment `(x1,y1)-(x2,y2)`.
    pub fn new_line_segment(x1: f64, y1: f64, x2: f64, y2: f64, dist: f64) -> Self {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let mut m = Self::new();
        if dist > 0.0 {
            m.multiply(&Self::new_scaling_uniform(
                (dx * dx + dy * dy).sqrt() / dist,
            ));
        }
        m.multiply(&Self::new_rotation(dy.atan2(dx)));
        m.multiply(&Self::new_translation(x1, y1));
        m
    }

    /// Reflection across a line through the origin at angle `a`.
    pub fn new_reflection(a: f64) -> Self {
        Self::new_reflection_unit(a.cos(), a.sin())
    }

    /// Reflection across a line through the origin containing the non-unit
    /// vector `(x, y)`.
    pub fn new_reflection_xy(x: f64, y: f64) -> Self {
        let d = (x * x + y * y).sqrt();
        Self::new_reflection_unit(x / d, y / d)
    }

    /// Reflection across a line through the origin containing unit vector
    /// `(ux, uy)`.
    pub fn new_reflection_unit(ux: f64, uy: f64) -> Self {
        Self::new_custom(
            2.0 * ux * ux - 1.0,
            2.0 * ux * uy,
            2.0 * ux * uy,
            2.0 * uy * uy - 1.0,
            0.0,
            0.0,
        )
    }

    // ====================================================================
    // Parallelogram transformations
    // ====================================================================

    /// Map one parallelogram to another.
    ///
    /// `src` and `dst` are `[x1,y1, x2,y2, x3,y3]` — three corners,
    /// the fourth is implicit.
    pub fn parl_to_parl(&mut self, src: &[f64; 6], dst: &[f64; 6]) -> &mut Self {
        self.sx = src[2] - src[0];
        self.shy = src[3] - src[1];
        self.shx = src[4] - src[0];
        self.sy = src[5] - src[1];
        self.tx = src[0];
        self.ty = src[1];
        self.invert();
        self.multiply(&TransAffine::new_custom(
            dst[2] - dst[0],
            dst[3] - dst[1],
            dst[4] - dst[0],
            dst[5] - dst[1],
            dst[0],
            dst[1],
        ));
        self
    }

    /// Map a rectangle to a parallelogram.
    pub fn rect_to_parl(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        parl: &[f64; 6],
    ) -> &mut Self {
        let src = [x1, y1, x2, y1, x2, y2];
        self.parl_to_parl(&src, parl);
        self
    }

    /// Map a parallelogram to a rectangle.
    pub fn parl_to_rect(
        &mut self,
        parl: &[f64; 6],
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> &mut Self {
        let dst = [x1, y1, x2, y1, x2, y2];
        self.parl_to_parl(parl, &dst);
        self
    }

    // ====================================================================
    // Operations (mutate self)
    // ====================================================================

    /// Reset to identity.
    pub fn reset(&mut self) -> &mut Self {
        self.sx = 1.0;
        self.sy = 1.0;
        self.shy = 0.0;
        self.shx = 0.0;
        self.tx = 0.0;
        self.ty = 0.0;
        self
    }

    /// Translate.
    pub fn translate(&mut self, x: f64, y: f64) -> &mut Self {
        self.tx += x;
        self.ty += y;
        self
    }

    /// Rotate by angle `a` (radians).
    pub fn rotate(&mut self, a: f64) -> &mut Self {
        let (sa, ca) = a.sin_cos();
        let t0 = self.sx * ca - self.shy * sa;
        let t2 = self.shx * ca - self.sy * sa;
        let t4 = self.tx * ca - self.ty * sa;
        self.shy = self.sx * sa + self.shy * ca;
        self.sy = self.shx * sa + self.sy * ca;
        self.ty = self.tx * sa + self.ty * ca;
        self.sx = t0;
        self.shx = t2;
        self.tx = t4;
        self
    }

    /// Non-uniform scale.
    pub fn scale(&mut self, x: f64, y: f64) -> &mut Self {
        self.sx *= x;
        self.shx *= x;
        self.tx *= x;
        self.shy *= y;
        self.sy *= y;
        self.ty *= y;
        self
    }

    /// Uniform scale.
    pub fn scale_uniform(&mut self, s: f64) -> &mut Self {
        self.scale(s, s)
    }

    /// Post-multiply: `self = self * m`.
    pub fn multiply(&mut self, m: &TransAffine) -> &mut Self {
        let t0 = self.sx * m.sx + self.shy * m.shx;
        let t2 = self.shx * m.sx + self.sy * m.shx;
        let t4 = self.tx * m.sx + self.ty * m.shx + m.tx;
        self.shy = self.sx * m.shy + self.shy * m.sy;
        self.sy = self.shx * m.shy + self.sy * m.sy;
        self.ty = self.tx * m.shy + self.ty * m.sy + m.ty;
        self.sx = t0;
        self.shx = t2;
        self.tx = t4;
        self
    }

    /// Pre-multiply: `self = m * self`.
    pub fn premultiply(&mut self, m: &TransAffine) -> &mut Self {
        let mut t = *m;
        t.multiply(self);
        *self = t;
        self
    }

    /// Post-multiply by inverse of `m`.
    pub fn multiply_inv(&mut self, m: &TransAffine) -> &mut Self {
        let mut t = *m;
        t.invert();
        self.multiply(&t);
        self
    }

    /// Pre-multiply by inverse of `m`.
    pub fn premultiply_inv(&mut self, m: &TransAffine) -> &mut Self {
        let mut t = *m;
        t.invert();
        t.multiply(self);
        *self = t;
        self
    }

    /// Invert the matrix in place.
    pub fn invert(&mut self) -> &mut Self {
        let d = self.determinant_reciprocal();
        let t0 = self.sy * d;
        self.sy = self.sx * d;
        self.shy = -self.shy * d;
        self.shx = -self.shx * d;
        let t4 = -self.tx * t0 - self.ty * self.shx;
        self.ty = -self.tx * self.shy - self.ty * self.sy;
        self.sx = t0;
        self.tx = t4;
        self
    }

    /// Mirror around X axis.
    pub fn flip_x(&mut self) -> &mut Self {
        self.sx = -self.sx;
        self.shy = -self.shy;
        self.tx = -self.tx;
        self
    }

    /// Mirror around Y axis.
    pub fn flip_y(&mut self) -> &mut Self {
        self.shx = -self.shx;
        self.sy = -self.sy;
        self.ty = -self.ty;
        self
    }

    // ====================================================================
    // Store / Load
    // ====================================================================

    /// Store to a `[6]` array.
    pub fn store_to(&self, m: &mut [f64; 6]) {
        m[0] = self.sx;
        m[1] = self.shy;
        m[2] = self.shx;
        m[3] = self.sy;
        m[4] = self.tx;
        m[5] = self.ty;
    }

    /// Load from a `[6]` array.
    pub fn load_from(&mut self, m: &[f64; 6]) -> &mut Self {
        self.sx = m[0];
        self.shy = m[1];
        self.shx = m[2];
        self.sy = m[3];
        self.tx = m[4];
        self.ty = m[5];
        self
    }

    // ====================================================================
    // Transformations
    // ====================================================================

    /// Forward transform: `(x, y) -> (x', y')`.
    #[inline]
    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        let tmp = *x;
        *x = tmp * self.sx + *y * self.shx + self.tx;
        *y = tmp * self.shy + *y * self.sy + self.ty;
    }

    /// Forward transform (2x2 only, no translation).
    #[inline]
    pub fn transform_2x2(&self, x: &mut f64, y: &mut f64) {
        let tmp = *x;
        *x = tmp * self.sx + *y * self.shx;
        *y = tmp * self.shy + *y * self.sy;
    }

    /// Inverse transform: `(x', y') -> (x, y)`.
    #[inline]
    pub fn inverse_transform(&self, x: &mut f64, y: &mut f64) {
        let d = self.determinant_reciprocal();
        let a = (*x - self.tx) * d;
        let b = (*y - self.ty) * d;
        *x = a * self.sy - b * self.shx;
        *y = b * self.sx - a * self.shy;
    }

    // ====================================================================
    // Auxiliary
    // ====================================================================

    /// Determinant of the 2x2 portion.
    #[inline]
    pub fn determinant(&self) -> f64 {
        self.sx * self.sy - self.shy * self.shx
    }

    /// Reciprocal of the determinant.
    #[inline]
    pub fn determinant_reciprocal(&self) -> f64 {
        1.0 / (self.sx * self.sy - self.shy * self.shx)
    }

    /// Average scale factor (useful for approximation_scale on curves).
    pub fn get_scale(&self) -> f64 {
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let x = s * self.sx + s * self.shx;
        let y = s * self.shy + s * self.sy;
        (x * x + y * y).sqrt()
    }

    /// Check if the matrix is non-degenerate.
    pub fn is_valid(&self, epsilon: f64) -> bool {
        self.sx.abs() > epsilon && self.sy.abs() > epsilon
    }

    /// Check if this is an identity matrix.
    pub fn is_identity(&self, epsilon: f64) -> bool {
        is_equal_eps(self.sx, 1.0, epsilon)
            && is_equal_eps(self.shy, 0.0, epsilon)
            && is_equal_eps(self.shx, 0.0, epsilon)
            && is_equal_eps(self.sy, 1.0, epsilon)
            && is_equal_eps(self.tx, 0.0, epsilon)
            && is_equal_eps(self.ty, 0.0, epsilon)
    }

    /// Check if two matrices are equal within epsilon.
    pub fn is_equal(&self, m: &TransAffine, epsilon: f64) -> bool {
        is_equal_eps(self.sx, m.sx, epsilon)
            && is_equal_eps(self.shy, m.shy, epsilon)
            && is_equal_eps(self.shx, m.shx, epsilon)
            && is_equal_eps(self.sy, m.sy, epsilon)
            && is_equal_eps(self.tx, m.tx, epsilon)
            && is_equal_eps(self.ty, m.ty, epsilon)
    }

    /// Extract the rotation angle.
    pub fn rotation(&self) -> f64 {
        let mut x1 = 0.0;
        let mut y1 = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 0.0;
        self.transform(&mut x1, &mut y1);
        self.transform(&mut x2, &mut y2);
        (y2 - y1).atan2(x2 - x1)
    }

    /// Extract the translation components.
    pub fn translation(&self) -> (f64, f64) {
        (self.tx, self.ty)
    }

    /// Extract scaling by removing rotation first.
    pub fn scaling(&self) -> (f64, f64) {
        let mut x1 = 0.0;
        let mut y1 = 0.0;
        let mut x2 = 1.0;
        let mut y2 = 1.0;
        let mut t = *self;
        t.multiply(&TransAffine::new_rotation(-self.rotation()));
        t.transform(&mut x1, &mut y1);
        t.transform(&mut x2, &mut y2);
        (x2 - x1, y2 - y1)
    }

    /// Absolute scaling (from matrix magnitudes).
    pub fn scaling_abs(&self) -> (f64, f64) {
        (
            (self.sx * self.sx + self.shx * self.shx).sqrt(),
            (self.shy * self.shy + self.sy * self.sy).sqrt(),
        )
    }
}

impl Default for TransAffine {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for TransAffine {
    fn eq(&self, other: &Self) -> bool {
        self.is_equal(other, AFFINE_EPSILON)
    }
}

impl std::ops::Mul for TransAffine {
    type Output = TransAffine;
    fn mul(self, rhs: TransAffine) -> TransAffine {
        let mut result = self;
        result.multiply(&rhs);
        result
    }
}

impl std::ops::MulAssign for TransAffine {
    fn mul_assign(&mut self, rhs: TransAffine) {
        self.multiply(&rhs);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const EPS: f64 = 1e-10;

    #[test]
    fn test_identity() {
        let m = TransAffine::new();
        assert!(m.is_identity(AFFINE_EPSILON));
        assert_eq!(m.determinant(), 1.0);
    }

    #[test]
    fn test_translation() {
        let m = TransAffine::new_translation(10.0, 20.0);
        let mut x = 5.0;
        let mut y = 3.0;
        m.transform(&mut x, &mut y);
        assert!((x - 15.0).abs() < EPS);
        assert!((y - 23.0).abs() < EPS);
    }

    #[test]
    fn test_scaling() {
        let m = TransAffine::new_scaling(2.0, 3.0);
        let mut x = 5.0;
        let mut y = 4.0;
        m.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < EPS);
        assert!((y - 12.0).abs() < EPS);
    }

    #[test]
    fn test_uniform_scaling() {
        let m = TransAffine::new_scaling_uniform(5.0);
        let mut x = 2.0;
        let mut y = 3.0;
        m.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < EPS);
        assert!((y - 15.0).abs() < EPS);
    }

    #[test]
    fn test_rotation_90() {
        let m = TransAffine::new_rotation(PI / 2.0);
        let mut x = 1.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!(x.abs() < EPS);
        assert!((y - 1.0).abs() < EPS);
    }

    #[test]
    fn test_rotation_180() {
        let m = TransAffine::new_rotation(PI);
        let mut x = 1.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x + 1.0).abs() < EPS);
        assert!(y.abs() < EPS);
    }

    #[test]
    fn test_multiply_translate_then_scale() {
        // First translate, then scale
        let mut m = TransAffine::new_translation(10.0, 0.0);
        m.multiply(&TransAffine::new_scaling(2.0, 2.0));
        let mut x = 0.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x - 20.0).abs() < EPS); // (0+10)*2
        assert!(y.abs() < EPS);
    }

    #[test]
    fn test_multiply_scale_then_translate() {
        let mut m = TransAffine::new_scaling(2.0, 2.0);
        m.multiply(&TransAffine::new_translation(10.0, 0.0));
        let mut x = 5.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x - 20.0).abs() < EPS); // 5*2 + 10
    }

    #[test]
    fn test_invert() {
        let mut m = TransAffine::new_scaling(2.0, 3.0);
        m.multiply(&TransAffine::new_translation(10.0, 20.0));

        let mut inv = m;
        inv.invert();

        // m * inv should be identity
        let result = m * inv;
        assert!(result.is_identity(1e-10));
    }

    #[test]
    fn test_inverse_transform() {
        let m = TransAffine::new_scaling(2.0, 3.0);
        let mut x = 5.0;
        let mut y = 4.0;
        m.transform(&mut x, &mut y);
        m.inverse_transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < EPS);
        assert!((y - 4.0).abs() < EPS);
    }

    #[test]
    fn test_transform_2x2() {
        let m = TransAffine::new_custom(2.0, 0.0, 0.0, 3.0, 100.0, 200.0);
        let mut x = 5.0;
        let mut y = 4.0;
        m.transform_2x2(&mut x, &mut y);
        // Should NOT include translation
        assert!((x - 10.0).abs() < EPS);
        assert!((y - 12.0).abs() < EPS);
    }

    #[test]
    fn test_premultiply() {
        let s = TransAffine::new_scaling(2.0, 2.0);
        let t = TransAffine::new_translation(10.0, 0.0);

        // premultiply(s) means self = s * self
        let mut m = t;
        m.premultiply(&s);

        // Result: scale first, then translate
        // s * t: for point (5,0) -> scale(5,0)=(10,0) -> translate -> (20,0)
        let mut x = 5.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x - 20.0).abs() < EPS);
    }

    #[test]
    fn test_flip_x() {
        let mut m = TransAffine::new();
        m.flip_x();
        let mut x = 5.0;
        let mut y = 3.0;
        m.transform(&mut x, &mut y);
        assert!((x + 5.0).abs() < EPS);
        assert!((y - 3.0).abs() < EPS);
    }

    #[test]
    fn test_flip_y() {
        let mut m = TransAffine::new();
        m.flip_y();
        let mut x = 5.0;
        let mut y = 3.0;
        m.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < EPS);
        assert!((y + 3.0).abs() < EPS);
    }

    #[test]
    fn test_reset() {
        let mut m = TransAffine::new_scaling(2.0, 3.0);
        m.reset();
        assert!(m.is_identity(AFFINE_EPSILON));
    }

    #[test]
    fn test_store_load() {
        let m = TransAffine::new_custom(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let mut arr = [0.0; 6];
        m.store_to(&mut arr);
        assert_eq!(arr, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let m2 = TransAffine::from_array(&arr);
        assert!(m.is_equal(&m2, EPS));
    }

    #[test]
    fn test_get_scale() {
        let m = TransAffine::new_scaling_uniform(3.0);
        assert!((m.get_scale() - 3.0).abs() < EPS);
    }

    #[test]
    fn test_is_valid() {
        let m = TransAffine::new();
        assert!(m.is_valid(AFFINE_EPSILON));

        let m2 = TransAffine::new_custom(0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        assert!(!m2.is_valid(AFFINE_EPSILON));
    }

    #[test]
    fn test_rotation_extraction() {
        let m = TransAffine::new_rotation(PI / 4.0);
        assert!((m.rotation() - PI / 4.0).abs() < EPS);
    }

    #[test]
    fn test_translation_extraction() {
        let m = TransAffine::new_translation(42.0, 17.0);
        let (dx, dy) = m.translation();
        assert!((dx - 42.0).abs() < EPS);
        assert!((dy - 17.0).abs() < EPS);
    }

    #[test]
    fn test_scaling_extraction() {
        let m = TransAffine::new_scaling(3.0, 5.0);
        let (sx, sy) = m.scaling();
        assert!((sx - 3.0).abs() < EPS);
        assert!((sy - 5.0).abs() < EPS);
    }

    #[test]
    fn test_scaling_abs() {
        let m = TransAffine::new_scaling(3.0, 5.0);
        let (ax, ay) = m.scaling_abs();
        assert!((ax - 3.0).abs() < EPS);
        assert!((ay - 5.0).abs() < EPS);
    }

    #[test]
    fn test_skewing() {
        let m = TransAffine::new_skewing(PI / 4.0, 0.0);
        let mut x = 0.0;
        let mut y = 1.0;
        m.transform(&mut x, &mut y);
        // shx = tan(PI/4) = 1.0, so x = 0*1 + 1*1 = 1
        assert!((x - 1.0).abs() < EPS);
        assert!((y - 1.0).abs() < EPS);
    }

    #[test]
    fn test_operator_mul() {
        let a = TransAffine::new_translation(10.0, 0.0);
        let b = TransAffine::new_scaling(2.0, 2.0);
        let c = a * b;

        let mut x = 1.0;
        let mut y = 0.0;
        c.transform(&mut x, &mut y);
        // a then b: (1+10)*2 = 22
        assert!((x - 22.0).abs() < EPS);
    }

    #[test]
    fn test_operator_mul_assign() {
        let mut m = TransAffine::new_translation(10.0, 0.0);
        m *= TransAffine::new_scaling(2.0, 2.0);

        let mut x = 1.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x - 22.0).abs() < EPS);
    }

    #[test]
    fn test_partial_eq() {
        let a = TransAffine::new_translation(10.0, 20.0);
        let b = TransAffine::new_translation(10.0, 20.0);
        assert_eq!(a, b);

        let c = TransAffine::new_translation(10.0, 21.0);
        assert_ne!(a, c);
    }

    #[test]
    fn test_determinant() {
        let m = TransAffine::new_scaling(2.0, 3.0);
        assert!((m.determinant() - 6.0).abs() < EPS);
    }

    #[test]
    fn test_combined_transform() {
        // Rotate 90, then translate (10, 0)
        let mut m = TransAffine::new();
        m.rotate(PI / 2.0);
        m.translate(10.0, 0.0);

        let mut x = 1.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        // rotate 90: (1,0) -> (0,1), then translate: (10, 1)
        assert!((x - 10.0).abs() < EPS);
        assert!((y - 1.0).abs() < EPS);
    }

    #[test]
    fn test_chain_methods() {
        let mut m = TransAffine::new();
        m.scale(2.0, 2.0);
        m.translate(5.0, 0.0);

        let mut x = 3.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        // scale: (3*2, 0) = (6, 0), then translate: (6+5, 0) = (11, 0)
        assert!((x - 11.0).abs() < EPS);
    }

    #[test]
    fn test_parl_to_parl() {
        // Identity: same parallelogram
        let src = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0];
        let dst = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0];
        let mut m = TransAffine::new();
        m.parl_to_parl(&src, &dst);
        assert!(m.is_identity(1e-10));
    }

    #[test]
    fn test_rect_to_parl() {
        let parl = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0];
        let mut m = TransAffine::new();
        m.rect_to_parl(0.0, 0.0, 1.0, 1.0, &parl);

        let mut x = 0.5;
        let mut y = 0.5;
        m.transform(&mut x, &mut y);
        assert!((x - 1.0).abs() < EPS);
        assert!((y - 1.0).abs() < EPS);
    }

    #[test]
    fn test_reflection() {
        // Reflect across X axis (angle 0)
        let m = TransAffine::new_reflection(0.0);
        let mut x = 1.0;
        let mut y = 1.0;
        m.transform(&mut x, &mut y);
        assert!((x - 1.0).abs() < EPS);
        assert!((y + 1.0).abs() < EPS);
    }

    #[test]
    fn test_line_segment() {
        let m = TransAffine::new_line_segment(0.0, 0.0, 10.0, 0.0, 10.0);
        let mut x = 5.0;
        let mut y = 0.0;
        m.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < EPS);
        assert!(y.abs() < EPS);
    }

    #[test]
    fn test_multiply_inv() {
        let m = TransAffine::new_scaling(2.0, 3.0);
        let mut result = TransAffine::new_scaling(2.0, 3.0);
        result.multiply_inv(&m);
        assert!(result.is_identity(1e-10));
    }

    #[test]
    fn test_default_trait() {
        let m: TransAffine = Default::default();
        assert!(m.is_identity(AFFINE_EPSILON));
    }
}

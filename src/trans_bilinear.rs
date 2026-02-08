//! Bilinear 2D transformation.
//!
//! Port of `agg_trans_bilinear.h` — maps arbitrary quadrilaterals using a
//! bilinear transformation (includes an x*y term, unlike affine).

use crate::simul_eq::simul_eq_solve;

// ============================================================================
// TransBilinear
// ============================================================================

/// Bilinear 2D transformation.
///
/// Solves for a 4x2 coefficient matrix that maps points between
/// two quadrilaterals using bilinear interpolation:
///
/// ```text
/// x' = m[0][0] + m[1][0]*x*y + m[2][0]*x + m[3][0]*y
/// y' = m[0][1] + m[1][1]*x*y + m[2][1]*x + m[3][1]*y
/// ```
///
/// Port of C++ `trans_bilinear`.
pub struct TransBilinear {
    mtx: [[f64; 2]; 4],
    valid: bool,
}

impl TransBilinear {
    /// Create an invalid (uninitialized) transform.
    pub fn new() -> Self {
        Self {
            mtx: [[0.0; 2]; 4],
            valid: false,
        }
    }

    /// Create from arbitrary quadrilateral to quadrilateral mapping.
    pub fn new_quad_to_quad(src: &[f64; 8], dst: &[f64; 8]) -> Self {
        let mut t = Self::new();
        t.quad_to_quad(src, dst);
        t
    }

    /// Create a rectangle → quadrilateral mapping.
    pub fn new_rect_to_quad(x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) -> Self {
        let mut t = Self::new();
        t.rect_to_quad(x1, y1, x2, y2, quad);
        t
    }

    /// Create a quadrilateral → rectangle mapping.
    pub fn new_quad_to_rect(quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut t = Self::new();
        t.quad_to_rect(quad, x1, y1, x2, y2);
        t
    }

    /// Set the transformation from two arbitrary quadrilaterals.
    ///
    /// Solves a 4×2 system via Gaussian elimination.
    pub fn quad_to_quad(&mut self, src: &[f64; 8], dst: &[f64; 8]) {
        let mut left = [[0.0_f64; 4]; 4];
        let mut right = [[0.0_f64; 2]; 4];

        for i in 0..4 {
            let ix = i * 2;
            let iy = ix + 1;
            left[i][0] = 1.0;
            left[i][1] = src[ix] * src[iy];
            left[i][2] = src[ix];
            left[i][3] = src[iy];

            right[i][0] = dst[ix];
            right[i][1] = dst[iy];
        }

        self.valid = simul_eq_solve(&left, &right, &mut self.mtx);
    }

    /// Set the direct transformation: rectangle → quadrilateral.
    pub fn rect_to_quad(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) {
        let src = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(&src, quad);
    }

    /// Set the reverse transformation: quadrilateral → rectangle.
    pub fn quad_to_rect(&mut self, quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) {
        let dst = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(quad, &dst);
    }

    /// Check if the equations were solved successfully.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Transform a point (x, y).
    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        let tx = *x;
        let ty = *y;
        let xy = tx * ty;
        *x = self.mtx[0][0] + self.mtx[1][0] * xy + self.mtx[2][0] * tx + self.mtx[3][0] * ty;
        *y = self.mtx[0][1] + self.mtx[1][1] * xy + self.mtx[2][1] * tx + self.mtx[3][1] * ty;
    }

    /// Create an incremental iterator for scanline walking.
    pub fn begin(&self, x: f64, y: f64, step: f64) -> IteratorX {
        IteratorX::new(x, y, step, &self.mtx)
    }
}

impl Default for TransBilinear {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// IteratorX — incremental scanline walker
// ============================================================================

/// Incremental iterator for scanline walking with bilinear transform.
///
/// Port of C++ `trans_bilinear::iterator_x`.
pub struct IteratorX {
    inc_x: f64,
    inc_y: f64,
    pub x: f64,
    pub y: f64,
}

impl IteratorX {
    fn new(tx: f64, ty: f64, step: f64, m: &[[f64; 2]; 4]) -> Self {
        Self {
            inc_x: m[1][0] * step * ty + m[2][0] * step,
            inc_y: m[1][1] * step * ty + m[2][1] * step,
            x: m[0][0] + m[1][0] * tx * ty + m[2][0] * tx + m[3][0] * ty,
            y: m[0][1] + m[1][1] * tx * ty + m[2][1] * tx + m[3][1] * ty,
        }
    }

    /// Advance to the next step.
    pub fn next(&mut self) {
        self.x += self.inc_x;
        self.y += self.inc_y;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_rect() {
        // Rectangle to same rectangle → identity transform
        let src = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let dst = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let t = TransBilinear::new_quad_to_quad(&src, &dst);
        assert!(t.is_valid());
        let mut x = 0.5;
        let mut y = 0.5;
        t.transform(&mut x, &mut y);
        assert!((x - 0.5).abs() < 1e-10);
        assert!((y - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_rect_to_quad_corners() {
        // Map unit square to a specific quad
        let quad = [10.0, 10.0, 20.0, 10.0, 20.0, 20.0, 10.0, 20.0];
        let t = TransBilinear::new_rect_to_quad(0.0, 0.0, 1.0, 1.0, &quad);
        assert!(t.is_valid());

        // Corner (0,0) → (10,10)
        let mut x = 0.0;
        let mut y = 0.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);

        // Corner (1,1) → (20,20)
        x = 1.0;
        y = 1.0;
        t.transform(&mut x, &mut y);
        assert!((x - 20.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_quad_to_rect() {
        // Reverse: quad → unit square
        let quad = [10.0, 10.0, 20.0, 10.0, 20.0, 20.0, 10.0, 20.0];
        let t = TransBilinear::new_quad_to_rect(&quad, 0.0, 0.0, 1.0, 1.0);
        assert!(t.is_valid());

        let mut x = 10.0;
        let mut y = 10.0;
        t.transform(&mut x, &mut y);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_round_trip_parallelogram() {
        // Parallelogram (affine case): round-trip should be exact
        let quad = [5.0, 0.0, 15.0, 2.0, 17.0, 12.0, 7.0, 10.0];
        let forward = TransBilinear::new_rect_to_quad(0.0, 0.0, 10.0, 10.0, &quad);
        let reverse = TransBilinear::new_quad_to_rect(&quad, 0.0, 0.0, 10.0, 10.0);
        assert!(forward.is_valid());
        assert!(reverse.is_valid());

        let mut x = 5.0;
        let mut y = 5.0;
        forward.transform(&mut x, &mut y);
        reverse.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < 1e-8);
        assert!((y - 5.0).abs() < 1e-8);
    }

    #[test]
    fn test_iterator() {
        let src = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        let dst = [0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0];
        let t = TransBilinear::new_quad_to_quad(&src, &dst);

        let mut it = t.begin(0.0, 0.0, 0.5);
        assert!((it.x - 0.0).abs() < 1e-10);
        assert!((it.y - 0.0).abs() < 1e-10);
        it.next();
        assert!((it.x - 5.0).abs() < 1e-10);
        assert!((it.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_default_is_invalid() {
        let t = TransBilinear::new();
        assert!(!t.is_valid());
    }

    #[test]
    fn test_scaling_transform() {
        // Map [0,0,1,1] → [0,0,2,2] — should scale by 2
        let quad = [0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0];
        let t = TransBilinear::new_rect_to_quad(0.0, 0.0, 1.0, 1.0, &quad);
        assert!(t.is_valid());

        let mut x = 0.5;
        let mut y = 0.5;
        t.transform(&mut x, &mut y);
        assert!((x - 1.0).abs() < 1e-10);
        assert!((y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_translation_transform() {
        // Map [0,0,1,1] → [10,20,11,21] — should translate by (10,20)
        let quad = [10.0, 20.0, 11.0, 20.0, 11.0, 21.0, 10.0, 21.0];
        let t = TransBilinear::new_rect_to_quad(0.0, 0.0, 1.0, 1.0, &quad);
        assert!(t.is_valid());

        let mut x = 0.0;
        let mut y = 0.0;
        t.transform(&mut x, &mut y);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);
    }
}

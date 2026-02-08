//! Per-pixel transform span interpolator.
//!
//! Port of `agg_span_interpolator_trans.h` — transforms every pixel individually
//! through an arbitrary transformer. Unlike the linear interpolator which transforms
//! only endpoints and interpolates, this calls `transform()` for each pixel.

use crate::basics::iround;
use crate::span_interpolator_linear::Transformer;

/// Subpixel precision constants (matching span_interpolator_linear).
const SUBPIXEL_SHIFT: u32 = 8;
const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;

// ============================================================================
// SpanInterpolatorTrans
// ============================================================================

/// Per-pixel transform span interpolator.
///
/// Transforms every single pixel through the given transformer. This is more
/// accurate than linear interpolation but significantly slower since `transform()`
/// is called for every pixel rather than just the span endpoints.
///
/// Port of C++ `span_interpolator_trans<Transformer, SubpixelShift>`.
pub struct SpanInterpolatorTrans<T: Transformer> {
    trans: T,
    x: f64,
    y: f64,
    ix: i32,
    iy: i32,
}

impl<T: Transformer> SpanInterpolatorTrans<T> {
    pub fn new(trans: T) -> Self {
        Self {
            trans,
            x: 0.0,
            y: 0.0,
            ix: 0,
            iy: 0,
        }
    }

    pub fn new_begin(trans: T, x: f64, y: f64, len: u32) -> Self {
        let mut s = Self::new(trans);
        s.begin(x, y, len);
        s
    }

    pub fn transformer(&self) -> &T {
        &self.trans
    }

    pub fn set_transformer(&mut self, trans: T) {
        self.trans = trans;
    }

    /// Initialize interpolation for a span starting at (x, y).
    pub fn begin(&mut self, x: f64, y: f64, _len: u32) {
        self.x = x;
        self.y = y;
        let mut tx = x;
        let mut ty = y;
        self.trans.transform(&mut tx, &mut ty);
        self.ix = iround(tx * SUBPIXEL_SCALE as f64);
        self.iy = iround(ty * SUBPIXEL_SCALE as f64);
    }

    /// Advance to the next pixel.
    #[inline]
    pub fn next(&mut self) {
        self.x += 1.0;
        let mut tx = self.x;
        let mut ty = self.y;
        self.trans.transform(&mut tx, &mut ty);
        self.ix = iround(tx * SUBPIXEL_SCALE as f64);
        self.iy = iround(ty * SUBPIXEL_SCALE as f64);
    }

    /// Get the current transformed coordinates (in subpixel units).
    #[inline]
    pub fn coordinates(&self, x: &mut i32, y: &mut i32) {
        *x = self.ix;
        *y = self.iy;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trans_affine::TransAffine;

    #[test]
    fn test_identity_transform() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorTrans::new(trans);
        interp.begin(10.0, 20.0, 5);

        let mut x = 0i32;
        let mut y = 0i32;
        interp.coordinates(&mut x, &mut y);
        // Identity: (10, 20) → (10*256, 20*256) = (2560, 5120)
        assert_eq!(x, 2560);
        assert_eq!(y, 5120);
    }

    #[test]
    fn test_next_increments_x() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorTrans::new(trans);
        interp.begin(5.0, 3.0, 10);

        let mut x = 0i32;
        let mut y = 0i32;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 5 * 256);
        assert_eq!(y, 3 * 256);

        interp.next();
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 6 * 256);
        assert_eq!(y, 3 * 256);
    }

    #[test]
    fn test_with_translation() {
        let trans = TransAffine::new_translation(10.0, 20.0);
        let mut interp = SpanInterpolatorTrans::new(trans);
        interp.begin(0.0, 0.0, 1);

        let mut x = 0i32;
        let mut y = 0i32;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 10 * 256);
        assert_eq!(y, 20 * 256);
    }

    #[test]
    fn test_with_scaling() {
        let trans = TransAffine::new_scaling(2.0, 3.0);
        let mut interp = SpanInterpolatorTrans::new(trans);
        interp.begin(5.0, 4.0, 1);

        let mut x = 0i32;
        let mut y = 0i32;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 10 * 256);
        assert_eq!(y, 12 * 256);
    }

    #[test]
    fn test_new_begin() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorTrans::new_begin(trans, 1.0, 2.0, 5);
        let mut x = 0i32;
        let mut y = 0i32;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 256);
        assert_eq!(y, 512);
    }
}

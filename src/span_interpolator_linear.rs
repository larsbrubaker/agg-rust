//! Linear span interpolator.
//!
//! Port of `agg_span_interpolator_linear.h` — transforms span coordinates
//! through an affine (or other) transformation using DDA interpolation.

use crate::basics::iround;
use crate::dda_line::Dda2LineInterpolator;
use crate::trans_affine::TransAffine;
use crate::trans_bilinear::TransBilinear;
use crate::trans_perspective::TransPerspective;

/// Subpixel precision constants for span interpolation.
pub const SUBPIXEL_SHIFT: u32 = 8;
pub const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;

// ============================================================================
// Transformer trait
// ============================================================================

/// Trait for coordinate transformers used by span interpolators.
pub trait Transformer {
    fn transform(&self, x: &mut f64, y: &mut f64);
}

impl Transformer for TransAffine {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        self.transform(x, y);
    }
}

impl Transformer for TransBilinear {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        self.transform(x, y);
    }
}

impl Transformer for TransPerspective {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        self.transform(x, y);
    }
}

// ============================================================================
// SpanInterpolator trait
// ============================================================================

/// Trait for span interpolators used by image filter span generators.
///
/// All span interpolators must provide `begin`, `next`, and `coordinates`
/// methods. This mirrors the C++ template interface used by span generators.
pub trait SpanInterpolator {
    fn begin(&mut self, x: f64, y: f64, len: u32);
    fn next(&mut self);
    fn coordinates(&self, x: &mut i32, y: &mut i32);
}

// ============================================================================
// SpanInterpolatorLinear
// ============================================================================

/// Linear span interpolator.
///
/// Transforms span coordinates through a transformation using two DDA
/// interpolators (one for X, one for Y). Transforms only the endpoints
/// and linearly interpolates between them.
///
/// Port of C++ `span_interpolator_linear<Transformer, SubpixelShift>`.
pub struct SpanInterpolatorLinear<T = TransAffine> {
    trans: T,
    li_x: Dda2LineInterpolator,
    li_y: Dda2LineInterpolator,
}

impl<T: Transformer> SpanInterpolatorLinear<T> {
    pub fn new(trans: T) -> Self {
        Self {
            trans,
            li_x: Dda2LineInterpolator::new_forward(0, 0, 1),
            li_y: Dda2LineInterpolator::new_forward(0, 0, 1),
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

    /// Initialize interpolation for a span starting at (x, y) with `len` pixels.
    pub fn begin(&mut self, x: f64, y: f64, len: u32) {
        let mut tx = x;
        let mut ty = y;
        self.trans.transform(&mut tx, &mut ty);
        let x1 = iround(tx * SUBPIXEL_SCALE as f64);
        let y1 = iround(ty * SUBPIXEL_SCALE as f64);

        let mut tx2 = x + len as f64;
        let mut ty2 = y;
        self.trans.transform(&mut tx2, &mut ty2);
        let x2 = iround(tx2 * SUBPIXEL_SCALE as f64);
        let y2 = iround(ty2 * SUBPIXEL_SCALE as f64);

        self.li_x = Dda2LineInterpolator::new_forward(x1, x2, len as i32);
        self.li_y = Dda2LineInterpolator::new_forward(y1, y2, len as i32);
    }

    /// Re-synchronize interpolation at a new endpoint.
    pub fn resynchronize(&mut self, mut xe: f64, mut ye: f64, len: u32) {
        self.trans.transform(&mut xe, &mut ye);
        self.li_x = Dda2LineInterpolator::new_forward(
            self.li_x.y(),
            iround(xe * SUBPIXEL_SCALE as f64),
            len as i32,
        );
        self.li_y = Dda2LineInterpolator::new_forward(
            self.li_y.y(),
            iround(ye * SUBPIXEL_SCALE as f64),
            len as i32,
        );
    }

    /// Advance to the next pixel.
    #[inline]
    pub fn next(&mut self) {
        self.li_x.inc();
        self.li_y.inc();
    }

    /// Get the current transformed coordinates (in subpixel units).
    #[inline]
    pub fn coordinates(&self, x: &mut i32, y: &mut i32) {
        *x = self.li_x.y();
        *y = self.li_y.y();
    }
}

impl<T: Transformer> SpanInterpolator for SpanInterpolatorLinear<T> {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.begin(x, y, len);
    }
    fn next(&mut self) {
        self.next();
    }
    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        self.coordinates(x, y);
    }
}

// ============================================================================
// SpanInterpolatorLinearSubdiv
// ============================================================================

/// Linear span interpolator with subdivision.
///
/// Periodically re-transforms coordinates to reduce error accumulation
/// for non-linear transforms used as linear approximations.
///
/// Port of C++ `span_interpolator_linear_subdiv<Transformer, SubpixelShift>`.
pub struct SpanInterpolatorLinearSubdiv<T = TransAffine> {
    subdiv_shift: u32,
    subdiv_size: u32,
    trans: T,
    li_x: Dda2LineInterpolator,
    li_y: Dda2LineInterpolator,
    src_x: i32,
    src_y: f64,
    pos: u32,
    len: u32,
}

impl<T: Transformer> SpanInterpolatorLinearSubdiv<T> {
    pub fn new(trans: T, subdiv_shift: u32) -> Self {
        Self {
            subdiv_shift,
            subdiv_size: 1 << subdiv_shift,
            trans,
            li_x: Dda2LineInterpolator::new_forward(0, 0, 1),
            li_y: Dda2LineInterpolator::new_forward(0, 0, 1),
            src_x: 0,
            src_y: 0.0,
            pos: 1,
            len: 0,
        }
    }

    pub fn new_default(trans: T) -> Self {
        Self::new(trans, 4)
    }

    pub fn transformer(&self) -> &T {
        &self.trans
    }

    pub fn set_transformer(&mut self, trans: T) {
        self.trans = trans;
    }

    pub fn subdiv_shift(&self) -> u32 {
        self.subdiv_shift
    }

    pub fn set_subdiv_shift(&mut self, shift: u32) {
        self.subdiv_shift = shift;
        self.subdiv_size = 1 << shift;
    }

    /// Initialize interpolation for a span starting at (x, y) with `len` pixels.
    pub fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.pos = 1;
        self.src_x = iround(x * SUBPIXEL_SCALE as f64) + SUBPIXEL_SCALE;
        self.src_y = y;
        self.len = len;

        let sub_len = if len > self.subdiv_size {
            self.subdiv_size
        } else {
            len
        };

        let mut tx = x;
        let mut ty = y;
        self.trans.transform(&mut tx, &mut ty);
        let x1 = iround(tx * SUBPIXEL_SCALE as f64);
        let y1 = iround(ty * SUBPIXEL_SCALE as f64);

        let mut tx2 = x + sub_len as f64;
        let mut ty2 = y;
        self.trans.transform(&mut tx2, &mut ty2);

        self.li_x = Dda2LineInterpolator::new_forward(
            x1,
            iround(tx2 * SUBPIXEL_SCALE as f64),
            sub_len as i32,
        );
        self.li_y = Dda2LineInterpolator::new_forward(
            y1,
            iround(ty2 * SUBPIXEL_SCALE as f64),
            sub_len as i32,
        );
    }

    /// Advance to the next pixel.
    pub fn next(&mut self) {
        self.li_x.inc();
        self.li_y.inc();
        if self.pos >= self.subdiv_size {
            let mut sub_len = self.len;
            if sub_len > self.subdiv_size {
                sub_len = self.subdiv_size;
            }
            let mut tx = self.src_x as f64 / SUBPIXEL_SCALE as f64 + sub_len as f64;
            let mut ty = self.src_y;
            self.trans.transform(&mut tx, &mut ty);
            self.li_x = Dda2LineInterpolator::new_forward(
                self.li_x.y(),
                iround(tx * SUBPIXEL_SCALE as f64),
                sub_len as i32,
            );
            self.li_y = Dda2LineInterpolator::new_forward(
                self.li_y.y(),
                iround(ty * SUBPIXEL_SCALE as f64),
                sub_len as i32,
            );
            self.pos = 0;
        }
        self.src_x += SUBPIXEL_SCALE;
        self.pos += 1;
        self.len = self.len.saturating_sub(1);
    }

    /// Get the current transformed coordinates (in subpixel units).
    #[inline]
    pub fn coordinates(&self, x: &mut i32, y: &mut i32) {
        *x = self.li_x.y();
        *y = self.li_y.y();
    }
}

impl<T: Transformer> SpanInterpolator for SpanInterpolatorLinearSubdiv<T> {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.begin(x, y, len);
    }
    fn next(&mut self) {
        self.next();
    }
    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        self.coordinates(x, y);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_identity_transform() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(10.0, 20.0, 5);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 10 * SUBPIXEL_SCALE);
        assert_eq!(y, 20 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_translation() {
        let trans = TransAffine::new_translation(100.0, 200.0);
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(10.0, 20.0, 5);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 110 * SUBPIXEL_SCALE);
        assert_eq!(y, 220 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_scaling() {
        let trans = TransAffine::new_scaling(2.0, 3.0);
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(10.0, 20.0, 5);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 20 * SUBPIXEL_SCALE);
        assert_eq!(y, 60 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_interpolation_advances() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(0.0, 0.0, 10);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 0);

        // Advance to end of span
        for _ in 0..10 {
            interp.next();
        }
        interp.coordinates(&mut x, &mut y);
        // Should be at approximately 10 * SUBPIXEL_SCALE
        assert!((x - 10 * SUBPIXEL_SCALE).abs() <= 1);
    }

    #[test]
    fn test_rotation_90_degrees() {
        let trans = TransAffine::new_rotation(PI / 2.0);
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(10.0, 0.0, 1);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        // After 90° rotation, (10, 0) → (0, 10)
        assert!(x.abs() <= 1, "x={} should be ~0", x);
        assert!(
            (y - 10 * SUBPIXEL_SCALE).abs() <= 1,
            "y={} should be ~{}",
            y,
            10 * SUBPIXEL_SCALE
        );
    }

    #[test]
    fn test_resynchronize() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        interp.begin(0.0, 0.0, 10);
        for _ in 0..5 {
            interp.next();
        }
        // Resync at new endpoint
        interp.resynchronize(15.0, 0.0, 5);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        // Should be at ~5 * SUBPIXEL_SCALE
        assert!(
            (x - 5 * SUBPIXEL_SCALE).abs() <= 1,
            "x={} should be ~{}",
            x,
            5 * SUBPIXEL_SCALE
        );
    }

    #[test]
    fn test_new_begin() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new_begin(trans, 5.0, 10.0, 3);
        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 5 * SUBPIXEL_SCALE);
        assert_eq!(y, 10 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_subdiv_identity() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinearSubdiv::new_default(trans);
        interp.begin(10.0, 20.0, 100);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 10 * SUBPIXEL_SCALE);
        assert_eq!(y, 20 * SUBPIXEL_SCALE);

        // Advance many steps — should stay on track
        for _ in 0..50 {
            interp.next();
        }
        interp.coordinates(&mut x, &mut y);
        assert!(
            (x - 60 * SUBPIXEL_SCALE).abs() <= 2,
            "x={} should be ~{}",
            x,
            60 * SUBPIXEL_SCALE
        );
    }

    #[test]
    fn test_subdiv_translation() {
        let trans = TransAffine::new_translation(50.0, 100.0);
        let mut interp = SpanInterpolatorLinearSubdiv::new_default(trans);
        interp.begin(0.0, 0.0, 20);

        let mut x = 0;
        let mut y = 0;
        interp.coordinates(&mut x, &mut y);
        assert_eq!(x, 50 * SUBPIXEL_SCALE);
        assert_eq!(y, 100 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_subdiv_shift_setter() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinearSubdiv::new(trans, 4);
        assert_eq!(interp.subdiv_shift(), 4);
        interp.set_subdiv_shift(6);
        assert_eq!(interp.subdiv_shift(), 6);
    }
}

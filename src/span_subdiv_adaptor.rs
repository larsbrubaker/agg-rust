//! Span subdivision adaptor.
//!
//! Port of `agg_span_subdiv_adaptor.h` — wraps any span interpolator and
//! periodically re-synchronizes it to correct accumulated error when using
//! linear approximation for non-linear transforms.

use core::marker::PhantomData;

use crate::basics::iround;
use crate::span_interpolator_linear::{
    SpanInterpolator, SpanInterpolatorLinear, SpanInterpolatorLocalScale, Transformer,
    SUBPIXEL_SCALE,
};
use crate::span_interpolator_persp::SpanInterpolatorPerspLerp;
use crate::trans_perspective::TransPerspective;

// ============================================================================
// SubdivInterpolator
// ============================================================================

/// An interpolator `SpanSubdivAdaptor` can drive: C++'s template interface
/// for `span_subdiv_adaptor<Interpolator>`'s interpolator, `begin`, `++`,
/// `coordinates` (from `SpanInterpolator`) plus `resynchronize`.
///
/// `Trans` is the interpolator's transformer type (C++'s `trans_type`); it is
/// what names the adaptor's first type parameter, so
/// `SpanSubdivAdaptor<TransAffine>` stays the adaptor over
/// `SpanInterpolatorLinear<TransAffine>`.
pub trait SubdivInterpolator: SpanInterpolator {
    type Trans;

    /// Restart the interpolation from the current point to the transformed
    /// (`xe`, `ye`) over `len` pixels.
    fn resynchronize(&mut self, xe: f64, ye: f64, len: u32);
}

impl<T: Transformer> SubdivInterpolator for SpanInterpolatorLinear<T> {
    type Trans = T;

    fn resynchronize(&mut self, xe: f64, ye: f64, len: u32) {
        SpanInterpolatorLinear::resynchronize(self, xe, ye, len);
    }
}

impl SubdivInterpolator for SpanInterpolatorPerspLerp {
    type Trans = TransPerspective;

    fn resynchronize(&mut self, xe: f64, ye: f64, len: u32) {
        SpanInterpolatorPerspLerp::resynchronize(self, xe, ye, len);
    }
}

// ============================================================================
// SpanSubdivAdaptor
// ============================================================================

/// Span subdivision adaptor.
///
/// Breaks long spans into sub-spans of `subdiv_size` pixels, calling
/// `resynchronize()` on the inner interpolator at each break point.
/// This corrects accumulated error for non-linear transforms used with
/// linear interpolation.
///
/// Port of C++ `span_subdiv_adaptor<Interpolator, SubpixelShift>`. `T` is
/// the interpolator's transformer type and `I` the interpolator, by default
/// `SpanInterpolatorLinear<T>`; `SpanSubdivAdaptor::new` infers both from the
/// interpolator it is given.
pub struct SpanSubdivAdaptor<T, I = SpanInterpolatorLinear<T>> {
    subdiv_shift: u32,
    subdiv_size: u32,
    subdiv_mask: u32,
    interpolator: I,
    src_x: i32,
    src_y: f64,
    pos: u32,
    len: u32,
    trans: PhantomData<T>,
}

impl<T, I: SubdivInterpolator<Trans = T>> SpanSubdivAdaptor<T, I> {
    /// Create a new subdivision adaptor with default subdivision shift of 4 (16 pixels).
    pub fn new(interpolator: I) -> Self {
        Self::new_with_shift(interpolator, 4)
    }

    /// Create with a custom subdivision shift.
    pub fn new_with_shift(interpolator: I, subdiv_shift: u32) -> Self {
        Self {
            subdiv_shift,
            subdiv_size: 1 << subdiv_shift,
            subdiv_mask: (1 << subdiv_shift) - 1,
            interpolator,
            src_x: 0,
            src_y: 0.0,
            pos: 0,
            len: 0,
            trans: PhantomData,
        }
    }

    pub fn interpolator(&self) -> &I {
        &self.interpolator
    }

    pub fn interpolator_mut(&mut self) -> &mut I {
        &mut self.interpolator
    }

    pub fn subdiv_shift(&self) -> u32 {
        self.subdiv_shift
    }

    pub fn set_subdiv_shift(&mut self, shift: u32) {
        self.subdiv_shift = shift;
        self.subdiv_size = 1 << shift;
        self.subdiv_mask = self.subdiv_size - 1;
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
        self.interpolator.begin(x, y, sub_len);
    }

    /// Advance to the next pixel.
    #[inline]
    pub fn next(&mut self) {
        self.interpolator.next();
        if self.pos >= self.subdiv_size {
            let mut sub_len = self.len;
            if sub_len > self.subdiv_size {
                sub_len = self.subdiv_size;
            }
            self.interpolator.resynchronize(
                self.src_x as f64 / SUBPIXEL_SCALE as f64 + sub_len as f64,
                self.src_y,
                sub_len,
            );
            self.pos = 0;
        }
        self.src_x += SUBPIXEL_SCALE;
        self.pos += 1;
        // C++ decrements an unsigned here; a caller stepping past the span's
        // end wraps it there, so wrap rather than panic.
        self.len = self.len.wrapping_sub(1);
    }

    /// Get the current transformed coordinates (in subpixel units).
    #[inline]
    pub fn coordinates(&self, x: &mut i32, y: &mut i32) {
        self.interpolator.coordinates(x, y);
    }
}

impl<T: Transformer> SpanSubdivAdaptor<T> {
    pub fn transformer(&self) -> &T {
        self.interpolator.transformer()
    }
}

impl<T, I: SubdivInterpolator<Trans = T>> SpanInterpolator for SpanSubdivAdaptor<T, I> {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        SpanSubdivAdaptor::begin(self, x, y, len);
    }
    fn next(&mut self) {
        SpanSubdivAdaptor::next(self);
    }
    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        SpanSubdivAdaptor::coordinates(self, x, y);
    }
}

impl<T, I: SubdivInterpolator<Trans = T> + SpanInterpolatorLocalScale> SpanInterpolatorLocalScale
    for SpanSubdivAdaptor<T, I>
{
    /// C++ `span_subdiv_adaptor::local_scale`: the inner interpolator's.
    fn local_scale(&self, x: &mut i32, y: &mut i32) {
        self.interpolator.local_scale(x, y);
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
    fn test_identity_subdiv() {
        let interp = SpanInterpolatorLinear::new(TransAffine::new());
        let mut subdiv = SpanSubdivAdaptor::new(interp);
        subdiv.begin(5.0, 10.0, 20);

        let mut x = 0i32;
        let mut y = 0i32;
        subdiv.coordinates(&mut x, &mut y);
        assert_eq!(x, 5 * 256);
        assert_eq!(y, 10 * 256);
    }

    #[test]
    fn test_next_advances() {
        let interp = SpanInterpolatorLinear::new(TransAffine::new());
        let mut subdiv = SpanSubdivAdaptor::new(interp);
        subdiv.begin(0.0, 0.0, 10);

        let mut x = 0i32;
        let mut y = 0i32;
        subdiv.coordinates(&mut x, &mut y);
        assert_eq!(x, 0);

        subdiv.next();
        subdiv.coordinates(&mut x, &mut y);
        assert_eq!(x, 256);
    }

    #[test]
    fn test_persp_lerp_resynchronizes_at_each_boundary() {
        // Over a perspective lerp interpolator the adaptor restarts the line
        // at every 16th pixel, so those pixels land on the exact transform.
        let quad = [100.0, 100.0, 500.0, 130.0, 470.0, 520.0, 90.0, 430.0];
        let lerp = SpanInterpolatorPerspLerp::new_quad_to_rect(&quad, -150.0, -150.0, 150.0, 150.0);
        let mut subdiv = SpanSubdivAdaptor::new(lerp);
        subdiv.begin(103.5, 250.5, 40);
        for i in 0..40 {
            if i % 16 == 0 {
                let (mut tx, mut ty) = (103.5 + i as f64, 250.5);
                subdiv.interpolator().transform(&mut tx, &mut ty);
                let (mut x, mut y) = (0, 0);
                subdiv.coordinates(&mut x, &mut y);
                assert_eq!(x, iround(tx * 256.0), "pixel {i}");
                assert_eq!(y, iround(ty * 256.0), "pixel {i}");
                let (mut sx, mut sy) = (0, 0);
                subdiv.local_scale(&mut sx, &mut sy);
                assert!(sx > 0 && sy > 0);
            }
            subdiv.next();
        }
    }

    #[test]
    fn test_subdiv_shift() {
        let interp = SpanInterpolatorLinear::new(TransAffine::new());
        let subdiv = SpanSubdivAdaptor::new_with_shift(interp, 3);
        assert_eq!(subdiv.subdiv_shift(), 3);
    }

    #[test]
    fn test_resynchronize_across_boundary() {
        let interp = SpanInterpolatorLinear::new(TransAffine::new());
        let mut subdiv = SpanSubdivAdaptor::new(interp);
        // subdiv_size = 16, so after 16 steps it should resynchronize
        subdiv.begin(0.0, 0.0, 32);

        for _ in 0..16 {
            subdiv.next();
        }
        let mut x = 0i32;
        let mut y = 0i32;
        subdiv.coordinates(&mut x, &mut y);
        // Should be at pixel 16 with identity transform
        assert_eq!(x, 16 * 256);
    }

    #[test]
    fn test_with_translation() {
        let trans = TransAffine::new_translation(100.0, 200.0);
        let interp = SpanInterpolatorLinear::new(trans);
        let mut subdiv = SpanSubdivAdaptor::new(interp);
        subdiv.begin(0.0, 0.0, 5);

        let mut x = 0i32;
        let mut y = 0i32;
        subdiv.coordinates(&mut x, &mut y);
        assert_eq!(x, 100 * 256);
        assert_eq!(y, 200 * 256);
    }

    #[test]
    fn test_set_subdiv_shift() {
        let interp = SpanInterpolatorLinear::new(TransAffine::new());
        let mut subdiv = SpanSubdivAdaptor::new(interp);
        subdiv.set_subdiv_shift(6);
        assert_eq!(subdiv.subdiv_shift(), 6);
    }
}

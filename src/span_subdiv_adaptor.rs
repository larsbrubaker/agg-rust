//! Span subdivision adaptor.
//!
//! Port of `agg_span_subdiv_adaptor.h` — wraps any span interpolator and
//! periodically re-synchronizes it to correct accumulated error when using
//! linear approximation for non-linear transforms.

use crate::basics::iround;
use crate::span_interpolator_linear::{SpanInterpolatorLinear, Transformer, SUBPIXEL_SCALE};

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
/// Port of C++ `span_subdiv_adaptor<Interpolator, SubpixelShift>`.
pub struct SpanSubdivAdaptor<T: Transformer> {
    subdiv_shift: u32,
    subdiv_size: u32,
    subdiv_mask: u32,
    interpolator: SpanInterpolatorLinear<T>,
    src_x: i32,
    src_y: f64,
    pos: u32,
    len: u32,
}

impl<T: Transformer> SpanSubdivAdaptor<T> {
    /// Create a new subdivision adaptor with default subdivision shift of 4 (16 pixels).
    pub fn new(interpolator: SpanInterpolatorLinear<T>) -> Self {
        Self {
            subdiv_shift: 4,
            subdiv_size: 1 << 4,
            subdiv_mask: (1 << 4) - 1,
            interpolator,
            src_x: 0,
            src_y: 0.0,
            pos: 0,
            len: 0,
        }
    }

    /// Create with a custom subdivision shift.
    pub fn new_with_shift(interpolator: SpanInterpolatorLinear<T>, subdiv_shift: u32) -> Self {
        Self {
            subdiv_shift,
            subdiv_size: 1 << subdiv_shift,
            subdiv_mask: (1 << subdiv_shift) - 1,
            interpolator,
            src_x: 0,
            src_y: 0.0,
            pos: 0,
            len: 0,
        }
    }

    pub fn interpolator(&self) -> &SpanInterpolatorLinear<T> {
        &self.interpolator
    }

    pub fn interpolator_mut(&mut self) -> &mut SpanInterpolatorLinear<T> {
        &mut self.interpolator
    }

    pub fn transformer(&self) -> &T {
        self.interpolator.transformer()
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
        self.len -= 1;
    }

    /// Get the current transformed coordinates (in subpixel units).
    #[inline]
    pub fn coordinates(&self, x: &mut i32, y: &mut i32) {
        self.interpolator.coordinates(x, y);
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

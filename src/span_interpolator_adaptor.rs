//! Span interpolator adaptor with distortion.
//!
//! Port of `agg_span_interpolator_adaptor.h`.
//! Wraps any `SpanInterpolator`, applying a distortion function
//! after `coordinates()`.

use crate::span_interpolator_linear::SpanInterpolator;

/// Trait for coordinate distortion applied after interpolation.
///
/// Port of the C++ `Distortion` template concept.
pub trait Distortion {
    fn calculate(&self, x: &mut i32, y: &mut i32);
}

/// Adaptor that wraps a span interpolator and applies distortion.
///
/// Port of C++ `span_interpolator_adaptor<Interpolator, Distortion>`.
/// After the base interpolator computes coordinates, the distortion
/// function modifies them (e.g., for wave, lens, or other effects).
pub struct SpanInterpolatorAdaptor<Interp, Dist> {
    interp: Interp,
    distortion: Dist,
}

impl<Interp: SpanInterpolator, Dist: Distortion> SpanInterpolatorAdaptor<Interp, Dist> {
    pub fn new(interp: Interp, distortion: Dist) -> Self {
        Self { interp, distortion }
    }

    pub fn interpolator(&self) -> &Interp {
        &self.interp
    }

    pub fn interpolator_mut(&mut self) -> &mut Interp {
        &mut self.interp
    }

    pub fn distortion(&self) -> &Dist {
        &self.distortion
    }

    pub fn distortion_mut(&mut self) -> &mut Dist {
        &mut self.distortion
    }
}

impl<Interp: SpanInterpolator, Dist: Distortion> SpanInterpolator
    for SpanInterpolatorAdaptor<Interp, Dist>
{
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.interp.begin(x, y, len);
    }

    fn next(&mut self) {
        self.interp.next();
    }

    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        self.interp.coordinates(x, y);
        self.distortion.calculate(x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span_interpolator_linear::{SpanInterpolatorLinear, SUBPIXEL_SCALE};
    use crate::trans_affine::TransAffine;

    /// A simple distortion that offsets coordinates by fixed amounts.
    struct OffsetDistortion {
        dx: i32,
        dy: i32,
    }

    impl Distortion for OffsetDistortion {
        fn calculate(&self, x: &mut i32, y: &mut i32) {
            *x += self.dx;
            *y += self.dy;
        }
    }

    #[test]
    fn test_identity_with_offset_distortion() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let dist = OffsetDistortion {
            dx: 10 * SUBPIXEL_SCALE,
            dy: 20 * SUBPIXEL_SCALE,
        };
        let mut adaptor = SpanInterpolatorAdaptor::new(interp, dist);

        adaptor.begin(5.0, 5.0, 1);
        let (mut x, mut y) = (0, 0);
        adaptor.coordinates(&mut x, &mut y);

        // Identity transform: (5,5) * 256 = (1280,1280), plus offset (2560, 5120)
        assert_eq!(x, 5 * SUBPIXEL_SCALE + 10 * SUBPIXEL_SCALE);
        assert_eq!(y, 5 * SUBPIXEL_SCALE + 20 * SUBPIXEL_SCALE);
    }

    #[test]
    fn test_zero_distortion_passthrough() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let dist = OffsetDistortion { dx: 0, dy: 0 };
        let mut adaptor = SpanInterpolatorAdaptor::new(interp, dist);

        adaptor.begin(10.0, 20.0, 1);
        let (mut x, mut y) = (0, 0);
        adaptor.coordinates(&mut x, &mut y);

        // Should match plain interpolator output
        let trans2 = TransAffine::new();
        let mut interp2 = SpanInterpolatorLinear::new(trans2);
        interp2.begin(10.0, 20.0, 1);
        let (mut x2, mut y2) = (0, 0);
        interp2.coordinates(&mut x2, &mut y2);

        assert_eq!(x, x2);
        assert_eq!(y, y2);
    }

    /// Sine-wave distortion for testing.
    struct WaveDistortion {
        amplitude: i32,
    }

    impl Distortion for WaveDistortion {
        fn calculate(&self, _x: &mut i32, y: &mut i32) {
            *y += self.amplitude;
        }
    }

    #[test]
    fn test_wave_distortion() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let dist = WaveDistortion { amplitude: 100 };
        let mut adaptor = SpanInterpolatorAdaptor::new(interp, dist);

        adaptor.begin(0.0, 0.0, 5);
        let (mut x, mut y) = (0, 0);
        adaptor.coordinates(&mut x, &mut y);
        assert_eq!(y, 100); // 0 + amplitude
    }
}

//! Base image filter span generators.
//!
//! Port of `agg_span_image_filter.h` — base structs shared by all image
//! transformation span generators (nearest, bilinear, general convolution,
//! and resampling variants).

use crate::basics::uround;
use crate::image_filters::{ImageFilterLut, IMAGE_SUBPIXEL_SCALE, IMAGE_SUBPIXEL_SHIFT};
use crate::span_interpolator_linear::SpanInterpolatorLinear;
use crate::trans_affine::TransAffine;

// ============================================================================
// SpanImageFilterBase
// ============================================================================

/// Base state for image filter span generators.
///
/// Holds a reference to the interpolator and optional filter LUT,
/// plus the subpixel offset (dx, dy) applied to coordinates before filtering.
///
/// Port of C++ `span_image_filter<Source, Interpolator>` (without the source,
/// which is handled by the concrete span generator).
pub struct SpanImageFilterBase<'a, I> {
    interpolator: &'a mut I,
    filter: Option<&'a ImageFilterLut>,
    dx_dbl: f64,
    dy_dbl: f64,
    dx_int: u32,
    dy_int: u32,
}

impl<'a, I> SpanImageFilterBase<'a, I> {
    /// Create a new base with interpolator, optional filter, and default 0.5 offset.
    pub fn new(interpolator: &'a mut I, filter: Option<&'a ImageFilterLut>) -> Self {
        Self {
            interpolator,
            filter,
            dx_dbl: 0.5,
            dy_dbl: 0.5,
            dx_int: IMAGE_SUBPIXEL_SCALE / 2,
            dy_int: IMAGE_SUBPIXEL_SCALE / 2,
        }
    }

    pub fn interpolator(&self) -> &I {
        self.interpolator
    }

    pub fn interpolator_mut(&mut self) -> &mut I {
        self.interpolator
    }

    pub fn filter(&self) -> Option<&ImageFilterLut> {
        self.filter
    }

    pub fn filter_dx_int(&self) -> u32 {
        self.dx_int
    }

    pub fn filter_dy_int(&self) -> u32 {
        self.dy_int
    }

    pub fn filter_dx_dbl(&self) -> f64 {
        self.dx_dbl
    }

    pub fn filter_dy_dbl(&self) -> f64 {
        self.dy_dbl
    }

    /// Set the subpixel filter offset.
    ///
    /// The offset shifts coordinates before filtering. Default is (0.5, 0.5)
    /// which centers the filter kernel on pixel centers.
    pub fn set_filter_offset(&mut self, dx: f64, dy: f64) {
        self.dx_dbl = dx;
        self.dy_dbl = dy;
        self.dx_int = uround(dx * IMAGE_SUBPIXEL_SCALE as f64);
        self.dy_int = uround(dy * IMAGE_SUBPIXEL_SCALE as f64);
    }

    /// Set equal filter offset for both axes.
    pub fn set_filter_offset_uniform(&mut self, d: f64) {
        self.set_filter_offset(d, d);
    }

    /// Set the filter LUT.
    pub fn set_filter(&mut self, filter: &'a ImageFilterLut) {
        self.filter = Some(filter);
    }

    /// No-op prepare (matches C++ `span_image_filter::prepare()`).
    pub fn prepare(&self) {
        // intentionally empty — subclasses override
    }
}

// ============================================================================
// SpanImageResampleAffine
// ============================================================================

/// Image resampling state for affine transformations.
///
/// Computes scale factors from the interpolator's affine matrix once per
/// scanline (in `prepare()`), limiting them to avoid excessive filter expansion.
///
/// Port of C++ `span_image_resample_affine<Source>`.
pub struct SpanImageResampleAffine<'a> {
    base: SpanImageFilterBase<'a, SpanInterpolatorLinear<TransAffine>>,
    scale_limit: f64,
    blur_x: f64,
    blur_y: f64,
    rx: i32,
    ry: i32,
    rx_inv: i32,
    ry_inv: i32,
}

impl<'a> SpanImageResampleAffine<'a> {
    /// Create with interpolator and filter.
    pub fn new(
        interpolator: &'a mut SpanInterpolatorLinear<TransAffine>,
        filter: &'a ImageFilterLut,
    ) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, Some(filter)),
            scale_limit: 200.0,
            blur_x: 1.0,
            blur_y: 1.0,
            rx: 0,
            ry: 0,
            rx_inv: 0,
            ry_inv: 0,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, SpanInterpolatorLinear<TransAffine>> {
        &self.base
    }

    pub fn base_mut(
        &mut self,
    ) -> &mut SpanImageFilterBase<'a, SpanInterpolatorLinear<TransAffine>> {
        &mut self.base
    }

    pub fn scale_limit(&self) -> u32 {
        uround(self.scale_limit)
    }

    pub fn set_scale_limit(&mut self, v: i32) {
        self.scale_limit = v as f64;
    }

    pub fn blur_x(&self) -> f64 {
        self.blur_x
    }

    pub fn blur_y(&self) -> f64 {
        self.blur_y
    }

    pub fn set_blur_x(&mut self, v: f64) {
        self.blur_x = v;
    }

    pub fn set_blur_y(&mut self, v: f64) {
        self.blur_y = v;
    }

    pub fn set_blur(&mut self, v: f64) {
        self.blur_x = v;
        self.blur_y = v;
    }

    pub fn rx(&self) -> i32 {
        self.rx
    }

    pub fn ry(&self) -> i32 {
        self.ry
    }

    pub fn rx_inv(&self) -> i32 {
        self.rx_inv
    }

    pub fn ry_inv(&self) -> i32 {
        self.ry_inv
    }

    /// Compute scale factors from the affine transformation.
    ///
    /// Port of C++ `span_image_resample_affine::prepare()`.
    pub fn prepare(&mut self) {
        let (mut scale_x, mut scale_y) = self.base.interpolator.transformer().scaling_abs();

        let scale_xy = scale_x * scale_y;
        if scale_xy > self.scale_limit {
            scale_x = scale_x * self.scale_limit / scale_xy;
            scale_y = scale_y * self.scale_limit / scale_xy;
        }

        if scale_x < 1.0 {
            scale_x = 1.0;
        }
        if scale_y < 1.0 {
            scale_y = 1.0;
        }

        if scale_x > self.scale_limit {
            scale_x = self.scale_limit;
        }
        if scale_y > self.scale_limit {
            scale_y = self.scale_limit;
        }

        scale_x *= self.blur_x;
        scale_y *= self.blur_y;

        if scale_x < 1.0 {
            scale_x = 1.0;
        }
        if scale_y < 1.0 {
            scale_y = 1.0;
        }

        self.rx = uround(scale_x * IMAGE_SUBPIXEL_SCALE as f64) as i32;
        self.rx_inv = uround(1.0 / scale_x * IMAGE_SUBPIXEL_SCALE as f64) as i32;

        self.ry = uround(scale_y * IMAGE_SUBPIXEL_SCALE as f64) as i32;
        self.ry_inv = uround(1.0 / scale_y * IMAGE_SUBPIXEL_SCALE as f64) as i32;
    }
}

// ============================================================================
// SpanImageResample
// ============================================================================

/// Image resampling state for generic (non-affine) interpolators.
///
/// Unlike the affine variant which computes scale once per scanline, this
/// version expects per-pixel scale values and provides `adjust_scale()` to
/// clamp and blur them.
///
/// Port of C++ `span_image_resample<Source, Interpolator>`.
pub struct SpanImageResample<'a, I> {
    base: SpanImageFilterBase<'a, I>,
    scale_limit: i32,
    blur_x: i32,
    blur_y: i32,
}

impl<'a, I> SpanImageResample<'a, I> {
    /// Create with interpolator and filter.
    pub fn new(interpolator: &'a mut I, filter: &'a ImageFilterLut) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, Some(filter)),
            scale_limit: 20,
            blur_x: IMAGE_SUBPIXEL_SCALE as i32,
            blur_y: IMAGE_SUBPIXEL_SCALE as i32,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }

    pub fn scale_limit(&self) -> i32 {
        self.scale_limit
    }

    pub fn set_scale_limit(&mut self, v: i32) {
        self.scale_limit = v;
    }

    pub fn blur_x(&self) -> f64 {
        self.blur_x as f64 / IMAGE_SUBPIXEL_SCALE as f64
    }

    pub fn blur_y(&self) -> f64 {
        self.blur_y as f64 / IMAGE_SUBPIXEL_SCALE as f64
    }

    pub fn set_blur_x(&mut self, v: f64) {
        self.blur_x = uround(v * IMAGE_SUBPIXEL_SCALE as f64) as i32;
    }

    pub fn set_blur_y(&mut self, v: f64) {
        self.blur_y = uround(v * IMAGE_SUBPIXEL_SCALE as f64) as i32;
    }

    pub fn set_blur(&mut self, v: f64) {
        let iv = uround(v * IMAGE_SUBPIXEL_SCALE as f64) as i32;
        self.blur_x = iv;
        self.blur_y = iv;
    }

    /// Adjust per-pixel scale factors: clamp to limits, apply blur.
    ///
    /// Port of C++ `span_image_resample::adjust_scale()`.
    #[inline]
    pub fn adjust_scale(&self, rx: &mut i32, ry: &mut i32) {
        let subpixel = IMAGE_SUBPIXEL_SCALE as i32;
        if *rx < subpixel {
            *rx = subpixel;
        }
        if *ry < subpixel {
            *ry = subpixel;
        }
        if *rx > subpixel * self.scale_limit {
            *rx = subpixel * self.scale_limit;
        }
        if *ry > subpixel * self.scale_limit {
            *ry = subpixel * self.scale_limit;
        }
        *rx = (*rx * self.blur_x) >> IMAGE_SUBPIXEL_SHIFT;
        *ry = (*ry * self.blur_y) >> IMAGE_SUBPIXEL_SHIFT;
        if *rx < subpixel {
            *rx = subpixel;
        }
        if *ry < subpixel {
            *ry = subpixel;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_default_offsets() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let base = SpanImageFilterBase::new(&mut interp, None);
        assert_eq!(base.filter_dx_int(), IMAGE_SUBPIXEL_SCALE / 2);
        assert_eq!(base.filter_dy_int(), IMAGE_SUBPIXEL_SCALE / 2);
        assert_eq!(base.filter_dx_dbl(), 0.5);
        assert_eq!(base.filter_dy_dbl(), 0.5);
    }

    #[test]
    fn test_base_set_offset() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut base = SpanImageFilterBase::new(&mut interp, None);
        base.set_filter_offset(0.25, 0.75);
        assert_eq!(base.filter_dx_dbl(), 0.25);
        assert_eq!(base.filter_dy_dbl(), 0.75);
        // 0.25 * 256 = 64, 0.75 * 256 = 192
        assert_eq!(base.filter_dx_int(), 64);
        assert_eq!(base.filter_dy_int(), 192);
    }

    #[test]
    fn test_base_set_offset_uniform() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut base = SpanImageFilterBase::new(&mut interp, None);
        base.set_filter_offset_uniform(0.0);
        assert_eq!(base.filter_dx_int(), 0);
        assert_eq!(base.filter_dy_int(), 0);
    }

    #[test]
    fn test_base_filter_reference() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let base = SpanImageFilterBase::new(&mut interp, None);
        assert!(base.filter().is_none());
    }

    #[test]
    fn test_base_with_filter() {
        use crate::image_filters::ImageFilterBilinear;
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let base = SpanImageFilterBase::new(&mut interp, Some(&lut));
        assert!(base.filter().is_some());
        assert_eq!(base.filter().unwrap().radius(), 1.0);
    }

    #[test]
    fn test_resample_affine_defaults() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let r = SpanImageResampleAffine::new(&mut interp, &lut);
        assert_eq!(r.scale_limit(), 200);
        assert_eq!(r.blur_x(), 1.0);
        assert_eq!(r.blur_y(), 1.0);
    }

    #[test]
    fn test_resample_affine_prepare_identity() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let mut r = SpanImageResampleAffine::new(&mut interp, &lut);
        r.prepare();
        // Identity: scale_x = 1, scale_y = 1
        // rx = uround(1.0 * 256) = 256
        assert_eq!(r.rx(), IMAGE_SUBPIXEL_SCALE as i32);
        assert_eq!(r.ry(), IMAGE_SUBPIXEL_SCALE as i32);
        // rx_inv = uround(1.0/1.0 * 256) = 256
        assert_eq!(r.rx_inv(), IMAGE_SUBPIXEL_SCALE as i32);
        assert_eq!(r.ry_inv(), IMAGE_SUBPIXEL_SCALE as i32);
    }

    #[test]
    fn test_resample_affine_prepare_scaled() {
        let trans = TransAffine::new_scaling(2.0, 3.0);
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let mut r = SpanImageResampleAffine::new(&mut interp, &lut);
        r.prepare();
        // scale_x=2, scale_y=3 (product=6 < 200 limit, both > 1)
        // rx = uround(2.0 * 256) = 512
        assert_eq!(r.rx(), 512);
        // ry = uround(3.0 * 256) = 768
        assert_eq!(r.ry(), 768);
        // rx_inv = uround(0.5 * 256) = 128
        assert_eq!(r.rx_inv(), 128);
        // ry_inv = uround(1.0/3.0 * 256) = 85
        assert_eq!(r.ry_inv(), 85);
    }

    #[test]
    fn test_resample_affine_blur() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let mut r = SpanImageResampleAffine::new(&mut interp, &lut);
        r.set_blur(2.0);
        r.prepare();
        // Identity scale = 1.0, blur = 2.0 → effective = 2.0
        assert_eq!(r.rx(), 512);
        assert_eq!(r.ry(), 512);
    }

    #[test]
    fn test_resample_generic_defaults() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let r = SpanImageResample::new(&mut interp, &lut);
        assert_eq!(r.scale_limit(), 20);
        assert!((r.blur_x() - 1.0).abs() < 1e-10);
        assert!((r.blur_y() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_resample_generic_adjust_scale_clamp_min() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let r = SpanImageResample::new(&mut interp, &lut);
        let mut rx = 10; // below IMAGE_SUBPIXEL_SCALE (256)
        let mut ry = 10;
        r.adjust_scale(&mut rx, &mut ry);
        // Should be clamped to IMAGE_SUBPIXEL_SCALE
        assert_eq!(rx, IMAGE_SUBPIXEL_SCALE as i32);
        assert_eq!(ry, IMAGE_SUBPIXEL_SCALE as i32);
    }

    #[test]
    fn test_resample_generic_adjust_scale_clamp_max() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let r = SpanImageResample::new(&mut interp, &lut);
        // scale_limit=20, IMAGE_SUBPIXEL_SCALE=256 → max = 5120
        let mut rx = 100_000;
        let mut ry = 100_000;
        r.adjust_scale(&mut rx, &mut ry);
        // After clamping to 5120 and applying blur (1.0 * 256 / 256 = same):
        assert_eq!(rx, 5120);
        assert_eq!(ry, 5120);
    }

    #[test]
    fn test_resample_generic_adjust_scale_with_blur() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let mut r = SpanImageResample::new(&mut interp, &lut);
        r.set_blur(2.0); // blur_x = blur_y = uround(2.0 * 256) = 512
        let mut rx = 256; // 1.0 in subpixel
        let mut ry = 256;
        r.adjust_scale(&mut rx, &mut ry);
        // rx = (256 * 512) >> 8 = 512
        assert_eq!(rx, 512);
        assert_eq!(ry, 512);
    }

    #[test]
    fn test_resample_generic_set_scale_limit() {
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let lut = ImageFilterLut::new_with_filter(&crate::image_filters::ImageFilterBilinear, true);
        let mut r = SpanImageResample::new(&mut interp, &lut);
        r.set_scale_limit(50);
        assert_eq!(r.scale_limit(), 50);
    }
}

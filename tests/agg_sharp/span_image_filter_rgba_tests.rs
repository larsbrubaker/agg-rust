//! agg-sharp's `SpanImageFilterRgbaTests` (`Tests/Agg.Tests/Agg/SpanImageFilterRgbaTests.cs`): the
//! general lookup-table `span_image_filter_rgba` (agg-rust's `SpanImageFilterRgbaGen`) on a 4x4
//! image, sampling frame pixel (1, 1) through a translation of (tx, ty). Its sums round
//! (agg-rust's documented fix in `src/span_image_filter_rgba.rs`; C++ truncates).
//!
//! Not ported: `StraightAlphaColorIsNotClampedToAlpha` pins agg-sharp-only behaviour. C++ clamps
//! each colour channel to alpha because its pixel formats are premultiplied; agg-sharp drops the
//! clamp for its straight-alpha `ImageBuffer`s. agg-rust keeps C++'s clamp.

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClone;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterLut, ImageFilterSpline36};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaGen;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

/// Halfway between 0 and 1 is 0.5, which rounds to 1.
#[test]
fn halfway_between_two_values_rounds() {
    let mut image = Image::new(4, 4);
    for y in 0..4 {
        image.set_pixel(2, y, [1, 1, 1, 1]);
    }

    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
    let sample = sample(&image, 0.5, 0.0, &lut);

    assert_eq!(sample.r, 1);
    assert_eq!(sample.a, 1);
}

/// A flat image keeps its exact value at every fractional offset. Through spline36's normalized
/// table the rounded weight products can sum to just under a whole; truncating, C++ gives 254 for
/// 255 there.
#[test]
fn flat_color_keeps_its_value_at_every_fractional_offset() {
    let mut image = Image::new(4, 4);
    image.fill([255, 128, 3, 255]);

    let lut = ImageFilterLut::new_with_filter(&ImageFilterSpline36, true);
    for i in 0..16 {
        for j in 0..16 {
            let sample = sample(&image, i as f64 / 16.0, j as f64 / 16.0, &lut);

            assert_eq!(sample, Rgba8::new(255, 128, 3, 255));
        }
    }
}

fn sample(image: &Image, tx: f64, ty: f64, lut: &ImageFilterLut) -> Rgba8 {
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty));
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    let mut filter = SpanImageFilterRgbaGen::new(&mut accessor, &mut interpolator, lut);
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, 1, 1, 1);
    span[0]
}

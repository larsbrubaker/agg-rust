//! agg-sharp's `SpanImageFilterRgba2x2Tests`
//! (`Tests/Agg.Tests/Agg/SpanImageFilterRgba2x2Tests.cs`): `span_image_filter_rgba_2x2` with the
//! bilinear lookup table on a 4x4 image, sampling frame pixel (1, 1) through a translation of
//! (tx, ty). Its sums round (agg-rust's documented fix in `src/span_image_filter_rgba.rs`).

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClone;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterLut};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgba2x2;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

/// Halfway between 0 and 1 is 0.5, which rounds to 1.
#[test]
fn halfway_between_two_values_rounds() {
    let mut image = Image::new(4, 4);
    for y in 0..4 {
        image.set_pixel(2, y, [1, 1, 1, 1]);
    }

    let sample = sample(&image, 0.5, 0.0);

    assert_eq!(sample.r, 1);
    assert_eq!(sample.a, 1);
}

/// A flat image keeps its exact value at any fractional offset: all four weights count.
#[test]
fn flat_color_keeps_its_value_at_a_fractional_offset() {
    let mut image = Image::new(4, 4);
    image.fill([255, 128, 3, 255]);

    let sample = sample(&image, 1.0 / 256.0, 255.0 / 256.0);

    assert_eq!(sample, Rgba8::new(255, 128, 3, 255));
}

fn sample(image: &Image, tx: f64, ty: f64) -> Rgba8 {
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty));
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    let mut filter = SpanImageFilterRgba2x2::new(&mut accessor, &mut interpolator, &lut);
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, 1, 1, 1);
    span[0]
}

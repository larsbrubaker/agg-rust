//! agg-sharp's `SpanImageFilterRgbaBilinearClipTests`
//! (`Tests/Agg.Tests/Agg/SpanImageFilterRgbaBilinearClipTests.cs`):
//! `span_image_filter_rgba_bilinear_clip` on a 4x4 image, sampling frame pixel (1, 1) through a
//! translation of (tx, ty). Its sums round (agg-rust's documented fix in
//! `src/span_image_filter_rgba.rs`; C++ truncates).

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
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

/// A flat image keeps its exact value at any fractional offset: every weight counts.
#[test]
fn flat_color_keeps_its_value_at_a_fractional_offset() {
    let mut image = Image::new(4, 4);
    image.fill([255, 128, 3, 255]);

    let sample = sample(&image, 1.0 / 256.0, 255.0 / 256.0);

    assert_eq!(sample, Rgba8::new(255, 128, 3, 255));
}

/// Under an identity matrix a sample wholly outside the image is the background color, as in C++.
#[test]
fn sample_outside_the_image_is_the_background_under_an_identity_matrix() {
    let mut image = Image::new(4, 4);
    image.fill([255, 0, 0, 255]);
    let background = Rgba8::new(10, 20, 30, 40);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new());
    let mut filter = SpanImageFilterRgbaBilinearClip::new(&image.ra, background, &mut interpolator);
    let mut span = [Rgba8::new(0, 0, 0, 0)];

    filter.generate(&mut span, 8, 1, 1);

    assert_eq!(span[0], background);
}

fn sample(image: &Image, tx: f64, ty: f64) -> Rgba8 {
    let white = Rgba8::new(255, 255, 255, 255);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty));
    let mut filter = SpanImageFilterRgbaBilinearClip::new(&image.ra, white, &mut interpolator);
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, 1, 1, 1);
    span[0]
}

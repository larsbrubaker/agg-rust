//! agg-sharp's `SpanImageResampleRgbTests` (`Tests/Agg.Tests/Agg/SpanImageResampleRgbTests.cs`):
//! the 24-bit resampling span generators (pattern_resample.cpp's). Their division by the total
//! weight rounds (agg-rust's documented fix in `src/span_image_filter_rgb.rs`; C++ truncates).

use crate::support::Image;
use agg_rust::basics::uround;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClip;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterHanning, ImageFilterLut};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgb::{SpanImageResampleRgb, SpanImageResampleRgbAffine};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::span_interpolator_persp::SpanInterpolatorPerspLerp;
use agg_rust::span_subdiv_adaptor::SpanSubdivAdaptor;
use agg_rust::trans_affine::TransAffine;

/// At scale 1 and half a pixel over, the bilinear table weighs the column of ones just over half,
/// which rounds to 1 where C++ truncates to 0.
#[test]
fn affine_resample_rounds_just_over_half() {
    let image = column_of_ones();
    let mut acc = clip(&image);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(0.5, 0.0));
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageResampleRgbAffine::new(&mut acc, &mut interpolator, &lut);

    let sample = sample(&mut filter);

    assert_eq!(sample.r, 1);
    assert_eq!(sample.g, 1);
    assert_eq!(sample.b, 1);
    assert_eq!(sample.a, 255);
}

/// The same through the perspective resampler, its scale from a lerp interpolator under the
/// subdiv adaptor.
#[test]
fn perspective_resample_rounds_just_over_half() {
    let image = column_of_ones();
    let mut acc = clip(&image);
    let mut interpolator = translated_by_half();
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageResampleRgb::new(&mut acc, &mut interpolator, &lut);

    assert_eq!(sample(&mut filter).r, 1);
}

/// A flat image keeps its exact value through both resamplers, shrunk and blurred: no weight is
/// skipped.
#[test]
fn flat_color_keeps_its_value_shrunk_and_blurred() {
    let flat = Rgba8::new(255, 128, 3, 255);
    let mut image = Image::new(64, 64, 3, 0);
    for px in image.bytes.as_chunks_mut::<3>().0 {
        px.copy_from_slice(&[255, 128, 3]);
    }
    let lut = ImageFilterLut::new_with_filter(&ImageFilterHanning, true);

    let mut acc = clip(&image);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_scaling(3.7, 2.2));
    let mut affine = SpanImageResampleRgbAffine::new(&mut acc, &mut interpolator, &lut);
    affine.resample_base_mut().set_blur(1.5);
    assert_eq!(sample(&mut affine), flat);

    let mut acc = clip(&image);
    let shrink = SpanInterpolatorPerspLerp::new_quad_to_rect(
        &[0.0, 0.0, 5.0, 0.0, 5.0, 5.0, 0.0, 5.0],
        0.0,
        0.0,
        20.0,
        20.0,
    );
    let mut interpolator = SpanSubdivAdaptor::new(shrink);
    let mut perspective = SpanImageResampleRgb::new(&mut acc, &mut interpolator, &lut);
    perspective.resample_base_mut().set_blur(1.5);
    assert_eq!(sample(&mut perspective), flat);
}

/// The affine resampler's footprint follows the transform's scale, times the blur, and never
/// drops below a pixel: C++ `span_image_resample_affine::prepare`.
#[test]
fn affine_prepare_scales_the_filter_radius() {
    let image = column_of_ones();
    let mut acc = clip(&image);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterHanning, true);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_scaling(3.0, 0.5));
    let mut filter = SpanImageResampleRgbAffine::new(&mut acc, &mut interpolator, &lut);
    filter.resample_base_mut().set_blur(1.5);

    filter.prepare();

    assert_eq!(
        filter.resample_base().rx(),
        uround(3.0 * 1.5 * 256.0) as i32
    );
    assert_eq!(filter.resample_base().ry(), uround(1.5 * 256.0) as i32);
}

// Frame pixel (x, y) -> (x + 0.5, y) as a perspective, quad to rectangle.
fn translated_by_half(
) -> SpanSubdivAdaptor<agg_rust::trans_perspective::TransPerspective, SpanInterpolatorPerspLerp> {
    let interpolator = SpanInterpolatorPerspLerp::new_quad_to_rect(
        &[-0.5, 0.0, 7.5, 0.0, 7.5, 8.0, -0.5, 8.0],
        0.0,
        0.0,
        8.0,
        8.0,
    );
    SpanSubdivAdaptor::new(interpolator)
}

fn clip(image: &Image) -> ImageAccessorClip<'_, 3> {
    ImageAccessorClip::<3>::new(&image.rows, &[0, 0, 0])
}

// An 8x8 black image with column 4 at 1.
fn column_of_ones() -> Image {
    let mut image = Image::new(8, 8, 3, 0);
    for y in 0..8 {
        let o = (y * 8 + 4) * 3;
        image.bytes[o..o + 3].copy_from_slice(&[1, 1, 1]);
    }
    image
}

// Frame pixel (3, 3), after the span generator's prepare as the scanline renderer calls it.
fn sample<G: SpanGenerator<Color = Rgba8>>(filter: &mut G) -> Rgba8 {
    filter.prepare();
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, 3, 3, 1);
    span[0]
}

//! agg-sharp's `SpanImageResampleRgbaTests`
//! (`Tests/Agg.Tests/Agg/SpanImageResampleRgbaTests.cs`): the 32-bit resampling span generators
//! (image_resample.cpp's). Their division by the total weight rounds (agg-rust's documented fix in
//! `src/span_image_filter_rgba.rs`; C++ truncates). Like C++, the result is premultiplied: color
//! above alpha is clamped to it.

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClip;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterHanning, ImageFilterLut};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::{SpanImageResampleRgba, SpanImageResampleRgbaAffine};
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
    let mut filter = SpanImageResampleRgbaAffine::new(&mut acc, &mut interpolator, &lut);

    assert_eq!(sample(&mut filter), Rgba8::new(1, 1, 1, 255));
}

/// The same through the perspective resampler, its scale from a lerp interpolator under the
/// subdiv adaptor.
#[test]
fn perspective_resample_rounds_just_over_half() {
    let image = column_of_ones();
    let mut acc = clip(&image);
    let lerp = SpanInterpolatorPerspLerp::new_quad_to_rect(
        &[-0.5, 0.0, 7.5, 0.0, 7.5, 8.0, -0.5, 8.0],
        0.0,
        0.0,
        8.0,
        8.0,
    );
    let mut interpolator = SpanSubdivAdaptor::new(lerp);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageResampleRgba::new(&mut acc, &mut interpolator, &lut);

    assert_eq!(sample(&mut filter), Rgba8::new(1, 1, 1, 255));
}

/// A flat image keeps its exact value, alpha too, through both resamplers, shrunk and blurred.
#[test]
fn flat_color_keeps_its_value_shrunk_and_blurred() {
    let flat = Rgba8::new(100, 60, 3, 128);
    let image = flat_image([100, 60, 3, 128]);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterHanning, true);

    let mut acc = clip(&image);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_scaling(3.7, 2.2));
    let mut affine = SpanImageResampleRgbaAffine::new(&mut acc, &mut interpolator, &lut);
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
    let mut perspective = SpanImageResampleRgba::new(&mut acc, &mut interpolator, &lut);
    perspective.resample_base_mut().set_blur(1.5);
    assert_eq!(sample(&mut perspective), flat);
}

/// C++ treats the source as premultiplied: a channel above alpha comes out at alpha.
#[test]
fn color_above_alpha_is_clamped_to_alpha() {
    let image = flat_image([255, 40, 200, 100]);
    let mut acc = clip(&image);
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new());
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageResampleRgbaAffine::new(&mut acc, &mut interpolator, &lut);

    assert_eq!(sample(&mut filter), Rgba8::new(100, 40, 100, 100));
}

fn clip(image: &Image) -> ImageAccessorClip<'_, 4> {
    ImageAccessorClip::<4>::new(&image.ra, &[0, 0, 0, 0])
}

fn flat_image(color: [u8; 4]) -> Image {
    let mut image = Image::new(64, 64);
    image.fill(color);
    image
}

// An 8x8 opaque black image with column 4 at 1.
fn column_of_ones() -> Image {
    let mut image = Image::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let v = if x == 4 { 1 } else { 0 };
            image.set_pixel(x, y, [v, v, v, 255]);
        }
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

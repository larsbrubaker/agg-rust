//! agg-sharp's `SpanImageFilterRgbTests` (`Tests/Agg.Tests/Agg/SpanImageFilterRgbTests.cs`): the
//! 24-bit span image filters (image_filters.cpp's), sampling frame pixel (3, 3) of an 8x8 image
//! through a translation of (tx, ty): each filter takes back the half pixel the interpolator adds,
//! so it reads the image at (3 + tx, 3 + ty). Their sums round (agg-rust's documented fix in
//! `src/span_image_filter_rgb.rs`; C++ starts them at 0 and truncates).

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClip;
use agg_rust::image_filters::{
    ImageFilterBicubic, ImageFilterBilinear, ImageFilterHanning, ImageFilterLut,
};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgb::{
    SpanImageFilterRgb, SpanImageFilterRgb2x2, SpanImageFilterRgbBilinear,
    SpanImageFilterRgbBilinearClip,
};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

const CLEAR: Rgba8 = Rgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

/// Halfway between 0 and 1 rounds to 1 in the bilinear filter.
#[test]
fn bilinear_clip_rounds_halfway() {
    let image = column_of_ones();
    let mut interpolator = translated(0.5, 0.0);
    let mut filter = SpanImageFilterRgbBilinearClip::new(&image.rows, CLEAR, &mut interpolator);

    let sample = sample(&mut filter);

    assert_eq!(sample.r, 1);
    assert_eq!(sample.a, 255);
}

/// Halfway between 0 and 1 rounds to 1 in the 2x2 filter.
#[test]
fn two_by_two_rounds_halfway() {
    let image = column_of_ones();
    let mut accessor = clip(&image);
    let mut interpolator = translated(0.5, 0.0);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageFilterRgb2x2::new(&mut accessor, &mut interpolator, &lut);

    let sample = sample(&mut filter);

    assert_eq!(sample.r, 1);
    assert_eq!(sample.g, 1);
    assert_eq!(sample.b, 1);
}

/// The general filter's bilinear table weighs the column of ones just over half (8256 of 16384),
/// which rounds to 1 where C++ truncates to 0.
#[test]
fn general_filter_rounds_just_over_half() {
    let image = column_of_ones();
    let mut accessor = clip(&image);
    let mut interpolator = translated(0.5, 0.0);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
    let mut filter = SpanImageFilterRgb::new(&mut accessor, &mut interpolator, &lut);

    assert_eq!(sample(&mut filter).r, 1);
}

/// A flat image keeps its exact value through every filter at a fractional offset: no weight is
/// skipped.
#[test]
fn flat_color_keeps_its_value_through_each_filter() {
    let flat = Rgba8::new(255, 128, 3, 255);
    let image = filled(8, [255, 128, 3]);

    let mut interpolator = translated(0.25, 0.75);
    let mut filter = SpanImageFilterRgbBilinearClip::new(&image.rows, CLEAR, &mut interpolator);
    assert_eq!(sample(&mut filter), flat);

    let mut accessor = clip(&image);
    let mut interpolator = translated(0.25, 0.75);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterHanning, true);
    let mut filter = SpanImageFilterRgb2x2::new(&mut accessor, &mut interpolator, &lut);
    assert_eq!(sample(&mut filter), flat);

    let mut accessor = clip(&image);
    let mut interpolator = translated(0.25, 0.75);
    let lut = ImageFilterLut::new_with_filter(&ImageFilterBicubic, true);
    let mut filter = SpanImageFilterRgb::new(&mut accessor, &mut interpolator, &lut);
    assert_eq!(sample(&mut filter), flat);
}

/// The plain bilinear filter reads through its accessor, as C++'s does: half on the image's last
/// column and half past it takes half the clip accessor's background, not the next row's first
/// pixel.
#[test]
fn bilinear_reads_past_the_edge_through_its_accessor() {
    let image = filled(8, [200, 100, 50]);
    let mut accessor = clip(&image);
    let mut interpolator = translated(4.5, 0.0);
    let mut filter = SpanImageFilterRgbBilinear::new(&mut accessor, &mut interpolator);

    assert_eq!(sample(&mut filter), Rgba8::new(100, 50, 25, 255));
}

fn translated(tx: f64, ty: f64) -> SpanInterpolatorLinear {
    SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty))
}

fn clip(image: &Image) -> ImageAccessorClip<'_, 3> {
    ImageAccessorClip::<3>::new(&image.rows, &[0, 0, 0])
}

fn filled(size: u32, rgb: [u8; 3]) -> Image {
    let mut image = Image::new(size, size, 3, 0);
    for px in image.bytes.as_chunks_mut::<3>().0 {
        px.copy_from_slice(&rgb);
    }
    image
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

fn sample<G: SpanGenerator<Color = Rgba8>>(filter: &mut G) -> Rgba8 {
    let mut span = [CLEAR];
    filter.generate(&mut span, 3, 3, 1);
    span[0]
}

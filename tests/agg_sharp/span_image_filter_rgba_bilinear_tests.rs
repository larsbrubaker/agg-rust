//! agg-sharp's `SpanImageFilterRgbaBilinearTests`
//! (`Tests/Agg.Tests/Agg/SpanImageFilterRgbaBilinearTests.cs`): the plain
//! `span_image_filter_rgba_bilinear`, sampling frame pixel (x, y) through a translation of
//! (tx, ty). Like C++, it reads through the image accessor, so a clamp (agg-rust's
//! `ImageAccessorClone`) decides what lies past the edges.

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::{ImageAccessorClone, ImageSource};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinear;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

/// A translucent image keeps its alpha.
#[test]
fn translucent_source_keeps_its_alpha() {
    let mut image = Image::new(4, 4);
    image.fill([100, 60, 3, 128]);
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    assert_eq!(
        sample(&mut accessor, 1, 1, 0.3, 0.6),
        Rgba8::new(100, 60, 3, 128)
    );
}

/// Sampling past the edges through a clamp accessor repeats the edge pixels.
#[test]
fn sampling_past_the_edges_goes_through_the_accessor() {
    let mut image = Image::new(4, 4);
    image.fill([10, 20, 30, 255]);
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    assert_eq!(
        sample(&mut accessor, 3, 3, 0.5, 0.5),
        Rgba8::new(10, 20, 30, 255)
    );
    assert_eq!(
        sample(&mut accessor, -2, -2, 0.25, 0.75),
        Rgba8::new(10, 20, 30, 255)
    );
}

fn sample<S: ImageSource>(accessor: &mut S, x: i32, y: i32, tx: f64, ty: f64) -> Rgba8 {
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty));
    let mut filter = SpanImageFilterRgbaBilinear::new(accessor, &mut interpolator);
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, x, y, 1);
    span[0]
}

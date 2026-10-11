//! agg-sharp's `SpanImageFilterRgbaNnTests` (`Tests/Agg.Tests/Agg/SpanImageFilterRgbaNnTests.cs`):
//! `span_image_filter_rgba_nn` reads each pixel through its image accessor, as C++ does, so what lies
//! outside the image is the accessor's to say: the edge pixel for a clamp (agg-rust's
//! `ImageAccessorClone`), the background for a clip.

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::image_accessors::{ImageAccessorClip, ImageAccessorClone, ImageSource};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaNn;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

/// Sampling left of the image through a clamp accessor gives the row's first pixel.
#[test]
fn outside_the_image_reads_the_clamped_edge() {
    let image = gradient();
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    assert_eq!(sample(&mut accessor, -10.0, 0.0, 3, 2), pixel(&image, 0, 2));
}

/// Sampling right of and below the image through a clip accessor gives its background color.
#[test]
fn outside_the_image_reads_the_clip_background() {
    let image = gradient();
    let mut accessor = ImageAccessorClip::<4>::new(&image.ra, &[9, 8, 7, 6]);
    assert_eq!(
        sample(&mut accessor, 10.0, -10.0, 1, 1),
        Rgba8::new(9, 8, 7, 6)
    );
}

/// Inside the image the nearest pixel comes through unchanged.
#[test]
fn inside_the_image_reads_the_pixel() {
    let image = gradient();
    let mut accessor = ImageAccessorClone::<4>::new(&image.ra);
    assert_eq!(sample(&mut accessor, 1.0, 1.0, 2, 1), pixel(&image, 3, 2));
}

// A 4x4 opaque image, every pixel different.
fn gradient() -> Image {
    let mut image = Image::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            image.set_pixel(x, y, [(40 + x * 50) as u8, (30 + y * 50) as u8, 200, 255]);
        }
    }
    image
}

fn pixel(image: &Image, x: u32, y: u32) -> Rgba8 {
    let [r, g, b, a] = image.pixel(x, y);
    Rgba8::new(r as u32, g as u32, b as u32, a as u32)
}

fn sample<S: ImageSource>(accessor: &mut S, tx: f64, ty: f64, x: i32, y: i32) -> Rgba8 {
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(tx, ty));
    let mut filter = SpanImageFilterRgbaNn::new(accessor, &mut interpolator);
    filter.prepare();
    let mut span = [Rgba8::new(0, 0, 0, 0)];
    filter.generate(&mut span, x, y, 1);
    span[0]
}

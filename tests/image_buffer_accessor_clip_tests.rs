//! Port of agg-sharp `ImageBufferAccessorClipTests` (Tests/Agg.Tests/Agg).
//!
//! `ImageAccessorClip` is C++'s `image_accessor_clip`: a read outside the
//! image returns the background color, laid out as the image's pixels are,
//! so a filter reading the pixel's components gets it back. The float case
//! reads a 16-byte RGBA `f32` pixel (C++ `pixfmt_rgba128`'s layout).

use agg_rust::color::Rgba8;
use agg_rust::image_accessors::ImageAccessorClip;
use agg_rust::image_filters::{ImageFilterLut, ImageFilterSpline16};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaGen;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

const INSIDE: [u8; 4] = [200, 150, 100, 255];
const BACKGROUND: [u8; 4] = [10, 20, 30, 40];

/// A `w` x `h` buffer of `bpp`-byte pixels, each holding `pixel`.
fn filled(w: u32, h: u32, pixel: &[u8]) -> Vec<u8> {
    pixel
        .iter()
        .copied()
        .cycle()
        .take((w * h) as usize * pixel.len())
        .collect()
}

fn attach(data: &mut [u8], w: u32, h: u32, bpp: u32) -> RowAccessor {
    unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), w, h, (w * bpp) as i32) }
}

#[test]
fn reads_outside_a32_bit_image_are_the_background() {
    let mut data = filled(2, 2, &INSIDE);
    let rbuf = attach(&mut data, 2, 2, 4);
    let mut accessor = ImageAccessorClip::<4>::new(&rbuf, &BACKGROUND);

    // A span starting left of the image: outside, inside, inside, outside on the right.
    assert_eq!(accessor.span(-1, 0, 4), BACKGROUND);
    assert_eq!(accessor.next_x(), INSIDE);
    assert_eq!(accessor.next_x(), INSIDE);
    assert_eq!(accessor.next_x(), BACKGROUND);

    // A span wholly inside, then the next row down runs off the top of the image.
    assert_eq!(accessor.span(0, 1, 2), INSIDE);
    assert_eq!(accessor.next_y(), BACKGROUND);
}

#[test]
fn reads_outside_a24_bit_image_are_the_background() {
    let mut data = filled(2, 2, &INSIDE[..3]);
    let rbuf = attach(&mut data, 2, 2, 3);
    let mut accessor = ImageAccessorClip::<3>::new(&rbuf, &BACKGROUND[..3]);

    assert_eq!(accessor.span(0, -1, 1), &BACKGROUND[..3]);
    assert_eq!(accessor.next_y(), &INSIDE[..3]);
}

fn float_pixel(rgba: [f32; 4]) -> Vec<u8> {
    rgba.iter().flat_map(|v| v.to_ne_bytes()).collect()
}

fn read_float(p: &[u8]) -> (f32, f32, f32, f32) {
    let c = |i: usize| f32::from_ne_bytes([p[i * 4], p[i * 4 + 1], p[i * 4 + 2], p[i * 4 + 3]]);
    (c(0), c(1), c(2), c(3))
}

#[test]
fn reads_outside_a_float_image_are_the_background() {
    let mut data = filled(2, 2, &float_pixel([0.8, 0.6, 0.4, 1.0]));
    let rbuf = attach(&mut data, 2, 2, 16);
    let background = float_pixel([0.1, 0.2, 0.3, 0.4]);
    let mut accessor = ImageAccessorClip::<16>::new(&rbuf, &background);

    assert_eq!(read_float(accessor.span(-1, 0, 3)), (0.1, 0.2, 0.3, 0.4));
    assert_eq!(read_float(accessor.next_x()), (0.8, 0.6, 0.4, 1.0));
    accessor.next_x();
    assert_eq!(read_float(accessor.next_x()), (0.1, 0.2, 0.3, 0.4));
}

/// `span_image_filter_rgba` (spline16) sampling across the image's left
/// edge, halfway between the transparent background and an opaque red
/// column: the filter must read each pixel from the slice the accessor
/// handed back for it, the background's for the outside taps and the
/// image's for the inside ones.
#[test]
fn the_rgba_filter_blends_into_the_background_across_the_edge() {
    let mut data = filled(8, 8, &[255, 0, 0, 255]);
    let rbuf = attach(&mut data, 8, 8, 4);
    let filter = ImageFilterLut::new_with_filter(&ImageFilterSpline16, true);

    // Frame x maps to source x + 0.5, so frame -1 samples the source at -0.5: half outside, half in.
    let mut interpolator = SpanInterpolatorLinear::new(TransAffine::new_translation(0.5, 0.0));
    let mut source = ImageAccessorClip::<4>::new(&rbuf, &[0, 0, 0, 0]);
    let mut generator = SpanImageFilterRgbaGen::new(&mut source, &mut interpolator, &filter);
    let mut span = [Rgba8::new(0, 0, 0, 0); 8];
    generator.generate(&mut span, -4, 4, 8);

    assert_eq!(span[0], Rgba8::new(0, 0, 0, 0));
    assert!((100..=156).contains(&span[3].a), "alpha {}", span[3].a);
    assert_eq!(span[3].r, span[3].a);
    assert_eq!(span[3].g, 0);
    assert_eq!(span[7], Rgba8::new(255, 0, 0, 255));
}

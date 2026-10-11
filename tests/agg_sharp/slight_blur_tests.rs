//! Port of agg-sharp `SlightBlurTests.cs`: C++ AGG's slight_blur (line_thickness.cpp's filter),
//! a 3x3 separable Gaussian whose channels go back to bytes rounded. C++ truncates, which on
//! 8-bit channels darkens blurred pixels by up to a level; agg-rust rounds as agg-sharp does
//! (the divergence is documented at `SlightBlur::calc_value`).

use crate::common::Image;
use agg_rust::basics::RectI;
use agg_rust::blur::{ChannelView, SlightBlur};

const BLACK: [u8; 4] = [0, 0, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn gray(v: u8) -> [u8; 4] {
    [v, v, v, 255]
}

fn blur(image: &mut Image, radius: f64, bounds: RectI) {
    SlightBlur::new(radius).blur(&mut ChannelView::rgba32(&mut image.ra), bounds);
}

/// One white pixel on black at radius 1 (weights 0.0177, 0.9647, 0.0177): across, 255 becomes
/// 5, 246, 5; down, the center column becomes 4, 237, 4 and the pixels beside the center 5.
/// Truncating gives 4, 245, 4 across, then 236 at the center and 3 beside it.
#[test]
fn impulse_spreads_to_its_neighbours_rounded() {
    let mut image = Image::new(5, 5);
    image.fill(BLACK);
    image.set_pixel(2, 2, WHITE);

    blur(&mut image, 1.0, RectI::new(0, 0, 4, 4));

    assert_eq!(image.pixel(2, 2), gray(237));
    assert_eq!(image.pixel(1, 2), gray(5));
    assert_eq!(image.pixel(3, 2), gray(5));
    assert_eq!(image.pixel(2, 1), gray(4));
    assert_eq!(image.pixel(2, 3), gray(4));
    assert_eq!(image.pixel(1, 1), gray(0));
    assert_eq!(image.pixel(0, 2), gray(0));
}

/// The edges repeat their own pixels, so a flat image - white included - comes back unchanged.
#[test]
fn flat_image_is_unchanged() {
    for radius in [0.5, 1.5, 2.0] {
        let mut image = Image::new(7, 6);
        image.fill(WHITE);

        blur(&mut image, radius, RectI::new(0, 0, 6, 5));

        for y in 0..image.height {
            for x in 0..image.width {
                assert_eq!(image.pixel(x, y), WHITE, "radius {radius} at {x},{y}");
            }
        }
    }
}

/// Only the inclusive bounds are blurred, and a box under three pixels wide or high not at all,
/// as in C++.
#[test]
fn blurs_only_inside_the_bounds() {
    let mut image = Image::new(8, 8);
    image.fill(BLACK);
    image.set_pixel(4, 4, WHITE);
    image.set_pixel(1, 1, WHITE);

    blur(&mut image, 1.0, RectI::new(3, 3, 5, 5));
    blur(&mut image, 1.0, RectI::new(0, 0, 1, 7));

    assert_eq!(image.pixel(4, 4), gray(237));
    assert_eq!(image.pixel(4, 5), gray(4));
    assert_eq!(image.pixel(4, 6), gray(0));
    assert_eq!(image.pixel(1, 1), WHITE);
    assert_eq!(image.pixel(2, 1), gray(0));
}

/// Divergence regression: each pass rounds the weighted sum to the nearest level, where C++'s
/// `value_type(...)` truncates it. The center of a lone white pixel is the rounded sum of the
/// rounded row pass, one level above what truncation gives.
#[test]
fn rust_only_slight_blur_rounds_where_cpp_truncates() {
    let filter = SlightBlur::new(1.0);
    let (g0, g1) = (filter.center_weight(), filter.neighbor_weight());
    let across = g0 * 255.0;
    let rounded_across = (across + 0.5) as u8;
    let rounded = (g0 * rounded_across as f64 + 0.5) as u8;
    let truncated = (g0 * (across as u8) as f64) as u8;
    assert_eq!((rounded, truncated), (237, 236));
    assert!(g1 > 0.0);

    let mut image = Image::new(3, 3);
    image.fill(BLACK);
    image.set_pixel(1, 1, WHITE);
    blur(&mut image, 1.0, RectI::new(0, 0, 2, 2));
    assert_eq!(image.pixel(1, 1), gray(rounded));
}

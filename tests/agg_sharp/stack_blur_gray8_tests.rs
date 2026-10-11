//! Port of agg-sharp `StackBlurGray8Tests.cs`: C++ AGG's stack_blur_gray8, the stack blur of one
//! byte per pixel, as blend_color blurs its shadow.

use crate::common::{GrayImage, Image};
use agg_rust::blur::{stack_blur_gray8, stack_blur_rgba32, ChannelView};

/// A single white pixel at radius 1: the [1 2 1] / 4 kernel each way, truncated after each pass.
#[test]
fn impulse_spreads_by_the_truncated_kernel() {
    let mut image = GrayImage::new(5, 5);
    image.set(2, 2, 255);

    stack_blur_gray8(&mut ChannelView::gray8(&mut image.ra), 1, 1);

    // Across: 63 127 63. Then down each column: 127 -> 31 63 31, 63 -> 15 31 15.
    let expected = [
        [0, 0, 0, 0, 0],
        [0, 15, 31, 15, 0],
        [0, 31, 63, 31, 0],
        [0, 15, 31, 15, 0],
        [0, 0, 0, 0, 0],
    ];
    for (y, row) in expected.iter().enumerate() {
        for (x, &v) in row.iter().enumerate() {
            assert_eq!(image.get(x as u32, y as u32), v, "at {x},{y}");
        }
    }
}

/// The gray blur is the colour stack blur's arithmetic on one channel, so a random image's green
/// channel blurred as gray matches the colour blur's green - edges and a radius wider than the
/// image included. (C# draws the image from `System.Random(radius)`; a seeded generator of our
/// own stands in, since the property holds for any image.)
#[test]
fn matches_the_colour_stack_blur_per_channel() {
    for radius in [1u32, 3, 7, 40] {
        let mut seed = radius.wrapping_mul(2_654_435_761).wrapping_add(1);
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as u8
        };
        let mut color = Image::new(23, 17);
        let mut gray = GrayImage::new(23, 17);
        for y in 0..17 {
            for x in 0..23 {
                let v = next();
                color.set_pixel(x, y, [0, v, 0, 255]);
                gray.set(x, y, v);
            }
        }

        stack_blur_rgba32(&mut color.ra, radius, radius);
        stack_blur_gray8(&mut ChannelView::gray8(&mut gray.ra), radius, radius);

        for y in 0..17 {
            for x in 0..23 {
                assert_eq!(
                    gray.get(x, y),
                    color.pixel(x, y)[1],
                    "radius {radius} at {x},{y}"
                );
            }
        }
    }
}

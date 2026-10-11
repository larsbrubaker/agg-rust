//! Port of agg-sharp `RecursiveBlurTests.cs`: the recursive (Gaussian) blur's calculators round
//! and clamp a channel back to a byte. C++ AGG 2.6 truncates, and its weights sum to just under
//! 1, so a white area came out 254 after one pass and 253 after both; agg-rust rounds and clamps
//! as agg-sharp does (the divergence is documented at `RecursiveBlurCalc::to_pix`).

use crate::common::Image;
use agg_rust::blur::{recursive_blur_rgba32, ChannelView, RecursiveBlur};

const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn flat_white_stays_white_and_opaque() {
    for radius in [3.0, 15.0, 25.0] {
        let mut image = Image::new(60, 40);
        image.fill(WHITE);

        // recursive_blur_calc_rgb: three channels, alpha untouched.
        RecursiveBlur::<3>::new().blur(&mut ChannelView::rgba32(&mut image.ra), radius);

        for y in 0..image.height {
            for x in 0..image.width {
                assert_eq!(image.pixel(x, y), WHITE, "radius {radius} at {x},{y}");
            }
        }
    }
}

/// A grey calculator over one channel of a colour image writes back the value it blurred, so a
/// flat channel stays flat and the other channels are untouched.
#[test]
fn gray_blur_of_one_channel_keeps_a_flat_channel_and_the_others() {
    let mut image = Image::new(30, 20);
    image.fill([200, 100, 50, 255]);

    // C++ pixfmt_alpha_blend_gray<blender_gray8, rendering_buffer, 4, 1>: the green channel.
    let mut green = ChannelView::channel(&mut image.ra, 4, 1);
    RecursiveBlur::<1>::new().blur(&mut green, 10.0);

    assert_eq!(image.pixel(15, 10), [200, 100, 50, 255]);
    assert_eq!(image.pixel(0, 0), [200, 100, 50, 255]);
}

/// Divergence regression: every flat level survives both passes of the RGBA calculator,
/// alpha included. C++'s truncation drops each one (white to 253).
#[test]
fn rust_only_recursive_blur_rounds_where_cpp_truncates() {
    for value in [1u8, 77, 128, 200, 254, 255] {
        let mut image = Image::new(40, 30);
        let color = [value, 255 - value, value / 2, value];
        image.fill(color);

        recursive_blur_rgba32(&mut image.ra, 12.0);

        for y in 0..image.height {
            for x in 0..image.width {
                assert_eq!(image.pixel(x, y), color, "value {value} at {x},{y}");
            }
        }
    }
}

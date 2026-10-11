//! Port of agg-sharp `ApplyGammaInvTests` (Tests/Agg.Tests/Agg/ApplyGammaInvTests.cs):
//! C++ pixfmt `apply_gamma_inv` / `apply_gamma_dir` on RGB24 and RGBA32.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::gamma::GammaLut;
use agg_rust::pixfmt_rgb::PixfmtRgb24;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32};

/// C++ pixfmt apply_gamma_inv: every pixel's red, green and blue through gamma.inv; alpha is left alone.
fn check_inverse_gamma(bit_depth: u32) {
    let gamma = GammaLut::new_with_gamma(2.0);
    let color = Rgba8::new(10, 100, 200, 77);
    let mut image = Image::new(3, 2, bit_depth / 8, 0);
    let pixel = if bit_depth == 24 {
        let mut pf = PixfmtRgb24::new(&mut image.rows);
        pf.copy_pixel(1, 1, &color);
        pf.apply_gamma_inv(&gamma);
        pf.pixel(1, 1)
    } else {
        let mut pf = PixfmtRgba32::new(&mut image.rows);
        pf.copy_pixel(1, 1, &color);
        pf.apply_gamma_inv(&gamma);
        pf.pixel(1, 1)
    };

    assert_eq!(pixel.r, gamma.inv(10));
    assert_eq!(pixel.g, gamma.inv(100));
    assert_eq!(pixel.b, gamma.inv(200));
    if bit_depth == 32 {
        assert_eq!(pixel.a, 77);
    }
}

/// C# `[Arguments(24)]` and `[Arguments(32)]`.
#[test]
fn maps_color_channels_through_the_inverse_gamma() {
    check_inverse_gamma(24);
    check_inverse_gamma(32);
}

/// C++ pixfmt apply_gamma_dir (pattern_resample's picture): red, green and blue through gamma.dir.
#[test]
fn maps_color_channels_through_the_direct_gamma() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = Image::new(3, 2, 3, 0);
    let mut pf = PixfmtRgb24::new(&mut image.rows);
    pf.copy_pixel(1, 1, &Rgba8::new(10, 100, 200, 255));

    pf.apply_gamma_dir(&gamma);

    let pixel = pf.pixel(1, 1);
    assert_eq!(pixel.r, gamma.dir(10));
    assert_eq!(pixel.g, gamma.dir(100));
    assert_eq!(pixel.b, gamma.dir(200));
}

//! Port of agg-sharp `BlenderGammaBGRTests` (Tests/Agg.Tests/Agg/BlenderGammaBGRTests.cs).
//!
//! `PixfmtRgb24Gamma` against C++ blender_rgb_gamma (agg_pixfmt_rgb.h) on a 24-bit buffer, with a
//! gamma_lut<int8u, int8u, 8, 8>(2.0): dir(100) = 39, inv(19) = 70. The same expectations as
//! agg-sharp's BlenderGammaBGRATests, worked by hand from the C++ source. agg-sharp's
//! `blend_hline(x1, y, x2, ...)` is inclusive of x2, so `x1..=x2` becomes a length here.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::gamma::GammaLut;
use agg_rust::pixfmt_rgb_blenders::PixfmtRgb24Gamma;
use agg_rust::pixfmt_rgba::PixelFormat;

fn gray(level: u8) -> Image {
    Image::new(3, 1, 3, level)
}

fn rgb(r: u32, g: u32, b: u32) -> Rgba8 {
    Rgba8::new(r, g, b, 255)
}

/// Black at half cover over 100 gray is inv((0 - 39) * 128 >> 8 + 39) = inv(19) = 70.
#[test]
fn partial_cover_blends_through_dir_and_inv_as_cpp() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = gray(100);
    let mut pf = PixfmtRgb24Gamma::new_with_gamma(&mut image.rows, &gamma);

    pf.blend_hline(0, 0, 1, &Rgba8::new(0, 0, 0, 255), 128);

    assert_eq!(pf.pixel(0, 0), rgb(70, 70, 70));
}

/// An opaque color at full cover is copied untouched - no inv on the way in.
#[test]
fn opaque_full_cover_copies_the_color() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = gray(100);
    let mut pf = PixfmtRgb24Gamma::new_with_gamma(&mut image.rows, &gamma);

    pf.blend_hline(0, 0, 1, &Rgba8::new(200, 10, 30, 255), 255);

    assert_eq!(pf.pixel(0, 0), rgb(200, 10, 30));
}

/// C++ copy_hline writes the color as is to every pixel of the run.
#[test]
fn copy_hline_copies_the_color_to_every_pixel() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = gray(100);
    let mut pf = PixfmtRgb24Gamma::new_with_gamma(&mut image.rows, &gamma);

    pf.copy_hline(0, 0, 3, &Rgba8::new(200, 10, 30, 255));

    for x in 0..3 {
        assert_eq!(pf.pixel(x, 0), rgb(200, 10, 30));
    }
}

/// A color whose alpha a cover brings to 0 (alpha 1, cover 1; alpha 100, cover 1) leaves the pixel
/// alone, where C++ would write inv(dir(10)) = 0 (the C++ bug `BlenderRgbGamma::blend_pix` fixes).
#[test]
fn cover_that_zeroes_the_alpha_leaves_the_pixel() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = gray(10);
    let mut pf = PixfmtRgb24Gamma::new_with_gamma(&mut image.rows, &gamma);

    pf.blend_hline(0, 0, 1, &Rgba8::new(0, 0, 0, 1), 1);
    pf.blend_hline(1, 0, 1, &Rgba8::new(0, 0, 0, 100), 1);

    assert_eq!(pf.pixel(0, 0), rgb(10, 10, 10));
    assert_eq!(pf.pixel(1, 0), rgb(10, 10, 10));
}

/// C++ blend_color_hspan: a translucent color blends, a transparent one is skipped, an opaque one copied.
#[test]
fn color_span_blends_each_pixel_by_its_alpha() {
    let gamma = GammaLut::new_with_gamma(2.0);
    let mut image = gray(100);
    let mut pf = PixfmtRgb24Gamma::new_with_gamma(&mut image.rows, &gamma);
    let colors = [
        Rgba8::new(0, 0, 0, 128),
        Rgba8::new(0, 0, 0, 0),
        Rgba8::new(200, 10, 30, 255),
    ];

    // agg-sharp's firstCoverForAll with cover 255 is C++'s null covers at cover_mask.
    pf.blend_color_hspan(0, 0, 3, &colors, &[], 255);

    assert_eq!(pf.pixel(0, 0), rgb(70, 70, 70));
    assert_eq!(pf.pixel(1, 0), rgb(100, 100, 100));
    assert_eq!(pf.pixel(2, 0), rgb(200, 10, 30));
}

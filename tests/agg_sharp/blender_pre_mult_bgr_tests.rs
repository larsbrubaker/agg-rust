//! Port of agg-sharp `BlenderPreMultBGRTests` (Tests/Agg.Tests/Agg/BlenderPreMultBGRTests.cs):
//! `PixfmtRgb24Pre` against C++ blender_rgb_pre.
//!
//! Not ported: `StraightAlphaColorSaturatesInsteadOfWrapping` asserts agg-sharp's saturating sum
//! (255, 127, 127); C++ `prelerp` computes in `value_type` and wraps, giving red 126 for that
//! non-premultiplied input, and this crate keeps the C++ result.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgb_blenders::PixfmtRgb24Pre;
use agg_rust::pixfmt_rgba::PixelFormat;

/// An opaque color at half cover over white blends as C++ blender_rgb_pre does: the color and alpha are
/// scaled by the cover (mult_cover), then prelerp.
#[test]
fn partial_cover_scales_an_opaque_color() {
    let mut image = Image::new(1, 1, 3, 255);
    let mut pf = PixfmtRgb24Pre::new(&mut image.rows);

    pf.blend_color_hspan(0, 0, 1, &[Rgba8::new(200, 100, 50, 255)], &[128], 255);

    assert_eq!(pf.pixel(0, 0), Rgba8::new(227, 177, 152, 255));
}

/// The same with one cover for the whole span, which must reach every pixel.
#[test]
fn partial_first_cover_for_all_scales_an_opaque_color() {
    let mut image = Image::new(2, 1, 3, 255);
    let mut pf = PixfmtRgb24Pre::new(&mut image.rows);
    let colors = [Rgba8::new(200, 100, 50, 255), Rgba8::new(200, 100, 50, 255)];

    // agg-sharp's firstCoverForAll is C++'s null covers with one uniform cover.
    pf.blend_color_hspan(0, 0, 2, &colors, &[], 128);

    assert_eq!(pf.pixel(1, 0), Rgba8::new(227, 177, 152, 255));
}

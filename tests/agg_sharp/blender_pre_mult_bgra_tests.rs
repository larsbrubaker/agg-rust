//! Port of agg-sharp `BlenderPreMultBGRATests` (Tests/Agg.Tests/Agg/BlenderPreMultBGRATests.cs):
//! `PixfmtRgba32Pre` against C++ blender_rgba_pre.
//!
//! Not ported: `PolyColorFullCoverIsExactSourceOver` drives agg-sharp's
//! `BlenderPolyColorPreMultBGRA`, which has no C++ AGG counterpart.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32Pre};

fn blend_one(destination: [u8; 4], color: Rgba8, covers: &[u8], cover: u8) -> Rgba8 {
    let mut image = Image::new(1, 1, 4, 0);
    image.bytes.copy_from_slice(&destination);
    let mut pf = PixfmtRgba32Pre::new(&mut image.rows);
    pf.blend_color_hspan(0, 0, 1, &[color], covers, cover);
    pf.pixel(0, 0)
}

/// An opaque color at half cover over white blends as C++ blender_rgba_pre does: the color and alpha are
/// scaled by the cover (mult_cover), then prelerp.
#[test]
fn partial_cover_scales_an_opaque_color() {
    let result = blend_one([255; 4], Rgba8::new(200, 100, 50, 255), &[128], 255);

    assert_eq!(result, Rgba8::new(227, 177, 152, 255));
}

/// A straight-alpha color (red above its alpha) is not valid premultiplied input, but it must not
/// wrap: red sums to 319 and saturates at 255 rather than wrapping to 63 (as C++ `prelerp` does in
/// `value_type`), a dark speck.
#[test]
fn straight_alpha_color_saturates_instead_of_wrapping() {
    let result = blend_one([255; 4], Rgba8::new(255, 0, 0, 128), &[128], 255);

    assert_eq!(result, Rgba8::new(255, 191, 191, 255));
}

/// The same with one cover for the whole span.
#[test]
fn partial_first_cover_for_all_scales_an_opaque_color() {
    let mut image = Image::new(2, 1, 4, 255);
    let mut pf = PixfmtRgba32Pre::new(&mut image.rows);
    let colors = [Rgba8::new(200, 100, 50, 255), Rgba8::new(200, 100, 50, 255)];

    pf.blend_color_hspan(0, 0, 2, &colors, &[], 128);

    assert_eq!(pf.pixel(1, 0), Rgba8::new(227, 177, 152, 255));
}

/// A non-opaque color at full cover blends by exact source-over: each channel is
/// prelerp(p, q, a) = p + q - multiply(p, a). Gray 9 under half-transparent black is 4.
#[test]
fn full_cover_of_a_translucent_color_is_exact_source_over() {
    let result = blend_one([9, 9, 9, 255], Rgba8::new(0, 0, 0, 128), &[255], 255);

    assert_eq!(result, Rgba8::new(4, 4, 4, 255));
}

/// Full cover agrees with prelerp for a spread of destinations and translucent colors, on both span paths.
#[test]
fn full_cover_matches_prelerp_for_several_colors() {
    let destinations = [
        Rgba8::new(255, 255, 255, 255),
        Rgba8::new(10, 200, 90, 180),
        Rgba8::new(0, 0, 0, 0),
        Rgba8::new(128, 64, 32, 255),
    ];
    let sources = [
        Rgba8::new(0, 0, 0, 128),
        Rgba8::new(100, 50, 20, 100),
        Rgba8::new(1, 2, 3, 3),
        Rgba8::new(200, 180, 160, 254),
        Rgba8::new(60, 120, 180, 200),
    ];
    // agg-sharp's Rgba8Math.Prelerp in int, clamped as the C# test does (the clamp never binds for
    // these premultiplied sources).
    let prelerp = |p: u8, q: u8, a: u8| -> u32 {
        (p as u32 + q as u32 - Rgba8::multiply(p, a) as u32).min(255)
    };
    for d in destinations {
        for s in sources {
            let expected = Rgba8::new(
                prelerp(d.r, s.r, s.a),
                prelerp(d.g, s.g, s.a),
                prelerp(d.b, s.b, s.a),
                prelerp(d.a, s.a, s.a),
            );
            let destination = [d.r, d.g, d.b, d.a];
            // firstCoverForAll false: one cover per pixel; true: C++'s null covers at cover_mask.
            assert_eq!(
                blend_one(destination, s, &[255], 255),
                expected,
                "{d:?} {s:?}"
            );
            assert_eq!(blend_one(destination, s, &[], 255), expected, "{d:?} {s:?}");
        }
    }
}

//! Port of agg-sharp `BlenderTableTests` (Tests/Agg.Tests/Agg/BlenderTableTests.cs).
//!
//! Not ported:
//! - `BlenderPreMultBGRABlendPixelMatchesReferenceAlgorithm` pins agg-sharp's table-driven
//!   `(p * (255 - a) + 255) >> 8` saturating blend, which is not C++ blender_rgba_pre (prelerp).
//! - `SubtractLookupClampsCorrectly` drives agg-sharp's ImageProcessing `Subtract`, which has no
//!   C++ AGG counterpart.
//! - `ConcurrentBlenderConstructionProducesConsistentResults` guards the C# blenders' lazily built
//!   static saturation tables (and agg-sharp-only gray blenders); this crate has no such tables.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32Pre};

/// A fully transparent source pixel followed by two opaque ones, at a uniform cover of 128: the
/// transparent pixel is skipped without losing the span's alignment, so the opaque colors still land
/// on pixels 1 and 2, byte-identical to blending them on their own.
#[test]
fn blend_pixels_with_transparent_source_keeps_remaining_pixels_aligned() {
    let source_colors = [
        Rgba8::new(0, 0, 0, 0),
        Rgba8::new(200, 0, 0, 255),
        Rgba8::new(0, 200, 0, 255),
    ];

    let mut dest = Image::new(3, 1, 4, 0);
    PixfmtRgba32Pre::new(&mut dest.rows).blend_color_hspan(0, 0, 3, &source_colors, &[], 128);

    // Pixel 0 came from the transparent color, so it must be untouched.
    assert_eq!(&dest.bytes[0..4], &[0, 0, 0, 0]);

    // Pixel 1 is the red source, pixel 2 the green one.
    assert!(dest.bytes[4] > 0);
    assert_eq!(dest.bytes[4 + 1], 0);
    assert!(dest.bytes[8 + 1] > 0);
    assert_eq!(dest.bytes[8], 0);

    let mut without_transparent = Image::new(2, 1, 4, 0);
    PixfmtRgba32Pre::new(&mut without_transparent.rows).blend_color_hspan(
        0,
        0,
        2,
        &source_colors[1..],
        &[],
        128,
    );

    assert_eq!(&dest.bytes[4..12], &without_transparent.bytes[..]);
}

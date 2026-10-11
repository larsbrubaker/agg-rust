//! Port of agg-sharp `BlenderCompOpBGRATests` (Tests/Agg.Tests/Agg/BlenderCompOpBGRATests.cs):
//! `PixfmtRgba32CompOp` against C++ comp_op_adaptor_rgba<rgba8, order_bgra>.
//!
//! Not ported: `BlendMatchesCppAgg` pins the reference renderer's *patched* agg_pixfmt_rgba.h.
//! 145 of its 150 cases match this crate; the other five are the two C++ bugs agg-sharp fixed and
//! this crate keeps for byte-identity with compiled C++ AGG: comp_op_rgba_src_atop computes blue
//! from the destination's green (agg_pixfmt_rgba.h:530, the four translucent SrcAtop cases), and
//! comp_op_rgba_color_burn of black over white burns the channels to 0 instead of leaving 255.

use crate::support::Image;
use agg_rust::color::Rgba8;
use agg_rust::comp_op::{CompOp, PixfmtRgba32CompOp};
use agg_rust::pixfmt_rgba::PixelFormat;

/// A span goes through the blender's operator pixel by pixel, each at its own cover.
#[test]
fn blend_pixels_uses_the_operator_and_each_cover() {
    let mut image = Image::new(2, 1, 4, 255);
    let mut pf = PixfmtRgba32CompOp::new_with_op(&mut image.rows, CompOp::Difference);
    let colors = [Rgba8::new(0, 0, 255, 255), Rgba8::new(0, 0, 255, 255)];

    pf.blend_color_hspan(0, 0, 2, &colors, &[255, 0], 255);

    assert_eq!(pf.pixel(0, 0), Rgba8::new(255, 255, 0, 255));
    assert_eq!(pf.pixel(1, 0), Rgba8::new(255, 255, 255, 255));
}

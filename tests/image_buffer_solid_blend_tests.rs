//! Port of agg-sharp `ImageBufferSolidBlendTests` (Tests/Agg.Tests/Agg).
//!
//! C++ pixfmt's solid blends (`blend_solid_hspan`, `blend_hline`,
//! `blend_vline`, `blend_pixel`) hand the cover to the blender with the
//! color. A premultiplied blender scales the color by the cover as well as
//! the alpha; folding the cover into the alpha alone adds the whole color to
//! a partly covered pixel, so premultiplied anti-aliased edges come out too
//! light. The color is (102, 77, 26), C++ rasterizers2's line color, at
//! cover 128 over (200, 200, 200).
//!
//! The C# premultiplied cases draw into a 24-bit BGR image; agg-rust has no
//! `pixfmt_rgb24_pre`, so they draw into `PixfmtRgba32Pre`, whose blender
//! runs the same per-channel `prelerp` (its alpha over an opaque pixel stays
//! 255). The draws go through `RendererBase`, whose vertical spans and lines
//! blend pixel by pixel as C++ pixfmt's `blend_solid_vspan`/`blend_vline` do.

use agg_rust::basics::CoverType;
use agg_rust::color::Rgba8;
use agg_rust::comp_op::{CompOp, PixfmtRgba32CompOp};
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32, PixfmtRgba32Pre};
use agg_rust::renderer_base::RendererBase;
use agg_rust::rendering_buffer::RowAccessor;

const LINE_COLOR: Rgba8 = Rgba8 {
    r: 102,
    g: 77,
    b: 26,
    a: 255,
};
const BACKGROUND: Rgba8 = Rgba8 {
    r: 200,
    g: 200,
    b: 200,
    a: 255,
};

/// C++ blender_rgb_pre: prelerp(200, multiply(c, 128), multiply(255, 128)).
const PREMULTIPLIED_RESULT: Rgba8 = Rgba8 {
    r: 151,
    g: 139,
    b: 113,
    a: 255,
};

/// C++ blender_rgba: lerp(200, c, multiply(255, 128)) - unchanged by passing the cover through.
const STRAIGHT_RESULT: Rgba8 = Rgba8 {
    r: 151,
    g: 138,
    b: 113,
    a: 255,
};

/// Runs `$draw` on a 2x2 image of pixel format `$pf::new(rbuf)` (or
/// `$pf::new_with_op(rbuf, op)`) filled with the background by
/// `copy_hline`, then hands back its pixels row by row.
macro_rules! with_image {
    ($make:expr, $draw:expr) => {{
        let mut data = vec![0u8; 2 * 2 * 4];
        let mut rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 2, 2, 8) };
        let mut ren = RendererBase::new($make(&mut rbuf));
        for y in 0..2 {
            ren.copy_hline(0, y, 1, &BACKGROUND);
        }
        $draw(&mut ren);
        let p = |x, y| ren.ren().pixel(x, y);
        [p(0, 0), p(1, 0), p(0, 1), p(1, 1)]
    }};
}

fn pre(draw: impl FnOnce(&mut RendererBase<PixfmtRgba32Pre>)) -> [Rgba8; 4] {
    with_image!(PixfmtRgba32Pre::new, draw)
}

fn straight(draw: impl FnOnce(&mut RendererBase<PixfmtRgba32>)) -> [Rgba8; 4] {
    with_image!(PixfmtRgba32::new, draw)
}

fn comp(op: CompOp, draw: impl FnOnce(&mut RendererBase<PixfmtRgba32CompOp>)) -> [Rgba8; 4] {
    with_image!(|r| PixfmtRgba32CompOp::new_with_op(r, op), draw)
}

#[test]
fn premultiplied_solid_hspan_scales_color_by_cover() {
    let image = pre(|ren| ren.blend_solid_hspan(0, 0, 1, &LINE_COLOR, &[128]));
    assert_eq!(image[0], PREMULTIPLIED_RESULT);
}

#[test]
fn premultiplied_solid_vspan_scales_color_by_cover() {
    let image = pre(|ren| ren.blend_solid_vspan(0, 0, 1, &LINE_COLOR, &[128]));
    assert_eq!(image[0], PREMULTIPLIED_RESULT);
}

#[test]
fn premultiplied_hline_and_vline_scale_color_by_cover() {
    let image = pre(|ren| {
        ren.blend_hline(0, 0, 1, &LINE_COLOR, 128);
        ren.blend_vline(0, 1, 1, &LINE_COLOR, 128);
    });
    assert_eq!(image[0], PREMULTIPLIED_RESULT);
    assert_eq!(image[1], PREMULTIPLIED_RESULT);
    assert_eq!(image[2], PREMULTIPLIED_RESULT);
    assert_eq!(image[3], BACKGROUND);
}

#[test]
fn blend_pixel_blends_at_cover() {
    let image = pre(|ren| ren.blend_pixel(1, 1, &LINE_COLOR, 128));
    assert_eq!(image[3], PREMULTIPLIED_RESULT);
    assert_eq!(image[0], BACKGROUND);

    // Opaque at full cover is a copy.
    let image = pre(|ren| ren.blend_pixel(0, 0, &LINE_COLOR, 255));
    assert_eq!(image[0], LINE_COLOR);
}

#[test]
fn straight_blender_is_unchanged() {
    let image = straight(|ren| {
        ren.blend_solid_hspan(0, 0, 1, &LINE_COLOR, &[128]);
        ren.blend_hline(1, 0, 1, &LINE_COLOR, 128);
    });
    assert_eq!(image[0], STRAIGHT_RESULT);
    assert_eq!(image[1], STRAIGHT_RESULT);
}

#[test]
fn straight_blender_vertical_blends_are_unchanged() {
    let image = straight(|ren| {
        ren.blend_solid_vspan(0, 0, 2, &LINE_COLOR, &[128, 128]);
        ren.blend_vline(1, 0, 1, &LINE_COLOR, 128);
    });
    assert_eq!(image, [STRAIGHT_RESULT; 4]);
}

/// A comp-op format composites every pixel through its operator, as C++
/// `pixfmt_custom_blend_rgba`'s `blend_pix` does: an opaque color at full
/// cover is not simply copied, nor a transparent one skipped.
#[test]
fn comp_op_multiply_blends_an_opaque_color_at_full_cover() {
    assert_comp_op(CompOp::Multiply, LINE_COLOR, 255);
}

#[test]
fn comp_op_src_with_a_transparent_color_clears() {
    let image = comp(CompOp::Src, |ren| {
        ren.blend_hline(0, 0, 1, &Rgba8::new(10, 20, 30, 0), 255)
    });
    assert_eq!(image[0], Rgba8::new(0, 0, 0, 0));
    assert_eq!(image[1], Rgba8::new(0, 0, 0, 0));
}

#[test]
fn comp_op_src_at_partial_cover_blends() {
    assert_comp_op(CompOp::Src, LINE_COLOR, 128);
}

fn assert_comp_op(op: CompOp, color: Rgba8, cover: CoverType) {
    let image = comp(op, |ren| {
        ren.blend_solid_hspan(0, 0, 1, &color, &[cover]);
        ren.blend_hline(1, 0, 1, &color, cover);
        ren.blend_vline(0, 1, 1, &color, cover);
        ren.blend_pixel(1, 1, &color, cover);
    });

    // The operator itself, applied to one pixel of the same background
    // through the pixel format (its blend_pixel is one comp_op_blend).
    let expected = comp(op, |ren| ren.ren_mut().blend_pixel(0, 0, &color, cover))[0];
    assert_ne!(expected, color);
    assert_eq!(image, [expected; 4]);
}

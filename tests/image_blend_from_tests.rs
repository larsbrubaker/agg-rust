//! Port of agg-sharp `ImageBlendFromTests` (Tests/Agg.Tests/Agg).
//!
//! C++ AGG's `renderer_base::blend_from_color` and `blend_from_lut`: a gray
//! image used as coverage (one color) or as an index (a 256-entry color
//! table), placed at an offset and clipped to the destination.

use agg_rust::basics::COVER_FULL;
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32};
use agg_rust::renderer_base::RendererBase;
use agg_rust::rendering_buffer::RowAccessor;

fn background() -> Rgba8 {
    Rgba8::new(255, 242, 242, 255)
}

/// A one-row gray8 buffer of `values` and its accessor.
fn new_gray(values: &mut [u8]) -> RowAccessor {
    let w = values.len() as u32;
    unsafe { RowAccessor::new_with_buf(values.as_mut_ptr(), w, 1, w as i32) }
}

/// Runs `draw` on a `w` x `h` RGBA destination cleared to the background,
/// then hands back the destination's pixels.
fn with_dest(w: u32, h: u32, draw: impl FnOnce(&mut RendererBase<PixfmtRgba32>)) -> Vec<Rgba8> {
    let mut data = vec![0u8; (w * h * 4) as usize];
    let mut rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), w, h, (w * 4) as i32) };
    let mut ren = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
    ren.clear(&background());
    draw(&mut ren);
    (0..h as i32)
        .flat_map(|y| (0..w as i32).map(move |x| (x, y)))
        .map(|(x, y)| ren.ren().pixel(x, y))
        .collect()
}

// C++ blender_rgba::blend_pix over an opaque pixel: each channel lerped by alpha, alpha kept.
fn lerped(p: Rgba8, c: Rgba8, alpha: u8) -> Rgba8 {
    Rgba8::new(
        Rgba8::lerp(p.r, c.r, alpha) as u32,
        Rgba8::lerp(p.g, c.g, alpha) as u32,
        Rgba8::lerp(p.b, c.b, alpha) as u32,
        255,
    )
}

#[test]
fn blend_from_color_uses_each_gray_as_the_colours_cover() {
    let mut values = [0, 128, 255];
    let gray = new_gray(&mut values);
    let color = Rgba8::new(0, 100, 0, 255);

    let dest = with_dest(6, 4, |ren| {
        ren.blend_from_color(&gray, &color, None, 1, 2, COVER_FULL)
    });

    let at = |x: usize, y: usize| dest[y * 6 + x];
    assert_eq!(at(1, 2), background());
    assert_eq!(at(2, 2), lerped(background(), color, 128));
    assert_eq!(at(3, 2), color);
    assert_eq!(at(4, 2), background());
    assert_eq!(at(2, 1), background());
}

#[test]
fn blend_from_color_scales_a_translucent_colours_alpha() {
    let mut values = [255, 128];
    let gray = new_gray(&mut values);
    let color = Rgba8::new(0, 100, 0, 128);

    let dest = with_dest(6, 4, |ren| {
        ren.blend_from_color(&gray, &color, None, 0, 0, COVER_FULL)
    });

    assert_eq!(dest[0], lerped(background(), color, 128));
    assert_eq!(
        dest[1],
        lerped(background(), color, Rgba8::multiply(128, 128))
    );
}

#[test]
fn blend_from_lut_blends_the_colour_each_gray_indexes() {
    let lut: Vec<Rgba8> = (0..256u32)
        .map(|i| Rgba8::new(i, 255 - i, 7, if i < 64 { i * 4 } else { 255 }))
        .collect();
    let mut values = [0, 10, 200];
    let gray = new_gray(&mut values);

    let dest = with_dest(6, 4, |ren| {
        ren.blend_from_lut(&gray, &lut, None, 2, 0, COVER_FULL)
    });

    assert_eq!(dest[2], background());
    assert_eq!(dest[3], lerped(background(), lut[10], 40));
    assert_eq!(dest[4], lut[200]);
}

/// A source hanging off every side of the destination draws only the overlap.
#[test]
fn placement_is_clipped_to_the_destination() {
    let mut values = [0u8; 25];
    values[5 + 1] = 255;
    let gray = unsafe { RowAccessor::new_with_buf(values.as_mut_ptr(), 5, 5, 5) };
    let black = Rgba8::new(0, 0, 0, 255);

    let dest = with_dest(3, 3, |ren| {
        ren.blend_from_color(&gray, &black, None, -1, -1, COVER_FULL);
        ren.blend_from_color(&gray, &black, None, 2, 2, COVER_FULL);
    });

    assert_eq!(dest[0], black);
    assert_eq!(dest[3 + 1], background());
    assert_eq!(dest[2 * 3 + 2], background());
}

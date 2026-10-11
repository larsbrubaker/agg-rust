//! Port of agg-sharp `ImageClippingProxyTests` (Tests/Agg.Tests/Agg).
//!
//! agg-sharp's `ImageClippingProxy` is C++'s `renderer_base`: a clip box
//! over a pixel format. C# `copy_hline`/`copy_vline` take a length;
//! `renderer_base` takes inclusive end coordinates, so a C# run of `len`
//! from `x` is `x..=x + len - 1` here. A C# proxy over a proxy clips to the
//! intersection of both boxes; `renderer_base` wraps a pixel format, not
//! another renderer, so that case is the outer box over the image.

use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32};
use agg_rust::renderer_base::RendererBase;
use agg_rust::rendering_buffer::RowAccessor;

fn attach(data: &mut [u8], w: u32, h: u32) -> RowAccessor {
    unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), w, h, (w * 4) as i32) }
}

fn black() -> Rgba8 {
    Rgba8::new(0, 0, 0, 255)
}

/// '0' for each transparent pixel of the buffer, '1' for each other one.
fn alphas(data: &[u8]) -> String {
    data.chunks(4)
        .map(|p| if p[3] == 0 { '0' } else { '1' })
        .collect()
}

fn ramp() -> Vec<Rgba8> {
    (0..5).map(|i| Rgba8::new(10 * i, 0, 0, 255)).collect()
}

/// A color span with one cover for all (C++'s `covers == 0`, as
/// `renderer_outline_image` passes) that starts left of the clip box skips
/// its clipped colors but keeps its one cover.
#[test]
fn color_hspan_with_one_cover_clips_its_colors_not_its_cover() {
    let mut data = vec![0u8; 6 * 4];
    let mut rbuf = attach(&mut data, 6, 1);
    let mut proxy = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
    proxy.clip_box_i(2, 0, 5, 0);

    proxy.blend_color_hspan(0, 0, 5, &ramp(), &[], 255);

    assert_eq!(proxy.ren().pixel(1, 0).r, 0);
    assert_eq!(proxy.ren().pixel(2, 0).r, 20);
    assert_eq!(proxy.ren().pixel(4, 0).r, 40);
}

/// The same for a vertical span starting below the clip box.
#[test]
fn color_vspan_with_one_cover_clips_its_colors_not_its_cover() {
    let mut data = vec![0u8; 6 * 4];
    let mut rbuf = attach(&mut data, 1, 6);
    let mut proxy = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
    proxy.clip_box_i(0, 2, 0, 5);

    proxy.blend_color_vspan(0, 0, 5, &ramp(), &[], 255);

    assert_eq!(proxy.ren().pixel(0, 1).r, 0);
    assert_eq!(proxy.ren().pixel(0, 2).r, 20);
    assert_eq!(proxy.ren().pixel(0, 4).r, 40);
}

/// A run is clipped to the box and the pixel format gets the clipped length.
#[test]
fn copy_hline_takes_a_length_through_a_proxy_and_a_proxy_over_a_proxy() {
    let mut data = vec![0u8; 10 * 4];
    {
        let mut rbuf = attach(&mut data, 10, 1);
        let mut proxy = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
        proxy.clip_box_i(2, 0, 8, 0);
        proxy.copy_hline(0, 0, 6 - 1, &black());
    }
    assert_eq!(alphas(&data), "0011110000");

    let mut data = vec![0u8; 10 * 4];
    {
        let mut rbuf = attach(&mut data, 10, 1);
        let mut outer = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
        outer.clip_box_i(3, 0, 9, 0);
        outer.copy_hline(4, 0, 4 + 3 - 1, &black());
    }
    assert_eq!(alphas(&data), "0000111000");
}

/// The same for `copy_vline`.
#[test]
fn copy_vline_takes_a_length_through_a_proxy_and_a_proxy_over_a_proxy() {
    let mut data = vec![0u8; 10 * 4];
    {
        let mut rbuf = attach(&mut data, 1, 10);
        let mut proxy = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
        proxy.clip_box_i(0, 2, 0, 8);
        proxy.copy_vline(0, 0, 6 - 1, &black());
    }
    assert_eq!(alphas(&data), "0011110000");

    let mut data = vec![0u8; 10 * 4];
    {
        let mut rbuf = attach(&mut data, 1, 10);
        let mut outer = RendererBase::new(PixfmtRgba32::new(&mut rbuf));
        outer.clip_box_i(0, 3, 0, 9);
        outer.copy_vline(0, 4, 4 + 3 - 1, &black());
    }
    assert_eq!(alphas(&data), "0000111000");
}

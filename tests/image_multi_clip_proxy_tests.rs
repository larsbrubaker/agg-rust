//! Port of agg-sharp `ImageMultiClipProxyTests` (Tests/Agg.Tests/Agg).
//!
//! agg-sharp's `ImageMultiClipProxy` is C++'s `renderer_mclip`: every draw
//! runs once per clip box. C# `copy_hline` takes a length; `renderer_mclip`
//! takes an inclusive end x, so a C# run of `len` from `x` is
//! `x..=x + len - 1` here.

use agg_rust::color::Rgba8;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_mclip::RendererMclip;
use agg_rust::rendering_buffer::RowAccessor;

fn attach(data: &mut [u8], w: u32, h: u32) -> RowAccessor {
    unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), w, h, (w * 4) as i32) }
}

fn black() -> Rgba8 {
    Rgba8::new(0, 0, 0, 255)
}

/// '0' for each transparent pixel of row `y`, '1' for each other one.
fn row_alphas(data: &[u8], w: usize, y: usize) -> String {
    data[y * w * 4..(y + 1) * w * 4]
        .chunks(4)
        .map(|p| if p[3] == 0 { '0' } else { '1' })
        .collect()
}

/// A span across two clip boxes and the gap between them lands only inside
/// the boxes, as C++ `renderer_mclip` draws it once per box.
#[test]
fn span_draws_only_inside_the_boxes() {
    let mut data = vec![0u8; 10 * 4 * 4];
    {
        let mut rbuf = attach(&mut data, 10, 4);
        let mut clipped = RendererMclip::new(RendererBase::new(PixfmtRgba32::new(&mut rbuf)));
        clipped.reset_clipping(false);
        clipped.add_clip_box(1, 0, 2, 3);
        clipped.add_clip_box(6, 0, 7, 3);

        clipped.blend_hline(0, 1, 9, &black(), 255);
    }
    assert_eq!(row_alphas(&data, 10, 1), "0110001100");
}

/// `copy_hline` copies its run into every box it crosses.
#[test]
fn copy_hline_takes_a_length_and_copies_into_every_box() {
    let mut data = vec![0u8; 10 * 4];
    {
        let mut rbuf = attach(&mut data, 10, 1);
        let mut clipped = RendererMclip::new(RendererBase::new(PixfmtRgba32::new(&mut rbuf)));
        clipped.reset_clipping(false);
        clipped.add_clip_box(1, 0, 2, 0);
        clipped.add_clip_box(6, 0, 9, 0);

        clipped.copy_hline(0, 0, 8 - 1, &black());
    }
    assert_eq!(row_alphas(&data, 10, 0), "0110001100");
}

/// A square marker drawn through the multi-clip renderer fills its part of every box, the bar
/// included (renderer_mclip::blend_bar); after `reset_clipping(false)` nothing is visible, and
/// after `reset_clipping(true)` everything is.
#[test]
fn marker_bar_fills_every_box_and_visibility_resets() {
    use agg_rust::pixfmt_rgba::PixelFormat;
    use agg_rust::renderer_markers::{MarkerType, RendererMarkers};

    // '0' for each transparent pixel of row `y`, '1' for each other one, read from the pixel
    // format (renderer_base's own `pixel` answers only inside its current clip box).
    fn alphas(clipped: &RendererMclip<PixfmtRgba32>, y: i32) -> String {
        (0..10)
            .map(|x| {
                if clipped.ren().ren().pixel(x, y).a == 0 {
                    '0'
                } else {
                    '1'
                }
            })
            .collect()
    }

    let mut data = vec![0u8; 10 * 10 * 4];
    let mut rbuf = attach(&mut data, 10, 10);
    let mut clipped = RendererMclip::new(RendererBase::new(PixfmtRgba32::new(&mut rbuf)));
    clipped.reset_clipping(false);
    clipped.add_clip_box(0, 0, 3, 9);
    clipped.add_clip_box(6, 0, 9, 9);
    {
        let mut markers = RendererMarkers::new(&mut clipped);
        markers.set_line_color(black());
        markers.set_fill_color(black());

        markers.marker(5, 5, 4, MarkerType::Square);
    }

    assert_eq!(alphas(&clipped, 5), "0111001111");

    clipped.reset_clipping(false);
    clipped.blend_hline(0, 0, 9, &black(), 255);
    assert_eq!(alphas(&clipped, 0), "0000000000");

    clipped.reset_clipping(true);
    clipped.blend_hline(0, 0, 9, &black(), 255);
    assert_eq!(alphas(&clipped, 0), "1111111111");
}

//! agg-sharp's `AlphaMaskAdaptorTests` (`Tests/Agg.Tests/Agg/AlphaMaskAdaptorTests.cs`):
//! `pixfmt_amask_adaptor` over an RGBA32 pixel format and a gray8 alpha mask, alone and inside a
//! clipping `RendererBase` (agg-sharp's `ImageClippingProxy`).

use agg_rust::alpha_mask_u8::{AlphaMaskGray8, AmaskNoClipGray8, OneComponentMask};
use agg_rust::color::Rgba8;
use agg_rust::pixfmt_amask_adaptor::PixfmtAmaskAdaptor;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32};
use agg_rust::renderer_base::RendererBase;

use crate::common::{GrayImage, Image};

fn black() -> Rgba8 {
    Rgba8::new(0, 0, 0, 255)
}

/// C++ pixfmt_amask_adaptor copies the covers into its own span before multiplying the mask in,
/// so a caller that reuses its covers (one scanline rendered to two targets) still has the
/// unmasked values.
#[test]
fn blend_solid_hspan_leaves_the_callers_covers_alone() {
    let mut mask = GrayImage::new(4, 1);
    mask.set(1, 0, 128);
    let mut destination = Image::new(4, 1);
    {
        let mut adaptor = PixfmtAmaskAdaptor::new(
            PixfmtRgba32::new(&mut destination.ra),
            AlphaMaskGray8::new(&mask.ra, OneComponentMask),
        );

        let covers: [u8; 5] = [7, 255, 255, 255, 9];
        adaptor.blend_solid_hspan(0, 0, 4, &black(), &covers[1..]);

        assert_eq!(covers, [7, 255, 255, 255, 9]);
    }

    // The mask still reaches the pixels: 0 blocks, 128 lets half through.
    assert_eq!(destination.pixel(0, 0)[3], 0);
    assert_eq!(destination.pixel(1, 0)[3], 128);
}

/// renderer_primitives and renderer_markers draw pixels, vertical lines and gradient spans
/// through renderer_base<pixfmt_amask_adaptor>: each is masked, and a pixel off the clip box is
/// dropped.
#[test]
fn pixels_vlines_and_color_spans_are_masked_and_clipped() {
    let mut mask = GrayImage::new(3, 3);
    mask.set(0, 0, 255);
    mask.set(1, 0, 128);
    mask.set(2, 1, 255);
    let mut destination = Image::new(3, 3);
    let covers: [u8; 3] = [255, 255, 255];
    {
        let mut target = RendererBase::new(PixfmtAmaskAdaptor::new(
            PixfmtRgba32::new(&mut destination.ra),
            AmaskNoClipGray8::new(&mask.ra, OneComponentMask),
        ));

        target.blend_pixel(0, 0, &black(), 255);
        target.blend_pixel(1, 0, &black(), 255);
        target.blend_pixel(-1, 0, &black(), 255);
        target.blend_pixel(0, 3, &black(), 255);
        assert_eq!(target.pixel(0, 0).a, 255);
        assert_eq!(target.pixel(1, 0).a, 128);

        // The vline covers rows 0 to 2 of column 2; only row 1 is open in the mask.
        target.blend_vline(2, 0, 2, &black(), 255);
        assert_eq!(target.pixel(2, 0).a, 0);
        assert_eq!(target.pixel(2, 1).a, 255);
        assert_eq!(target.pixel(2, 2).a, 0);

        let colors = [black(), black(), black()];
        target.blend_color_hspan(0, 1, 3, &colors, &covers, 255);
    }
    assert_eq!(destination.pixel(0, 1)[3], 0);
    assert_eq!(covers, [255, 255, 255]);
}

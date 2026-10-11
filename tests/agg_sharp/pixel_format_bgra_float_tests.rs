//! agg-sharp's `PixelFormatBGRAFloatTests` (`Tests/Agg.Tests/Agg/PixelFormatBGRAFloatTests.cs`):
//! C++ `pixfmt_alpha_blend_rgba` over `rgba32` (`pixfmt_bgra128` and `pixfmt_bgra128_pre`)
//! pinned bit for bit to a C++ trace. One sequence touches each operation and its full-cover,
//! opaque and transparent paths.

use agg_rust::color::OrderBgra;
use agg_rust::color::Rgba8;
use agg_rust::color_rgba32::Rgba32;
use agg_rust::pixfmt_rgba::PixelFormat;
use agg_rust::pixfmt_rgba_float::{
    BlenderRgbaF32, BlenderRgbaF32Pre, BlenderRgbaF32Straight, PixfmtAlphaBlendRgbaF32,
};
use agg_rust::renderer_base::RendererBase;

const W: u32 = 8;
const H: u32 = 3;
const STRIDE: usize = W as usize * 4;

#[test]
fn straight_blender_matches_cpp() {
    assert_sequence_matches::<BlenderRgbaF32Straight>(&STRAIGHT_EXPECTED);
}

#[test]
fn premultiplied_blender_matches_cpp() {
    assert_sequence_matches::<BlenderRgbaF32Pre>(&PRE_EXPECTED);
}

fn c(r: f32, g: f32, b: f32, a: f32) -> Rgba32 {
    Rgba32::new(r, g, b, a)
}

fn srgb(r: u8, g: u8, b: u8) -> Rgba32 {
    Rgba32::from_srgba8(&Rgba8::new(r as u32, g as u32, b as u32, 255))
}

fn assert_sequence_matches<B: BlenderRgbaF32>(expected: &[u32]) {
    let mut buf = vec![0f32; STRIDE * H as usize];
    let mut buf2 = vec![0f32; STRIDE * H as usize];
    let mut ren = RendererBase::new(PixfmtAlphaBlendRgbaF32::<B, OrderBgra>::new(
        &mut buf, W, H, STRIDE,
    ));
    let mut ren2 = RendererBase::new(PixfmtAlphaBlendRgbaF32::<B, OrderBgra>::new(
        &mut buf2, W, H, STRIDE,
    ));

    ren.clear(&srgb(255, 255, 255));
    ren.copy_bar(1, 0, 3, 1, &srgb(0xdf, 0xdf, 0xdf));
    ren.copy_bar(-2, 2, 1, 5, &c(0.2, 0.3, 0.4, 0.5));
    let pf = ren.ren_mut();
    pf.blend_hline(0, 0, 8, &c(0.8, 0.4, 0.2, 0.7), 255);
    pf.blend_hline(2, 1, 5, &c(0.1, 0.9, 0.3, 0.6), 128);
    pf.blend_hline(4, 2, 3, &c(0.5, 0.5, 0.5, 1.0), 200);
    let covers: [u8; 8] = [0, 1, 64, 128, 200, 254, 255, 255];
    pf.blend_solid_hspan(0, 1, 8, &c(0.3, 0.6, 0.9, 0.45), &covers);
    pf.blend_solid_hspan(0, 2, 8, &c(0.9, 0.1, 0.4, 1.0), &covers);
    let cs = [
        c(0.0, 0.0, 0.0, 0.0),
        c(1.0, 0.0, 0.0, 1.0),
        c(0.2, 0.4, 0.6, 0.8),
        c(0.9, 0.8, 0.7, 0.3),
        c(0.05, 0.1, 0.15, 0.2),
        c(0.5, 0.25, 0.125, 0.5),
        c(0.3, 0.3, 0.3, 1.0),
        c(0.7, 0.2, 0.9, 0.9),
    ];
    pf.blend_color_hspan(0, 0, 8, &cs, &covers, 255);
    pf.blend_color_hspan(0, 1, 8, &cs, &[], 255);
    pf.blend_color_hspan(0, 2, 8, &cs, &[], 100);
    // C#'s null covers array is agg-rust's empty slice.
    ren2.clear(&c(0.0, 0.0, 0.0, 0.0));
    let pf2 = ren2.ren_mut();
    pf2.blend_color_hspan(0, 0, 8, &cs, &[], 255);
    pf2.blend_color_hspan(0, 1, 8, &cs, &covers, 255);
    pf2.blend_color_hspan(1, 2, 6, &cs, &[], 255);
    ren.blend_from(ren2.ren(), None, 0, 0, 191);
    ren.blend_from(ren2.ren(), None, 1, 1, 255);
    drop(ren);

    assert_eq!(buf.len(), expected.len());
    for (i, (v, e)) in buf.iter().zip(expected).enumerate() {
        assert_eq!(v.to_bits(), *e, "float {i}");
    }
}

const STRAIGHT_EXPECTED: [u32; 96] = [
    0x3EE147AE, 0x3F147AE2, 0x3F5C28F6, 0x3F800000, 0x3DB9052A, 0x3E00597B, 0x3F720213, 0x3F800000,
    0x3EE73FD4, 0x3EC4E1A0, 0x3EB97933, 0x3F800000, 0x3EBBD974, 0x3EF47BA3, 0x3F2E2914, 0x3F800000,
    0x3EAE0801, 0x3EDD3B92, 0x3F1FE886, 0x3F800000, 0x3E4D4E57, 0x3E9D147B, 0x3F04F78C, 0x3F800000,
    0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000, 0x3F5307DE, 0x3E4BACC0, 0x3F287462, 0x3F800000,
    0x3F800000, 0x3F800000, 0x3F800000, 0x3F800000, 0x00000000, 0x00000000, 0x3F7F4041, 0x3F800000,
    0x00000000, 0x00000000, 0x3F800000, 0x3F800000, 0x3F01D55E, 0x3EC99DF4, 0x3E759E6C, 0x3F800000,
    0x3EFC51E2, 0x3F001D5D, 0x3EBF1AA2, 0x3F800000, 0x3E885815, 0x3E9C6B41, 0x3EAC2F11, 0x3F800000,
    0x3E39999A, 0x3E59999A, 0x3E8CCCCD, 0x3F800000, 0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000,
    0x3ECCCCCD, 0x3E99999A, 0x3E4CCCCD, 0x3F000000, 0x3E78F8FA, 0x3E3A3DC2, 0x3F03F0DE, 0x3F32804E,
    0x3E456A4A, 0x3E281FEF, 0x3F6DD5CF, 0x3F800000, 0x3EF4B5AB, 0x3EB57CF6, 0x3EC62AC7, 0x3F800000,
    0x3EAAD671, 0x3E48FDA4, 0x3F151DE2, 0x3F800000, 0x3E831659, 0x3DCA8C8A, 0x3F173C08, 0x3F800000,
    0x3E1FC685, 0x3E10EDB6, 0x3EC26A57, 0x3F800000, 0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000,
];

const PRE_EXPECTED: [u32; 96] = [
    0x3F000000, 0x3F333334, 0x3F8CCCCD, 0x3F800000, 0x3DD7BD5E, 0x3E1F11AE, 0x3F80AF16, 0x3F800000,
    0x3F250D74, 0x3F09F3DE, 0x3EFE8164, 0x3F800000, 0x3F898028, 0x3FA8ED9A, 0x3FD936E0, 0x3F800000,
    0x3F121F6D, 0x3F24B3A9, 0x3F5BFB8E, 0x3F800000, 0x3EA83038, 0x3F10134A, 0x3F8404D2, 0x3F800000,
    0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000, 0x3F7BD426, 0x3E737FCB, 0x3F49CC9D, 0x3F800000,
    0x3F800000, 0x3F800000, 0x3F800000, 0x3F800000, 0x00000000, 0x00000000, 0x3F800000, 0x3F800000,
    0x00000000, 0x00000000, 0x3F800000, 0x3F800000, 0x3F65A330, 0x3F3B87B3, 0x3F001FE9, 0x3F800000,
    0x3FBCC6EE, 0x3FC06F5C, 0x3FA75410, 0x3F800000, 0x3F21316A, 0x3F2F06E9, 0x3F471DF7, 0x3F800000,
    0x3E8CCCCD, 0x3ECCCCCD, 0x3F266666, 0x3F800000, 0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000,
    0x3ECCCCCD, 0x3E99999A, 0x3E4CCCCD, 0x3F000000, 0x3E78F8FA, 0x3E3A3DC2, 0x3F03F0DE, 0x3F32804E,
    0x3E51764A, 0x3E3027EF, 0x3F6FD6CE, 0x3F800000, 0x3F4B925E, 0x3F186458, 0x3F0D2994, 0x3F800000,
    0x3F8D7ECE, 0x3F882214, 0x3FC5CA7C, 0x3F800000, 0x3EF4FD54, 0x3E8A90EF, 0x3F3AB5C4, 0x3F800000,
    0x3E91CF18, 0x3ECC4E86, 0x3F650CD7, 0x3F800000, 0x3E99999A, 0x3E99999A, 0x3E99999A, 0x3F800000,
];

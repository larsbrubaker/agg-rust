//! agg-sharp's `RendererBaseFloatTests` (`Tests/Agg.Tests/Agg/RendererBaseFloatTests.cs`):
//! the scanline renderers over float pixel formats, `span_gradient` with
//! `gradient_linear_color<rgba32>`, and `pixfmt_custom_blend_rgba<comp_op_adaptor_rgba<rgba32>>`,
//! pinned to C++ AGG over rgba32 (`pixfmt_bgra128`) by rendering compositing.cpp's two shapes
//! into a 600x400 float buffer. The expected values are FNV-1a hashes of the whole buffer's float
//! bytes from a C++ trace. The last two cases move the shapes over the image edges, so the
//! renderer_base clipping runs.

use agg_rust::color::{OrderBgra, Rgba8};
use agg_rust::color_rgba32::Rgba32;
use agg_rust::comp_op::CompOp;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLinearColorF32;
use agg_rust::math::calc_distance;
use agg_rust::pixfmt_rgba::PixelFormat;
use agg_rust::pixfmt_rgba_float::{PixfmtBgra128, PixfmtCompOpRgbaF32};
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid};
use agg_rust::rounded_rect::RoundedRect;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_gradient::{GradientX, SpanGradient};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

const WIDTH: u32 = 600;
const HEIGHT: u32 = 400;
const STRIDE: usize = WIDTH as usize * 4;

/// C++ `comp_op_e` by number, as the C# test passes it (agg-sharp's `CompOp` order). C++ AGG 2.6
/// comments `comp_op_minus` out of its enum and operator table, so `Plus` is 12 and `Multiply`
/// 13; agg-sharp appends `Minus` last, and agg-rust's `CompOp` discriminants (`Minus = 13`) are
/// not C++'s numbers.
const OPS: [CompOp; 25] = [
    CompOp::Clear,
    CompOp::Src,
    CompOp::Dst,
    CompOp::SrcOver,
    CompOp::DstOver,
    CompOp::SrcIn,
    CompOp::DstIn,
    CompOp::SrcOut,
    CompOp::DstOut,
    CompOp::SrcAtop,
    CompOp::DstAtop,
    CompOp::Xor,
    CompOp::Plus,
    CompOp::Multiply,
    CompOp::Screen,
    CompOp::Overlay,
    CompOp::Darken,
    CompOp::Lighten,
    CompOp::ColorDodge,
    CompOp::ColorBurn,
    CompOp::HardLight,
    CompOp::SoftLight,
    CompOp::Difference,
    CompOp::Exclusion,
    CompOp::Minus,
];

#[test]
fn compositing_shapes_match_cpp() {
    let cases: [(usize, f64, f64, f64, f64, u64); 6] = [
        (3, 0.75, 1.0, 0.0, 0.0, 0xCE45AAB6230B9F62),
        (19, 0.75, 1.0, 0.0, 0.0, 0xB7A01D17F8E91096),
        (13, 0.5, 0.5, 0.0, 0.0, 0xD67A8428D8DC9A06),
        (0, 0.75, 0.6, 0.0, 0.0, 0xB549935540085C52),
        (21, 0.8, 0.9, -200.0, -150.0, 0x756C7970997D3D79),
        (12, 0.4, 0.7, 330.0, 180.0, 0x4F6BE97A7497567C),
    ];
    for (op, src_alpha, dst_alpha, ox, oy, expected) in cases {
        let hash = render(OPS[op], src_alpha, dst_alpha, ox, oy);
        assert_eq!(
            hash, expected,
            "op {op}, src alpha {src_alpha}, dst alpha {dst_alpha}, offset ({ox}, {oy})"
        );
    }
}

fn render(op: CompOp, src_alpha: f64, dst_alpha: f64, ox: f64, oy: f64) -> u64 {
    let mut buf = vec![0f32; STRIDE * HEIGHT as usize];
    {
        let mut prim = RendererBase::new(PixfmtBgra128::new(&mut buf, WIDTH, HEIGHT, STRIDE));
        prim.clear(&Rgba32::new(0.0, 0.0, 0.0, 0.0));

        // C++ unsigned(alpha * 255): truncation.
        let da = (dst_alpha * 255.0) as u32;
        circle(
            &mut prim,
            srgba(0xFD, 0xF0, 0x6F, da),
            srgba(0xFE, 0x9F, 0x34, da),
            (70.0 * 3.0) + ox,
            100.0 + (24.0 * 3.0) + oy,
            (37.0 * 3.0) + ox,
            100.0 + (79.0 * 3.0) + oy,
            dst_alpha,
        );
    }
    {
        let mut comp = RendererBase::new(PixfmtCompOpRgbaF32::<OrderBgra>::new(
            &mut buf, WIDTH, HEIGHT, STRIDE, op,
        ));
        let sa = (src_alpha * 255.0) as u32;
        src_shape(
            &mut comp,
            srgba(0x7F, 0xC1, 0xFF, sa),
            srgba(0x05, 0x00, 0x5F, sa),
            300.0 + 50.0 + ox,
            100.0 + (24.0 * 3.0) + oy,
            107.0 + 50.0 + ox,
            100.0 + (79.0 * 3.0) + oy,
        );
    }
    fnv1a(&buf)
}

fn srgba(r: u32, g: u32, b: u32, a: u32) -> Rgba32 {
    Rgba32::from_srgba8(&Rgba8::new(r, g, b, a))
}

/// compositing.cpp's `gradient_affine`: gradient_x's 0..d2 run laid along x1,y1 -> x2,y2.
fn gradient_affine(x1: f64, y1: f64, x2: f64, y2: f64, d2: f64) -> TransAffine {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_scaling_uniform(
        ((dx * dx) + (dy * dy)).sqrt() / d2,
    ));
    mtx.multiply(&TransAffine::new_rotation(dy.atan2(dx)));
    mtx.multiply(&TransAffine::new_translation(x1, y1));
    mtx.invert();
    mtx
}

fn render_gradient<PF: PixelFormat<ColorType = Rgba32>>(
    ras: &mut RasterizerScanlineAa,
    ren: &mut RendererBase<PF>,
    c1: Rgba32,
    c2: Rgba32,
    (x1, y1, x2, y2): (f64, f64, f64, f64),
) {
    let colors = GradientLinearColorF32::new(c1, c2, 256);
    let interpolator = SpanInterpolatorLinear::new(gradient_affine(x1, y1, x2, y2, 100.0));
    let mut sg = SpanGradient::new(interpolator, GradientX, &colors, 0.0, 100.0);
    let mut alloc = SpanAllocator::new();
    let mut sl = ScanlineU8::new();
    render_scanlines_aa(ras, &mut sl, ren, &mut alloc, &mut sg);
}

#[allow(clippy::too_many_arguments)]
fn circle(
    ren: &mut RendererBase<PixfmtBgra128>,
    c1: Rgba32,
    c2: Rgba32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    shadow_alpha: f64,
) {
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let r = calc_distance(x1, y1, x2, y2) / 2.0;
    let mut e = Ellipse::new(
        ((x1 + x2) / 2.0) + 5.0,
        ((y1 + y2) / 2.0) - 3.0,
        r,
        r,
        100,
        false,
    );
    ras.add_path(&mut e, 0);
    let shadow = Rgba32::new(
        0.6f64 as f32,
        0.6f64 as f32,
        0.6f64 as f32,
        (0.7 * shadow_alpha) as f32,
    );
    render_scanlines_aa_solid(&mut ras, &mut sl, ren, &shadow);

    ras.reset();
    let mut e = Ellipse::new((x1 + x2) / 2.0, (y1 + y2) / 2.0, r, r, 100, false);
    ras.add_path(&mut e, 0);
    render_gradient(&mut ras, ren, c1, c2, (x1, y1, x2, y2));
}

fn src_shape(
    ren: &mut RendererBase<PixfmtCompOpRgbaF32<OrderBgra>>,
    c1: Rgba32,
    c2: Rgba32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) {
    let mut ras = RasterizerScanlineAa::new();
    let mut rr = RoundedRect::new(x1, y1, x2, y2, 40.0);
    ras.add_path(&mut rr, 0);
    render_gradient(&mut ras, ren, c1, c2, (x1, y1, x2, y2));
}

fn fnv1a(buf: &[f32]) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for v in &buf[..WIDTH as usize * HEIGHT as usize * 4] {
        for byte in v.to_bits().to_le_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

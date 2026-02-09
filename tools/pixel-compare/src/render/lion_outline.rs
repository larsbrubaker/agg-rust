// Copyright 2025. Native lion_outline renderer for pixel comparison.
//
// This must produce identical output to the WASM version in
// demo/wasm/src/render/compositing.rs::lion_outline().

use agg_rust::bounding_rect::bounding_rect;
use agg_rust::color::Rgba8;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ctrl::{render_ctrl, CboxCtrl, SliderCtrl};
use agg_rust::math_stroke::LineJoin;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_outline_aa::{OutlineAaJoin, RasterizerOutlineAa};
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_outline_aa::{LineProfileAa, RendererOutlineAa};
use agg_rust::renderer_scanline::render_scanlines_aa_solid;
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::trans_affine::TransAffine;

/// Render the lion outline demo.
///
/// params[0] = angle (radians)
/// params[1] = scale
/// params[2] = skew_x
/// params[3] = skew_y
/// params[4] = line_width
/// params[5] = use_scanline (0 = outline AA, 1 = scanline rasterizer)
pub fn render(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_rad = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(2).copied().unwrap_or(0.0);
    let skew_y = params.get(3).copied().unwrap_or(0.0);
    let line_width = params.get(4).copied().unwrap_or(1.0).max(0.01);
    let use_scanline = params.get(5).copied().unwrap_or(0.0) > 0.5;

    let (mut path, colors, path_idx) = super::parse_lion();

    // Compute bounding box to match C++ exactly (not hardcoded values)
    let path_ids_u32: Vec<u32> = path_idx.iter().map(|&x| x as u32).collect();
    let npaths = colors.len();
    let rect = bounding_rect(&mut path, &path_ids_u32, 0, npaths)
        .expect("Lion path has no vertices");
    let base_dx = (rect.x2 - rect.x1) / 2.0;
    let base_dy = (rect.y2 - rect.y1) / 2.0;

    let stride = (width * 4) as i32;
    let mut buf = vec![255u8; (width * height * 4) as usize];
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle_rad + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_skewing(skew_x / 1000.0, skew_y / 1000.0));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    let npaths = colors.len();

    if use_scanline {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        for i in 0..npaths {
            let start = path_idx[i] as u32;
            let mut stroke = ConvStroke::new(&mut path);
            stroke.set_width(line_width);
            stroke.set_line_join(LineJoin::Round);
            let mut transformed = ConvTransform::new(&mut stroke, mtx);
            ras.reset();
            ras.add_path(&mut transformed, start);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
        }
    } else {
        let w = line_width * mtx.get_scale();
        let profile = LineProfileAa::with_width(w);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        let mut ras_oaa = RasterizerOutlineAa::new();
        // Match C++ defaults: accurate_join_only()=false → outline_round_join, round_cap=false
        ras_oaa.set_round_cap(false);
        ras_oaa.set_line_join(OutlineAaJoin::Round);

        // params[7] > 0 = only render first N paths (for debugging)
        let max_paths = params.get(7).copied().unwrap_or(0.0) as usize;
        let render_count = if max_paths > 0 { max_paths.min(npaths) } else { npaths };
        for i in 0..render_count {
            let start = path_idx[i] as u32;
            let mut transformed = ConvTransform::new(&mut path, mtx);
            ren_oaa.set_color(colors[i]);
            ras_oaa.add_path(&mut transformed, start, &mut ren_oaa);
        }

        drop(ren_oaa);
    }

    // Controls — render exactly as the WASM version does
    // (skip controls when param[6] == 1 for comparison debugging)
    let skip_controls = params.get(6).copied().unwrap_or(0.0) > 0.5;
    if !skip_controls {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s_width = SliderCtrl::new(5.0, 5.0, 150.0, 12.0);
        s_width.range(0.0, 4.0);
        s_width.set_value(line_width);
        s_width.label("Width=%3.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

        let mut cbox = CboxCtrl::new(160.0, 5.0, "Use Scanline Rasterizer");
        cbox.set_status(use_scanline);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox);
    }

    buf
}

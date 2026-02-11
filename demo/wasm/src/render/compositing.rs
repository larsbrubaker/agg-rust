//! Advanced demo render functions: blend_color, component_rendering, polymorphic_renderer,
//! scanline_boolean, scanline_boolean2, pattern_fill, pattern_perspective, pattern_resample,
//! lion_outline, rasterizers2, line_patterns, line_patterns_clip, compositing, compositing2,
//! flash_rasterizer, flash_rasterizer2, rasterizer_compound.

use agg_rust::basics::{is_stop, is_vertex, VertexSource};
use agg_rust::blur::stack_blur_rgba32;
use agg_rust::color::{Gray8, Rgba8};
use agg_rust::comp_op::{CompOp, PixfmtRgba32CompOp};
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ctrl::{render_ctrl, CboxCtrl, RboxCtrl, SliderCtrl};
use agg_rust::ellipse::Ellipse;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_accessors::{ImageAccessorWrap, WrapModeRepeat};
use agg_rust::math_stroke::{LineCap, LineJoin};
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_gray::PixfmtGray8;
use agg_rust::pixfmt_rgb::PixfmtRgb24;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_compound_aa::{RasterizerCompoundAa, LayerOrder};
use agg_rust::rasterizer_outline::RasterizerOutline;
use agg_rust::rasterizer_outline_aa::{RasterizerOutlineAa, OutlineAaJoin};
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_outline_aa::{LineProfileAa, RendererOutlineAa};
use agg_rust::renderer_primitives::RendererPrimitives;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::rounded_rect::RoundedRect;
use agg_rust::basics::FillingRule;
use agg_rust::scanline_boolean_algebra::{SBoolOp, sbool_combine_shapes_aa, sbool_combine_shapes_bin};
use agg_rust::scanline_storage_aa::ScanlineStorageAa;
use agg_rust::scanline_storage_bin::ScanlineStorageBin;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::gradient_lut::{GradientLinearColor, GradientLut};
use agg_rust::span_gradient::{GradientRadial, GradientX, SpanGradient};
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::span_interpolator_trans::SpanInterpolatorTrans;
use agg_rust::span_pattern_rgba::SpanPatternRgba;
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_bilinear::TransBilinear;
use agg_rust::trans_perspective::TransPerspective;
use agg_rust::trans_viewport::{AspectRatio, TransViewport};
use super::setup_renderer;


// ============================================================================
// Blend Color — shape with blurred shadow
// ============================================================================

/// Blurred shadow under a shape — demonstrates blur compositing.
///
/// params[0] = blur_radius (0-40, default 15)
/// params[1] = shadow_dx (default 10)
/// params[2] = shadow_dy (default 10)
pub fn blend_color(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let blur_radius = params.first().copied().unwrap_or(15.0).clamp(0.0, 40.0);
    let shadow_dx = params.get(1).copied().unwrap_or(10.0);
    let shadow_dy = params.get(2).copied().unwrap_or(10.0);

    let w = width as f64;
    let h = height as f64;

    // Build a shape — an "E"-like path
    let mut shape = PathStorage::new();
    let cx = w / 2.0;
    let cy = h / 2.0;
    let sz = (w.min(h) / 2.0 - 40.0).max(30.0);

    // Outer rectangle
    shape.move_to(cx - sz, cy - sz);
    shape.line_to(cx + sz, cy - sz);
    shape.line_to(cx + sz, cy - sz + sz * 0.25);
    shape.line_to(cx - sz + sz * 0.35, cy - sz + sz * 0.25);
    shape.line_to(cx - sz + sz * 0.35, cy - sz * 0.15);
    shape.line_to(cx + sz * 0.6, cy - sz * 0.15);
    shape.line_to(cx + sz * 0.6, cy + sz * 0.15);
    shape.line_to(cx - sz + sz * 0.35, cy + sz * 0.15);
    shape.line_to(cx - sz + sz * 0.35, cy + sz - sz * 0.25);
    shape.line_to(cx + sz, cy + sz - sz * 0.25);
    shape.line_to(cx + sz, cy + sz);
    shape.line_to(cx - sz, cy + sz);
    shape.close_polygon(0);

    // 1) Render shadow into a separate RGBA buffer, then blur it
    let mut shadow_buf = vec![0u8; (width * height * 4) as usize];
    {
        let mut shadow_ra = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { shadow_ra.attach(shadow_buf.as_mut_ptr(), width, height, stride) };
        let shadow_pf = PixfmtRgba32::new(&mut shadow_ra);
        let mut shadow_rb = RendererBase::new(shadow_pf);
        shadow_rb.clear(&Rgba8::new(0, 0, 0, 0));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Offset shape for shadow
        let mut shadow_shape = PathStorage::new();
        let (mut x, mut y) = (0.0, 0.0);
        shape.rewind(0);
        loop {
            let cmd = shape.vertex(&mut x, &mut y);
            if is_stop(cmd) { break; }
            if is_vertex(cmd) {
                if (cmd & 0x07) == 1 {
                    shadow_shape.move_to(x + shadow_dx, y + shadow_dy);
                } else {
                    shadow_shape.line_to(x + shadow_dx, y + shadow_dy);
                }
            } else {
                shadow_shape.close_polygon(0);
            }
        }

        ras.add_path(&mut shadow_shape, 0);
        // Dark shadow color with partial alpha
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut shadow_rb, &Rgba8::new(0, 0, 0, 180));
    }

    // Blur the shadow
    if blur_radius > 0.5 {
        let mut shadow_ra = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { shadow_ra.attach(shadow_buf.as_mut_ptr(), width, height, stride) };
        stack_blur_rgba32(&mut shadow_ra, blur_radius as u32, blur_radius as u32);
    }

    // 2) Create main buffer, composite shadow then shape
    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Composite blurred shadow onto main buffer
    for y in 0..height {
        for x in 0..width {
            let off = ((y * width + x) * 4) as usize;
            let sa = shadow_buf[off + 3] as u32;
            if sa > 0 {
                let sr = shadow_buf[off] as u32;
                let sg = shadow_buf[off + 1] as u32;
                let sb = shadow_buf[off + 2] as u32;
                let da = 255u32;
                let dr = buf[off] as u32;
                let dg = buf[off + 1] as u32;
                let db = buf[off + 2] as u32;
                // src-over compositing
                buf[off] = (dr + (sr * sa - dr * sa + 127) / 255) as u8;
                buf[off + 1] = (dg + (sg * sa - dg * sa + 127) / 255) as u8;
                buf[off + 2] = (db + (sb * sa - db * sa + 127) / 255) as u8;
                buf[off + 3] = (da + (sa * (255 - da) + 127) / 255) as u8;
            }
        }
    }

    // Re-attach for rendering the main shape
    let stride = (width * 4) as i32;
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    ras.add_path(&mut shape, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(100, 80, 60, 255));

    // Controls
    let mut s_blur = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_blur.range(0.0, 40.0);
    s_blur.label("Blur Radius=%.1f");
    s_blur.set_value(blur_radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_blur);

    buf
}


// ============================================================================
// Component Rendering — RGB channels as independent gray layers
// ============================================================================

/// Three overlapping circles rendered to separate gray channels, composited into RGB.
///
/// params[0] = alpha (0-255, default 255)
pub fn component_rendering(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let alpha = params.first().copied().unwrap_or(255.0).clamp(0.0, 255.0) as u8;

    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r = (w.min(h) / 3.0).max(40.0);

    // Render each circle to a separate gray8 buffer
    let make_gray_circle = |ell_cx: f64, ell_cy: f64| -> Vec<u8> {
        let mut gbuf = vec![255u8; (width * height) as usize];
        let mut gra = RowAccessor::new();
        let stride = width as i32;
        unsafe { gra.attach(gbuf.as_mut_ptr(), width, height, stride) };
        let gpf = PixfmtGray8::new(&mut gra);
        let mut grb = RendererBase::new(gpf);
        grb.clear(&Gray8::new(255, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut ell = Ellipse::new(ell_cx, ell_cy, r, r, 100, false);
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut grb, &Gray8::new(0, alpha as u32));

        gbuf
    };

    // Three circles offset from center: red (top-left), green (top-right), blue (bottom)
    let offset = r * 0.5;
    let r_buf = make_gray_circle(cx - offset, cy - offset * 0.6);
    let g_buf = make_gray_circle(cx + offset, cy - offset * 0.6);
    let b_buf = make_gray_circle(cx, cy + offset * 0.8);

    // Composite gray channels into RGBA output
    let mut buf = vec![255u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let gi = (y * width + x) as usize;
            let oi = gi * 4;
            buf[oi] = r_buf[gi];     // R channel from red circle gray
            buf[oi + 1] = g_buf[gi]; // G channel from green circle gray
            buf[oi + 2] = b_buf[gi]; // B channel from blue circle gray
            buf[oi + 3] = 255;       // fully opaque
        }
    }

    // Re-attach for rendering controls
    let mut ra = RowAccessor::new();
    let stride = (width * 4) as i32;
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let mut s_alpha = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_alpha.range(0.0, 255.0);
    s_alpha.label("Alpha=%.0f");
    s_alpha.set_value(alpha as f64);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_alpha);

    buf
}


// ============================================================================
// Polymorphic Renderer — triangle rendered via different pixel formats
// ============================================================================

/// Triangle rendered with PixfmtRgb24, demonstrating multiple pixel format support.
///
/// params[0] = format (0=rgba32, 1=rgb24, 2=gray8)
pub fn polymorphic_renderer(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let format = params.first().copied().unwrap_or(0.0) as u32;

    let w = width as f64;
    let h = height as f64;

    // Build a triangle
    let mut tri = PathStorage::new();
    tri.move_to(w * 0.25, h * 0.15);
    tri.line_to(w * 0.92, h * 0.43);
    tri.line_to(w * 0.36, h * 0.78);
    tri.close_polygon(0);

    // Also add a circle
    let mut ell = Ellipse::new(w * 0.65, h * 0.6, w * 0.15, h * 0.2, 100, false);

    // Common output buffer (RGBA for WASM)
    let mut buf = vec![255u8; (width * height * 4) as usize];

    match format {
        1 => {
            // Render with RGB24 into a temporary 3-bpp buffer, then convert to RGBA
            let mut rgb_buf = vec![255u8; (width * height * 3) as usize];
            {
                let mut rgb_ra = RowAccessor::new();
                let stride = (width * 3) as i32;
                unsafe { rgb_ra.attach(rgb_buf.as_mut_ptr(), width, height, stride) };
                let rgb_pf = PixfmtRgb24::new(&mut rgb_ra);
                let mut rgb_rb = RendererBase::new(rgb_pf);
                rgb_rb.clear(&Rgba8::new(255, 255, 255, 255));

                let mut ras = RasterizerScanlineAa::new();
                let mut sl = ScanlineU8::new();

                ras.add_path(&mut tri, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rgb_rb, &Rgba8::new(80, 30, 20, 255));

                ras.reset();
                ras.add_path(&mut ell, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rgb_rb, &Rgba8::new(20, 80, 40, 200));
            }
            // Convert RGB24 → RGBA32
            for y in 0..height {
                for x in 0..width {
                    let ri = ((y * width + x) * 3) as usize;
                    let oi = ((y * width + x) * 4) as usize;
                    buf[oi] = rgb_buf[ri];
                    buf[oi + 1] = rgb_buf[ri + 1];
                    buf[oi + 2] = rgb_buf[ri + 2];
                    buf[oi + 3] = 255;
                }
            }
        }
        2 => {
            // Render with Gray8 into 1-bpp buffer, then convert to RGBA
            let mut gray_buf = vec![255u8; (width * height) as usize];
            {
                let mut gray_ra = RowAccessor::new();
                let stride = width as i32;
                unsafe { gray_ra.attach(gray_buf.as_mut_ptr(), width, height, stride) };
                let gray_pf = PixfmtGray8::new(&mut gray_ra);
                let mut gray_rb = RendererBase::new(gray_pf);
                gray_rb.clear(&Gray8::new(255, 255));

                let mut ras = RasterizerScanlineAa::new();
                let mut sl = ScanlineU8::new();

                ras.add_path(&mut tri, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut gray_rb, &Gray8::new(30, 255));

                ras.reset();
                ras.add_path(&mut ell, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut gray_rb, &Gray8::new(80, 200));
            }
            // Convert Gray8 → RGBA32
            for y in 0..height {
                for x in 0..width {
                    let gi = (y * width + x) as usize;
                    let oi = gi * 4;
                    let v = gray_buf[gi];
                    buf[oi] = v;
                    buf[oi + 1] = v;
                    buf[oi + 2] = v;
                    buf[oi + 3] = 255;
                }
            }
        }
        _ => {
            // Render with standard RGBA32
            let mut ra = RowAccessor::new();
            let stride = (width * 4) as i32;
            unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
            let pf = PixfmtRgba32::new(&mut ra);
            let mut rb = RendererBase::new(pf);
            rb.clear(&Rgba8::new(255, 255, 255, 255));

            let mut ras = RasterizerScanlineAa::new();
            let mut sl = ScanlineU8::new();

            ras.add_path(&mut tri, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(80, 30, 20, 255));

            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(20, 80, 40, 200));
        }
    }

    // Overlay format label
    let mut ra = RowAccessor::new();
    let stride = (width * 4) as i32;
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let labels = ["RGBA32 (4 bpp)", "RGB24 (3 bpp)", "Gray8 (1 bpp)"];
    let label = labels.get(format as usize).unwrap_or(&"RGBA32");
    let mut txt = GsvText::new();
    txt.size(14.0, 0.0);
    txt.start_point(10.0, h - 25.0);
    txt.text(label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.5);
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

/// Helper: render stored scanlines (from ScanlineStorageAa) into a RendererBase.
fn render_storage_solid(
    storage: &mut ScanlineStorageAa,
    sl: &mut ScanlineU8,
    ren: &mut RendererBase<PixfmtRgba32>,
    color: &Rgba8,
) {
    use agg_rust::rasterizer_scanline_aa::Scanline;
    if storage.rewind_scanlines() {
        sl.reset(storage.min_x(), storage.max_x());
        while storage.sweep_scanline(sl) {
            let y = Scanline::y(sl);
            for span in sl.begin() {
                let x = span.x;
                let len = span.len;
                if len > 0 {
                    ren.blend_solid_hspan(
                        x, y, len, color,
                        &sl.covers()[span.cover_offset..span.cover_offset + len as usize],
                    );
                }
            }
        }
    }
}


// ============================================================================
// Scanline Boolean — two shape groups with boolean operations
// ============================================================================

/// C++-matching scanline_boolean demo: two circle groups generated from draggable quads.
///
/// params[0]  = operation (0=Union, 1=Intersection, 2=Linear XOR, 3=Saddle XOR, 4=Abs Diff XOR, 5=A-B, 6=B-A)
/// params[1]  = opacity1 (0..1)
/// params[2]  = opacity2 (0..1)
/// params[3..10]   = quad1 (x0,y0,x1,y1,x2,y2,x3,y3)
/// params[11..18]  = quad2 (x0,y0,x1,y1,x2,y2,x3,y3)
pub fn scanline_boolean(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let op_idx = params.first().copied().unwrap_or(0.0) as i32;
    let op = match op_idx {
        1 => SBoolOp::And,
        2 => SBoolOp::Xor,
        3 => SBoolOp::XorSaddle,
        4 => SBoolOp::XorAbsDiff,
        5 => SBoolOp::AMinusB,
        6 => SBoolOp::BMinusA,
        _ => SBoolOp::Or,
    };
    let opacity1 = params.get(1).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let opacity2 = params.get(2).copied().unwrap_or(1.0).clamp(0.0, 1.0);

    let w = width as f64;
    let h = height as f64;
    let defaults_quad1 = [
        50.0, 180.0,
        w / 2.0 - 25.0, 200.0,
        w / 2.0 - 25.0, h - 70.0,
        50.0, h - 50.0,
    ];
    let defaults_quad2 = [
        w / 2.0 + 25.0, 180.0,
        w - 50.0, 200.0,
        w - 50.0, h - 70.0,
        w / 2.0 + 25.0, h - 50.0,
    ];

    let mut quad1 = defaults_quad1;
    let mut quad2 = defaults_quad2;
    for i in 0..8 {
        if let Some(v) = params.get(3 + i) {
            quad1[i] = *v;
        }
        if let Some(v) = params.get(11 + i) {
            quad2[i] = *v;
        }
    }

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Build path with circles on each quad edge exactly like C++ generate_circles().
    let generate_circles = |quad: &[f64; 8]| -> PathStorage {
        let mut ps = PathStorage::new();
        let num_circles = 5usize;
        let radius = 20.0;
        for i in 0..4usize {
            let n1 = i * 2;
            let n2 = if i < 3 { i * 2 + 2 } else { 0 };
            for j in 0..num_circles {
                let cx = quad[n1] + (quad[n2] - quad[n1]) * j as f64 / num_circles as f64;
                let cy = quad[n1 + 1] + (quad[n2 + 1] - quad[n1 + 1]) * j as f64 / num_circles as f64;
                let mut ell = Ellipse::new(cx, cy, radius, radius, 100, false);
                ps.concat_path(&mut ell, 0);
            }
        }
        ps
    };

    let mut ps1 = generate_circles(&quad1);
    let mut ps2 = generate_circles(&quad2);

    let mut sl = ScanlineU8::new();
    let mut ras = RasterizerScanlineAa::new();
    ras.clip_box(0.0, 0.0, w, h);

    let mut ras1 = RasterizerScanlineAa::new();
    ras1.filling_rule(FillingRule::EvenOdd);
    ras1.add_path(&mut ps1, 0);

    let mut ras2 = RasterizerScanlineAa::new();
    ras2.add_path(&mut ps2, 0);

    let a1 = (100.0 * opacity1).round().clamp(0.0, 255.0) as u32;
    let a2 = (100.0 * opacity2).round().clamp(0.0, 255.0) as u32;
    render_scanlines_aa_solid(&mut ras1, &mut sl, &mut rb, &Rgba8::new(240, 255, 200, a1));
    render_scanlines_aa_solid(&mut ras2, &mut sl, &mut rb, &Rgba8::new(255, 240, 240, a2));

    let mut sl1 = ScanlineU8::new();
    let mut sl2 = ScanlineU8::new();
    let mut sl_result = ScanlineU8::new();
    let mut storage1 = ScanlineStorageAa::new();
    let mut storage2 = ScanlineStorageAa::new();
    let mut storage_result = ScanlineStorageAa::new();
    sbool_combine_shapes_aa(
        op, &mut ras1, &mut ras2,
        &mut sl1, &mut sl2, &mut sl_result,
        &mut storage1, &mut storage2, &mut storage_result,
    );
    render_storage_solid(&mut storage_result, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Render interactive quad guides (edges + vertex handles).
    for quad in [&quad1, &quad2] {
        let mut edge_path = PathStorage::new();
        edge_path.move_to(quad[0], quad[1]);
        edge_path.line_to(quad[2], quad[3]);
        edge_path.line_to(quad[4], quad[5]);
        edge_path.line_to(quad[6], quad[7]);
        edge_path.close_polygon(0);

        let mut edge_stroke = ConvStroke::new(&mut edge_path);
        edge_stroke.set_width(1.5);
        ras.reset();
        ras.add_path(&mut edge_stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 77, 128, 153));

        for i in 0..4usize {
            let x = quad[i * 2];
            let y = quad[i * 2 + 1];
            let mut h = Ellipse::new(x, y, 4.5, 4.5, 32, false);
            ras.reset();
            ras.add_path(&mut h, 0);
            // Match C++ tool look better: circular handles with softer contrast.
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 102, 140, 150));
        }
    }

    // Render C++-matching controls.
    let mut m_trans_type = RboxCtrl::new(420.0, 5.0, 550.0, 145.0);
    m_trans_type.add_item("Union");
    m_trans_type.add_item("Intersection");
    m_trans_type.add_item("Linear XOR");
    m_trans_type.add_item("Saddle XOR");
    m_trans_type.add_item("Abs Diff XOR");
    m_trans_type.add_item("A-B");
    m_trans_type.add_item("B-A");
    m_trans_type.set_cur_item(op_idx.clamp(0, 6));
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_trans_type);

    let mut m_reset = CboxCtrl::new(350.0, 5.0, "Reset");
    m_reset.set_status(false);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_reset);

    let mut m_mul1 = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    m_mul1.range(0.0, 1.0);
    m_mul1.set_value(opacity1);
    m_mul1.label("Opacity1=%.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_mul1);

    let mut m_mul2 = SliderCtrl::new(5.0, 20.0, 340.0, 27.0);
    m_mul2.range(0.0, 1.0);
    m_mul2.set_value(opacity2);
    m_mul2.label("Opacity2=%.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_mul2);

    buf
}


// ============================================================================
// Scanline Boolean 2 — more complex boolean ops with paths
// ============================================================================

/// Complex shapes combined with boolean operations and different scanline types.
///
/// params[0] = test case (0-3)
/// params[1] = operation (0=Or, 1=And, 2=Xor, 3=AMinusB, 4=BMinusA)
/// Scanline Boolean 2 — matching C++ scanline_boolean2.cpp exactly.
///
/// params[0] = polygon type (0-4): Two Simple Paths, Closed Stroke, GB+Arrows, GB+Spiral, Spiral+Glyph
/// params[1] = fill rule (0=Even-Odd, 1=Non-Zero)
/// params[2] = scanline type (0=scanline_p, 1=scanline_u, 2=scanline_bin)
/// params[3] = operation (0=None, 1=OR, 2=AND, 3=XOR Linear, 4=XOR Saddle, 5=A-B, 6=B-A)
/// params[4] = mouse_x
/// params[5] = mouse_y
pub fn scanline_boolean2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    use super::gb_poly::{make_gb_poly, make_arrows, Spiral};
    use agg_rust::ctrl::RboxCtrl;

    let polygon_idx = params.first().copied().unwrap_or(3.0) as u32;
    let fill_rule_idx = params.get(1).copied().unwrap_or(1.0) as u32;
    let scanline_type_idx = params.get(2).copied().unwrap_or(1.0) as u32;
    let operation_idx = params.get(3).copied().unwrap_or(2.0) as u32;
    let mouse_x = params.get(4).copied().unwrap_or(width as f64 / 2.0);
    let mouse_y = params.get(5).copied().unwrap_or(height as f64 / 2.0);

    let initial_width = 655.0_f64;
    let initial_height = 520.0_f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut sl_u8 = ScanlineU8::new();
    let mut ras = RasterizerScanlineAa::new();

    // Render controls
    let mut m_polygons = RboxCtrl::new(5.0, 5.0, 5.0 + 205.0, 110.0);
    m_polygons.add_item("Two Simple Paths");
    m_polygons.add_item("Closed Stroke");
    m_polygons.add_item("Great Britain and Arrows");
    m_polygons.add_item("Great Britain and Spiral");
    m_polygons.add_item("Spiral and Glyph");
    m_polygons.set_cur_item(polygon_idx as i32);
    render_ctrl(&mut ras, &mut sl_u8, &mut rb, &mut m_polygons);

    let mut m_fill_rule = RboxCtrl::new(200.0, 5.0, 200.0 + 105.0, 50.0);
    m_fill_rule.add_item("Even-Odd");
    m_fill_rule.add_item("Non Zero");
    m_fill_rule.set_cur_item(fill_rule_idx as i32);
    render_ctrl(&mut ras, &mut sl_u8, &mut rb, &mut m_fill_rule);

    let mut m_scanline_type = RboxCtrl::new(300.0, 5.0, 300.0 + 115.0, 70.0);
    m_scanline_type.add_item("scanline_p");
    m_scanline_type.add_item("scanline_u");
    m_scanline_type.add_item("scanline_bin");
    m_scanline_type.set_cur_item(scanline_type_idx as i32);
    render_ctrl(&mut ras, &mut sl_u8, &mut rb, &mut m_scanline_type);

    let mut m_operation = RboxCtrl::new(535.0, 5.0, 535.0 + 115.0, 145.0);
    m_operation.add_item("None");
    m_operation.add_item("OR");
    m_operation.add_item("AND");
    m_operation.add_item("XOR Linear");
    m_operation.add_item("XOR Saddle");
    m_operation.add_item("A-B");
    m_operation.add_item("B-A");
    m_operation.set_cur_item(operation_idx as i32);
    render_ctrl(&mut ras, &mut sl_u8, &mut rb, &mut m_operation);

    // Set fill rule
    let fill_rule = if fill_rule_idx != 0 {
        FillingRule::NonZero
    } else {
        FillingRule::EvenOdd
    };

    let mut ras1 = RasterizerScanlineAa::new();
    let mut ras2 = RasterizerScanlineAa::new();
    ras1.filling_rule(fill_rule);
    ras2.filling_rule(fill_rule);

    // Build shapes and render preview based on polygon type
    match polygon_idx {
        0 => {
            // Two Simple Paths
            let mut ps1 = PathStorage::new();
            let mut ps2 = PathStorage::new();

            let x = mouse_x - initial_width / 2.0 + 100.0;
            let y = mouse_y - initial_height / 2.0 + 100.0;
            ps1.move_to(x + 140.0, y + 145.0);
            ps1.line_to(x + 225.0, y + 44.0);
            ps1.line_to(x + 296.0, y + 219.0);
            ps1.close_polygon(0);

            ps1.line_to(x + 226.0, y + 289.0);
            ps1.line_to(x + 82.0, y + 292.0);

            ps1.move_to(x + 220.0, y + 222.0);
            ps1.line_to(x + 363.0, y + 249.0);
            ps1.line_to(x + 265.0, y + 331.0);

            ps1.move_to(x + 242.0, y + 243.0);
            ps1.line_to(x + 325.0, y + 261.0);
            ps1.line_to(x + 268.0, y + 309.0);

            ps1.move_to(x + 259.0, y + 259.0);
            ps1.line_to(x + 273.0, y + 288.0);
            ps1.line_to(x + 298.0, y + 266.0);

            ps2.move_to(100.0 + 32.0, 100.0 + 77.0);
            ps2.line_to(100.0 + 473.0, 100.0 + 263.0);
            ps2.line_to(100.0 + 351.0, 100.0 + 290.0);
            ps2.line_to(100.0 + 354.0, 100.0 + 374.0);

            ras1.reset();
            ras1.add_path(&mut ps1, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 0, 0, 26)); // rgba(0,0,0,0.1)

            ras2.reset();
            ras2.add_path(&mut ps2, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 153, 0, 26)); // rgba(0,0.6,0,0.1)

            // Re-add for boolean
            ras1.reset();
            ras1.filling_rule(fill_rule);
            ras1.add_path(&mut ps1, 0);
            ras2.reset();
            ras2.filling_rule(fill_rule);
            ras2.add_path(&mut ps2, 0);
        }
        1 => {
            // Closed Stroke
            let mut ps1 = PathStorage::new();
            let mut ps2 = PathStorage::new();

            let x = mouse_x - initial_width / 2.0 + 100.0;
            let y = mouse_y - initial_height / 2.0 + 100.0;
            ps1.move_to(x + 140.0, y + 145.0);
            ps1.line_to(x + 225.0, y + 44.0);
            ps1.line_to(x + 296.0, y + 219.0);
            ps1.close_polygon(0);

            ps1.line_to(x + 226.0, y + 289.0);
            ps1.line_to(x + 82.0, y + 292.0);

            ps1.move_to(x + 220.0 - 50.0, y + 222.0);
            ps1.line_to(x + 363.0 - 50.0, y + 249.0);
            ps1.line_to(x + 265.0 - 50.0, y + 331.0);
            ps1.close_polygon(0);

            ps2.move_to(100.0 + 32.0, 100.0 + 77.0);
            ps2.line_to(100.0 + 473.0, 100.0 + 263.0);
            ps2.line_to(100.0 + 351.0, 100.0 + 290.0);
            ps2.line_to(100.0 + 354.0, 100.0 + 374.0);
            ps2.close_polygon(0);

            let mut stroke = ConvStroke::new(&mut ps2);
            stroke.set_width(15.0);

            ras1.reset();
            ras1.add_path(&mut ps1, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 0, 0, 26));

            ras2.reset();
            ras2.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 153, 0, 26));

            // Re-add for boolean
            ras1.reset();
            ras1.filling_rule(fill_rule);
            ras1.add_path(&mut ps1, 0);
            ras2.reset();
            ras2.filling_rule(fill_rule);
            ras2.add_path(&mut stroke, 0);
        }
        2 => {
            // Great Britain and Arrows
            let mut gb_poly = PathStorage::new();
            let mut arrows = PathStorage::new();
            make_gb_poly(&mut gb_poly);
            make_arrows(&mut arrows);

            let mut mtx1 = TransAffine::new();
            mtx1.multiply(&TransAffine::new_translation(-1150.0, -1150.0));
            mtx1.multiply(&TransAffine::new_scaling_uniform(2.0));

            let mut mtx2 = mtx1.clone();
            mtx2.multiply(&TransAffine::new_translation(
                mouse_x - initial_width / 2.0,
                mouse_y - initial_height / 2.0,
            ));

            let mut trans_gb_poly = ConvTransform::new(&mut gb_poly, mtx1.clone());
            let mut trans_arrows = ConvTransform::new(&mut arrows, mtx2);

            ras2.add_path(&mut trans_gb_poly, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(128, 128, 0, 26)); // rgba(0.5,0.5,0,0.1)

            let mut stroke_gb_poly = ConvStroke::new(&mut trans_gb_poly);
            stroke_gb_poly.set_width(0.1);
            ras1.add_path(&mut stroke_gb_poly, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 0, 0, 255));

            ras2.add_path(&mut trans_arrows, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 128, 128, 26)); // rgba(0,0.5,0.5,0.1)

            ras1.reset();
            ras1.filling_rule(fill_rule);
            ras1.add_path(&mut trans_gb_poly, 0);
            // ras2 already has arrows from above
        }
        3 => {
            // Great Britain and a Spiral
            let mut sp = Spiral::new(mouse_x, mouse_y, 10.0, 150.0, 30.0, 0.0);
            let mut stroke = ConvStroke::new(&mut sp);
            stroke.set_width(15.0);

            let mut gb_poly = PathStorage::new();
            make_gb_poly(&mut gb_poly);

            let mut mtx = TransAffine::new();
            mtx.multiply(&TransAffine::new_translation(-1150.0, -1150.0));
            mtx.multiply(&TransAffine::new_scaling_uniform(2.0));

            let mut trans_gb_poly = ConvTransform::new(&mut gb_poly, mtx);

            ras1.add_path(&mut trans_gb_poly, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(128, 128, 0, 26));

            let mut stroke_gb_poly = ConvStroke::new(&mut trans_gb_poly);
            stroke_gb_poly.set_width(0.1);
            ras1.reset();
            ras1.add_path(&mut stroke_gb_poly, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 0, 0, 255));

            ras2.reset();
            ras2.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 128, 128, 26));

            ras1.reset();
            ras1.filling_rule(fill_rule);
            ras1.add_path(&mut trans_gb_poly, 0);
            // ras2 already has spiral stroke
        }
        4 => {
            // Spiral and glyph
            let mut sp = Spiral::new(mouse_x, mouse_y, 10.0, 150.0, 30.0, 0.0);
            let mut stroke = ConvStroke::new(&mut sp);
            stroke.set_width(15.0);

            let mut glyph = PathStorage::new();
            glyph.move_to(28.47, 6.45);
            glyph.curve3(21.58, 1.12, 19.82, 0.29);
            glyph.curve3(17.19, -0.93, 14.21, -0.93);
            glyph.curve3(9.57, -0.93, 6.57, 2.25);
            glyph.curve3(3.56, 5.42, 3.56, 10.60);
            glyph.curve3(3.56, 13.87, 5.03, 16.26);
            glyph.curve3(7.03, 19.58, 11.99, 22.51);
            glyph.curve3(16.94, 25.44, 28.47, 29.64);
            glyph.line_to(28.47, 31.40);
            glyph.curve3(28.47, 38.09, 26.34, 40.58);
            glyph.curve3(24.22, 43.07, 20.17, 43.07);
            glyph.curve3(17.09, 43.07, 15.28, 41.41);
            glyph.curve3(13.43, 39.75, 13.43, 37.60);
            glyph.line_to(13.53, 34.77);
            glyph.curve3(13.53, 32.52, 12.38, 31.30);
            glyph.curve3(11.23, 30.08, 9.38, 30.08);
            glyph.curve3(7.57, 30.08, 6.42, 31.35);
            glyph.curve3(5.27, 32.62, 5.27, 34.81);
            glyph.curve3(5.27, 39.01, 9.57, 42.53);
            glyph.curve3(13.87, 46.04, 21.63, 46.04);
            glyph.curve3(27.59, 46.04, 31.40, 44.04);
            glyph.curve3(34.28, 42.53, 35.64, 39.31);
            glyph.curve3(36.52, 37.21, 36.52, 30.71);
            glyph.line_to(36.52, 15.53);
            glyph.curve3(36.52, 9.13, 36.77, 7.69);
            glyph.curve3(37.01, 6.25, 37.57, 5.76);
            glyph.curve3(38.13, 5.27, 38.87, 5.27);
            glyph.curve3(39.65, 5.27, 40.23, 5.62);
            glyph.curve3(41.26, 6.25, 44.19, 9.18);
            glyph.line_to(44.19, 6.45);
            glyph.curve3(38.72, -0.88, 33.74, -0.88);
            glyph.curve3(31.35, -0.88, 29.93, 0.78);
            glyph.curve3(28.52, 2.44, 28.47, 6.45);
            glyph.close_polygon(0);

            glyph.move_to(28.47, 9.62);
            glyph.line_to(28.47, 26.66);
            glyph.curve3(21.09, 23.73, 18.95, 22.51);
            glyph.curve3(15.09, 20.36, 13.43, 18.02);
            glyph.curve3(11.77, 15.67, 11.77, 12.89);
            glyph.curve3(11.77, 9.38, 13.87, 7.06);
            glyph.curve3(15.97, 4.74, 18.70, 4.74);
            glyph.curve3(22.41, 4.74, 28.47, 9.62);
            glyph.close_polygon(0);

            let mut mtx = TransAffine::new();
            mtx.multiply(&TransAffine::new_scaling(4.0, 4.0));
            mtx.multiply(&TransAffine::new_translation(220.0, 200.0));
            let mut trans = ConvTransform::new(&mut glyph, mtx);
            let mut curve = ConvCurve::new(&mut trans);

            ras1.reset();
            ras1.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras1, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 0, 0, 26));

            ras2.reset();
            ras2.add_path(&mut curve, 0);
            render_scanlines_aa_solid(&mut ras2, &mut sl_u8, &mut rb,
                &Rgba8::new(0, 153, 0, 26));

            // Re-add for boolean
            ras1.reset();
            ras1.filling_rule(fill_rule);
            ras1.add_path(&mut stroke, 0);
            ras2.reset();
            ras2.filling_rule(fill_rule);
            ras2.add_path(&mut curve, 0);
        }
        _ => {}
    }

    // Perform boolean operation if operation > 0 (not "None")
    if operation_idx > 0 {
        let op = match operation_idx {
            1 => SBoolOp::Or,
            2 => SBoolOp::And,
            3 => SBoolOp::Xor,
            4 => SBoolOp::XorSaddle,
            5 => SBoolOp::AMinusB,
            6 => SBoolOp::BMinusA,
            _ => SBoolOp::Or,
        };

        let result_color = Rgba8::new(128, 0, 0, 128); // rgba(0.5, 0, 0, 0.5)

        match scanline_type_idx {
            2 => {
                // scanline_bin — binary mode
                let mut sl1 = ScanlineU8::new();
                let mut sl2 = ScanlineU8::new();
                let mut sl_result = ScanlineU8::new();
                let mut st1 = ScanlineStorageBin::new();
                let mut st2 = ScanlineStorageBin::new();
                let mut st_result = ScanlineStorageBin::new();

                sbool_combine_shapes_bin(
                    op, &mut ras1, &mut ras2,
                    &mut sl1, &mut sl2, &mut sl_result,
                    &mut st1, &mut st2, &mut st_result,
                );

                render_storage_bin_solid(&mut st_result, &mut sl_u8, &mut rb, &result_color);
            }
            _ => {
                // scanline_p8 (0) or scanline_u8 (1) — AA mode
                let mut sl1 = ScanlineU8::new();
                let mut sl2 = ScanlineU8::new();
                let mut sl_result = ScanlineU8::new();
                let mut st1 = ScanlineStorageAa::new();
                let mut st2 = ScanlineStorageAa::new();
                let mut st_result = ScanlineStorageAa::new();

                sbool_combine_shapes_aa(
                    op, &mut ras1, &mut ras2,
                    &mut sl1, &mut sl2, &mut sl_result,
                    &mut st1, &mut st2, &mut st_result,
                );

                render_storage_solid(&mut st_result, &mut sl_u8, &mut rb, &result_color);
            }
        }

        // Render timing/spans text (use GsvText for display)
        {
            let label = format!("Combine=N/A\n\nRender=N/A\n\nnum_spans=N/A");
            let mut txt = GsvText::new();
            txt.size(8.0, 0.0);
            txt.start_point(420.0, 40.0);
            txt.text(&label);
            let mut txt_stroke = ConvStroke::new(&mut txt);
            txt_stroke.set_width(1.0);
            txt_stroke.set_line_cap(LineCap::Round);
            ras.reset();
            ras.add_path(&mut txt_stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl_u8, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }
    }

    buf
}

/// Render binary scanline storage as a solid color (no AA blending).
fn render_storage_bin_solid(
    storage: &mut ScanlineStorageBin,
    sl: &mut ScanlineU8,
    ren: &mut RendererBase<PixfmtRgba32>,
    color: &Rgba8,
) {
    use agg_rust::rasterizer_scanline_aa::Scanline;
    if storage.rewind_scanlines() {
        sl.reset(storage.min_x(), storage.max_x());
        while storage.sweep_scanline(sl) {
            let y = Scanline::y(sl);
            for span in sl.begin() {
                let x = span.x;
                let len = span.len;
                if len > 0 {
                    // Binary: full coverage for all pixels
                    ren.blend_hline(x, y, x + len - 1, color, 255);
                }
            }
        }
    }
}


// ============================================================================
// Pattern Fill — repeating pattern fills a polygon
// ============================================================================

/// Repeating pattern fill on a star polygon — matching C++ pattern_fill.cpp.
///
/// params[0] = pattern_size (10-60, default 30)
/// params[1] = polygon_angle (-180-180, default 0)
pub fn pattern_fill(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let pat_size = params.first().copied().unwrap_or(30.0).clamp(10.0, 60.0) as u32;
    let poly_angle = params.get(1).copied().unwrap_or(0.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let w = width as f64;
    let h = height as f64;

    // Generate a small pattern image (RGBA)
    let ps = pat_size;
    let mut pat_buf = vec![255u8; (ps * ps * 4) as usize];
    {
        let mut pat_ra = RowAccessor::new();
        let stride = (ps * 4) as i32;
        unsafe { pat_ra.attach(pat_buf.as_mut_ptr(), ps, ps, stride) };
        let pat_pf = PixfmtRgba32::new(&mut pat_ra);
        let mut pat_rb = RendererBase::new(pat_pf);
        pat_rb.clear(&Rgba8::new(230, 230, 230, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Draw a small diamond/star in the pattern
        let pc = ps as f64 / 2.0;
        let pr = pc * 0.8;
        let mut star = PathStorage::new();
        for i in 0..6 {
            let angle = i as f64 * std::f64::consts::PI / 3.0 - std::f64::consts::PI / 2.0;
            let r = if i % 2 == 0 { pr } else { pr * 0.4 };
            let px = pc + r * angle.cos();
            let py = pc + r * angle.sin();
            if i == 0 { star.move_to(px, py); } else { star.line_to(px, py); }
        }
        star.close_polygon(0);
        ras.add_path(&mut star, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut pat_rb, &Rgba8::new(100, 60, 160, 255));
    }

    // Create pattern source using ImageAccessorWrap for tiling
    let mut pat_ra = RowAccessor::new();
    let pat_stride = (ps * 4) as i32;
    unsafe { pat_ra.attach(pat_buf.as_mut_ptr(), ps, ps, pat_stride) };
    let wrap_src = ImageAccessorWrap::<4, WrapModeRepeat, WrapModeRepeat>::new(&pat_ra);
    let mut pattern_gen = SpanPatternRgba::new(wrap_src, 0, 0);
    let mut sa: SpanAllocator<Rgba8> = SpanAllocator::new();

    // Build a 14-pointed star polygon
    let cx = w / 2.0;
    let cy = h / 2.0;
    let r_outer = (w.min(h) / 2.0 - 20.0).max(40.0);
    let r_inner = r_outer * 0.5;
    let n = 14;
    let angle_offset = poly_angle * std::f64::consts::PI / 180.0;

    let mut polygon = PathStorage::new();
    for i in 0..(n * 2) {
        let angle = (i as f64) * std::f64::consts::PI / n as f64 + angle_offset;
        let r = if i % 2 == 0 { r_outer } else { r_inner };
        let px = cx + r * angle.cos();
        let py = cy + r * angle.sin();
        if i == 0 { polygon.move_to(px, py); } else { polygon.line_to(px, py); }
    }
    polygon.close_polygon(0);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    ras.add_path(&mut polygon, 0);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut pattern_gen);

    // Outline
    polygon.rewind(0);
    let mut outline = ConvStroke::new(&mut polygon);
    outline.set_width(2.0);
    ras.reset();
    ras.add_path(&mut outline, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Controls
    let mut s_size = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_size.range(10.0, 60.0);
    s_size.label("Pattern Size=%.0f");
    s_size.set_value(pat_size as f64);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_size);

    let mut s_angle = SliderCtrl::new(5.0, 20.0, w - 5.0, 27.0);
    s_angle.range(-180.0, 180.0);
    s_angle.label("Angle=%.1f");
    s_angle.set_value(poly_angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_angle);

    buf
}


// ============================================================================
// Pattern Perspective — pattern with perspective transform
// ============================================================================

/// Perspective-transformed pattern fill in a quad — matching C++ pattern_perspective.cpp.
///
/// params[0..8] = quad vertices (4 x,y pairs)
/// params[8] = transform type (0=affine, 1=bilinear, 2=perspective)
pub fn pattern_perspective(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let default_quad = [
        w * 0.17, h * 0.17,
        w * 0.83, h * 0.08,
        w * 0.83, h * 0.83,
        w * 0.17, h * 0.83,
    ];
    let quad: Vec<f64> = (0..8)
        .map(|i| params.get(i).copied().unwrap_or(default_quad[i]))
        .collect();
    let trans_type = params.get(8).copied().unwrap_or(0.0) as u32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Generate a procedural pattern image
    let img_w = 100u32;
    let img_h = 100u32;
    let mut img_buf = vec![255u8; (img_w * img_h * 4) as usize];
    {
        let mut img_ra = RowAccessor::new();
        let stride = (img_w * 4) as i32;
        unsafe { img_ra.attach(img_buf.as_mut_ptr(), img_w, img_h, stride) };
        let img_pf = PixfmtRgba32::new(&mut img_ra);
        let mut img_rb = RendererBase::new(img_pf);
        img_rb.clear(&Rgba8::new(240, 235, 230, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Checkerboard with colored circles
        let sq = 20;
        for ty in 0..(img_h / sq) {
            for tx in 0..(img_w / sq) {
                if (tx + ty) % 2 == 0 {
                    let mut rect = PathStorage::new();
                    let rx = (tx * sq) as f64;
                    let ry = (ty * sq) as f64;
                    rect.move_to(rx, ry);
                    rect.line_to(rx + sq as f64, ry);
                    rect.line_to(rx + sq as f64, ry + sq as f64);
                    rect.line_to(rx, ry + sq as f64);
                    rect.close_polygon(0);
                    ras.reset();
                    ras.add_path(&mut rect, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut img_rb, &Rgba8::new(200, 210, 220, 255));
                }
            }
        }
        // Central circle
        let mut ell = Ellipse::new(50.0, 50.0, 35.0, 35.0, 50, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut img_rb, &Rgba8::new(60, 100, 180, 255));
    }

    // Set up image source
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_buf.as_mut_ptr(), img_w, img_h, img_stride) };

    // Build quad path
    let mut quad_path = PathStorage::new();
    quad_path.move_to(quad[0], quad[1]);
    quad_path.line_to(quad[2], quad[3]);
    quad_path.line_to(quad[4], quad[5]);
    quad_path.line_to(quad[6], quad[7]);
    quad_path.close_polygon(0);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Render based on transform type
    let dst_quad: [f64; 8] = [quad[0], quad[1], quad[2], quad[3], quad[4], quad[5], quad[6], quad[7]];
    match trans_type {
        1 => {
            // Bilinear
            let tb = TransBilinear::new_quad_to_rect(&dst_quad, 0.0, 0.0, img_w as f64, img_h as f64);
            if tb.is_valid() {
                let mut interp = SpanInterpolatorTrans::new(tb);
                let bg_color = Rgba8::new(255, 255, 255, 255);
                let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut interp);
                let mut sa = SpanAllocator::new();
                ras.add_path(&mut quad_path, 0);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
        2 => {
            // Perspective
            let mut tp = TransPerspective::new();
            tp.quad_to_rect(&dst_quad, 0.0, 0.0, img_w as f64, img_h as f64);
            if tp.is_valid() {
                let mut interp = SpanInterpolatorTrans::new(tp);
                let bg_color = Rgba8::new(255, 255, 255, 255);
                let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut interp);
                let mut sa = SpanAllocator::new();
                ras.add_path(&mut quad_path, 0);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
        _ => {
            // Affine (use first 3 corners for parallelogram mapping)
            let src_parl: [f64; 6] = [0.0, 0.0, img_w as f64, 0.0, img_w as f64, img_h as f64];
            let dst_parl: [f64; 6] = [dst_quad[0], dst_quad[1], dst_quad[2], dst_quad[3], dst_quad[4], dst_quad[5]];
            let mut mtx = TransAffine::new();
            mtx.parl_to_parl(&dst_parl, &src_parl);
            let mut interp = SpanInterpolatorLinear::new(mtx);
            let bg_color = Rgba8::new(255, 255, 255, 255);
            let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut interp);
            let mut sa = SpanAllocator::new();
            ras.add_path(&mut quad_path, 0);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
        }
    }

    // Quad outline
    quad_path.rewind(0);
    let mut outline = ConvStroke::new(&mut quad_path);
    outline.set_width(2.0);
    ras.reset();
    ras.add_path(&mut outline, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Format label
    let labels = ["Affine", "Bilinear", "Perspective"];
    let label = labels.get(trans_type as usize).unwrap_or(&"Affine");
    let mut txt = GsvText::new();
    txt.size(12.0, 0.0);
    txt.start_point(10.0, h - 20.0);
    txt.text(label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.5);
    ras.reset();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}


// ============================================================================
// Pattern Resample — resampled pattern with quality controls
// ============================================================================

/// Perspective pattern with resampling — matching C++ pattern_resample.cpp.
///
/// params[0..8] = quad vertices
/// params[8] = gamma (0.5-3.0, default 1.0)
/// params[9] = blur (0.5-2.0, default 1.0)
pub fn pattern_resample(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let default_quad = [
        w * 0.2, h * 0.2,
        w * 0.8, h * 0.15,
        w * 0.85, h * 0.8,
        w * 0.15, h * 0.85,
    ];
    let quad: Vec<f64> = (0..8)
        .map(|i| params.get(i).copied().unwrap_or(default_quad[i]))
        .collect();
    let gamma_val = params.get(8).copied().unwrap_or(1.0).clamp(0.5, 3.0);
    let _blur_val = params.get(9).copied().unwrap_or(1.0).clamp(0.5, 2.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Generate a more detailed procedural image with gamma applied
    let img_w = 150u32;
    let img_h = 150u32;
    let mut img_buf = vec![255u8; (img_w * img_h * 4) as usize];
    for iy in 0..img_h {
        for ix in 0..img_w {
            let off = ((iy * img_w + ix) * 4) as usize;
            let u = ix as f64 / img_w as f64;
            let v = iy as f64 / img_h as f64;
            // Gradient with concentric circles
            let dx = u - 0.5;
            let dy = v - 0.5;
            let d = (dx * dx + dy * dy).sqrt() * 4.0;
            let r = ((128.0 + 127.0 * (d * 6.28).sin()) as f64).powf(gamma_val).clamp(0.0, 255.0);
            let g = ((128.0 + 127.0 * (d * 6.28 + 2.1).sin()) as f64).powf(gamma_val).clamp(0.0, 255.0);
            let b = ((128.0 + 127.0 * (d * 6.28 + 4.2).sin()) as f64).powf(gamma_val).clamp(0.0, 255.0);
            img_buf[off] = r as u8;
            img_buf[off + 1] = g as u8;
            img_buf[off + 2] = b as u8;
            img_buf[off + 3] = 255;
        }
    }

    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_buf.as_mut_ptr(), img_w, img_h, img_stride) };

    // Perspective transform
    let dst_quad: [f64; 8] = [quad[0], quad[1], quad[2], quad[3], quad[4], quad[5], quad[6], quad[7]];
    let mut tp = TransPerspective::new();
    tp.quad_to_rect(&dst_quad, 0.0, 0.0, img_w as f64, img_h as f64);
    if tp.is_valid() {
        let mut quad_path = PathStorage::new();
        quad_path.move_to(quad[0], quad[1]);
        quad_path.line_to(quad[2], quad[3]);
        quad_path.line_to(quad[4], quad[5]);
        quad_path.line_to(quad[6], quad[7]);
        quad_path.close_polygon(0);

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut interp = SpanInterpolatorTrans::new(tp);
        let bg_color = Rgba8::new(255, 255, 255, 255);
        let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut interp);
        let mut sa = SpanAllocator::new();

        ras.add_path(&mut quad_path, 0);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

        // Outline
        quad_path.rewind(0);
        let mut outline = ConvStroke::new(&mut quad_path);
        outline.set_width(2.0);
        ras.reset();
        ras.add_path(&mut outline, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

        // Controls
        let mut s_gamma = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
        s_gamma.range(0.5, 3.0);
        s_gamma.label("Gamma=%.2f");
        s_gamma.set_value(gamma_val);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);
    }

    buf
}

// ============================================================================
// Lion Outline
// ============================================================================

/// Render the lion with anti-aliased outlines (matching C++ lion_outline.cpp).
///
/// params[0] = angle (radians)
/// params[1] = scale
/// params[2] = skew_x
/// params[3] = skew_y
/// params[4] = line_width
/// params[5] = use_scanline (0 = outline AA, 1 = scanline rasterizer)
pub fn lion_outline(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_rad = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(2).copied().unwrap_or(0.0);
    let skew_y = params.get(3).copied().unwrap_or(0.0);
    let line_width = params.get(4).copied().unwrap_or(1.0).max(0.01);
    let use_scanline = params.get(5).copied().unwrap_or(0.0) > 0.5;

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let base_dx = 120.0;
    let base_dy = 190.0;

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
        // Scanline rasterizer path — conv_stroke + conv_transform
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
        // Outline AA rasterizer path
        let w = line_width * mtx.get_scale();
        let profile = LineProfileAa::with_width(w);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        let mut ras_oaa = RasterizerOutlineAa::new();
        ras_oaa.set_round_cap(true);
        ras_oaa.set_line_join(OutlineAaJoin::Round);

        for i in 0..npaths {
            let start = path_idx[i] as u32;
            let mut transformed = ConvTransform::new(&mut path, mtx);
            ren_oaa.set_color(colors[i]);
            ras_oaa.add_path(&mut transformed, start, &mut ren_oaa);
        }

        // Need to get rb back from ren_oaa for controls
        drop(ren_oaa);
    }

    // Controls
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

    buf
}

// ============================================================================
// Rasterizers2
// ============================================================================

/// Spiral vertex source for rasterizers2 demo.
struct Spiral {
    cx: f64,
    cy: f64,
    r1: f64,
    r2: f64,
    da: f64,
    dr: f64,
    start_angle: f64,
    angle: f64,
    curr_r: f64,
    start: bool,
}

impl Spiral {
    fn new(cx: f64, cy: f64, r1: f64, r2: f64, step: f64, start_angle: f64) -> Self {
        let da = (8.0_f64).to_radians();
        Self {
            cx, cy, r1, r2, da,
            dr: step / 45.0,
            start_angle,
            angle: start_angle,
            curr_r: r1,
            start: true,
        }
    }
}

impl VertexSource for Spiral {
    fn rewind(&mut self, _path_id: u32) {
        self.angle = self.start_angle;
        self.curr_r = self.r1;
        self.start = true;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        use agg_rust::basics::{PATH_CMD_MOVE_TO, PATH_CMD_LINE_TO, PATH_CMD_STOP};
        if self.curr_r > self.r2 {
            return PATH_CMD_STOP;
        }
        *x = self.cx + self.angle.cos() * self.curr_r;
        *y = self.cy + self.angle.sin() * self.curr_r;
        self.curr_r += self.dr;
        self.angle += self.da;
        if self.start {
            self.start = false;
            PATH_CMD_MOVE_TO
        } else {
            PATH_CMD_LINE_TO
        }
    }
}

/// ARGB32 pixmap chain-link pattern for the "Arbitrary Image Pattern" spiral.
///
/// Exact copy of the C++ `pixmap_chain` data from rasterizers2.cpp.
/// Format: [width, height, pixel0, pixel1, ...] where each pixel is 0xAARRGGBB.
static PIXMAP_CHAIN: [u32; 114] = [
    16, 7,
    0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0xb4c29999, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x0cfbf9f9, 0xff9a5757, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x5ae0cccc, 0xffa46767, 0xff660000, 0xff975252, 0x7ed4b8b8, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0xa8c6a0a0, 0xff7f2929, 0xff670202, 0x9ecaa6a6, 0x5ae0cccc, 0x00ffffff,
    0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xa4c7a2a2, 0x3affff00, 0x3affff00, 0xff975151, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000,
    0x00ffffff, 0x5ae0cccc, 0xffa46767, 0xff660000, 0xff954f4f, 0x7ed4b8b8, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0x5ae0cccc, 0xa8c6a0a0, 0xff7f2929, 0xff670202, 0x9ecaa6a6, 0x5ae0cccc, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x0cfbf9f9, 0xff9a5757, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xff660000, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff,
    0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0xb4c29999, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xff9a5757, 0xb4c29999, 0x00ffffff, 0x00ffffff, 0x00ffffff, 0x00ffffff,
];

/// Pattern source reading from an ARGB32 pixmap — port of C++ `pattern_pixmap_argb32`.
///
/// Extracts ARGB components from 32-bit values and returns straight-alpha Rgba8.
/// Note: The C++ version premultiplies because it uses `pixfmt_pre`; our `PixfmtRgba32`
/// uses standard alpha blending, so we return straight (non-premultiplied) colors and
/// let `blend_color_hspan` handle the compositing correctly.
struct PatternPixmapArgb32 {
    pixmap: &'static [u32],
}

impl PatternPixmapArgb32 {
    fn new(pixmap: &'static [u32]) -> Self {
        Self { pixmap }
    }
    fn pw(&self) -> u32 { self.pixmap[0] }
    fn ph(&self) -> u32 { self.pixmap[1] }
}

impl agg_rust::renderer_outline_image::ImagePatternSource for PatternPixmapArgb32 {
    fn width(&self) -> f64 { self.pw() as f64 }
    fn height(&self) -> f64 { self.ph() as f64 }
    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let p = self.pixmap[(y as u32 * self.pw() + x as u32 + 2) as usize];
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;
        let a = p >> 24;
        Rgba8::new(r, g, b, a)
    }
}

/// Render spiral comparison: aliased, AA outline, scanline, and image pattern
/// (matching C++ rasterizers2.cpp).
///
/// params[0] = step (rotation speed, unused in static render)
/// params[1] = line width
/// params[2] = accurate_joins (0 or 1)
/// params[3] = start_angle (degrees)
/// params[4] = scale_pattern (0 or 1, default 1)
/// params[5] = rotate (0 or 1, for control display)
/// params[6] = test_performance (0 or 1, for control display)
pub fn rasterizers2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    use agg_rust::pattern_filters_rgba::PatternFilterBilinearRgba;
    use agg_rust::renderer_outline_image::{
        LineImagePatternPow2, LineImageScale, RendererOutlineImage,
    };

    let line_width = params.get(1).copied().unwrap_or(3.0).max(0.1);
    let accurate_joins = params.get(2).copied().unwrap_or(0.0) > 0.5;
    let start_angle = params.get(3).copied().unwrap_or(0.0).to_radians();
    let scale_pattern = params.get(4).copied().unwrap_or(1.0) > 0.5;
    let rotate = params.get(5).copied().unwrap_or(0.0) > 0.5;
    let test_performance = params.get(6).copied().unwrap_or(0.0) > 0.5;

    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 242, 255));

    let color = Rgba8::new(102, 77, 26, 255);

    // 1. Aliased pixel accuracy (top-left) — Bresenham with rounded coords
    {
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(102, 77, 26, 255));
        let mut ras_al = RasterizerOutline::new(&mut prim);
        let mut s1 = Spiral::new(w / 5.0, h / 4.0 + 50.0, 5.0, 70.0, 8.0, start_angle);
        // For pixel accuracy, manually round coordinates
        let mut px = PathStorage::new();
        s1.rewind(0);
        let (mut vx, mut vy) = (0.0, 0.0);
        loop {
            let cmd = s1.vertex(&mut vx, &mut vy);
            if is_stop(cmd) { break; }
            if is_vertex(cmd) {
                let rx = vx.floor();
                let ry = vy.floor();
                if cmd == agg_rust::basics::PATH_CMD_MOVE_TO {
                    px.move_to(rx, ry);
                } else {
                    px.line_to(rx, ry);
                }
            }
        }
        ras_al.add_path(&mut px, 0);
    }

    // 2. Aliased subpixel accuracy (top-right) — Bresenham direct
    {
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(102, 77, 26, 255));
        let mut ras_al = RasterizerOutline::new(&mut prim);
        let mut s2 = Spiral::new(w / 2.0, h / 4.0 + 50.0, 5.0, 70.0, 8.0, start_angle);
        ras_al.add_path(&mut s2, 0);
    }

    // 3. Anti-aliased outline (bottom-left)
    {
        let profile = LineProfileAa::with_width(line_width);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        ren_oaa.set_color(color);
        let mut ras_oaa = RasterizerOutlineAa::new();
        ras_oaa.set_round_cap(true);
        ras_oaa.set_line_join(if accurate_joins {
            OutlineAaJoin::MiterAccurate
        } else {
            OutlineAaJoin::Round
        });
        let mut s3 = Spiral::new(w / 5.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle);
        ras_oaa.add_path(&mut s3, 0, &mut ren_oaa);
    }

    // 4. Scanline rasterizer (bottom-center)
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s4 = Spiral::new(w / 2.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle);
        let mut stroke = ConvStroke::new(&mut s4);
        stroke.set_width(line_width);
        stroke.set_line_cap(LineCap::Round);
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // 5. Anti-aliased outline with image pattern (bottom-right)
    {
        let src = PatternPixmapArgb32::new(&PIXMAP_CHAIN);
        let src_scaled = LineImageScale::new(&src, line_width);

        let pattern = if scale_pattern {
            LineImagePatternPow2::<PatternFilterBilinearRgba>::with_source(&src_scaled)
        } else {
            LineImagePatternPow2::<PatternFilterBilinearRgba>::with_source(&src)
        };

        let mut ren_img = RendererOutlineImage::new(&mut rb, &pattern);
        if scale_pattern {
            ren_img.set_scale_x(line_width / src.ph() as f64);
        }

        let mut ras_img = RasterizerOutlineAa::new();
        let mut s5 = Spiral::new(
            w - w / 5.0, h - h / 4.0 + 20.0, 5.0, 70.0, 8.0, start_angle,
        );
        ras_img.add_path(&mut s5, 0, &mut ren_img);
    }

    // Labels
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let labels = [
            (50.0, 80.0, "Bresenham lines,\n\nregular accuracy"),
            (w / 2.0 - 50.0, 80.0, "Bresenham lines,\n\nsubpixel accuracy"),
            (50.0, h / 2.0 + 50.0, "Anti-aliased lines"),
            (w / 2.0 - 50.0, h / 2.0 + 50.0, "Scanline rasterizer"),
            (w - w / 5.0 - 50.0, h / 2.0 + 50.0, "Arbitrary Image Pattern"),
        ];
        for (lx, ly, txt) in labels {
            let mut t = GsvText::new();
            t.size(8.0, 0.0);
            t.text(txt);
            t.start_point(lx, ly);
            let mut ts = ConvStroke::new(&mut t);
            ts.set_width(0.7);
            ras.reset();
            ras.add_path(&mut ts, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }
    }

    // Controls — match C++ layout
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut s_step = SliderCtrl::new(10.0, 14.0, 150.0, 22.0);
        s_step.range(0.0, 2.0);
        s_step.set_value(params.get(0).copied().unwrap_or(0.1));
        s_step.label("Step=%1.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_step);

        let mut s_width = SliderCtrl::new(150.0 + 10.0, 14.0, 400.0 - 10.0, 22.0);
        s_width.range(0.0, 14.0);
        s_width.set_value(line_width);
        s_width.label("Width=%1.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

        let mut cbox_test = CboxCtrl::new(10.0, 30.0, "Test Performance");
        cbox_test.text_size(9.0, 7.0);
        cbox_test.set_status(test_performance);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_test);

        let mut cbox_rotate = CboxCtrl::new(130.0 + 10.0, 30.0, "Rotate");
        cbox_rotate.text_size(9.0, 7.0);
        cbox_rotate.set_status(rotate);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_rotate);

        let mut cbox_aj = CboxCtrl::new(200.0 + 10.0, 30.0, "Accurate Joins");
        cbox_aj.text_size(9.0, 7.0);
        cbox_aj.set_status(accurate_joins);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_aj);

        let mut cbox_sp = CboxCtrl::new(310.0 + 10.0, 30.0, "Scale Pattern");
        cbox_sp.text_size(9.0, 7.0);
        cbox_sp.set_status(scale_pattern);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_sp);
    }

    buf
}

// ============================================================================
// Line Patterns
// ============================================================================

/// Brightness-to-alpha lookup table — exact match of C++ `brightness_to_alpha`.
/// Maps brightness index (0..768) → alpha value.
static BRIGHTNESS_TO_ALPHA: [u8; 768] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 254, 254, 254, 254, 254,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 254, 253, 253,
    253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 253, 252,
    252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 251, 251, 251, 251, 251,
    251, 251, 251, 251, 250, 250, 250, 250, 250, 250, 250, 250, 249, 249, 249, 249,
    249, 249, 249, 248, 248, 248, 248, 248, 248, 248, 247, 247, 247, 247, 247, 246,
    246, 246, 246, 246, 246, 245, 245, 245, 245, 245, 244, 244, 244, 244, 243, 243,
    243, 243, 243, 242, 242, 242, 242, 241, 241, 241, 241, 240, 240, 240, 239, 239,
    239, 239, 238, 238, 238, 238, 237, 237, 237, 236, 236, 236, 235, 235, 235, 234,
    234, 234, 233, 233, 233, 232, 232, 232, 231, 231, 230, 230, 230, 229, 229, 229,
    228, 228, 227, 227, 227, 226, 226, 225, 225, 224, 224, 224, 223, 223, 222, 222,
    221, 221, 220, 220, 219, 219, 219, 218, 218, 217, 217, 216, 216, 215, 214, 214,
    213, 213, 212, 212, 211, 211, 210, 210, 209, 209, 208, 207, 207, 206, 206, 205,
    204, 204, 203, 203, 202, 201, 201, 200, 200, 199, 198, 198, 197, 196, 196, 195,
    194, 194, 193, 192, 192, 191, 190, 190, 189, 188, 188, 187, 186, 186, 185, 184,
    183, 183, 182, 181, 180, 180, 179, 178, 177, 177, 176, 175, 174, 174, 173, 172,
    171, 171, 170, 169, 168, 167, 166, 166, 165, 164, 163, 162, 162, 161, 160, 159,
    158, 157, 156, 156, 155, 154, 153, 152, 151, 150, 149, 148, 148, 147, 146, 145,
    144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
    128, 128, 127, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
    112, 111, 110, 109, 108, 107, 106, 105, 104, 102, 101, 100,  99,  98,  97,  96,
     95,  94,  93,  91,  90,  89,  88,  87,  86,  85,  84,  82,  81,  80,  79,  78,
     77,  75,  74,  73,  72,  71,  70,  69,  67,  66,  65,  64,  63,  61,  60,  59,
     58,  57,  56,  54,  53,  52,  51,  50,  48,  47,  46,  45,  44,  42,  41,  40,
     39,  37,  36,  35,  34,  33,  31,  30,  29,  28,  27,  25,  24,  23,  22,  20,
     19,  18,  17,  15,  14,  13,  12,  11,   9,   8,   7,   6,   4,   3,   2,   1,
];

/// Pattern source that reads from RGBA pixel data and converts brightness to alpha.
/// Port of C++ `pattern_src_brightness_to_alpha`.
struct PatternSrcBrightnessToAlpha {
    data: Vec<u8>,   // RGBA data
    w: u32,
    h: u32,
}

impl PatternSrcBrightnessToAlpha {
    fn new(data: Vec<u8>, w: u32, h: u32) -> Self {
        Self { data, w, h }
    }
}

impl agg_rust::renderer_outline_image::ImagePatternSource for PatternSrcBrightnessToAlpha {
    fn width(&self) -> f64 { self.w as f64 }
    fn height(&self) -> f64 { self.h as f64 }
    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let x = x.max(0).min(self.w as i32 - 1) as usize;
        let y = y.max(0).min(self.h as i32 - 1) as usize;
        let off = (y * self.w as usize + x) * 4;
        let r = self.data[off] as u32;
        let g = self.data[off + 1] as u32;
        let b = self.data[off + 2] as u32;
        let sum = r + g + b;
        // C++: i = sum * sizeof(brightness_to_alpha) / (3 * color_type::full_value())
        //    = sum * 768 / 765
        let i = (sum * BRIGHTNESS_TO_ALPHA.len() as u32 / (3 * 255)).min(BRIGHTNESS_TO_ALPHA.len() as u32 - 1) as usize;
        let cover = BRIGHTNESS_TO_ALPHA[i];
        // mult_cover: (255 * cover + 255) >> 8
        let a = ((255u32 * cover as u32) + 255) >> 8;
        Rgba8::new(r, g, b, a)
    }
}

/// Load an embedded pattern image from a .rgba file (8-byte header: u32 LE width, u32 LE height,
/// then width*height*4 bytes of RGBA pixel data).
/// These are the original AGG line pattern images (1.bmp–9.bmp) converted from the PPM sources.
fn load_embedded_pattern(data: &[u8]) -> (u32, u32, Vec<u8>) {
    let w = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let h = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let pixels = data[8..].to_vec();
    debug_assert_eq!(pixels.len(), (w * h * 4) as usize);
    (w, h, pixels)
}

// Embed the original AGG line pattern images (converted from PPM to raw RGBA).
static PATTERN_1: &[u8] = include_bytes!("../../assets/1.rgba");
static PATTERN_2: &[u8] = include_bytes!("../../assets/2.rgba");
static PATTERN_3: &[u8] = include_bytes!("../../assets/3.rgba");
static PATTERN_4: &[u8] = include_bytes!("../../assets/4.rgba");
static PATTERN_5: &[u8] = include_bytes!("../../assets/5.rgba");
static PATTERN_6: &[u8] = include_bytes!("../../assets/6.rgba");
static PATTERN_7: &[u8] = include_bytes!("../../assets/7.rgba");
static PATTERN_8: &[u8] = include_bytes!("../../assets/8.rgba");
static PATTERN_9: &[u8] = include_bytes!("../../assets/9.rgba");

/// Get the original AGG pattern image for a given curve index (0-8).
fn get_pattern(index: usize) -> (u32, u32, Vec<u8>) {
    let data = match index {
        0 => PATTERN_1,
        1 => PATTERN_2,
        2 => PATTERN_3,
        3 => PATTERN_4,
        4 => PATTERN_5,
        5 => PATTERN_6,
        6 => PATTERN_7,
        7 => PATTERN_8,
        8 => PATTERN_9,
        _ => PATTERN_1,
    };
    load_embedded_pattern(data)
}

/// Render bezier curves with image patterns — port of C++ line_patterns.cpp.
///
/// params[0] = scale_x (0.2..3.0, default 1.0)
/// params[1] = start_x (0.0..10.0, default 0.0)
pub fn line_patterns(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    use agg_rust::pattern_filters_rgba::PatternFilterBilinearRgba;
    use agg_rust::renderer_outline_image::{LineImagePattern, RendererOutlineImage};

    let scale_x = params.get(0).copied().unwrap_or(1.0).clamp(0.2, 3.0);
    let start_x = params.get(1).copied().unwrap_or(0.0).clamp(0.0, 10.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    // Match C++ background: rgba(1.0, 1.0, 0.95) → Rgba8(255, 255, 242, 255)
    rb.clear(&Rgba8::new(255, 255, 242, 255));

    // Default bezier curve control points — exact match of C++ line_patterns.cpp
    let defaults: [(f64, f64, f64, f64, f64, f64, f64, f64); 9] = [
        ( 64.0,  19.0,  14.0, 126.0, 118.0, 266.0,  19.0, 265.0),
        (112.0, 113.0, 178.0,  32.0, 200.0, 132.0, 125.0, 438.0),
        (401.0,  24.0, 326.0, 149.0, 285.0,  11.0, 177.0,  77.0),
        (188.0, 427.0, 129.0, 295.0,  19.0, 283.0,  25.0, 410.0),
        (451.0, 346.0, 302.0, 218.0, 265.0, 441.0, 459.0, 400.0),
        (454.0, 198.0,  14.0,  13.0, 220.0, 291.0, 483.0, 283.0),
        (301.0, 398.0, 355.0, 231.0, 209.0, 211.0, 170.0, 353.0),
        (484.0, 101.0, 222.0,  33.0, 486.0, 435.0, 487.0, 138.0),
        (143.0, 147.0,  11.0,  45.0,  83.0, 427.0, 132.0, 197.0),
    ];

    // Read control points from params[2..74] if provided (from interactive JS drag),
    // otherwise use the C++ defaults.
    let curves: [(f64, f64, f64, f64, f64, f64, f64, f64); 9] = if params.len() >= 74 {
        let p = &params[2..];
        let mut c = defaults;
        for i in 0..9 {
            let o = i * 8;
            c[i] = (p[o], p[o+1], p[o+2], p[o+3], p[o+4], p[o+5], p[o+6], p[o+7]);
        }
        c
    } else {
        defaults
    };

    // Draw each bezier curve with its own pattern
    for (i, &(x1, y1, x2, y2, x3, y3, x4, y4)) in curves.iter().enumerate() {
        let (pw, ph, pdata) = get_pattern(i);
        let src = PatternSrcBrightnessToAlpha::new(pdata, pw, ph);
        let pat = LineImagePattern::<PatternFilterBilinearRgba>::with_source(&src);

        let mut ren_img = RendererOutlineImage::new(&mut rb, &pat);
        ren_img.set_scale_x(scale_x);
        ren_img.set_start_x(start_x);

        let mut ras_img = RasterizerOutlineAa::new();
        ras_img.set_line_join(OutlineAaJoin::MiterAccurate);

        // Create bezier curve path
        let mut path = PathStorage::new();
        path.move_to(x1, y1);
        path.curve4(x2, y2, x3, y3, x4, y4);
        let mut curve = ConvCurve::new(&mut path);

        ras_img.add_path(&mut curve, 0, &mut ren_img);
    }

    // Render bezier control visualizations (matching C++ bezier_ctrl rendering).
    // Color: rgba(0, 0.3, 0.5, 0.3) = Rgba8(0, 77, 128, 77)
    let ctrl_color = Rgba8::new(0, 77, 128, 77);
    let point_radius = 5.0;

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    for &(x1, y1, x2, y2, x3, y3, x4, y4) in curves.iter() {
        // Path 0: Control line P1→P2 (stroked straight line)
        {
            let mut line_path = PathStorage::new();
            line_path.move_to(x1, y1);
            line_path.line_to(x2, y2);
            let mut stroke = ConvStroke::new(&mut line_path);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &ctrl_color);
        }

        // Path 1: Control line P3→P4 (stroked straight line)
        {
            let mut line_path = PathStorage::new();
            line_path.move_to(x3, y3);
            line_path.line_to(x4, y4);
            let mut stroke = ConvStroke::new(&mut line_path);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &ctrl_color);
        }

        // Path 2: The bezier curve itself (stroked thin line)
        {
            let mut curve_path = PathStorage::new();
            curve_path.move_to(x1, y1);
            curve_path.curve4(x2, y2, x3, y3, x4, y4);
            let mut conv = ConvCurve::new(&mut curve_path);
            let mut stroke = ConvStroke::new(&mut conv);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &ctrl_color);
        }

        // Paths 3-6: Filled ellipses at each control point
        for &(px, py) in &[(x1, y1), (x2, y2), (x3, y3), (x4, y4)] {
            let mut ell = Ellipse::new(px, py, point_radius, point_radius, 20, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &ctrl_color);
        }
    }

    // Render slider controls
    let w = width as f64;

    let mut s_scale = SliderCtrl::new(5.0, 5.0, 240.0, 12.0);
    s_scale.range(0.2, 3.0);
    s_scale.set_value(scale_x);
    s_scale.label("Scale X=%.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_scale);

    let mut s_start = SliderCtrl::new(250.0, 5.0, w - 5.0, 12.0);
    s_start.range(0.0, 10.0);
    s_start.set_value(start_x);
    s_start.label("Start X=%.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_start);

    buf
}

// ============================================================================
// Line Patterns Clip
// ============================================================================

/// Render AA outline patterns with clip regions (simplified from C++ line_patterns_clip.cpp).
///
/// params[0] = line width
/// params[1] = accurate_joins (0 or 1)
/// params[2] = start_angle (degrees)
pub fn line_patterns_clip(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let line_width = params.get(0).copied().unwrap_or(3.0).max(0.1);
    let accurate_joins = params.get(1).copied().unwrap_or(0.0) > 0.5;
    let start_angle = params.get(2).copied().unwrap_or(0.0).to_radians();

    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let join = if accurate_joins {
        OutlineAaJoin::MiterAccurate
    } else {
        OutlineAaJoin::Round
    };

    // Draw clip region outlines
    let clip_x1 = w * 0.1;
    let clip_y1 = h * 0.1;
    let clip_x2 = w * 0.9;
    let clip_y2 = h * 0.9;
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut rect = PathStorage::new();
        rect.move_to(clip_x1, clip_y1);
        rect.line_to(clip_x2, clip_y1);
        rect.line_to(clip_x2, clip_y2);
        rect.line_to(clip_x1, clip_y2);
        rect.close_polygon(0);
        let mut rs = ConvStroke::new(&mut rect);
        rs.set_width(1.0);
        ras.add_path(&mut rs, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(180, 180, 180, 255));
    }

    // Draw spirals with clipping applied to the outline AA renderer
    let configs: [(f64, f64, Rgba8); 3] = [
        (w * 0.25, h * 0.5, Rgba8::new(153, 0, 0, 255)),
        (w * 0.5, h * 0.5, Rgba8::new(0, 153, 0, 255)),
        (w * 0.75, h * 0.5, Rgba8::new(0, 0, 153, 255)),
    ];

    for (cx, cy, color) in configs {
        let profile = LineProfileAa::with_width(line_width);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        ren_oaa.set_color(color);
        ren_oaa.set_clip_box(clip_x1, clip_y1, clip_x2, clip_y2);
        let mut ras_oaa = RasterizerOutlineAa::new();
        ras_oaa.set_round_cap(true);
        ras_oaa.set_line_join(join);
        let mut spiral = Spiral::new(cx, cy, 5.0, 100.0, 14.0, start_angle);
        ras_oaa.add_path(&mut spiral, 0, &mut ren_oaa);
    }

    // Controls
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut s_width = SliderCtrl::new(10.0, 14.0, w - 10.0, 22.0);
    s_width.range(0.5, 10.0);
    s_width.set_value(line_width);
    s_width.label("Width=%1.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut cbox = CboxCtrl::new(10.0, 30.0, "Accurate Joins");
    cbox.set_status(accurate_joins);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox);

    buf
}

// ============================================================================
// Compositing
// ============================================================================

fn comp_op_from_index(i: u32) -> CompOp {
    // Match C++ compositing/compositing2 demos exactly (minus is intentionally omitted).
    match i {
        0 => CompOp::Clear,
        1 => CompOp::Src,
        2 => CompOp::Dst,
        3 => CompOp::SrcOver,
        4 => CompOp::DstOver,
        5 => CompOp::SrcIn,
        6 => CompOp::DstIn,
        7 => CompOp::SrcOut,
        8 => CompOp::DstOut,
        9 => CompOp::SrcAtop,
        10 => CompOp::DstAtop,
        11 => CompOp::Xor,
        12 => CompOp::Plus,
        13 => CompOp::Multiply,
        14 => CompOp::Screen,
        15 => CompOp::Overlay,
        16 => CompOp::Darken,
        17 => CompOp::Lighten,
        18 => CompOp::ColorDodge,
        19 => CompOp::ColorBurn,
        20 => CompOp::HardLight,
        21 => CompOp::SoftLight,
        22 => CompOp::Difference,
        23 => CompOp::Exclusion,
        _ => CompOp::SrcOver,
    }
}

fn build_resize_mtx(width: u32, height: u32, keep_aspect: bool) -> TransAffine {
    if keep_aspect {
        let mut vp = TransViewport::new();
        vp.preserve_aspect_ratio(0.5, 0.5, AspectRatio::Meet);
        vp.set_world_viewport(0.0, 0.0, 600.0, 400.0);
        vp.set_device_viewport(0.0, 0.0, width as f64, height as f64);
        vp.to_affine()
    } else {
        TransAffine::new_scaling(width as f64 / 600.0, height as f64 / 400.0)
    }
}

fn gradient_affine(x1: f64, y1: f64, x2: f64, y2: f64, d2: f64) -> TransAffine {
    let mut mtx = TransAffine::new();
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    mtx.multiply(&TransAffine::new_scaling_uniform(len / d2));
    mtx.multiply(&TransAffine::new_rotation(dy.atan2(dx)));
    mtx.multiply(&TransAffine::new_translation(x1, y1));
    mtx.invert();
    mtx
}

fn render_circle_gradient(
    rb: &mut RendererBase<PixfmtRgba32>,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    c1: Rgba8,
    c2: Rgba8,
    shadow_alpha: f64,
) {
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc = SpanAllocator::new();
    let lut = GradientLinearColor::new(c1, c2, 256);
    let grad = GradientX;
    let mtx = gradient_affine(x1, y1, x2, y2, 100.0);
    let interp = SpanInterpolatorLinear::new(mtx);
    let mut span = SpanGradient::new(interp, grad, &lut, 0.0, 100.0);

    let r = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt() / 2.0;
    let mut ell = Ellipse::new((x1 + x2) * 0.5 + 5.0, (y1 + y2) * 0.5 - 3.0, r, r, 100, false);
    ras.add_path(&mut ell, 0);
    render_scanlines_aa_solid(
        &mut ras,
        &mut sl,
        rb,
        &Rgba8::new(153, 153, 153, (0.7 * shadow_alpha * 255.0) as u32),
    );

    ras.reset();
    let mut ell2 = Ellipse::new((x1 + x2) * 0.5, (y1 + y2) * 0.5, r, r, 100, false);
    ras.add_path(&mut ell2, 0);
    render_scanlines_aa(&mut ras, &mut sl, rb, &mut alloc, &mut span);
}

fn render_src_shape_gradient(
    rb: &mut RendererBase<PixfmtRgba32CompOp>,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    c1: Rgba8,
    c2: Rgba8,
) {
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc = SpanAllocator::new();
    let lut = GradientLinearColor::new(c1, c2, 256);
    let grad = GradientX;
    let mtx = gradient_affine(x1, y1, x2, y2, 100.0);
    let interp = SpanInterpolatorLinear::new(mtx);
    let mut span = SpanGradient::new(interp, grad, &lut, 0.0, 100.0);
    let mut rr = RoundedRect::new(x1, y1, x2, y2, 40.0);
    ras.add_path(&mut rr, 0);
    render_scanlines_aa(&mut ras, &mut sl, rb, &mut alloc, &mut span);
}

fn alpha_blend_rgba_over(dst: &mut [u8], src: &[u8], sa_mul: f64) {
    let sa = (src[3] as f64 / 255.0) * sa_mul;
    if sa <= 0.0 {
        return;
    }
    let da = dst[3] as f64 / 255.0;
    let sr = src[0] as f64 / 255.0;
    let sg = src[1] as f64 / 255.0;
    let sb = src[2] as f64 / 255.0;
    let dr = dst[0] as f64 / 255.0;
    let dg = dst[1] as f64 / 255.0;
    let db = dst[2] as f64 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    let out_r = sr * sa + dr * (1.0 - sa);
    let out_g = sg * sa + dg * (1.0 - sa);
    let out_b = sb * sa + db * (1.0 - sa);
    dst[0] = (out_r * 255.0 + 0.5) as u8;
    dst[1] = (out_g * 255.0 + 0.5) as u8;
    dst[2] = (out_b * 255.0 + 0.5) as u8;
    dst[3] = (out_a * 255.0 + 0.5) as u8;
}

fn parse_p6_ppm(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if data.len() < 3 || data[0] != b'P' || data[1] != b'6' {
        return None;
    }
    let mut i = 2usize;
    let mut tokens: Vec<String> = Vec::new();
    while tokens.len() < 3 && i < data.len() {
        while i < data.len() && data[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= data.len() {
            break;
        }
        if data[i] == b'#' {
            while i < data.len() && data[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < data.len() && !data[i].is_ascii_whitespace() {
            i += 1;
        }
        if let Ok(tok) = std::str::from_utf8(&data[start..i]) {
            tokens.push(tok.to_string());
        } else {
            return None;
        }
    }
    if tokens.len() != 3 {
        return None;
    }
    let w = tokens[0].parse::<u32>().ok()?;
    let h = tokens[1].parse::<u32>().ok()?;
    let maxv = tokens[2].parse::<u32>().ok()?;
    if maxv != 255 {
        return None;
    }
    while i < data.len() && data[i].is_ascii_whitespace() {
        i += 1;
    }
    let rgb_len = (w * h * 3) as usize;
    if i + rgb_len > data.len() {
        return None;
    }
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for p in 0..(w * h) as usize {
        rgba[p * 4] = data[i + p * 3];
        rgba[p * 4 + 1] = data[i + p * 3 + 1];
        rgba[p * 4 + 2] = data[i + p * 3 + 2];
        rgba[p * 4 + 3] = 255;
    }
    Some((w, h, rgba))
}

fn draw_checkerboard(rb: &mut RendererBase<PixfmtRgba32>) {
    let h = rb.height() as u32;
    let w = rb.width() as u32;
    let mut y = 0u32;
    while y < h {
        let mut x = (((y >> 3) & 1) << 3) as u32;
        while x < w {
            rb.blend_bar(
                x as i32,
                y as i32,
                (x + 7) as i32,
                (y + 7) as i32,
                &Rgba8::new(0xdf, 0xdf, 0xdf, 255),
                255,
            );
            x += 16;
        }
        y += 8;
    }
}

#[cfg(target_arch = "wasm32")]
fn measure_scene_ms<F: FnOnce()>(f: F) -> f64 {
    // std::time::Instant::now() can panic on wasm depending on runtime support.
    f();
    0.0
}

#[cfg(not(target_arch = "wasm32"))]
fn measure_scene_ms<F: FnOnce()>(f: F) -> f64 {
    let t0 = std::time::Instant::now();
    f();
    t0.elapsed().as_secs_f64() * 1000.0
}

/// params[0] = comp_op index (0-23), params[1] = src alpha (0..1), params[2] = dst alpha (0..1)
pub fn compositing(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let comp_op_idx = params.first().copied().unwrap_or(3.0).clamp(0.0, 23.0) as u32;
    let src_alpha = params.get(1).copied().unwrap_or(0.75).clamp(0.0, 1.0);
    let dst_alpha = params.get(2).copied().unwrap_or(1.0).clamp(0.0, 1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(255, 255, 255, 255));
        draw_checkerboard(&mut rb);
    }

    let mut img0 = vec![0u8; (width * height * 4) as usize];
    let stride = (width * 4) as i32;
    let mut ra_img = RowAccessor::new();
    unsafe { ra_img.attach(img0.as_mut_ptr(), width, height, stride) };
    {
        let pf = PixfmtRgba32::new(&mut ra_img);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(0, 0, 0, 0));

        // Destination image (compositing.ppm) at (250, 180), alpha-scaled by Dst Alpha.
        let ppm = include_bytes!("../../../../cpp-references/agg-src/examples/art/compositing.ppm");
        if let Some((iw, ih, rgba)) = parse_p6_ppm(ppm) {
            for y in 0..ih.min(height.saturating_sub(180)) {
                for x in 0..iw.min(width.saturating_sub(250)) {
                    let si = ((y * iw + x) * 4) as usize;
                    let di = (((y + 180) * width + (x + 250)) * 4) as usize;
                    alpha_blend_rgba_over(&mut img0[di..di + 4], &rgba[si..si + 4], dst_alpha);
                }
            }
        }

        render_circle_gradient(
            &mut rb,
            70.0 * 3.0,
            100.0 + 24.0 * 3.0,
            37.0 * 3.0,
            100.0 + 79.0 * 3.0,
            Rgba8::new(0xFD, 0xF0, 0x6F, (dst_alpha * 255.0) as u32),
            Rgba8::new(0xFE, 0x9F, 0x34, (dst_alpha * 255.0) as u32),
            dst_alpha,
        );
    }

    let scene_ms = measure_scene_ms(|| {
        let mut pf = PixfmtRgba32CompOp::new(&mut ra_img);
        pf.set_comp_op(comp_op_from_index(comp_op_idx));
        let mut rb = RendererBase::new(pf);
        render_src_shape_gradient(
            &mut rb,
            350.0,
            100.0 + 24.0 * 3.0,
            157.0,
            100.0 + 79.0 * 3.0,
            Rgba8::new(0x7F, 0xC1, 0xFF, (src_alpha * 255.0) as u32),
            Rgba8::new(0x05, 0x00, 0x5F, (src_alpha * 255.0) as u32),
        );
    });

    // Blend scene image over checkerboard.
    for i in (0..buf.len()).step_by(4) {
        let src = [img0[i], img0[i + 1], img0[i + 2], img0[i + 3]];
        alpha_blend_rgba_over(&mut buf[i..i + 4], &src, 1.0);
    }

    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let mut t = GsvText::new();
    t.size(10.0, 0.0);
    t.start_point(10.0, 35.0);
    t.text(&format!("{:.2} ms", scene_ms));
    let mut ts = ConvStroke::new(&mut t);
    ts.set_width(1.5);
    ras.add_path(&mut ts, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    let mut s_src = SliderCtrl::new(5.0, 5.0, 400.0, 11.0);
    s_src.label("Src Alpha=%.2f");
    s_src.range(0.0, 1.0);
    s_src.set_value(src_alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_src);

    let mut s_dst = SliderCtrl::new(5.0, 20.0, 400.0, 26.0);
    s_dst.label("Dst Alpha=%.2f");
    s_dst.range(0.0, 1.0);
    s_dst.set_value(dst_alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_dst);

    let mut comp = RboxCtrl::new(420.0, 5.0, 590.0, 340.0);
    comp.text_size(6.8, 0.0);
    for item in [
        "clear", "src", "dst", "src-over", "dst-over", "src-in", "dst-in", "src-out",
        "dst-out", "src-atop", "dst-atop", "xor", "plus", "multiply", "screen", "overlay",
        "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light",
        "difference", "exclusion",
    ] {
        comp.add_item(item);
    }
    comp.set_cur_item(comp_op_idx as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut comp);

    buf
}

fn radial_shape(
    rb: &mut RendererBase<PixfmtRgba32CompOp>,
    ramp: &GradientLut,
    resize_mtx: &TransAffine,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) {
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc = SpanAllocator::new();
    let grad = GradientRadial;
    let cx = (x1 + x2) * 0.5;
    let cy = (y1 + y2) * 0.5;
    let r = 0.5 * (x2 - x1).min(y2 - y1);

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_scaling_uniform(r / 100.0));
    mtx.multiply(&TransAffine::new_translation(cx, cy));
    mtx.multiply(resize_mtx);
    mtx.invert();
    let interp = SpanInterpolatorLinear::new(mtx);
    let mut span = SpanGradient::new(interp, grad, ramp, 0.0, 100.0);

    let mut ell = Ellipse::new(cx, cy, r, r, 100, false);
    let mut trans = ConvTransform::new(&mut ell, *resize_mtx);
    ras.add_path(&mut trans, 0);
    render_scanlines_aa(&mut ras, &mut sl, rb, &mut alloc, &mut span);
}

fn generate_color_ramp(c1: Rgba8, c2: Rgba8, c3: Rgba8, c4: Rgba8) -> GradientLut {
    let mut lut = GradientLut::new(256);
    lut.remove_all();
    lut.add_color(0.0, c1);
    lut.add_color(85.0 / 255.0, c2);
    lut.add_color(170.0 / 255.0, c3);
    lut.add_color(1.0, c4);
    lut.build_lut();
    lut
}

/// params[0] = comp_op (0..23), params[1] = src alpha (0..1), params[2] = dst alpha (0..1)
pub fn compositing2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let comp_op_idx = params.first().copied().unwrap_or(3.0).clamp(0.0, 23.0) as u32;
    let src_alpha = params.get(1).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let dst_alpha = params.get(2).copied().unwrap_or(1.0).clamp(0.0, 1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));
    drop(rb);

    let resize_mtx = build_resize_mtx(width, height, true);
    let ramp1 = generate_color_ramp(
        Rgba8::new(0, 0, 0, (dst_alpha * 255.0) as u32),
        Rgba8::new(0, 0, 255, (dst_alpha * 255.0) as u32),
        Rgba8::new(0, 255, 0, (dst_alpha * 255.0) as u32),
        Rgba8::new(255, 0, 0, 0),
    );
    let ramp2 = generate_color_ramp(
        Rgba8::new(0, 0, 0, (src_alpha * 255.0) as u32),
        Rgba8::new(0, 0, 255, (src_alpha * 255.0) as u32),
        Rgba8::new(0, 255, 0, (src_alpha * 255.0) as u32),
        Rgba8::new(255, 0, 0, 0),
    );

    let mut pf_comp = PixfmtRgba32CompOp::new(&mut ra);
    pf_comp.set_comp_op(CompOp::Difference);
    let mut rb_comp = RendererBase::new(pf_comp);
    radial_shape(&mut rb_comp, &ramp1, &resize_mtx, 50.0, 50.0, 370.0, 370.0);

    rb_comp.ren_mut().set_comp_op(comp_op_from_index(comp_op_idx));
    let cx = 50.0;
    let cy = 50.0;
    radial_shape(&mut rb_comp, &ramp2, &resize_mtx, cx + 50.0, cy + 50.0, cx + 190.0, cy + 190.0);
    radial_shape(&mut rb_comp, &ramp2, &resize_mtx, cx + 130.0, cy + 50.0, cx + 270.0, cy + 190.0);
    radial_shape(&mut rb_comp, &ramp2, &resize_mtx, cx + 50.0, cy + 130.0, cx + 190.0, cy + 270.0);
    radial_shape(&mut rb_comp, &ramp2, &resize_mtx, cx + 130.0, cy + 130.0, cx + 270.0, cy + 270.0);
    drop(rb_comp);

    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let mut s_dst = SliderCtrl::new(5.0, 5.0, 400.0, 11.0);
    s_dst.label("Dst Alpha=%.2f");
    s_dst.range(0.0, 1.0);
    s_dst.set_value(dst_alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_dst);

    let mut s_src = SliderCtrl::new(5.0, 20.0, 400.0, 26.0);
    s_src.label("Src Alpha=%.2f");
    s_src.range(0.0, 1.0);
    s_src.set_value(src_alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_src);

    let mut comp = RboxCtrl::new(420.0, 5.0, 590.0, 340.0);
    comp.text_size(6.8, 0.0);
    for item in [
        "clear", "src", "dst", "src-over", "dst-over", "src-in", "dst-in", "src-out",
        "dst-out", "src-atop", "dst-atop", "xor", "plus", "multiply", "screen", "overlay",
        "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light",
        "difference", "exclusion",
    ] {
        comp.add_item(item);
    }
    comp.set_cur_item(comp_op_idx as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut comp);

    buf
}

/// Helper: render compound rasterizer output using per-style solid colors.
fn render_compound(
    rasc: &mut RasterizerCompoundAa,
    rb: &mut RendererBase<PixfmtRgba32>,
    colors: &[Rgba8],
) {
    use agg_rust::rasterizer_scanline_aa::Scanline;
    if !rasc.rewind_scanlines() {
        return;
    }
    let mut sl = ScanlineU8::new();
    sl.reset(rasc.min_x(), rasc.max_x());
    loop {
        let num_styles = rasc.sweep_styles();
        if num_styles == 0 {
            break;
        }
        for s in 0..num_styles {
            let style_id = rasc.style(s) as usize;
            if rasc.sweep_scanline(&mut sl, s as i32) {
                let color = if style_id < colors.len() {
                    &colors[style_id]
                } else {
                    &colors[colors.len() - 1]
                };
                let y = Scanline::y(&sl);
                for span in sl.begin() {
                    let x = span.x;
                    let len = span.len;
                    if len > 0 {
                        rb.blend_solid_hspan(
                            x,
                            y,
                            len,
                            color,
                            &sl.covers()[span.cover_offset..span.cover_offset + len as usize],
                        );
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct FlashPathStyle {
    path_id: u32,
    left_fill: i32,
    right_fill: i32,
    line: i32,
}

#[derive(Clone)]
struct FlashShape {
    path: PathStorage,
    styles: Vec<FlashPathStyle>,
}

fn parse_flash_shapes() -> Vec<FlashShape> {
    let txt = include_str!("../../../../cpp-references/agg-src/examples/art/shapes.txt");
    let mut out: Vec<FlashShape> = Vec::new();
    let mut current = FlashShape {
        path: PathStorage::new(),
        styles: Vec::new(),
    };
    let mut started = false;

    for line in txt.lines() {
        let s = line.trim();
        if s.starts_with("=======BeginShape") {
            if started && !current.styles.is_empty() {
                out.push(current.clone());
                current = FlashShape {
                    path: PathStorage::new(),
                    styles: Vec::new(),
                };
            }
            started = true;
            continue;
        }
        if !started {
            continue;
        }
        if let Some(rest) = s.strip_prefix("Path ") {
            let vals: Vec<&str> = rest.split_whitespace().collect();
            if vals.len() >= 5 {
                let left = vals[0].parse::<i32>().unwrap_or(-1);
                let right = vals[1].parse::<i32>().unwrap_or(-1);
                let line_style = vals[2].parse::<i32>().unwrap_or(-1);
                let ax = vals[3].parse::<f64>().unwrap_or(0.0);
                let ay = vals[4].parse::<f64>().unwrap_or(0.0);
                let pid = current.path.start_new_path();
                current.path.move_to(ax, ay);
                current.styles.push(FlashPathStyle {
                    path_id: pid as u32,
                    left_fill: left,
                    right_fill: right,
                    line: line_style,
                });
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix("Curve ") {
            let vals: Vec<&str> = rest.split_whitespace().collect();
            if vals.len() >= 4 {
                let cx = vals[0].parse::<f64>().unwrap_or(0.0);
                let cy = vals[1].parse::<f64>().unwrap_or(0.0);
                let ax = vals[2].parse::<f64>().unwrap_or(0.0);
                let ay = vals[3].parse::<f64>().unwrap_or(0.0);
                current.path.curve3(cx, cy, ax, ay);
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix("Line ") {
            let vals: Vec<&str> = rest.split_whitespace().collect();
            if vals.len() >= 2 {
                let ax = vals[0].parse::<f64>().unwrap_or(0.0);
                let ay = vals[1].parse::<f64>().unwrap_or(0.0);
                current.path.line_to(ax, ay);
            }
        }
    }
    if started && !current.styles.is_empty() {
        out.push(current);
    }
    out
}

fn flash_rand_byte(state: &mut u32) -> u8 {
    // LCG variant used by the AGG demo output we're matching.
    *state = state.wrapping_mul(1103515245).wrapping_add(12345);
    ((*state >> 16) & 0xFF) as u8
}

fn flash_palette() -> Vec<Rgba8> {
    let mut holdrand: u32 = 1;
    let mut colors = vec![Rgba8::new(0, 0, 0, 255); 100];
    for c in &mut colors {
        *c = Rgba8::new(
            flash_rand_byte(&mut holdrand) as u32,
            flash_rand_byte(&mut holdrand) as u32,
            flash_rand_byte(&mut holdrand) as u32,
            230,
        );
        c.premultiply();
    }
    colors
}

fn flash_shape_transform(shape: &FlashShape, width: u32, height: u32, scale: f64, rotation: f64) -> TransAffine {
    let mut path = shape.path.clone();
    let path_ids: Vec<u32> = shape.styles.iter().map(|s| s.path_id).collect();
    let mut base = TransAffine::new();
    if let Some(rect) = agg_rust::bounding_rect::bounding_rect(&mut path, &path_ids, 0, path_ids.len()) {
        let mut vp = TransViewport::new();
        vp.preserve_aspect_ratio(0.5, 0.5, AspectRatio::Meet);
        vp.set_world_viewport(rect.x1, rect.y1, rect.x2, rect.y2);
        vp.set_device_viewport(0.0, 0.0, width as f64, height as f64);
        base = vp.to_affine();
    }
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.5;
    let mut user = TransAffine::new();
    user.multiply(&TransAffine::new_translation(-cx, -cy));
    user.multiply(&TransAffine::new_scaling_uniform(scale));
    user.multiply(&TransAffine::new_rotation(rotation));
    user.multiply(&TransAffine::new_translation(cx, cy));
    user.multiply(&base);
    user
}

fn flash_user_transform_from_params(width: u32, height: u32, params: &[f64], shape: &FlashShape) -> (usize, TransAffine) {
    // Backward compatible path:
    // params[0] = scale, params[1] = rotation_degrees, params[2] = shape_index
    if params.len() < 7 {
        let scale_val = params.first().copied().unwrap_or(1.0).max(0.01);
        let rotation = params.get(1).copied().unwrap_or(0.0).to_radians();
        let shape_index = params.get(2).copied().unwrap_or(0.0).max(0.0) as usize;
        return (shape_index, flash_shape_transform(shape, width, height, scale_val, rotation));
    }

    // Extended state:
    // params[0]      = shape_index
    // params[1..=6]  = user affine [sx, shy, shx, sy, tx, ty], pre-base
    let shape_index = params[0].max(0.0) as usize;
    let mut user = TransAffine::new_custom(
        params[1],
        params[2],
        params[3],
        params[4],
        params[5],
        params[6],
    );
    let mut path = shape.path.clone();
    let path_ids: Vec<u32> = shape.styles.iter().map(|s| s.path_id).collect();
    let mut base = TransAffine::new();
    if let Some(rect) = agg_rust::bounding_rect::bounding_rect(&mut path, &path_ids, 0, path_ids.len()) {
        let mut vp = TransViewport::new();
        vp.preserve_aspect_ratio(0.5, 0.5, AspectRatio::Meet);
        vp.set_world_viewport(rect.x1, rect.y1, rect.x2, rect.y2);
        vp.set_device_viewport(0.0, 0.0, width as f64, height as f64);
        base = vp.to_affine();
    }
    user.multiply(&base);
    (shape_index, user)
}

fn flash_apply_vertex_overrides(path: &mut PathStorage, params: &[f64], start: usize) {
    let total = path.total_vertices();
    let mut i = start;
    while i + 2 < params.len() {
        let idx = params[i].round() as isize;
        let x = params[i + 1];
        let y = params[i + 2];
        if idx >= 0 {
            let u = idx as usize;
            if u < total {
                path.modify_vertex(u, x, y);
            }
        }
        i += 3;
    }
}

fn flash_params_state_start(params: &[f64]) -> usize {
    // Legacy params have no extra state.
    if params.len() < 7 {
        return params.len();
    }
    10
}

fn flash_user_scale(params: &[f64]) -> f64 {
    if params.len() >= 7 {
        let sx = params[1];
        let shx = params[3];
        (sx * sx + shx * shx).sqrt().max(0.001)
    } else {
        params.first().copied().unwrap_or(1.0).abs().max(0.001)
    }
}

pub fn flash_pick_vertex(demo2: bool, width: u32, height: u32, params: &[f64], x: f64, y: f64, radius: f64) -> i32 {
    let shapes = parse_flash_shapes();
    if shapes.is_empty() {
        return -1;
    }
    let shape_for_index = &shapes[0];
    let (shape_index, _) = flash_user_transform_from_params(width, height, params, shape_for_index);
    let shape = &shapes[shape_index % shapes.len()];
    let (_, mtx) = flash_user_transform_from_params(width, height, params, shape);

    let mut path = shape.path.clone();
    flash_apply_vertex_overrides(&mut path, params, flash_params_state_start(params));

    // C++ logic:
    // m_scale.inverse_transform(x, y); m_shape.hit_test(x, y, 4/m_scale.scale())
    let mut sx = x;
    let mut sy = y;
    let mut inv_user = mtx;
    inv_user.invert();
    inv_user.transform(&mut sx, &mut sy);
    let scale = mtx.scaling_abs().0.max(0.001);
    let r = radius / scale;
    let _ = demo2;

    for i in 0..path.total_vertices() {
        let mut vx = 0.0;
        let mut vy = 0.0;
        let cmd = path.vertex_idx(i, &mut vx, &mut vy);
        if is_vertex(cmd) {
            let dx = sx - vx;
            let dy = sy - vy;
            if (dx * dx + dy * dy).sqrt() <= r {
                return i as i32;
            }
        }
    }
    -1
}

pub fn flash_screen_to_shape(width: u32, height: u32, params: &[f64], x: f64, y: f64) -> Vec<f64> {
    let shapes = parse_flash_shapes();
    if shapes.is_empty() {
        return vec![x, y];
    }
    let shape_for_index = &shapes[0];
    let (shape_index, mtx0) = flash_user_transform_from_params(width, height, params, shape_for_index);
    let shape = &shapes[shape_index % shapes.len()];
    let (_, mtx) = flash_user_transform_from_params(width, height, params, shape);
    let _ = mtx0; // Keep decode symmetry; mtx from selected shape is the one we need.

    let mut sx = x;
    let mut sy = y;
    let mut inv = mtx;
    inv.invert();
    inv.transform(&mut sx, &mut sy);
    vec![sx, sy]
}

/// Legacy params: [scale, rotation_degrees, shape_index]
/// Extended params:
/// [shape_index, m0, m1, m2, m3, m4, m5, hit_x, hit_y, hit_active, ...vertex_overrides(idx,x,y)]
pub fn flash_rasterizer(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let shapes = parse_flash_shapes();
    if shapes.is_empty() {
        let mut buf = Vec::new();
        let mut ra = RowAccessor::new();
        setup_renderer(&mut buf, &mut ra, width, height);
        return buf;
    }
    let (shape_index, _) = flash_user_transform_from_params(width, height, params, &shapes[0]);
    let shape = &shapes[shape_index % shapes.len()];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 242, 255));

    let (_, mtx) = flash_user_transform_from_params(width, height, params, shape);
    let colors = flash_palette();
    let mut fill_src_path = shape.path.clone();
    flash_apply_vertex_overrides(&mut fill_src_path, params, flash_params_state_start(params));

    // Fill (compound rasterizer), matching flash_rasterizer.cpp behavior.
    let mut rasc = RasterizerCompoundAa::new();
    rasc.clip_box(0.0, 0.0, width as f64, height as f64);
    rasc.layer_order(LayerOrder::Direct);
    let mut fill_path = fill_src_path.clone();
    let mut fill_curve = ConvCurve::new(&mut fill_path);
    fill_curve.set_approximation_scale(flash_user_scale(params).max(1.0));
    let mut trans_shape = ConvTransform::new(&mut fill_curve, mtx);
    for st in &shape.styles {
        if st.left_fill >= 0 || st.right_fill >= 0 {
            rasc.styles(st.left_fill, st.right_fill);
            rasc.add_path(&mut trans_shape, st.path_id);
        }
    }
    render_compound(&mut rasc, &mut rb, &colors);

    // Hit-test behavior from C++: right mouse button suppresses stroke rendering
    // when point falls inside any filled region.
    let mut draw_strokes = true;
    if params.len() >= 10 && params[9] > 0.5 {
        let mut ras_hit = RasterizerScanlineAa::new();
        ras_hit.clip_box(0.0, 0.0, width as f64, height as f64);
        let mut fill_hit_path = fill_src_path.clone();
        let mut fill_hit_curve = ConvCurve::new(&mut fill_hit_path);
        fill_hit_curve.set_approximation_scale(flash_user_scale(params).max(1.0));
        let mut trans_hit = ConvTransform::new(&mut fill_hit_curve, mtx);
        for st in &shape.styles {
            if st.left_fill >= 0 || st.right_fill >= 0 {
                ras_hit.add_path(&mut trans_hit, st.path_id);
            }
        }
        let hx = params[7] as i32;
        let hy_top_down = params[8] as i32;
        let hy_bottom_up = (height as f64 - params[8]) as i32;
        if ras_hit.hit_test(hx, hy_top_down) || ras_hit.hit_test(hx, hy_bottom_up) {
            draw_strokes = false;
        }
    }

    // Strokes
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut stroke_path = fill_src_path.clone();
    let mut curve = ConvCurve::new(&mut stroke_path);
    let user_scale = flash_user_scale(params);
    curve.set_approximation_scale(user_scale.max(1.0));
    let mut trans_curve = ConvTransform::new(&mut curve, mtx);
    let mut stroke = ConvStroke::new(&mut trans_curve);
    stroke.set_width(user_scale.sqrt());
    stroke.set_line_join(LineJoin::Round);
    stroke.set_line_cap(LineCap::Round);
    if draw_strokes {
        for st in &shape.styles {
            if st.line >= 0 {
                ras.reset();
                ras.add_path(&mut stroke, st.path_id);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 128));
            }
        }
    }

    let mut t = GsvText::new();
    t.size(8.0, 0.0);
    t.flip(true);
    t.start_point(10.0, 20.0);
    t.text("Space: Next Shape\n\n+/- : ZoomIn/ZoomOut (with respect to the mouse pointer)");
    let mut ts = ConvStroke::new(&mut t);
    ts.set_width(1.6);
    ts.set_line_cap(LineCap::Round);
    ras.reset();
    ras.add_path(&mut ts, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

/// Legacy params: [scale, rotation_degrees, shape_index]
/// Extended params:
/// [shape_index, m0, m1, m2, m3, m4, m5, _, _, _, ...vertex_overrides(idx,x,y)]
pub fn flash_rasterizer2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let shapes = parse_flash_shapes();
    if shapes.is_empty() {
        let mut buf = Vec::new();
        let mut ra = RowAccessor::new();
        setup_renderer(&mut buf, &mut ra, width, height);
        return buf;
    }
    let (shape_index, _) = flash_user_transform_from_params(width, height, params, &shapes[0]);
    let shape = &shapes[shape_index % shapes.len()];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 242, 255));

    let (_, mtx) = flash_user_transform_from_params(width, height, params, shape);
    let colors = flash_palette();
    let min_style = shape
        .styles
        .iter()
        .flat_map(|s| [s.left_fill, s.right_fill])
        .filter(|v| *v >= 0)
        .min()
        .unwrap_or(0);
    let max_style = shape
        .styles
        .iter()
        .flat_map(|s| [s.left_fill, s.right_fill])
        .filter(|v| *v >= 0)
        .max()
        .unwrap_or(0);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    ras.auto_close(false);
    let mut fill_path = shape.path.clone();
    flash_apply_vertex_overrides(&mut fill_path, params, flash_params_state_start(params));
    let mut fill_curve = ConvCurve::new(&mut fill_path);
    fill_curve.set_approximation_scale(flash_user_scale(params).max(1.0));
    let mut trans_shape = ConvTransform::new(&mut fill_curve, mtx);
    for s in min_style..=max_style {
        ras.reset();
        for st in &shape.styles {
            if st.left_fill != st.right_fill {
                if st.left_fill == s {
                    ras.add_path(&mut trans_shape, st.path_id);
                }
                if st.right_fill == s {
                    let mut tmp = PathStorage::new();
                    tmp.concat_path(&mut trans_shape, st.path_id);
                    tmp.invert_polygon(0);
                    ras.add_path(&mut tmp, 0);
                }
            }
        }
        let color = colors.get(s as usize).copied().unwrap_or(Rgba8::new(0, 0, 0, 255));
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }
    ras.auto_close(true);

    let mut stroke_path = shape.path.clone();
    flash_apply_vertex_overrides(&mut stroke_path, params, flash_params_state_start(params));
    let mut curve = ConvCurve::new(&mut stroke_path);
    let user_scale = flash_user_scale(params);
    curve.set_approximation_scale(user_scale.max(1.0));
    let mut trans_curve = ConvTransform::new(&mut curve, mtx);
    let mut stroke = ConvStroke::new(&mut trans_curve);
    stroke.set_width(user_scale.sqrt());
    stroke.set_line_join(LineJoin::Round);
    stroke.set_line_cap(LineCap::Round);
    for st in &shape.styles {
        if st.line >= 0 {
            ras.reset();
            ras.add_path(&mut stroke, st.path_id);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 128));
        }
    }

    let mut t = GsvText::new();
    t.size(8.0, 0.0);
    t.flip(true);
    t.start_point(10.0, 20.0);
    t.text("Space: Next Shape\n\n+/- : ZoomIn/ZoomOut (with respect to the mouse pointer)");
    let mut ts = ConvStroke::new(&mut t);
    ts.set_width(1.6);
    ts.set_line_cap(LineCap::Round);
    ras.reset();
    ras.add_path(&mut ts, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

fn compose_rasterizer_compound_path(path: &mut PathStorage) {
    path.move_to(28.47, 6.45);
    path.curve3(21.58, 1.12, 19.82, 0.29);
    path.curve3(17.19, -0.93, 14.21, -0.93);
    path.curve3(9.57, -0.93, 6.57, 2.25);
    path.curve3(3.56, 5.42, 3.56, 10.60);
    path.curve3(3.56, 13.87, 5.03, 16.26);
    path.curve3(7.03, 19.58, 11.99, 22.51);
    path.curve3(16.94, 25.44, 28.47, 29.64);
    path.line_to(28.47, 31.40);
    path.curve3(28.47, 38.09, 26.34, 40.58);
    path.curve3(24.22, 43.07, 20.17, 43.07);
    path.curve3(17.09, 43.07, 15.28, 41.41);
    path.curve3(13.43, 39.75, 13.43, 37.60);
    path.line_to(13.53, 34.77);
    path.curve3(13.53, 32.52, 12.38, 31.30);
    path.curve3(11.23, 30.08, 9.38, 30.08);
    path.curve3(7.57, 30.08, 6.42, 31.35);
    path.curve3(5.27, 32.62, 5.27, 34.81);
    path.curve3(5.27, 39.01, 9.57, 42.53);
    path.curve3(13.87, 46.04, 21.63, 46.04);
    path.curve3(27.59, 46.04, 31.40, 44.04);
    path.curve3(34.28, 42.53, 35.64, 39.31);
    path.curve3(36.52, 37.21, 36.52, 30.71);
    path.line_to(36.52, 15.53);
    path.curve3(36.52, 9.13, 36.77, 7.69);
    path.curve3(37.01, 6.25, 37.57, 5.76);
    path.curve3(38.13, 5.27, 38.87, 5.27);
    path.curve3(39.65, 5.27, 40.23, 5.62);
    path.curve3(41.26, 6.25, 44.19, 9.18);
    path.line_to(44.19, 6.45);
    path.curve3(38.72, -0.88, 33.74, -0.88);
    path.curve3(31.35, -0.88, 29.93, 0.78);
    path.curve3(28.52, 2.44, 28.47, 6.45);
    path.close_polygon(0);

    path.move_to(28.47, 9.62);
    path.line_to(28.47, 26.66);
    path.curve3(21.09, 23.73, 18.95, 22.51);
    path.curve3(15.09, 20.36, 13.43, 18.02);
    path.curve3(11.77, 15.67, 11.77, 12.89);
    path.curve3(11.77, 9.38, 13.87, 7.06);
    path.curve3(15.97, 4.74, 18.70, 4.74);
    path.curve3(22.41, 4.74, 28.47, 9.62);
    path.close_polygon(0);
}

/// params[0] = width (-20..50)
/// params[1] = alpha1 (0..1)
/// params[2] = alpha2 (0..1)
/// params[3] = alpha3 (0..1)
/// params[4] = alpha4 (0..1)
/// params[5] = invert layer order (0/1)
pub fn rasterizer_compound(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let stroke_width = params.get(0).copied().unwrap_or(10.0).clamp(-20.0, 50.0);
    let alpha1 = params.get(1).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let alpha2 = params.get(2).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let alpha3 = params.get(3).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let alpha4 = params.get(4).copied().unwrap_or(1.0).clamp(0.0, 1.0);
    let invert_order = params.get(5).copied().unwrap_or(0.0) > 0.5;
    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // C++: clear with a horizontal gradient from yellow to cyan.
    for y in 0..height as usize {
        for x in 0..width as usize {
            let k = x as f64 / w.max(1.0);
            let c = Rgba8::new(255, 255, 0, 255).gradient(&Rgba8::new(0, 255, 255, 255), k);
            let i = (y * width as usize + x) * 4;
            buf[i] = c.r as u8;
            buf[i + 1] = c.g as u8;
            buf[i + 2] = c.b as u8;
            buf[i + 3] = c.a as u8;
        }
    }

    // Draw the same two background triangles as C++.
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    ras.move_to_d(0.0, 0.0);
    ras.line_to_d(w, 0.0);
    ras.line_to_d(w, h);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 100, 0, 255));
    ras.reset();
    ras.move_to_d(0.0, 0.0);
    ras.line_to_d(0.0, h);
    ras.line_to_d(w, 0.0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 100, 100, 255));

    // Build transformed glyph path.
    let mut base_path = PathStorage::new();
    compose_rasterizer_compound_path(&mut base_path);
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_scaling_uniform(4.0));
    mtx.multiply(&TransAffine::new_translation(150.0, 100.0));
    let mut trans = ConvTransform::new(&mut base_path, mtx);
    let mut curve = ConvCurve::new(&mut trans);
    let mut stroke = ConvStroke::new(&mut curve);
    stroke.set_width(stroke_width);

    let mut rasc = RasterizerCompoundAa::new();
    rasc.clip_box(0.0, 0.0, w, h);
    rasc.layer_order(if invert_order { LayerOrder::Inverse } else { LayerOrder::Direct });

    let mut ell = Ellipse::new(220.0, 180.0, 120.0, 10.0, 128, false);
    let mut str_ell = ConvStroke::new(&mut ell);
    str_ell.set_width(stroke_width / 2.0);

    rasc.styles(3, -1);
    rasc.add_path(&mut str_ell, 0);
    rasc.styles(2, -1);
    rasc.add_path(&mut ell, 0);
    rasc.styles(1, -1);
    rasc.add_path(&mut stroke, 0);
    rasc.styles(0, -1);
    rasc.add_path(&mut curve, 0);

    let mut colors = [
        Rgba8::new(0, 0, 255, 255),
        Rgba8::new(143, 90, 6, 255),
        Rgba8::new(51, 0, 151, 255),
        Rgba8::new(255, 0, 108, 255),
    ];
    colors[3].set_opacity(alpha1);
    colors[2].set_opacity(alpha2);
    colors[1].set_opacity(alpha3);
    colors[0].set_opacity(alpha4);
    for c in &mut colors {
        c.premultiply();
    }
    render_compound(&mut rasc, &mut rb, &colors);

    // Controls rendered on the canvas (matching C++ placement/labels).
    let mut s_sw = SliderCtrl::new(190.0, 5.0, 430.0, 12.0);
    s_sw.range(-20.0, 50.0);
    s_sw.set_value(stroke_width);
    s_sw.label("Width=%1.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_sw);

    let mut s_a1 = SliderCtrl::new(5.0, 5.0, 180.0, 12.0);
    s_a1.range(0.0, 1.0);
    s_a1.set_value(alpha1);
    s_a1.label("Alpha1=%1.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_a1);

    let mut s_a2 = SliderCtrl::new(5.0, 25.0, 180.0, 32.0);
    s_a2.range(0.0, 1.0);
    s_a2.set_value(alpha2);
    s_a2.label("Alpha2=%1.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_a2);

    let mut s_a3 = SliderCtrl::new(5.0, 45.0, 180.0, 52.0);
    s_a3.range(0.0, 1.0);
    s_a3.set_value(alpha3);
    s_a3.label("Alpha3=%1.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_a3);

    let mut s_a4 = SliderCtrl::new(5.0, 65.0, 180.0, 72.0);
    s_a4.range(0.0, 1.0);
    s_a4.set_value(alpha4);
    s_a4.label("Alpha4=%1.3f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_a4);

    let mut cbox_rc = CboxCtrl::new(190.0, 25.0, "Invert Z-Order");
    cbox_rc.set_status(invert_order);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_rc);
    buf
}

#[cfg(test)]
mod tests {
    use super::{flash_palette, flash_rand_byte};

    #[test]
    fn flash_rand_matches_reference_sequence() {
        let mut state = 1u32;
        let seq: Vec<u8> = (0..9).map(|_| flash_rand_byte(&mut state)).collect();
        assert_eq!(seq, vec![198, 126, 129, 107, 75, 251, 226, 251, 84]);
    }

    #[test]
    fn flash_palette_first_entry_matches_cpp_style() {
        let colors = flash_palette();
        assert_eq!(colors[0].a, 230);
        // First generated RGB before premultiply is (198, 126, 129).
        // After AGG premultiply with alpha=230:
        assert_eq!(colors[0].r, 179);
        assert_eq!(colors[0].g, 114);
        assert_eq!(colors[0].b, 116);
    }
}

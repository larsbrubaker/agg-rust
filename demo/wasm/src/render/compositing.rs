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
use agg_rust::ctrl::{render_ctrl, SliderCtrl, CboxCtrl};
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
use agg_rust::scanline_boolean_algebra::{SBoolOp, sbool_combine_shapes_aa};
use agg_rust::scanline_storage_aa::ScanlineStorageAa;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::span_interpolator_trans::SpanInterpolatorTrans;
use agg_rust::span_pattern_rgba::SpanPatternRgba;
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_bilinear::TransBilinear;
use agg_rust::trans_perspective::TransPerspective;
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

/// Two overlapping shapes combined with boolean operations.
///
/// params[0] = operation (0=Or, 1=And, 2=Xor, 3=AMinusB, 4=BMinusA)
pub fn scanline_boolean(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let op_idx = params.first().copied().unwrap_or(0.0) as u32;
    let op = match op_idx {
        1 => SBoolOp::And,
        2 => SBoolOp::Xor,
        3 => SBoolOp::AMinusB,
        4 => SBoolOp::BMinusA,
        _ => SBoolOp::Or,
    };

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let w = width as f64;
    let h = height as f64;

    // Shape A: circle spiral (left group)
    let mut ras1 = RasterizerScanlineAa::new();
    for i in 0..5 {
        let angle = i as f64 * std::f64::consts::PI * 2.0 / 5.0;
        let cx = w * 0.35 + 50.0 * angle.cos();
        let cy = h * 0.5 + 50.0 * angle.sin();
        let mut ell = Ellipse::new(cx, cy, 40.0, 40.0, 32, false);
        ras1.add_path(&mut ell, 0);
    }

    // Shape B: circle spiral (right group)
    let mut ras2 = RasterizerScanlineAa::new();
    for i in 0..5 {
        let angle = i as f64 * std::f64::consts::PI * 2.0 / 5.0 + 0.3;
        let cx = w * 0.65 + 50.0 * angle.cos();
        let cy = h * 0.5 + 50.0 * angle.sin();
        let mut ell = Ellipse::new(cx, cy, 40.0, 40.0, 32, false);
        ras2.add_path(&mut ell, 0);
    }

    // Render the individual shapes semi-transparently for reference
    let mut sl = ScanlineU8::new();
    {
        let mut ras_a = RasterizerScanlineAa::new();
        for i in 0..5 {
            let angle = i as f64 * std::f64::consts::PI * 2.0 / 5.0;
            let cx = w * 0.35 + 50.0 * angle.cos();
            let cy = h * 0.5 + 50.0 * angle.sin();
            let mut ell = Ellipse::new(cx, cy, 40.0, 40.0, 32, false);
            ras_a.add_path(&mut ell, 0);
        }
        render_scanlines_aa_solid(&mut ras_a, &mut sl, &mut rb, &Rgba8::new(240, 200, 200, 128));
    }
    {
        let mut ras_b = RasterizerScanlineAa::new();
        for i in 0..5 {
            let angle = i as f64 * std::f64::consts::PI * 2.0 / 5.0 + 0.3;
            let cx = w * 0.65 + 50.0 * angle.cos();
            let cy = h * 0.5 + 50.0 * angle.sin();
            let mut ell = Ellipse::new(cx, cy, 40.0, 40.0, 32, false);
            ras_b.add_path(&mut ell, 0);
        }
        render_scanlines_aa_solid(&mut ras_b, &mut sl, &mut rb, &Rgba8::new(200, 200, 240, 128));
    }

    // Perform boolean operation
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

    // Render the boolean result in solid color
    render_storage_solid(&mut storage_result, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Label
    let op_names = ["OR (Union)", "AND (Intersect)", "XOR", "A - B", "B - A"];
    let op_name = op_names.get(op_idx as usize).unwrap_or(&"OR");
    let mut txt = GsvText::new();
    txt.size(12.0, 0.0);
    txt.start_point(10.0, h - 20.0);
    txt.text(op_name);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.5);
    let mut ras = RasterizerScanlineAa::new();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}


// ============================================================================
// Scanline Boolean 2 — more complex boolean ops with paths
// ============================================================================

/// Complex shapes combined with boolean operations and different scanline types.
///
/// params[0] = test case (0-3)
/// params[1] = operation (0=Or, 1=And, 2=Xor, 3=AMinusB, 4=BMinusA)
pub fn scanline_boolean2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let test_case = params.first().copied().unwrap_or(0.0) as u32;
    let op_idx = params.get(1).copied().unwrap_or(0.0) as u32;
    let op = match op_idx {
        1 => SBoolOp::And,
        2 => SBoolOp::Xor,
        3 => SBoolOp::AMinusB,
        4 => SBoolOp::BMinusA,
        _ => SBoolOp::Or,
    };

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let w = width as f64;
    let h = height as f64;

    // Create shape A based on test case
    let mut ras1 = RasterizerScanlineAa::new();
    let mut ras2 = RasterizerScanlineAa::new();

    match test_case {
        1 => {
            // Overlapping rectangles
            let mut rect1 = PathStorage::new();
            rect1.move_to(w * 0.1, h * 0.2);
            rect1.line_to(w * 0.6, h * 0.2);
            rect1.line_to(w * 0.6, h * 0.8);
            rect1.line_to(w * 0.1, h * 0.8);
            rect1.close_polygon(0);
            ras1.add_path(&mut rect1, 0);

            let mut rect2 = PathStorage::new();
            rect2.move_to(w * 0.4, h * 0.3);
            rect2.line_to(w * 0.9, h * 0.3);
            rect2.line_to(w * 0.9, h * 0.7);
            rect2.line_to(w * 0.4, h * 0.7);
            rect2.close_polygon(0);
            ras2.add_path(&mut rect2, 0);
        }
        2 => {
            // Star and circle
            let n = 5;
            let cx = w * 0.4;
            let cy = h * 0.5;
            let r_outer = h * 0.35;
            let r_inner = h * 0.15;
            let mut star = PathStorage::new();
            for i in 0..(n * 2) {
                let angle = (i as f64) * std::f64::consts::PI / n as f64 - std::f64::consts::PI / 2.0;
                let r = if i % 2 == 0 { r_outer } else { r_inner };
                let px = cx + r * angle.cos();
                let py = cy + r * angle.sin();
                if i == 0 { star.move_to(px, py); } else { star.line_to(px, py); }
            }
            star.close_polygon(0);
            ras1.add_path(&mut star, 0);

            let mut ell = Ellipse::new(w * 0.6, h * 0.5, h * 0.3, h * 0.3, 100, false);
            ras2.add_path(&mut ell, 0);
        }
        3 => {
            // Thick stroke (converted to filled shape) vs polygon
            let mut line = PathStorage::new();
            line.move_to(w * 0.1, h * 0.5);
            line.line_to(w * 0.9, h * 0.5);
            let mut thick = ConvStroke::new(&mut line);
            thick.set_width(h * 0.3);
            thick.set_line_cap(LineCap::Round);
            ras1.add_path(&mut thick, 0);

            let mut tri = PathStorage::new();
            tri.move_to(w * 0.3, h * 0.1);
            tri.line_to(w * 0.7, h * 0.9);
            tri.line_to(w * 0.1, h * 0.9);
            tri.close_polygon(0);
            ras2.add_path(&mut tri, 0);
        }
        _ => {
            // Default: two overlapping ellipses
            let mut ell1 = Ellipse::new(w * 0.35, h * 0.5, w * 0.25, h * 0.35, 100, false);
            ras1.add_path(&mut ell1, 0);

            let mut ell2 = Ellipse::new(w * 0.65, h * 0.5, w * 0.25, h * 0.35, 100, false);
            ras2.add_path(&mut ell2, 0);
        }
    }

    // Render individual shapes semi-transparently
    let mut sl = ScanlineU8::new();
    // Recreate rasterizers for the transparent overlay since we consumed them
    {
        let mut ras_a = RasterizerScanlineAa::new();
        let mut ras_b = RasterizerScanlineAa::new();
        match test_case {
            1 => {
                let mut r1 = PathStorage::new();
                r1.move_to(w * 0.1, h * 0.2); r1.line_to(w * 0.6, h * 0.2);
                r1.line_to(w * 0.6, h * 0.8); r1.line_to(w * 0.1, h * 0.8);
                r1.close_polygon(0);
                ras_a.add_path(&mut r1, 0);
                let mut r2 = PathStorage::new();
                r2.move_to(w * 0.4, h * 0.3); r2.line_to(w * 0.9, h * 0.3);
                r2.line_to(w * 0.9, h * 0.7); r2.line_to(w * 0.4, h * 0.7);
                r2.close_polygon(0);
                ras_b.add_path(&mut r2, 0);
            }
            2 => {
                let n = 5;
                let cx = w * 0.4; let cy = h * 0.5;
                let r_outer = h * 0.35; let r_inner = h * 0.15;
                let mut star = PathStorage::new();
                for i in 0..(n * 2) {
                    let angle = (i as f64) * std::f64::consts::PI / n as f64 - std::f64::consts::PI / 2.0;
                    let r = if i % 2 == 0 { r_outer } else { r_inner };
                    let px = cx + r * angle.cos(); let py = cy + r * angle.sin();
                    if i == 0 { star.move_to(px, py); } else { star.line_to(px, py); }
                }
                star.close_polygon(0);
                ras_a.add_path(&mut star, 0);
                let mut ell = Ellipse::new(w * 0.6, h * 0.5, h * 0.3, h * 0.3, 100, false);
                ras_b.add_path(&mut ell, 0);
            }
            3 => {
                let mut line = PathStorage::new();
                line.move_to(w * 0.1, h * 0.5); line.line_to(w * 0.9, h * 0.5);
                let mut thick = ConvStroke::new(&mut line);
                thick.set_width(h * 0.3); thick.set_line_cap(LineCap::Round);
                ras_a.add_path(&mut thick, 0);
                let mut tri = PathStorage::new();
                tri.move_to(w * 0.3, h * 0.1); tri.line_to(w * 0.7, h * 0.9);
                tri.line_to(w * 0.1, h * 0.9); tri.close_polygon(0);
                ras_b.add_path(&mut tri, 0);
            }
            _ => {
                let mut e1 = Ellipse::new(w * 0.35, h * 0.5, w * 0.25, h * 0.35, 100, false);
                ras_a.add_path(&mut e1, 0);
                let mut e2 = Ellipse::new(w * 0.65, h * 0.5, w * 0.25, h * 0.35, 100, false);
                ras_b.add_path(&mut e2, 0);
            }
        }
        render_scanlines_aa_solid(&mut ras_a, &mut sl, &mut rb, &Rgba8::new(240, 200, 200, 100));
        render_scanlines_aa_solid(&mut ras_b, &mut sl, &mut rb, &Rgba8::new(200, 200, 240, 100));
    }

    // Boolean combine
    let mut sl1 = ScanlineU8::new();
    let mut sl2 = ScanlineU8::new();
    let mut sl_result = ScanlineU8::new();
    let mut st1 = ScanlineStorageAa::new();
    let mut st2 = ScanlineStorageAa::new();
    let mut st_result = ScanlineStorageAa::new();

    sbool_combine_shapes_aa(op, &mut ras1, &mut ras2,
        &mut sl1, &mut sl2, &mut sl_result,
        &mut st1, &mut st2, &mut st_result);

    render_storage_solid(&mut st_result, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Labels
    let op_names = ["OR", "AND", "XOR", "A-B", "B-A"];
    let case_names = ["Ellipses", "Rectangles", "Star & Circle", "Stroke & Triangle"];
    let op_name = op_names.get(op_idx as usize).unwrap_or(&"OR");
    let case_name = case_names.get(test_case as usize).unwrap_or(&"Ellipses");
    let label = format!("{} — {}", case_name, op_name);
    let mut txt = GsvText::new();
    txt.size(12.0, 0.0);
    txt.start_point(10.0, h - 20.0);
    txt.text(&label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.5);
    let mut ras = RasterizerScanlineAa::new();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
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

/// Render spiral comparison: aliased, AA outline, and scanline (matching C++ rasterizers2.cpp).
///
/// params[0] = step (rotation speed, unused in static render)
/// params[1] = line width
/// params[2] = accurate_joins (0 or 1)
/// params[3] = start_angle (degrees)
pub fn rasterizers2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let line_width = params.get(1).copied().unwrap_or(3.0).max(0.1);
    let accurate_joins = params.get(2).copied().unwrap_or(0.0) > 0.5;
    let start_angle = params.get(3).copied().unwrap_or(0.0).to_radians();

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
        let mut s1 = Spiral::new(w / 5.0, h / 4.0 + 50.0, 5.0, 70.0, 16.0, start_angle);
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
        let mut s2 = Spiral::new(w / 2.0, h / 4.0 + 50.0, 5.0, 70.0, 16.0, start_angle);
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
        let mut s3 = Spiral::new(w / 5.0, h - h / 4.0 + 20.0, 5.0, 70.0, 16.0, start_angle);
        ras_oaa.add_path(&mut s3, 0, &mut ren_oaa);
    }

    // 4. Scanline rasterizer (bottom-right)
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s4 = Spiral::new(w / 2.0, h - h / 4.0 + 20.0, 5.0, 70.0, 16.0, start_angle);
        let mut stroke = ConvStroke::new(&mut s4);
        stroke.set_width(line_width);
        stroke.set_line_cap(LineCap::Round);
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // Labels
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let labels = [
            (50.0, 80.0, "Bresenham lines,\nregular accuracy"),
            (w / 2.0 - 50.0, 80.0, "Bresenham lines,\nsubpixel accuracy"),
            (50.0, h / 2.0 + 50.0, "Anti-aliased lines"),
            (w / 2.0 - 50.0, h / 2.0 + 50.0, "Scanline rasterizer"),
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

    // Controls
    {
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s_width = SliderCtrl::new(150.0 + 10.0, 14.0, w - 10.0, 22.0);
        s_width.range(0.0, 14.0);
        s_width.set_value(line_width);
        s_width.label("Width=%1.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

        let mut cbox = CboxCtrl::new(200.0 + 10.0, 30.0, "Accurate Joins");
        cbox.set_status(accurate_joins);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox);
    }

    buf
}

// ============================================================================
// Line Patterns
// ============================================================================

/// Render AA outline patterns on spirals (simplified from C++ line_patterns.cpp).
///
/// Since renderer_outline_image is not ported, we show solid AA outlines
/// with configurable width and join modes on spirals at different positions.
///
/// params[0] = line width
/// params[1] = accurate_joins (0 or 1)
/// params[2] = start_angle (degrees)
pub fn line_patterns(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
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

    // Draw spirals with different colors and widths
    let configs: [(f64, f64, Rgba8, f64); 5] = [
        (w * 0.2, h * 0.3, Rgba8::new(153, 87, 87, 255), line_width),
        (w * 0.5, h * 0.3, Rgba8::new(87, 153, 87, 255), line_width * 0.7),
        (w * 0.8, h * 0.3, Rgba8::new(87, 87, 153, 255), line_width * 1.3),
        (w * 0.35, h * 0.7, Rgba8::new(153, 153, 87, 255), line_width * 0.5),
        (w * 0.65, h * 0.7, Rgba8::new(153, 87, 153, 255), line_width * 1.5),
    ];

    for (cx, cy, color, lw) in configs {
        let profile = LineProfileAa::with_width(lw);
        let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
        ren_oaa.set_color(color);
        let mut ras_oaa = RasterizerOutlineAa::new();
        ras_oaa.set_round_cap(true);
        ras_oaa.set_line_join(join);
        let mut spiral = Spiral::new(cx, cy, 5.0, 60.0, 14.0, start_angle);
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
    match i {
        0 => CompOp::Clear, 1 => CompOp::Src, 2 => CompOp::Dst,
        3 => CompOp::SrcOver, 4 => CompOp::DstOver,
        5 => CompOp::SrcIn, 6 => CompOp::DstIn,
        7 => CompOp::SrcOut, 8 => CompOp::DstOut,
        9 => CompOp::SrcAtop, 10 => CompOp::DstAtop,
        11 => CompOp::Xor, 12 => CompOp::Plus, 13 => CompOp::Minus,
        14 => CompOp::Multiply, 15 => CompOp::Screen,
        16 => CompOp::Overlay, 17 => CompOp::Darken, 18 => CompOp::Lighten,
        19 => CompOp::ColorDodge, 20 => CompOp::ColorBurn,
        21 => CompOp::HardLight, 22 => CompOp::SoftLight,
        23 => CompOp::Difference, 24 => CompOp::Exclusion,
        _ => CompOp::SrcOver,
    }
}

/// params[0] = comp_op index (0-24), params[1] = src alpha, params[2] = dst alpha
pub fn compositing(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let comp_op_idx = params.get(0).copied().unwrap_or(3.0) as u32;
    let src_alpha = params.get(1).copied().unwrap_or(255.0).clamp(0.0, 255.0) as u32;
    let dst_alpha = params.get(2).copied().unwrap_or(255.0).clamp(0.0, 255.0) as u32;
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = vec![0u8; (width * height * 4) as usize];
    let stride = (width * 4) as i32;
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };

    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(255, 255, 255, 255));
    }

    // Destination (blue circle)
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut ell = Ellipse::new(cx - 30.0, cy, 80.0, 80.0, 100, false);
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 80, 200, dst_alpha));
    }

    // Source (red rounded rect) with comp_op
    {
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.set_comp_op(comp_op_from_index(comp_op_idx));
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut rr = RoundedRect::new(cx - 50.0, cy - 80.0, cx + 80.0, cy + 50.0, 20.0);
        ras.add_path(&mut rr, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 30, 30, src_alpha));
    }

    // Controls
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s_src = SliderCtrl::new(5.0, 5.0, w / 2.0 - 5.0, 12.0);
        s_src.range(0.0, 255.0);
        s_src.set_value(src_alpha as f64);
        s_src.label("Src Alpha=%.0f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_src);
        let mut s_dst = SliderCtrl::new(w / 2.0 + 5.0, 5.0, w - 5.0, 12.0);
        s_dst.range(0.0, 255.0);
        s_dst.set_value(dst_alpha as f64);
        s_dst.label("Dst Alpha=%.0f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_dst);
    }
    buf
}

/// params[0] = comp_op, params[1] = src alpha, params[2] = dst alpha
pub fn compositing2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let comp_op_idx = params.get(0).copied().unwrap_or(3.0) as u32;
    let src_alpha = params.get(1).copied().unwrap_or(200.0).clamp(0.0, 255.0) as u32;
    let dst_alpha = params.get(2).copied().unwrap_or(200.0).clamp(0.0, 255.0) as u32;
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = vec![0u8; (width * height * 4) as usize];
    let stride = (width * 4) as i32;
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };

    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(255, 255, 255, 255));
    }

    // Destination (large blue circle)
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut ell = Ellipse::new(cx, cy, 100.0, 100.0, 100, false);
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 80, 180, dst_alpha));
    }

    // Source circles with comp_op
    {
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.set_comp_op(comp_op_from_index(comp_op_idx));
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let colors = [
            Rgba8::new(200, 30, 30, src_alpha),
            Rgba8::new(30, 200, 30, src_alpha),
            Rgba8::new(200, 200, 30, src_alpha),
        ];
        let offsets: [(f64, f64); 3] = [(-50.0, -30.0), (50.0, -30.0), (0.0, 40.0)];
        for (i, (dx, dy)) in offsets.iter().enumerate() {
            let mut ell = Ellipse::new(cx + dx, cy + dy, 60.0, 60.0, 80, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
        }
    }

    // Controls
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut s_src = SliderCtrl::new(5.0, 5.0, w / 2.0 - 5.0, 12.0);
        s_src.range(0.0, 255.0);
        s_src.set_value(src_alpha as f64);
        s_src.label("Src Alpha=%.0f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_src);
        let mut s_dst = SliderCtrl::new(w / 2.0 + 5.0, 5.0, w - 5.0, 12.0);
        s_dst.range(0.0, 255.0);
        s_dst.set_value(dst_alpha as f64);
        s_dst.label("Dst Alpha=%.0f");
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_dst);
    }
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

/// params[0] = scale, params[1] = rotation degrees
pub fn flash_rasterizer(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let scale_val = params.get(0).copied().unwrap_or(1.0).max(0.1);
    let rotation = params.get(1).copied().unwrap_or(0.0).to_radians();
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-cx, -cy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale_val));
    mtx.multiply(&TransAffine::new_rotation(rotation));
    mtx.multiply(&TransAffine::new_translation(cx, cy));

    let mut rasc = RasterizerCompoundAa::new();
    rasc.clip_box(0.0, 0.0, w, h);
    rasc.layer_order(LayerOrder::Direct);

    // Style 0: Large ellipse
    {
        let mut ell = Ellipse::new(cx, cy, 120.0, 90.0, 100, false);
        let mut t = ConvTransform::new(&mut ell, mtx);
        rasc.styles(0, -1);
        rasc.add_path(&mut t, 0);
    }
    // Style 1: Rectangle
    {
        let mut r = PathStorage::new();
        r.move_to(cx - 70.0, cy - 50.0);
        r.line_to(cx + 70.0, cy - 50.0);
        r.line_to(cx + 70.0, cy + 50.0);
        r.line_to(cx - 70.0, cy + 50.0);
        r.close_polygon(0);
        let mut t = ConvTransform::new(&mut r, mtx);
        rasc.styles(1, -1);
        rasc.add_path(&mut t, 0);
    }
    // Style 2: Small circle
    {
        let mut ell = Ellipse::new(cx + 60.0, cy - 40.0, 40.0, 40.0, 60, false);
        let mut t = ConvTransform::new(&mut ell, mtx);
        rasc.styles(2, -1);
        rasc.add_path(&mut t, 0);
    }
    // Style 3: Triangle
    {
        let mut tri = PathStorage::new();
        tri.move_to(cx - 80.0, cy + 60.0);
        tri.line_to(cx, cy - 80.0);
        tri.line_to(cx + 80.0, cy + 60.0);
        tri.close_polygon(0);
        let mut t = ConvTransform::new(&mut tri, mtx);
        rasc.styles(3, -1);
        rasc.add_path(&mut t, 0);
    }

    let colors = [
        Rgba8::new(100, 150, 200, 180),
        Rgba8::new(200, 100, 100, 180),
        Rgba8::new(100, 200, 100, 200),
        Rgba8::new(200, 200, 100, 180),
    ];
    render_compound(&mut rasc, &mut rb, &colors);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut s_sc = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_sc.range(0.2, 3.0);
    s_sc.set_value(scale_val);
    s_sc.label("Scale=%.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_sc);
    buf
}

/// params[0] = scale, params[1] = rotation degrees
pub fn flash_rasterizer2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let scale_val = params.get(0).copied().unwrap_or(1.0).max(0.1);
    let rotation = params.get(1).copied().unwrap_or(0.0).to_radians();
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-cx, -cy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale_val));
    mtx.multiply(&TransAffine::new_rotation(rotation));
    mtx.multiply(&TransAffine::new_translation(cx, cy));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let colors = [
        Rgba8::new(100, 150, 200, 180),
        Rgba8::new(200, 100, 100, 180),
        Rgba8::new(100, 200, 100, 200),
        Rgba8::new(200, 200, 100, 180),
    ];

    // Render each style separately with regular rasterizer
    {
        let mut ell = Ellipse::new(cx, cy, 120.0, 90.0, 100, false);
        let mut t = ConvTransform::new(&mut ell, mtx);
        ras.reset();
        ras.add_path(&mut t, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[0]);
    }
    {
        let mut r = PathStorage::new();
        r.move_to(cx - 70.0, cy - 50.0);
        r.line_to(cx + 70.0, cy - 50.0);
        r.line_to(cx + 70.0, cy + 50.0);
        r.line_to(cx - 70.0, cy + 50.0);
        r.close_polygon(0);
        let mut t = ConvTransform::new(&mut r, mtx);
        ras.reset();
        ras.add_path(&mut t, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[1]);
    }
    {
        let mut ell = Ellipse::new(cx + 60.0, cy - 40.0, 40.0, 40.0, 60, false);
        let mut t = ConvTransform::new(&mut ell, mtx);
        ras.reset();
        ras.add_path(&mut t, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[2]);
    }
    {
        let mut tri = PathStorage::new();
        tri.move_to(cx - 80.0, cy + 60.0);
        tri.line_to(cx, cy - 80.0);
        tri.line_to(cx + 80.0, cy + 60.0);
        tri.close_polygon(0);
        let mut t = ConvTransform::new(&mut tri, mtx);
        ras.reset();
        ras.add_path(&mut t, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[3]);
    }

    let mut s_sc = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_sc.range(0.2, 3.0);
    s_sc.set_value(scale_val);
    s_sc.label("Scale=%.2f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_sc);
    buf
}

/// params[0] = stroke width, params[1] = invert layer order (0/1)
pub fn rasterizer_compound(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let stroke_width = params.get(0).copied().unwrap_or(2.0).max(0.1);
    let invert_order = params.get(1).copied().unwrap_or(0.0) > 0.5;
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 240, 255));

    let mut rasc = RasterizerCompoundAa::new();
    rasc.clip_box(0.0, 0.0, w, h);
    rasc.layer_order(if invert_order { LayerOrder::Inverse } else { LayerOrder::Direct });

    // Style 0: Large stroked ellipse
    {
        let mut ell = Ellipse::new(cx, cy, 130.0, 100.0, 100, false);
        let mut stroke = ConvStroke::new(&mut ell);
        stroke.set_width(stroke_width * 3.0);
        rasc.styles(0, -1);
        rasc.add_path(&mut stroke, 0);
    }
    // Style 1: Filled circle
    {
        let mut ell = Ellipse::new(cx - 60.0, cy - 30.0, 50.0, 50.0, 80, false);
        rasc.styles(1, -1);
        rasc.add_path(&mut ell, 0);
    }
    // Style 2: Triangle
    {
        let mut tri = PathStorage::new();
        tri.move_to(cx + 20.0, cy - 60.0);
        tri.line_to(cx + 90.0, cy + 40.0);
        tri.line_to(cx - 50.0, cy + 40.0);
        tri.close_polygon(0);
        rasc.styles(2, -1);
        rasc.add_path(&mut tri, 0);
    }
    // Style 3: Stroked curve
    {
        let mut c = PathStorage::new();
        c.move_to(cx - 100.0, cy + 30.0);
        c.curve4(cx - 50.0, cy - 80.0, cx + 50.0, cy + 80.0, cx + 100.0, cy - 30.0);
        let mut cc = ConvCurve::new(&mut c);
        let mut stroke = ConvStroke::new(&mut cc);
        stroke.set_width(stroke_width * 5.0);
        rasc.styles(3, -1);
        rasc.add_path(&mut stroke, 0);
    }

    let colors = [
        Rgba8::new(80, 80, 80, 200),
        Rgba8::new(200, 60, 60, 200),
        Rgba8::new(60, 200, 60, 200),
        Rgba8::new(60, 60, 200, 200),
    ];
    render_compound(&mut rasc, &mut rb, &colors);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut s_sw = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_sw.range(0.5, 10.0);
    s_sw.set_value(stroke_width);
    s_sw.label("Stroke=%.1f");
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_sw);
    let mut cbox_rc = CboxCtrl::new(5.0, 20.0, "Invert Z-Order");
    cbox_rc.set_status(invert_order);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cbox_rc);
    buf
}

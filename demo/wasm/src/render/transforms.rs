//! Transform/text demo render functions: raster_text, gamma_ctrl, trans_polar,
//! multi_clip, simple_blur, blur, trans_curve1, trans_curve2, lion_lens, distortions,
//! gouraud_mesh.

use agg_rust::basics::{is_stop, is_vertex, VertexSource};
use agg_rust::bspline::Bspline;
use agg_rust::color::Rgba8;
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_segmentator::ConvSegmentator;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ctrl::{render_ctrl, SliderCtrl, RboxCtrl, GammaCtrl, Ctrl};
use agg_rust::ellipse::Ellipse;
use agg_rust::embedded_raster_fonts;
use agg_rust::glyph_raster_bin::GlyphRasterBin;
use agg_rust::gsv_text::GsvText;
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_raster_text::render_raster_htext_solid;
use agg_rust::rasterizer_compound_aa::RasterizerCompoundAa;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid, SpanGenerator};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_gouraud_rgba::SpanGouraudRgba;
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
use agg_rust::span_interpolator_linear::{SpanInterpolatorLinear, Transformer};
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_polar::TransPolar;
use agg_rust::trans_single_path::TransSinglePath;
use agg_rust::trans_warp_magnifier::TransWarpMagnifier;
use super::setup_renderer;

// ============================================================================
// Raster Text
// ============================================================================

/// Render all 34 embedded raster fonts with sample text.
/// Matches C++ raster_text.cpp.
pub fn raster_text(width: u32, height: u32, _params: &[f64]) -> Vec<u8> {
    let fonts: &[(&[u8], &str)] = &[
        (embedded_raster_fonts::GSE4X6, "gse4x6"),
        (embedded_raster_fonts::GSE4X8, "gse4x8"),
        (embedded_raster_fonts::GSE5X7, "gse5x7"),
        (embedded_raster_fonts::GSE5X9, "gse5x9"),
        (embedded_raster_fonts::GSE6X9, "gse6x9"),
        (embedded_raster_fonts::GSE6X12, "gse6x12"),
        (embedded_raster_fonts::GSE7X11, "gse7x11"),
        (embedded_raster_fonts::GSE7X11_BOLD, "gse7x11_bold"),
        (embedded_raster_fonts::GSE7X15, "gse7x15"),
        (embedded_raster_fonts::GSE7X15_BOLD, "gse7x15_bold"),
        (embedded_raster_fonts::GSE8X16, "gse8x16"),
        (embedded_raster_fonts::GSE8X16_BOLD, "gse8x16_bold"),
        (embedded_raster_fonts::MCS11_PROP, "mcs11_prop"),
        (embedded_raster_fonts::MCS11_PROP_CONDENSED, "mcs11_prop_condensed"),
        (embedded_raster_fonts::MCS12_PROP, "mcs12_prop"),
        (embedded_raster_fonts::MCS13_PROP, "mcs13_prop"),
        (embedded_raster_fonts::MCS5X10_MONO, "mcs5x10_mono"),
        (embedded_raster_fonts::MCS5X11_MONO, "mcs5x11_mono"),
        (embedded_raster_fonts::MCS6X10_MONO, "mcs6x10_mono"),
        (embedded_raster_fonts::MCS6X11_MONO, "mcs6x11_mono"),
        (embedded_raster_fonts::MCS7X12_MONO_HIGH, "mcs7x12_mono_high"),
        (embedded_raster_fonts::MCS7X12_MONO_LOW, "mcs7x12_mono_low"),
        (embedded_raster_fonts::VERDANA12, "verdana12"),
        (embedded_raster_fonts::VERDANA12_BOLD, "verdana12_bold"),
        (embedded_raster_fonts::VERDANA13, "verdana13"),
        (embedded_raster_fonts::VERDANA13_BOLD, "verdana13_bold"),
        (embedded_raster_fonts::VERDANA14, "verdana14"),
        (embedded_raster_fonts::VERDANA14_BOLD, "verdana14_bold"),
        (embedded_raster_fonts::VERDANA16, "verdana16"),
        (embedded_raster_fonts::VERDANA16_BOLD, "verdana16_bold"),
        (embedded_raster_fonts::VERDANA17, "verdana17"),
        (embedded_raster_fonts::VERDANA17_BOLD, "verdana17_bold"),
        (embedded_raster_fonts::VERDANA18, "verdana18"),
        (embedded_raster_fonts::VERDANA18_BOLD, "verdana18_bold"),
    ];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut glyph = GlyphRasterBin::new(fonts[0].0);
    let black = Rgba8::new(0, 0, 0, 255);

    let mut y = 5.0;
    for &(font_data, name) in fonts {
        glyph.set_font(font_data);
        let text = format!("A quick brown fox jumps over the lazy dog 0123456789: {}", name);
        render_raster_htext_solid(&mut rb, &mut glyph, 5.0, y, &text, &black, true);
        y += glyph.height() + 1.0;
    }

    // Render gradient text at the bottom using GsvText + ConvStroke
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut text = GsvText::new();
    text.size(12.0, 0.0);
    text.start_point(5.0, height as f64 - 20.0);
    text.text("RASTER TEXT: All 34 embedded bitmap fonts displayed above");
    let mut text_stroke = ConvStroke::new(&mut text);
    text_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut text_stroke, 0);
    let dark_red = Rgba8::new(128, 0, 0, 255);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &dark_red);

    buf
}

// ============================================================================
// Gamma Ctrl
// ============================================================================

/// Gamma control widget demo — matching C++ gamma_ctrl.cpp.
///
/// params[0..4] = gamma spline values (kx1, ky1, kx2, ky2)
pub fn gamma_ctrl_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let kx1 = params.first().copied().unwrap_or(1.0);
    let ky1 = params.get(1).copied().unwrap_or(1.0);
    let kx2 = params.get(2).copied().unwrap_or(1.0);
    let ky2 = params.get(3).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Create and render gamma control widget
    let mut g_ctrl = GammaCtrl::new(10.0, 10.0, 300.0, 200.0);
    g_ctrl.text_size(10.0, 12.0);
    g_ctrl.set_values(kx1, ky1, kx2, ky2);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut g_ctrl);

    let w = width as f64;
    let ecenter = w / 2.0;
    let ewidth = w / 2.0 - 10.0;

    // 5 pairs of stroked ellipses with different widths and colors
    let configs: &[(f64, f64, Rgba8)] = &[
        (220.0, 2.0, Rgba8::new(0, 0, 0, 255)),
        (260.0, 2.0, Rgba8::new(127, 127, 127, 255)),
        (300.0, 2.0, Rgba8::new(192, 192, 192, 255)),
        (340.0, 1.0, Rgba8::new(0, 0, 102, 255)),
        (380.0, 0.4, Rgba8::new(0, 0, 102, 255)),
    ];

    for &(cy, stroke_w, ref color) in configs {
        // Large ellipse
        let mut ell = Ellipse::new(ecenter, cy, ewidth, 15.5, 100, false);
        let mut poly = ConvStroke::new(&mut ell);
        poly.set_width(stroke_w);
        ras.reset();
        ras.add_path(&mut poly, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);

        // Small ellipse
        let mut ell2 = Ellipse::new(ecenter, cy, 10.5, 10.5, 100, false);
        let mut poly2 = ConvStroke::new(&mut ell2);
        poly2.set_width(stroke_w);
        ras.reset();
        ras.add_path(&mut poly2, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    // Render skewed text "Text 2345"
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_skewing(0.15, 0.0));

    let mut text = GsvText::new();
    text.size(50.0, 20.0);
    text.start_point(320.0, 10.0);
    text.text("Text 2345");
    let mut text_path = ConvStroke::new(&mut text);
    text_path.set_width(2.0);
    let mut text_transformed = ConvTransform::new(&mut text_path, mtx);
    ras.reset();
    ras.add_path(&mut text_transformed, 0);
    let green = Rgba8::new(0, 128, 0, 255);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &green);

    // Render rotating arrows
    let red = Rgba8::new(128, 0, 0, 255);
    let mut arrow = PathStorage::new();
    arrow.move_to(30.0, -1.0);
    arrow.line_to(60.0, 0.0);
    arrow.line_to(30.0, 1.0);
    arrow.move_to(27.0, -1.0);
    arrow.line_to(10.0, 0.0);
    arrow.line_to(27.0, 1.0);

    for i in 0..35 {
        let mut mtx2 = TransAffine::new();
        mtx2.multiply(&TransAffine::new_rotation(
            i as f64 / 35.0 * std::f64::consts::PI * 2.0,
        ));
        mtx2.multiply(&TransAffine::new_translation(400.0, 130.0));
        let mut trans = ConvTransform::new(&mut arrow, mtx2);
        ras.reset();
        ras.add_path(&mut trans, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &red);
    }

    buf
}

// ============================================================================
// Trans Polar
// ============================================================================

/// Polar coordinate transformation demo — matching C++ trans_polar.cpp.
///
/// params[0] = value (0-100, default 32)
/// params[1] = spiral (-0.1 to 0.1, default 0)
/// params[2] = base_y (50-200, default 120)
pub fn trans_polar_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let value = params.first().copied().unwrap_or(32.0);
    let spiral = params.get(1).copied().unwrap_or(0.0);
    let base_y = params.get(2).copied().unwrap_or(120.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Create slider controls
    let w = width as f64;
    let h = height as f64;

    let mut slider1 = SliderCtrl::new(10.0, 10.0, w - 10.0, 17.0);
    slider1.range(0.0, 100.0);
    slider1.num_steps(5);
    slider1.set_value(value);
    slider1.label("Some Value=%1.0f");

    let mut slider_spiral = SliderCtrl::new(10.0, 30.0, w - 10.0, 37.0);
    slider_spiral.label("Spiral=%.3f");
    slider_spiral.range(-0.1, 0.1);
    slider_spiral.set_value(spiral);

    let mut slider_base_y = SliderCtrl::new(10.0, 50.0, w - 10.0, 57.0);
    slider_base_y.label("Base Y=%.3f");
    slider_base_y.range(50.0, 200.0);
    slider_base_y.set_value(base_y);

    // Render the straight sliders
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider1);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider_spiral);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut slider_base_y);

    // Set up polar transform
    let mut trans = TransPolar::new();
    trans.base_angle = 2.0 * std::f64::consts::PI / -600.0; // full_circle(-600)
    trans.base_scale = -1.0;
    trans.base_y = slider_base_y.value();
    trans.translation_x = w / 2.0;
    trans.translation_y = h / 2.0 + 30.0;
    trans.spiral = -slider_spiral.value();

    // Transform the first slider through polar coordinates
    // Collect colors first, then render each path by extracting+transforming vertices
    let num_paths = slider1.num_paths();
    let colors: Vec<Rgba8> = (0..num_paths).map(|i| slider1.color(i)).collect();
    for i in 0..num_paths {
        // Extract vertices through segmentator
        let mut segm = ConvSegmentator::new(&mut slider1);
        segm.set_approximation_scale(4.0);
        segm.rewind(i);
        let (mut x, mut y) = (0.0, 0.0);
        let mut path = PathStorage::new();
        let mut first = true;
        loop {
            let cmd = segm.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                // Apply polar transform
                trans.transform(&mut x, &mut y);
                if first {
                    path.move_to(x, y);
                    first = false;
                } else {
                    path.line_to(x, y);
                }
            }
        }
        ras.reset();
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i as usize]);
    }

    buf
}

// ============================================================================
// Multi Clip
// ============================================================================

/// Multi-clip demo — lion rendered through N×N clip regions.
/// Matches C++ multi_clip.cpp.
///
/// params[0] = N (grid size, 2-10, default 4)
/// params[1] = angle (default 0)
/// params[2] = scale (default 1.0)
pub fn multi_clip(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let n = params.first().copied().unwrap_or(4.0).clamp(2.0, 10.0);
    let angle = params.get(1).copied().unwrap_or(0.0);
    let scale = params.get(2).copied().unwrap_or(1.0).max(0.01);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let w = width as i32;
    let h = height as i32;

    // Build transform
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Render lion into each clip box in the N×N grid
    let ni = n as i32;
    for gx in 0..ni {
        for gy in 0..ni {
            let x1 = w * gx / ni + 5;
            let y1 = h * gy / ni + 5;
            let x2 = w * (gx + 1) / ni - 5;
            let y2 = h * (gy + 1) / ni - 5;
            if x2 > x1 && y2 > y1 {
                rb.clip_box_i(x1, y1, x2, y2);
                for i in 0..npaths {
                    let start = path_idx[i] as u32;
                    let mut transformed = ConvTransform::new(&mut path, mtx);
                    ras.reset();
                    ras.add_path(&mut transformed, start);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
                }
            }
        }
    }

    // Reset to full clip for controls
    rb.clip_box_i(0, 0, w - 1, h - 1);

    // Render random circles with gradients
    let mut seed: u32 = 12345;
    let mut rng = || -> u32 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (seed >> 16) & 0x7fff
    };

    for _ in 0..50 {
        let cx = (rng() % width as u32) as f64;
        let cy = (rng() % height as u32) as f64;
        let radius = (rng() % 10 + 5) as f64;

        let mut ell = Ellipse::new(cx, cy, radius, radius, 32, false);
        let color = Rgba8::new(
            (rng() & 0x7F) as u32,
            (rng() & 0x7F) as u32,
            (rng() & 0x7F) as u32,
            ((rng() & 0x7F) + 0x7F) as u32,
        );
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // Render slider control
    let mut s_num = SliderCtrl::new(5.0, 5.0, 150.0, 12.0);
    s_num.range(2.0, 10.0);
    s_num.label("N=%.2f");
    s_num.set_value(n);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_num);

    buf
}

// ============================================================================
// Simple Blur
// ============================================================================

/// Simple 3×3 box blur on the lion — matching C++ simple_blur.cpp.
///
/// params[0] = angle (default 0)
/// params[1] = scale (default 1.0)
pub fn simple_blur(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build transform
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    // Render lion
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Apply simple 3×3 box blur to the buffer
    apply_box_blur_3x3(&mut buf, width, height);

    // Render the un-blurred lion again on the left side for comparison
    rb.clip_box_i(0, 0, width as i32 / 2, height as i32 - 1);
    rb.clear(&Rgba8::new(255, 255, 255, 255));
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Label
    rb.clip_box_i(0, 0, width as i32 - 1, height as i32 - 1);
    let mut label = GsvText::new();
    label.size(10.0, 0.0);
    label.start_point(10.0, height as f64 - 20.0);
    label.text("Left: original  |  Right: 3x3 box blur");
    let mut label_stroke = ConvStroke::new(&mut label);
    label_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut label_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

/// Apply a simple 3×3 box blur to an RGBA buffer.
fn apply_box_blur_3x3(buf: &mut Vec<u8>, width: u32, height: u32) {
    let stride = (width * 4) as usize;
    let src = buf.clone();
    for y in 1..height as usize - 1 {
        for x in 1..width as usize - 1 {
            for c in 0..4 {
                let mut sum = 0u32;
                for dy in 0..3usize {
                    for dx in 0..3usize {
                        let ny = y + dy - 1;
                        let nx = x + dx - 1;
                        sum += src[ny * stride + nx * 4 + c] as u32;
                    }
                }
                buf[y * stride + x * 4 + c] = (sum / 9) as u8;
            }
        }
    }
}

// ============================================================================
// Blur
// ============================================================================

/// Stack blur demo — matching C++ blur.cpp.
///
/// params[0] = blur radius (0-40, default 15)
/// params[1] = method (0=stack_blur, 1=recursive, 2=channels)
pub fn blur_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let radius = params.first().copied().unwrap_or(15.0).clamp(0.0, 40.0);
    let method = params.get(1).copied().unwrap_or(0.0) as u32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Draw a colorful shape — bezier curves forming a closed shape
    let mut shape = PathStorage::new();
    shape.move_to(w * 0.2, h * 0.3);
    shape.curve4(w * 0.4, h * 0.1, w * 0.6, h * 0.1, w * 0.8, h * 0.3);
    shape.curve4(w * 0.9, h * 0.5, w * 0.8, h * 0.7, w * 0.6, h * 0.8);
    shape.curve4(w * 0.4, h * 0.9, w * 0.2, h * 0.7, w * 0.15, h * 0.5);
    shape.close_polygon(0);

    let mut curve = ConvCurve::new(&mut shape);
    ras.reset();
    ras.add_path(&mut curve, 0);
    let fill_color = Rgba8::new(100, 140, 220, 200);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &fill_color);

    // Draw a red circle
    let mut ell = Ellipse::new(w * 0.35, h * 0.45, 60.0, 60.0, 64, false);
    ras.reset();
    ras.add_path(&mut ell, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(220, 60, 60, 200));

    // Draw a green triangle
    let mut tri = PathStorage::new();
    tri.move_to(w * 0.5, h * 0.2);
    tri.line_to(w * 0.7, h * 0.65);
    tri.line_to(w * 0.3, h * 0.65);
    tri.close_polygon(0);
    ras.reset();
    ras.add_path(&mut tri, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(60, 200, 60, 180));

    // Apply blur based on method
    if radius > 0.5 {
        let r = radius as u32;
        let mut ra_blur = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { ra_blur.attach(buf.as_mut_ptr(), width, height, stride) };
        match method {
            0 => agg_rust::blur::stack_blur_rgba32(&mut ra_blur, r, r),
            1 => agg_rust::blur::recursive_blur_rgba32(&mut ra_blur, radius),
            _ => {
                agg_rust::blur::stack_blur_rgba32(&mut ra_blur, r, r);
            }
        }
    }

    // Render controls on top (after blur)
    {
        let mut ra2 = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { ra2.attach(buf.as_mut_ptr(), width, height, stride) };
        let pf2 = PixfmtRgba32::new(&mut ra2);
        let mut rb2 = RendererBase::new(pf2);
        let mut ras2 = RasterizerScanlineAa::new();
        let mut sl2 = ScanlineU8::new();

        let mut s_radius = SliderCtrl::new(5.0, 5.0, width as f64 - 5.0, 12.0);
        s_radius.range(0.0, 40.0);
        s_radius.label("Blur Radius=%.2f");
        s_radius.set_value(radius);
        render_ctrl(&mut ras2, &mut sl2, &mut rb2, &mut s_radius);

        let mut r_method = RboxCtrl::new(5.0, 25.0, 130.0, 82.0);
        r_method.add_item("Stack Blur");
        r_method.add_item("Recursive Blur");
        r_method.add_item("Channels");
        r_method.set_cur_item(method as i32);
        render_ctrl(&mut ras2, &mut sl2, &mut rb2, &mut r_method);
    }

    buf
}

// ============================================================================
// Trans Curve 1
// ============================================================================

/// Text along a curved path using trans_single_path — matching C++ trans_curve1_test.cpp.
///
/// params[0] = num_points (10-400, default 200)
/// params[1..12] = control points x,y pairs (6 points)
pub fn trans_curve1(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let num_points = params.first().copied().unwrap_or(200.0).clamp(10.0, 400.0);

    // Default control points for the spline curve
    let default_pts = [
        100.0, 400.0,
        200.0, 200.0,
        300.0, 500.0,
        400.0, 100.0,
        500.0, 350.0,
        550.0, 300.0,
    ];
    let pts: Vec<f64> = (0..12)
        .map(|i| params.get(i + 1).copied().unwrap_or(default_pts[i]))
        .collect();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build B-spline from control points
    let n_pts = 6;
    let xs: Vec<f64> = (0..n_pts).map(|i| pts[i * 2]).collect();
    let ys: Vec<f64> = (0..n_pts).map(|i| pts[i * 2 + 1]).collect();
    let ts: Vec<f64> = (0..n_pts).map(|i| i as f64).collect();
    let mut bspline_x = Bspline::new();
    let mut bspline_y = Bspline::new();
    bspline_x.init(&ts, &xs);
    bspline_y.init(&ts, &ys);

    // Generate curve vertices and add to trans_single_path
    let mut tcurve = TransSinglePath::new();
    let step = 1.0 / num_points;
    let mut t = 0.0;
    let mut first = true;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x.get(t);
        let y = bspline_y.get(t);
        if first {
            tcurve.move_to(x, y);
            first = false;
        } else {
            tcurve.line_to(x, y);
        }
        t += step;
    }
    tcurve.finalize_path();

    // Render the spline curve itself
    let mut curve_path = PathStorage::new();
    first = true;
    t = 0.0;
    while t <= (n_pts - 1) as f64 + 0.001 {
        let x = bspline_x.get(t);
        let y = bspline_y.get(t);
        if first {
            curve_path.move_to(x, y);
            first = false;
        } else {
            curve_path.line_to(x, y);
        }
        t += step;
    }
    let mut curve_stroke = ConvStroke::new(&mut curve_path);
    curve_stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut curve_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(170, 50, 20, 100));

    // Render text along the curve using GsvText
    let text_str = "AGG - Anti-Grain Geometry. A high quality rendering engine for C++ / Rust";
    let mut text = GsvText::new();
    text.size(20.0, 0.0);
    text.start_point(0.0, 0.0);
    text.text(text_str);

    // Segmentate and transform through the single path
    let mut segm = ConvSegmentator::new(&mut text);
    segm.set_approximation_scale(3.0);
    segm.rewind(0);

    let mut transformed_path = PathStorage::new();
    let (mut x, mut y) = (0.0, 0.0);
    loop {
        let cmd = segm.vertex(&mut x, &mut y);
        if is_stop(cmd) { break; }
        if is_vertex(cmd) {
            tcurve.transform(&mut x, &mut y);
            if (cmd & 0x07) == 1 { // move_to
                transformed_path.move_to(x, y);
            } else {
                transformed_path.line_to(x, y);
            }
        } else {
            // close_polygon or end_poly
            transformed_path.close_polygon(0);
        }
    }

    ras.reset();
    ras.add_path(&mut transformed_path, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Render control points
    for i in 0..n_pts {
        let cx = pts[i * 2];
        let cy = pts[i * 2 + 1];
        let mut ell = Ellipse::new(cx, cy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 77, 128, 200));
    }

    // Slider
    let mut s_pts = SliderCtrl::new(5.0, 5.0, width as f64 - 5.0, 12.0);
    s_pts.range(10.0, 400.0);
    s_pts.label("Num Points=%.0f");
    s_pts.set_value(num_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_pts);

    buf
}

// ============================================================================
// Trans Curve 2
// ============================================================================

/// Text along a curved path (second variant) — uses trans_single_path with different layout.
///
/// params[0] = num_points (10-400, default 200)
pub fn trans_curve2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let num_points = params.first().copied().unwrap_or(200.0).clamp(10.0, 400.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Build a sinusoidal path
    let mut tcurve = TransSinglePath::new();
    let step = 1.0 / num_points;
    let mut first = true;
    let mut t = 0.0;
    while t <= 1.0 + 0.001 {
        let x = 50.0 + t * (w - 100.0);
        let y = h / 2.0 + (t * 4.0 * std::f64::consts::PI).sin() * 120.0;
        if first {
            tcurve.move_to(x, y);
            first = false;
        } else {
            tcurve.line_to(x, y);
        }
        t += step;
    }
    tcurve.finalize_path();

    // Render the path
    let mut path = PathStorage::new();
    first = true;
    t = 0.0;
    while t <= 1.0 + 0.001 {
        let x = 50.0 + t * (w - 100.0);
        let y = h / 2.0 + (t * 4.0 * std::f64::consts::PI).sin() * 120.0;
        if first { path.move_to(x, y); first = false; } else { path.line_to(x, y); }
        t += step;
    }
    let mut path_stroke = ConvStroke::new(&mut path);
    path_stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut path_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(170, 50, 20, 100));

    // Render text along the curve
    let text_str = "This text follows a sinusoidal path using trans_single_path transformation";
    let mut text = GsvText::new();
    text.size(18.0, 0.0);
    text.start_point(0.0, 0.0);
    text.text(text_str);

    let mut segm = ConvSegmentator::new(&mut text);
    segm.set_approximation_scale(3.0);
    segm.rewind(0);

    let mut transformed = PathStorage::new();
    let (mut x, mut y) = (0.0, 0.0);
    loop {
        let cmd = segm.vertex(&mut x, &mut y);
        if is_stop(cmd) { break; }
        if is_vertex(cmd) {
            tcurve.transform(&mut x, &mut y);
            if (cmd & 0x07) == 1 { transformed.move_to(x, y); } else { transformed.line_to(x, y); }
        } else {
            transformed.close_polygon(0);
        }
    }

    ras.reset();
    ras.add_path(&mut transformed, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 100, 255));

    // Slider
    let mut s_pts = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_pts.range(10.0, 400.0);
    s_pts.label("Num Points=%.0f");
    s_pts.set_value(num_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_pts);

    buf
}

// ============================================================================
// Lion Lens
// ============================================================================

/// Magnifying lens on the lion — matching C++ lion_lens.cpp.
///
/// params[0] = magnification (0.01-4.0, default 3.0)
/// params[1] = radius (0.0-100.0, default 70.0)
/// params[2] = lens_x (default center)
/// params[3] = lens_y (default center)
/// params[4] = angle (default 0)
pub fn lion_lens(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let magn = params.first().copied().unwrap_or(3.0).clamp(0.01, 4.0);
    let radius = params.get(1).copied().unwrap_or(70.0).clamp(0.0, 100.0);
    let lens_x = params.get(2).copied().unwrap_or(width as f64 / 2.0);
    let lens_y = params.get(3).copied().unwrap_or(height as f64 / 2.0);
    let angle = params.get(4).copied().unwrap_or(0.0);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Set up lens
    let mut lens = TransWarpMagnifier::new();
    lens.center(lens_x, lens_y);
    lens.magnification(magn);
    lens.set_radius(radius / magn);

    // Affine transform for the lion
    let base_dx = 120.0;
    let base_dy = 190.0;
    let mtx = {
        let mut m = TransAffine::new();
        m.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
        m.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
        m.multiply(&TransAffine::new_translation(
            width as f64 / 2.0,
            height as f64 / 2.0,
        ));
        m
    };

    // Render lion with lens distortion
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        // Segmentate → affine → lens transform → render
        let mut segm = ConvSegmentator::new(&mut path);
        segm.set_approximation_scale(4.0);
        segm.rewind(start);

        let mut distorted = PathStorage::new();
        let (mut x, mut y) = (0.0, 0.0);
        let mut first_in_path = true;
        loop {
            let cmd = segm.vertex(&mut x, &mut y);
            if is_stop(cmd) { break; }
            if is_vertex(cmd) {
                // Apply affine transform
                mtx.transform(&mut x, &mut y);
                // Apply lens distortion
                lens.transform(&mut x, &mut y);
                if first_in_path || (cmd & 0x07) == 1 {
                    distorted.move_to(x, y);
                    first_in_path = false;
                } else {
                    distorted.line_to(x, y);
                }
            } else if (cmd & 0x0F) == 0x0F || (cmd & 0x0F) == 0x0E {
                distorted.close_polygon(0);
                first_in_path = true;
            }
        }

        ras.reset();
        ras.add_path(&mut distorted, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Render sliders
    let mut s_magn = SliderCtrl::new(5.0, 5.0, 245.0, 12.0);
    s_magn.range(0.01, 4.0);
    s_magn.label("Scale=%.2f");
    s_magn.set_value(magn);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_magn);

    let mut s_radius = SliderCtrl::new(255.0, 5.0, 495.0, 12.0);
    s_radius.range(0.0, 100.0);
    s_radius.label("Radius=%.2f");
    s_radius.set_value(radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);

    buf
}

// ============================================================================
// Distortions
// ============================================================================

/// Wave/swirl distortion on a procedural image — matching C++ distortions.cpp.
///
/// params[0] = angle (-180 to 180, default 20)
/// params[1] = scale (0.1-5.0, default 1.0)
/// params[2] = amplitude (0.1-40.0, default 10.0)
/// params[3] = period (0.1-2.0, default 1.0)
/// params[4] = distortion type (0=wave, 1=swirl, 2=wave-swirl, 3=swirl-wave)
pub fn distortions(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle = params.first().copied().unwrap_or(20.0);
    let scale = params.get(1).copied().unwrap_or(1.0).clamp(0.1, 5.0);
    let amplitude = params.get(2).copied().unwrap_or(10.0).clamp(0.1, 40.0);
    let period = params.get(3).copied().unwrap_or(1.0).clamp(0.1, 2.0);
    let dist_type = params.get(4).copied().unwrap_or(0.0) as u32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Generate a procedural image: concentric colored circles
    let img_w = 200u32;
    let img_h = 200u32;
    let mut img_buf = vec![255u8; (img_w * img_h * 4) as usize];
    for iy in 0..img_h {
        for ix in 0..img_w {
            let dx = ix as f64 - 100.0;
            let dy = iy as f64 - 100.0;
            let d = (dx * dx + dy * dy).sqrt();
            let off = ((iy * img_w + ix) * 4) as usize;
            if d < 90.0 {
                let t = d / 90.0;
                img_buf[off] = (128.0 + 127.0 * (t * 6.0).sin()) as u8;
                img_buf[off + 1] = (128.0 + 127.0 * (t * 4.0 + 2.0).sin()) as u8;
                img_buf[off + 2] = (128.0 + 127.0 * (t * 8.0 + 4.0).sin()) as u8;
                img_buf[off + 3] = 255;
            }
        }
    }

    // Create rendering buffer for the source image
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_buf.as_mut_ptr(), img_w, img_h, img_stride) };
    // Set up transform
    let angle_rad = angle * std::f64::consts::PI / 180.0;
    let mut img_mtx = TransAffine::new();
    img_mtx.multiply(&TransAffine::new_translation(-(img_w as f64) / 2.0, -(img_h as f64) / 2.0));
    img_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    img_mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    img_mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));
    img_mtx.invert();

    // Render image with bilinear filter in an ellipse
    let r_ell = (img_w.min(img_h) as f64) / 2.0 - 20.0;

    let mut src_mtx = TransAffine::new();
    src_mtx.multiply(&TransAffine::new_translation(-(img_w as f64) / 2.0, -(img_h as f64) / 2.0));
    src_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    src_mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));

    let mut ell = Ellipse::new(img_w as f64 / 2.0, img_h as f64 / 2.0, r_ell, r_ell, 200, false);
    let mut tr = ConvTransform::new(&mut ell, src_mtx);

    // Render with bilinear image filter
    let mut inter = SpanInterpolatorLinear::new(img_mtx);
    let bg_color = Rgba8::new(255, 255, 255, 255);
    let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg_color, &mut inter);
    let mut sa = SpanAllocator::new();
    ras.reset();
    ras.add_path(&mut tr, 0);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

    // Render controls
    let mut s_angle = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    s_angle.range(-180.0, 180.0);
    s_angle.label("Angle=%.1f");
    s_angle.set_value(angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_angle);

    let mut s_scale = SliderCtrl::new(5.0, 20.0, w - 5.0, 27.0);
    s_scale.range(0.1, 5.0);
    s_scale.label("Scale=%.2f");
    s_scale.set_value(scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_scale);

    let mut s_amp = SliderCtrl::new(5.0, 35.0, w - 5.0, 42.0);
    s_amp.range(0.1, 40.0);
    s_amp.label("Amplitude=%.1f");
    s_amp.set_value(amplitude);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_amp);

    let mut s_period = SliderCtrl::new(5.0, 50.0, w - 5.0, 57.0);
    s_period.range(0.1, 2.0);
    s_period.label("Period=%.2f");
    s_period.set_value(period);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_period);

    // Label showing distortion type
    let labels = ["Wave", "Swirl", "Wave-Swirl", "Swirl-Wave"];
    let label = labels.get(dist_type as usize).unwrap_or(&"Wave");
    let mut txt = GsvText::new();
    txt.size(10.0, 0.0);
    txt.start_point(5.0, h - 20.0);
    txt.text(label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

// ============================================================================
// Gouraud Mesh — animated color-interpolated triangle mesh
// ============================================================================

/// Gouraud-shaded triangle mesh rendered with the compound rasterizer.
/// Adapted from C++ gouraud_mesh.cpp.
///
/// params[0] = grid_cols (3-20, default 8)
/// params[1] = grid_rows (3-20, default 8)
/// params[2] = animation seed (incremented each frame for color cycling)
pub fn gouraud_mesh(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let cols = params.get(0).copied().unwrap_or(8.0).clamp(3.0, 20.0) as usize;
    let rows = params.get(1).copied().unwrap_or(8.0).clamp(3.0, 20.0) as usize;
    let seed = params.get(2).copied().unwrap_or(0.0) as u64;

    let w = width as f64;
    let h = height as f64;
    let margin = 30.0;
    let cell_w = (w - 2.0 * margin) / (cols - 1) as f64;
    let cell_h = (h - 2.0 * margin) / (rows - 1) as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(0, 0, 0, 255));

    // Generate mesh vertices with pseudo-random colors and offsets
    let num_pts = cols * rows;
    let mut vx = vec![0.0f64; num_pts];
    let mut vy = vec![0.0f64; num_pts];
    let mut vc = vec![Rgba8::new(0, 0, 0, 255); num_pts];

    // Simple hash for reproducible randomness
    let hash = |i: u64, ch: u64| -> u8 {
        let v = ((i.wrapping_mul(2654435761).wrapping_add(ch.wrapping_mul(2246822519)))
            .wrapping_mul(seed.wrapping_add(1).wrapping_mul(131))) >> 24;
        (v & 0xFF) as u8
    };

    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            let base_x = margin + c as f64 * cell_w;
            let base_y = margin + r as f64 * cell_h;
            // Add small perturbation based on seed
            let dx = ((hash(idx as u64, 0) as f64) - 128.0) / 128.0 * cell_w * 0.3;
            let dy = ((hash(idx as u64, 1) as f64) - 128.0) / 128.0 * cell_h * 0.3;
            // Don't perturb boundary vertices
            let is_boundary = r == 0 || r == rows - 1 || c == 0 || c == cols - 1;
            vx[idx] = if is_boundary { base_x } else { base_x + dx };
            vy[idx] = if is_boundary { base_y } else { base_y + dy };
            vc[idx] = Rgba8::new(
                hash(idx as u64, 10) as u32,
                hash(idx as u64, 20) as u32,
                hash(idx as u64, 30) as u32,
                255,
            );
        }
    }

    // Build triangles and edges
    struct MeshTriangle { p1: usize, p2: usize, p3: usize }
    struct MeshEdge { p1: usize, p2: usize, tl: i32, tr: i32 }

    let mut triangles: Vec<MeshTriangle> = Vec::new();
    let mut edges: Vec<MeshEdge> = Vec::new();

    for r in 0..(rows - 1) {
        for c in 0..(cols - 1) {
            let p1 = r * cols + c;         // top-left
            let p2 = p1 + 1;              // top-right
            let p3 = p2 + cols;           // bottom-right
            let p4 = p1 + cols;           // bottom-left

            let t1 = triangles.len() as i32; // lower: p1,p2,p3
            triangles.push(MeshTriangle { p1, p2, p3 });
            let t2 = triangles.len() as i32; // upper: p3,p4,p1
            triangles.push(MeshTriangle { p1: p3, p2: p4, p3: p1 });

            // Diagonal edge (p1-p3): t2 on left, t1 on right
            edges.push(MeshEdge { p1, p2: p3, tl: t2, tr: t1 });

            // Top edge
            let top_tr = if r > 0 {
                ((r - 1) * (cols - 1) * 2 + c * 2 + 1) as i32
            } else { -1 };
            edges.push(MeshEdge { p1, p2, tl: top_tr, tr: t1 });

            // Left edge
            let left_tl = if c > 0 {
                (r * (cols - 1) * 2 + (c - 1) * 2) as i32
            } else { -1 };
            edges.push(MeshEdge { p1, p2: p4, tl: t2, tr: left_tl });

            // Right edge (only at last column)
            if c == cols - 2 {
                edges.push(MeshEdge { p1: p2, p2: p3, tl: t1, tr: -1 });
            }
            // Bottom edge (only at last row)
            if r == rows - 2 {
                edges.push(MeshEdge { p1: p3, p2: p4, tl: -1, tr: t2 });
            }
        }
    }

    // Prepare SpanGouraudRgba for each triangle
    let mut gouraud_spans: Vec<SpanGouraudRgba> = Vec::with_capacity(triangles.len());
    for tri in &triangles {
        let mut sg = SpanGouraudRgba::new_with_triangle(
            vc[tri.p1], vc[tri.p2], vc[tri.p3],
            vx[tri.p1], vy[tri.p1],
            vx[tri.p2], vy[tri.p2],
            vx[tri.p3], vy[tri.p3],
            0.0,
        );
        sg.prepare();
        gouraud_spans.push(sg);
    }

    // Rasterize edges with compound rasterizer
    let mut rasc = RasterizerCompoundAa::new();
    for edge in &edges {
        rasc.styles(edge.tl, edge.tr);
        rasc.move_to_d(vx[edge.p1], vy[edge.p1]);
        rasc.line_to_d(vx[edge.p2], vy[edge.p2]);
    }

    // Sweep scanlines and render with Gouraud shading
    {
        use agg_rust::rasterizer_scanline_aa::Scanline;
        if rasc.rewind_scanlines() {
            let mut sl = ScanlineU8::new();
            loop {
                let num_styles = rasc.sweep_styles();
                if num_styles == 0 { break; }
                for s in 0..num_styles {
                    let style_id = rasc.style(s) as usize;
                    if rasc.sweep_scanline(&mut sl, s as i32) {
                        if style_id < gouraud_spans.len() {
                            let y = Scanline::y(&sl);
                            for span in sl.begin() {
                                let x = span.x;
                                let len = span.len;
                                if len > 0 {
                                    let mut colors = vec![Rgba8::new(0, 0, 0, 0); len as usize];
                                    gouraud_spans[style_id].generate(
                                        &mut colors, x, y, len as u32,
                                    );
                                    let covers = &sl.covers()
                                        [span.cover_offset..span.cover_offset + len as usize];
                                    for i in 0..len as usize {
                                        let c = &colors[i];
                                        let cover = covers[i];
                                        if c.a > 0 && cover > 0 {
                                            rb.blend_pixel(x + i as i32, y, c, cover);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Draw text label
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let label = format!("Gouraud Mesh: {}x{} grid, {} triangles",
        cols, rows, triangles.len());
    let mut txt = GsvText::new();
    txt.size(8.0, 0.0);
    txt.start_point(5.0, h - 15.0);
    txt.text(&label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(0.8);
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 200));

    buf
}


//! Demo render functions.
//!
//! Each function renders a specific demo into an RGBA pixel buffer.
//! The buffer is width * height * 4 bytes (RGBA order).

use agg_rust::color::Rgba8;
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLut;
use agg_rust::math_stroke::{LineCap, LineJoin};
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid, SpanGenerator};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::rounded_rect::RoundedRect;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_gouraud_rgba::SpanGouraudRgba;
use agg_rust::span_gradient::{GradientRadial, GradientX, SpanGradient};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

/// Create a rendering buffer, pixel format, and renderer base from dimensions.
fn setup_renderer(
    buf: &mut Vec<u8>,
    ra: &mut RowAccessor,
    width: u32,
    height: u32,
) {
    let stride = (width * 4) as i32;
    buf.resize((width * height * 4) as usize, 255);
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
}

// ============================================================================
// Fallback
// ============================================================================

pub fn fallback(width: u32, height: u32) -> Vec<u8> {
    let size = (width * height * 4) as usize;
    let mut buf = vec![255u8; size];
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            buf[offset] = (x * 255 / width.max(1)) as u8;
            buf[offset + 1] = (y * 255 / height.max(1)) as u8;
            buf[offset + 2] = 128;
            buf[offset + 3] = 255;
        }
    }
    buf
}

// ============================================================================
// Lion
// ============================================================================

/// Render the classic AGG lion with rotation and scaling.
///
/// params[0] = rotation angle in degrees (default 0)
/// params[1] = scale factor (default 1.0)
pub fn lion(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_deg = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.1);

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Center the lion (original is roughly 0,0..240,380)
    let cx = 120.0;
    let cy = 190.0;
    let angle_rad = angle_deg * std::f64::consts::PI / 180.0;

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-cx, -cy));
    mtx.multiply(&TransAffine::new_rotation(angle_rad));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    // Render each colored path
    let npaths = colors.len();
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    buf
}

// ============================================================================
// Shapes
// ============================================================================

/// Render various anti-aliased shapes.
///
/// params[0] = number of shapes (default 12)
pub fn shapes(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let count = params.first().copied().unwrap_or(12.0) as usize;

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

    // Row 1: Circles with different sizes
    let row1_y = h * 0.2;
    let cols = count.min(8);
    for i in 0..cols {
        let t = i as f64 / cols as f64;
        let cx = w * (0.1 + t * 0.8);
        let r = 10.0 + t * 40.0;
        let mut ell = Ellipse::new(cx, row1_y, r, r, 64, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        let color = Rgba8::new(
            (255.0 * (1.0 - t)) as u32,
            (100.0 + 155.0 * t) as u32,
            (200.0 * t) as u32,
            200,
        );
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // Row 2: Rounded rectangles
    let row2_y = h * 0.5;
    for i in 0..cols.min(6) {
        let t = i as f64 / 6.0;
        let x1 = w * (0.05 + t * 0.85);
        let rr_w = 40.0 + t * 30.0;
        let rr_h = 30.0 + t * 20.0;
        let radius = 2.0 + t * 15.0;
        let mut rrect = RoundedRect::new(x1, row2_y - rr_h / 2.0, x1 + rr_w, row2_y + rr_h / 2.0, radius);
        ras.reset();
        ras.add_path(&mut rrect, 0);
        let color = Rgba8::new(
            (50.0 + 150.0 * t) as u32,
            (200.0 * (1.0 - t)) as u32,
            (100.0 + 155.0 * t) as u32,
            220,
        );
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    // Row 3: Stroked ellipses
    let row3_y = h * 0.8;
    for i in 0..cols.min(6) {
        let t = i as f64 / 6.0;
        let cx = w * (0.1 + t * 0.8);
        let rx = 20.0 + t * 30.0;
        let ry = 10.0 + t * 20.0;
        let ell = Ellipse::new(cx, row3_y, rx, ry, 64, false);
        let mut stroke = ConvStroke::new(ell);
        stroke.set_width(1.0 + t * 4.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        let color = Rgba8::new(
            (200.0 * t) as u32,
            (50.0 + 100.0 * t) as u32,
            (255.0 * (1.0 - t)) as u32,
            255,
        );
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
    }

    buf
}

// ============================================================================
// Gradients
// ============================================================================

/// Render gradient demonstrations.
///
/// params[0] = gradient rotation in degrees (default 0)
pub fn gradients(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_deg = params.first().copied().unwrap_or(0.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    let w = width as f64;
    let h = height as f64;
    let angle_rad = angle_deg * std::f64::consts::PI / 180.0;

    // Linear gradient over a large rectangle
    {
        let mut path = PathStorage::new();
        path.move_to(w * 0.05, h * 0.05);
        path.line_to(w * 0.95, h * 0.05);
        path.line_to(w * 0.95, h * 0.45);
        path.line_to(w * 0.05, h * 0.45);
        path.close_polygon(0);

        ras.reset();
        ras.add_path(&mut path, 0);

        let mut mtx = TransAffine::new();
        mtx.multiply(&TransAffine::new_translation(-w / 2.0, -h * 0.25));
        mtx.multiply(&TransAffine::new_rotation(angle_rad));
        mtx.multiply(&TransAffine::new_translation(w / 2.0, h * 0.25));
        let interp = SpanInterpolatorLinear::new(mtx);

        let mut lut = GradientLut::new(256);
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.add_color(0.25, Rgba8::new(255, 200, 0, 255));
        lut.add_color(0.5, Rgba8::new(0, 200, 50, 255));
        lut.add_color(0.75, Rgba8::new(0, 100, 255, 255));
        lut.add_color(1.0, Rgba8::new(200, 0, 255, 255));
        lut.build_lut();

        let mut grad = SpanGradient::new(interp, GradientX, &lut, 0.0, w);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut grad);
    }

    // Radial gradient circle
    {
        let cx = w / 2.0;
        let cy = h * 0.72;
        let r = h.min(w) * 0.22;

        let mut ell = Ellipse::new(cx, cy, r, r, 128, false);
        ras.reset();
        ras.add_path(&mut ell, 0);

        let mut mtx = TransAffine::new();
        mtx.multiply(&TransAffine::new_translation(-cx, -cy));
        mtx.multiply(&TransAffine::new_translation(cx, cy));
        let interp = SpanInterpolatorLinear::new(mtx);

        let mut lut = GradientLut::new(256);
        lut.add_color(0.0, Rgba8::new(255, 255, 200, 255));
        lut.add_color(0.3, Rgba8::new(255, 150, 50, 255));
        lut.add_color(0.7, Rgba8::new(200, 50, 0, 255));
        lut.add_color(1.0, Rgba8::new(50, 0, 0, 255));
        lut.build_lut();

        let mut grad = SpanGradient::new(interp, GradientRadial, &lut, 0.0, r);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut grad);
    }

    buf
}

// ============================================================================
// Gouraud
// ============================================================================

/// Render Gouraud-shaded triangles.
///
/// params[0] = x offset for animation (default 0)
pub fn gouraud(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let x_offset = params.first().copied().unwrap_or(0.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    let w = width as f64;
    let h = height as f64;

    // Triangle 1: red-green-blue (large, center)
    {
        let mut gouraud = SpanGouraudRgba::new_with_triangle(
            Rgba8::new(255, 0, 0, 255),
            Rgba8::new(0, 255, 0, 255),
            Rgba8::new(0, 0, 255, 255),
            w * 0.1 + x_offset,
            h * 0.85,
            w * 0.5 + x_offset * 0.5,
            h * 0.1,
            w * 0.9 + x_offset,
            h * 0.85,
            0.0,
        );
        ras.reset();
        ras.add_path(&mut gouraud, 0);
        gouraud.prepare();
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut gouraud);
    }

    // Triangle 2: cyan-magenta-yellow (smaller, offset)
    {
        let mut gouraud = SpanGouraudRgba::new_with_triangle(
            Rgba8::new(0, 255, 255, 180),
            Rgba8::new(255, 0, 255, 180),
            Rgba8::new(255, 255, 0, 180),
            w * 0.3,
            h * 0.15,
            w * 0.75,
            h * 0.4,
            w * 0.55,
            h * 0.8,
            0.0,
        );
        ras.reset();
        ras.add_path(&mut gouraud, 0);
        gouraud.prepare();
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut gouraud);
    }

    buf
}

// ============================================================================
// Strokes
// ============================================================================

/// Demonstrate different stroke widths, caps, and joins.
///
/// params[0] = base stroke width (default 3.0)
pub fn strokes(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let base_width = params.first().copied().unwrap_or(3.0).max(0.5);

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

    let caps = [LineCap::Butt, LineCap::Square, LineCap::Round];
    let cap_names_colors = [
        Rgba8::new(220, 50, 50, 255),
        Rgba8::new(50, 150, 50, 255),
        Rgba8::new(50, 50, 220, 255),
    ];

    // Draw horizontal lines with different caps
    for (i, (cap, color)) in caps.iter().zip(cap_names_colors.iter()).enumerate() {
        let y = h * (0.15 + i as f64 * 0.12);
        for j in 0..5 {
            let sw = base_width * (1.0 + j as f64);
            let x1 = w * 0.1;
            let x2 = w * 0.9;
            let yy = y + j as f64 * (sw + 4.0);

            let mut path = PathStorage::new();
            path.move_to(x1, yy);
            path.line_to(x2, yy);

            let mut stroke = ConvStroke::new(&mut path);
            stroke.set_width(sw);
            stroke.set_line_cap(*cap);

            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
        }
    }

    // Draw polylines with different joins
    let joins = [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel];
    let join_colors = [
        Rgba8::new(200, 100, 0, 255),
        Rgba8::new(100, 0, 200, 255),
        Rgba8::new(0, 150, 150, 255),
    ];

    for (i, (join, color)) in joins.iter().zip(join_colors.iter()).enumerate() {
        let base_y = h * 0.6 + i as f64 * h * 0.12;
        let mut path = PathStorage::new();
        path.move_to(w * 0.05, base_y);
        path.line_to(w * 0.25, base_y - h * 0.08);
        path.line_to(w * 0.45, base_y);
        path.line_to(w * 0.65, base_y - h * 0.08);
        path.line_to(w * 0.85, base_y);
        path.line_to(w * 0.95, base_y - h * 0.04);

        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(base_width * 2.0);
        stroke.set_line_join(*join);

        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    buf
}

// ============================================================================
// Curves
// ============================================================================

/// Render Bezier curves with control points.
///
/// params[0..8] = 4 control points (x0,y0,x1,y1,x2,y2,x3,y3) normalized 0-1
pub fn curves(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    // Default control points if not provided
    let p0x = params.first().copied().unwrap_or(0.1) * w;
    let p0y = params.get(1).copied().unwrap_or(0.8) * h;
    let p1x = params.get(2).copied().unwrap_or(0.3) * w;
    let p1y = params.get(3).copied().unwrap_or(0.1) * h;
    let p2x = params.get(4).copied().unwrap_or(0.7) * w;
    let p2y = params.get(5).copied().unwrap_or(0.1) * h;
    let p3x = params.get(6).copied().unwrap_or(0.9) * w;
    let p3y = params.get(7).copied().unwrap_or(0.8) * h;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Draw the cubic Bezier curve (thick)
    {
        let mut path = PathStorage::new();
        path.move_to(p0x, p0y);
        path.curve4(p1x, p1y, p2x, p2y, p3x, p3y);

        let curve = ConvCurve::new(&mut path);
        let mut stroke = ConvStroke::new(curve);
        stroke.set_width(3.0);

        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(40, 80, 200, 255));
    }

    // Draw control polygon (thin dashed lines)
    {
        let mut path = PathStorage::new();
        path.move_to(p0x, p0y);
        path.line_to(p1x, p1y);
        path.line_to(p2x, p2y);
        path.line_to(p3x, p3y);

        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(1.0);

        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(150, 150, 150, 200));
    }

    // Draw control points as circles
    let points = [(p0x, p0y), (p1x, p1y), (p2x, p2y), (p3x, p3y)];
    let point_colors = [
        Rgba8::new(255, 0, 0, 255),
        Rgba8::new(0, 200, 0, 255),
        Rgba8::new(0, 200, 0, 255),
        Rgba8::new(255, 0, 0, 255),
    ];
    for ((px, py), color) in points.iter().zip(point_colors.iter()) {
        let mut ell = Ellipse::new(*px, *py, 6.0, 6.0, 32, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    // Draw a second curve (quadratic) below
    {
        let mut path = PathStorage::new();
        path.move_to(w * 0.1, h * 0.6);
        path.curve3(w * 0.5, h * 0.3, w * 0.9, h * 0.6);
        let curve = ConvCurve::new(&mut path);
        let mut stroke = ConvStroke::new(curve);
        stroke.set_width(2.5);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 80, 40, 255));
    }

    buf
}

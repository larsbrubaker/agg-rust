//! Demo render functions.
//!
//! Each function renders a specific demo into an RGBA pixel buffer.
//! The buffer is width * height * 4 bytes (RGBA order).

use agg_rust::color::Rgba8;
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_dash::ConvDash;
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
use agg_rust::span_gradient::{
    GradientConic, GradientDiamond, GradientRadial, GradientSqrtXY, GradientX, GradientXY,
    SpanGradient,
};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::basics::{is_stop, is_vertex, VertexSource};
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

/// Render the classic AGG lion with rotation, scaling, and skewing.
/// Matches C++ lion.cpp transform stack exactly.
///
/// params[0] = rotation angle in radians (default 0)
/// params[1] = scale factor (default 1.0)
/// params[2] = skew_x raw mouse coord (default 0, divided by 1000 internally)
/// params[3] = skew_y raw mouse coord (default 0, divided by 1000 internally)
/// params[4] = alpha 0–255 (default 255)
pub fn lion(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_rad = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(2).copied().unwrap_or(0.0);
    let skew_y = params.get(3).copied().unwrap_or(0.0);
    let alpha = params.get(4).copied().unwrap_or(255.0).clamp(0.0, 255.0) as u32;

    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Center of the lion bounding box (original data roughly 0,0..240,380)
    let base_dx = 120.0;
    let base_dy = 190.0;

    // Transform stack matching C++ lion.cpp:
    //   translate(-base) → scale → rotate(angle+PI) → skew(x/1000,y/1000) → translate(center)
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_rotation(angle_rad + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_skewing(skew_x / 1000.0, skew_y / 1000.0));
    mtx.multiply(&TransAffine::new_translation(
        width as f64 / 2.0,
        height as f64 / 2.0,
    ));

    // Render each colored path with alpha applied
    let npaths = colors.len();
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        let mut transformed = ConvTransform::new(&mut path, mtx);
        ras.reset();
        ras.add_path(&mut transformed, start);
        let mut c = colors[i];
        c.a = ((c.a as u32 * alpha) / 255) as u8;
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &c);
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

/// Render gradient sphere — 6 gradient types, matching C++ gradients.cpp.
///
/// params[0] = center_x (default width/2)
/// params[1] = center_y (default height/2)
/// params[2] = angle in radians (default 0)
/// params[3] = scale (default 1.0)
/// params[4] = gradient type (0=radial, 1=diamond, 2=linear, 3=xy, 4=sqrt_xy, 5=conic)
/// params[5] = scale_x (default 1.0)
/// params[6] = scale_y (default 1.0)
pub fn gradients(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let cx = params.get(0).copied().unwrap_or(w / 2.0);
    let cy = params.get(1).copied().unwrap_or(h / 2.0);
    let angle = params.get(2).copied().unwrap_or(0.0);
    let scale = params.get(3).copied().unwrap_or(1.0).max(0.01);
    let grad_type = params.get(4).copied().unwrap_or(0.0) as i32;
    let scale_x = params.get(5).copied().unwrap_or(1.0).max(0.01);
    let scale_y = params.get(6).copied().unwrap_or(1.0).max(0.01);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(0, 0, 0, 255)); // Black background, matching C++

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    // Rainbow color LUT (matching C++ spline default appearance)
    let mut lut = GradientLut::new(256);
    lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
    lut.add_color(0.15, Rgba8::new(255, 200, 0, 255));
    lut.add_color(0.3, Rgba8::new(0, 255, 0, 255));
    lut.add_color(0.5, Rgba8::new(0, 200, 255, 255));
    lut.add_color(0.65, Rgba8::new(0, 0, 255, 255));
    lut.add_color(0.8, Rgba8::new(200, 0, 255, 255));
    lut.add_color(1.0, Rgba8::new(255, 0, 100, 255));
    lut.build_lut();

    // Full-screen ellipse shape (centered, matching C++ ellipse of r=110 but scaled)
    let shape_r = w.min(h) * 0.45;
    let mut ell = Ellipse::new(cx, cy, shape_r, shape_r, 128, false);
    ras.reset();
    ras.add_path(&mut ell, 0);

    // Gradient transform (inverted for sampling) — matches C++ gradients.cpp
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_scaling(scale, scale));
    mtx.multiply(&TransAffine::new_scaling(scale_x, scale_y));
    mtx.multiply(&TransAffine::new_rotation(angle));
    mtx.multiply(&TransAffine::new_translation(cx, cy));
    mtx.invert();

    let d1 = 0.0;
    let d2 = 150.0; // Gradient extent, matching C++

    // Dispatch on gradient type using macro to avoid lifetime issues
    macro_rules! do_render {
        ($gf:expr) => {{
            let interp = SpanInterpolatorLinear::new(mtx);
            let mut grad = SpanGradient::new(interp, $gf, &lut, d1, d2);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut grad);
        }};
    }

    match grad_type {
        0 => do_render!(GradientRadial),
        1 => do_render!(GradientDiamond),
        2 => do_render!(GradientX),
        3 => do_render!(GradientXY),
        4 => do_render!(GradientSqrtXY),
        5 => do_render!(GradientConic),
        _ => do_render!(GradientRadial),
    }

    buf
}

// ============================================================================
// Gouraud
// ============================================================================

/// Render Gouraud-shaded triangles — 6 sub-triangles matching C++ gouraud.cpp.
///
/// params[0..6] = x0,y0, x1,y1, x2,y2 (3 vertex positions)
/// params[6] = dilation (default 0.175)
/// params[7] = gamma (default 0.809, currently unused)
/// params[8] = alpha 0.0–1.0 (default 1.0)
pub fn gouraud(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let vx0 = params.get(0).copied().unwrap_or(57.0);
    let vy0 = params.get(1).copied().unwrap_or(60.0);
    let vx1 = params.get(2).copied().unwrap_or(369.0);
    let vy1 = params.get(3).copied().unwrap_or(170.0);
    let vx2 = params.get(4).copied().unwrap_or(143.0);
    let vy2 = params.get(5).copied().unwrap_or(310.0);
    let d = params.get(6).copied().unwrap_or(0.175);
    let _gamma = params.get(7).copied().unwrap_or(0.809);
    let alpha = params.get(8).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u32;

    // Centroid
    let xc = (vx0 + vx1 + vx2) / 3.0;
    let yc = (vy0 + vy1 + vy2) / 3.0;

    // Reflected midpoints (reflection of centroid across each edge midpoint)
    let rx01 = vx0 + vx1 - xc;
    let ry01 = vy0 + vy1 - yc;
    let rx12 = vx1 + vx2 - xc;
    let ry12 = vy1 + vy2 - yc;
    let rx20 = vx2 + vx0 - xc;
    let ry20 = vy2 + vy0 - yc;

    let red = Rgba8::new(255, 0, 0, a);
    let green = Rgba8::new(0, 255, 0, a);
    let blue = Rgba8::new(0, 0, 255, a);
    let black = Rgba8::new(0, 0, 0, a);
    let white = Rgba8::new(255, 255, 255, a);

    // 6 sub-triangles matching C++ gouraud.cpp exactly:
    // Inner 3 (edge vertices → centroid, brc=0 → black)
    // Outer 3 (edge vertices → reflected midpoint, brc=1 → white)
    struct Tri {
        x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64,
        c0: Rgba8, c1: Rgba8, c2: Rgba8,
    }

    let triangles = [
        // Inner 3
        Tri { x0: vx0, y0: vy0, x1: vx1, y1: vy1, x2: xc, y2: yc, c0: red, c1: green, c2: black },
        Tri { x0: vx1, y0: vy1, x1: vx2, y1: vy2, x2: xc, y2: yc, c0: green, c1: blue, c2: black },
        Tri { x0: vx2, y0: vy2, x1: vx0, y1: vy0, x2: xc, y2: yc, c0: blue, c1: red, c2: black },
        // Outer 3
        Tri { x0: vx0, y0: vy0, x1: vx1, y1: vy1, x2: rx01, y2: ry01, c0: red, c1: green, c2: white },
        Tri { x0: vx1, y0: vy1, x1: vx2, y1: vy2, x2: rx12, y2: ry12, c0: green, c1: blue, c2: white },
        Tri { x0: vx2, y0: vy2, x1: vx0, y1: vy0, x2: rx20, y2: ry20, c0: blue, c1: red, c2: white },
    ];

    for tri in &triangles {
        let mut g = SpanGouraudRgba::new_with_triangle(
            tri.c0, tri.c1, tri.c2,
            tri.x0, tri.y0, tri.x1, tri.y1, tri.x2, tri.y2,
            d,
        );
        ras.reset();
        ras.add_path(&mut g, 0);
        g.prepare();
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut g);
    }

    buf
}

// ============================================================================
// Conv Stroke (replaces old "strokes" demo — matches C++ conv_stroke.cpp)
// ============================================================================

/// Demonstrate conv_stroke with draggable vertices matching C++ conv_stroke.cpp.
///
/// params[0..6] = x0,y0, x1,y1, x2,y2 (3 vertex positions)
/// params[6] = join type (0=miter, 1=miter_revert, 2=round, 3=bevel)
/// params[7] = cap type (0=butt, 1=square, 2=round)
/// params[8] = stroke width (default 20.0)
/// params[9] = miter limit (default 4.0)
pub fn conv_stroke(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let vx0 = params.get(0).copied().unwrap_or(157.0);
    let vy0 = params.get(1).copied().unwrap_or(60.0);
    let vx1 = params.get(2).copied().unwrap_or(469.0);
    let vy1 = params.get(3).copied().unwrap_or(170.0);
    let vx2 = params.get(4).copied().unwrap_or(243.0);
    let vy2 = params.get(5).copied().unwrap_or(310.0);
    let join_idx = params.get(6).copied().unwrap_or(2.0) as i32;
    let cap_idx = params.get(7).copied().unwrap_or(2.0) as i32;
    let sw = params.get(8).copied().unwrap_or(20.0).max(0.5);
    let miter_limit = params.get(9).copied().unwrap_or(4.0);

    let join = match join_idx {
        0 => LineJoin::Miter,
        1 => LineJoin::MiterRevert,
        3 => LineJoin::Bevel,
        _ => LineJoin::Round,
    };
    let cap = match cap_idx {
        0 => LineCap::Butt,
        1 => LineCap::Square,
        _ => LineCap::Round,
    };

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build the path matching C++ conv_stroke.cpp:
    // Sub-path 1: v0 → mid(v0,v1) → v1 → v2
    // Sub-path 2: mid(v0,v1) → mid(v1,v2) → mid(v2,v0), closed
    let mx01 = (vx0 + vx1) / 2.0;
    let my01 = (vy0 + vy1) / 2.0;
    let mx12 = (vx1 + vx2) / 2.0;
    let my12 = (vy1 + vy2) / 2.0;
    let mx20 = (vx2 + vx0) / 2.0;
    let my20 = (vy2 + vy0) / 2.0;

    let mut path = PathStorage::new();
    path.move_to(vx0, vy0);
    path.line_to(mx01, my01);
    path.line_to(vx1, vy1);
    path.line_to(vx2, vy2);
    path.line_to(vx2, vy2); // duplicate point, matching C++

    path.move_to(mx01, my01);
    path.line_to(mx12, my12);
    path.line_to(mx20, my20);
    path.close_polygon(0);

    // Layer 1: Main thick stroke (beige/tan)
    {
        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_line_join(join);
        stroke.set_line_cap(cap);
        stroke.set_miter_limit(miter_limit);
        stroke.set_width(sw);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(204, 179, 153, 255));
    }

    // Layer 2: Thin outline (black, 1.5)
    {
        let mut outline = ConvStroke::new(&mut path);
        outline.set_width(1.5);
        ras.reset();
        ras.add_path(&mut outline, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Layer 3: Dashed overlay (stroke → dash → stroke)
    {
        let mut inner = ConvStroke::new(&mut path);
        inner.set_line_join(join);
        inner.set_line_cap(cap);
        inner.set_miter_limit(miter_limit);
        inner.set_width(sw);
        let mut dash = ConvDash::new(inner);
        dash.add_dash(20.0, sw / 2.5);
        let mut outer = ConvStroke::new(dash);
        outer.set_width(sw / 5.0);
        outer.set_line_cap(cap);
        outer.set_line_join(join);
        outer.set_miter_limit(4.0);
        ras.reset();
        ras.add_path(&mut outer, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 77, 255));
    }

    // Layer 4: Original path as transparent fill
    {
        ras.reset();
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 51));
    }

    // Draw vertex markers
    let vertices = [(vx0, vy0), (vx1, vy1), (vx2, vy2)];
    for (vx, vy) in &vertices {
        let mut ell = Ellipse::new(*vx, *vy, 5.0, 5.0, 20, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    buf
}

// ============================================================================
// Bezier Div (replaces old "curves" demo — matches C++ bezier_div.cpp)
// ============================================================================

/// Render a cubic Bezier with draggable control points, matching C++ bezier_div.cpp.
///
/// params[0..8] = x1,y1, x2,y2, x3,y3, x4,y4 (4 control points, absolute coords)
/// params[8] = stroke width (default 50.0)
/// params[9] = show_points (0 or 1, default 1)
/// params[10] = show_outline (0 or 1, default 1)
pub fn bezier_div(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    // Default control points from C++ bezier_div.cpp
    let p1x = params.get(0).copied().unwrap_or(170.0);
    let p1y = params.get(1).copied().unwrap_or(424.0);
    let p2x = params.get(2).copied().unwrap_or(13.0);
    let p2y = params.get(3).copied().unwrap_or(87.0);
    let p3x = params.get(4).copied().unwrap_or(488.0);
    let p3y = params.get(5).copied().unwrap_or(423.0);
    let p4x = params.get(6).copied().unwrap_or(26.0);
    let p4y = params.get(7).copied().unwrap_or(333.0);
    let sw = params.get(8).copied().unwrap_or(50.0);
    let show_points = params.get(9).copied().unwrap_or(1.0) > 0.5;
    let show_outline = params.get(10).copied().unwrap_or(1.0) > 0.5;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 242, 255)); // warm white, matching C++

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Render filled stroke (green, semi-transparent) — matching C++ color rgba(0, 0.5, 0, 0.5)
    {
        let mut path = PathStorage::new();
        path.move_to(p1x, p1y);
        path.curve4(p2x, p2y, p3x, p3y, p4x, p4y);
        let curve = ConvCurve::new(&mut path);
        let mut stroke = ConvStroke::new(curve);
        stroke.set_width(sw);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 128, 0, 128));
    }

    // Show curve vertex dots if enabled
    if show_points {
        let mut path = PathStorage::new();
        path.move_to(p1x, p1y);
        path.curve4(p2x, p2y, p3x, p3y, p4x, p4y);
        let mut curve = ConvCurve::new(&mut path);
        curve.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = curve.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                let mut ell = Ellipse::new(x, y, 1.5, 1.5, 8, false);
                ras.reset();
                ras.add_path(&mut ell, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 128));
            }
        }
    }

    // Show stroke outline if enabled
    if show_outline {
        let mut path = PathStorage::new();
        path.move_to(p1x, p1y);
        path.curve4(p2x, p2y, p3x, p3y, p4x, p4y);
        let curve = ConvCurve::new(&mut path);
        let stroke = ConvStroke::new(curve);
        let mut outline = ConvStroke::new(stroke);
        outline.set_width(1.0);
        ras.reset();
        ras.add_path(&mut outline, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 128));
    }

    // Draw control polygon (thin lines)
    {
        let mut poly = PathStorage::new();
        poly.move_to(p1x, p1y);
        poly.line_to(p2x, p2y);
        poly.line_to(p3x, p3y);
        poly.line_to(p4x, p4y);
        let mut stroke = ConvStroke::new(&mut poly);
        stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(150, 150, 150, 200));
    }

    // Draw control points as circles
    let points = [(p1x, p1y), (p2x, p2y), (p3x, p3y), (p4x, p4y)];
    let colors = [
        Rgba8::new(255, 0, 0, 255),
        Rgba8::new(0, 180, 0, 255),
        Rgba8::new(0, 180, 0, 255),
        Rgba8::new(255, 0, 0, 255),
    ];
    for (pt, color) in points.iter().zip(colors.iter()) {
        let mut ell = Ellipse::new(pt.0, pt.1, 6.0, 6.0, 20, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    buf
}

// ============================================================================
// Circles — random circles, matching C++ circles.cpp
// ============================================================================

/// Render random anti-aliased circles.
///
/// params[0] = number of circles (default 200)
/// params[1] = min radius (default 3)
/// params[2] = max radius (default 30)
/// params[3] = seed (default 12345)
pub fn circles(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let n = params.get(0).copied().unwrap_or(200.0) as usize;
    let min_r = params.get(1).copied().unwrap_or(3.0);
    let max_r = params.get(2).copied().unwrap_or(30.0);
    let seed = params.get(3).copied().unwrap_or(12345.0) as u64;

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

    // Simple LCG RNG for reproducible results
    let mut rng = seed;
    let next = |state: &mut u64| -> f64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*state >> 33) as f64 / (1u64 << 31) as f64
    };

    for _ in 0..n {
        let x = next(&mut rng) * w;
        let y = next(&mut rng) * h;
        let r = min_r + next(&mut rng) * (max_r - min_r);
        let cr = (next(&mut rng) * 255.0) as u32;
        let cg = (next(&mut rng) * 255.0) as u32;
        let cb = (next(&mut rng) * 255.0) as u32;
        let ca = (next(&mut rng) * 180.0 + 75.0) as u32;

        let steps = (r * 4.0).max(8.0).min(64.0) as u32;
        let mut ell = Ellipse::new(x, y, r, r, steps, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(cr, cg, cb, ca));
    }

    buf
}

// ============================================================================
// Rounded Rect — matches C++ rounded_rect.cpp
// ============================================================================

/// Render a rounded rectangle with draggable corners.
///
/// params[0..4] = x1,y1, x2,y2 (two corners)
/// params[4] = corner radius (default 20)
/// params[5] = outline width (default 2)
pub fn rounded_rect_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let x1 = params.get(0).copied().unwrap_or(100.0);
    let y1 = params.get(1).copied().unwrap_or(80.0);
    let x2 = params.get(2).copied().unwrap_or(400.0);
    let y2 = params.get(3).copied().unwrap_or(280.0);
    let radius = params.get(4).copied().unwrap_or(20.0);
    let outline_w = params.get(5).copied().unwrap_or(2.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Fill
    let mut rrect = RoundedRect::new(x1, y1, x2, y2, radius);
    ras.reset();
    ras.add_path(&mut rrect, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 230, 200, 255));

    // Stroke
    let mut rrect2 = RoundedRect::new(x1, y1, x2, y2, radius);
    let mut stroke = ConvStroke::new(&mut rrect2);
    stroke.set_width(outline_w);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(100, 60, 30, 255));

    // Inscribed circle
    {
        let cx = (x1 + x2) / 2.0;
        let cy = (y1 + y2) / 2.0;
        let r = ((x2 - x1).min(y2 - y1) / 2.0 - radius).max(10.0);
        let mut ell = Ellipse::new(cx, cy, r, r, 64, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(130, 180, 220, 200));
    }

    // Corner markers
    let corners = [(x1, y1), (x2, y2)];
    for (cx, cy) in &corners {
        let mut ell = Ellipse::new(*cx, *cy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    buf
}

// ============================================================================
// AA Demo — enlarged pixel triangle, matching C++ aa_demo.cpp
// ============================================================================

/// Render the anti-aliasing demo with enlarged pixel view.
///
/// params[0..6] = x0,y0, x1,y1, x2,y2 (triangle in screen coords)
/// params[6] = pixel_size (default 32)
pub fn aa_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let vx0 = params.get(0).copied().unwrap_or(100.0);
    let vy0 = params.get(1).copied().unwrap_or(48.0);
    let vx1 = params.get(2).copied().unwrap_or(369.0);
    let vy1 = params.get(3).copied().unwrap_or(170.0);
    let vx2 = params.get(4).copied().unwrap_or(143.0);
    let vy2 = params.get(5).copied().unwrap_or(310.0);
    let pixel_size = params.get(6).copied().unwrap_or(32.0).max(4.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    // Don't use PixfmtRgba32 for the main buffer yet — we'll write pixels directly first
    // then use rasterizer for grid and outline
    buf.resize((width * height * 4) as usize, 255);

    let ps = pixel_size;
    let nx = (width as f64 / ps).ceil() as u32 + 1;
    let ny = (height as f64 / ps).ceil() as u32 + 1;

    // Scale triangle vertices to logical pixel coords
    let sx0 = vx0 / ps;
    let sy0 = vy0 / ps;
    let sx1 = vx1 / ps;
    let sy1 = vy1 / ps;
    let sx2 = vx2 / ps;
    let sy2 = vy2 / ps;

    // Render triangle into a small temp buffer
    let small_stride = (nx * 4) as i32;
    let mut small_buf = vec![255u8; (nx * ny * 4) as usize];
    {
        let mut small_ra = RowAccessor::new();
        unsafe { small_ra.attach(small_buf.as_mut_ptr(), nx, ny, small_stride) };
        let small_pf = PixfmtRgba32::new(&mut small_ra);
        let mut small_rb = RendererBase::new(small_pf);
        small_rb.clear(&Rgba8::new(255, 255, 255, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        let mut path = PathStorage::new();
        path.move_to(sx0, sy0);
        path.line_to(sx1, sy1);
        path.line_to(sx2, sy2);
        path.close_polygon(0);
        ras.reset();
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut small_rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Upscale: each pixel becomes a pixel_size x pixel_size block
    let psi = ps as u32;
    for py in 0..ny {
        for px in 0..nx {
            let si = ((py * nx + px) * 4) as usize;
            if si + 3 >= small_buf.len() {
                continue;
            }
            let r = small_buf[si];
            let g = small_buf[si + 1];
            let b = small_buf[si + 2];
            if r == 255 && g == 255 && b == 255 {
                continue;
            }
            for dy in 0..psi {
                for dx in 0..psi {
                    let bx = px * psi + dx;
                    let by = py * psi + dy;
                    if bx >= width || by >= height {
                        continue;
                    }
                    let di = ((by * width + bx) * 4) as usize;
                    buf[di] = r;
                    buf[di + 1] = g;
                    buf[di + 2] = b;
                    buf[di + 3] = 255;
                }
            }
        }
    }

    // Now use rasterizer for grid and outline overlay
    let stride = (width * 4) as i32;
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Grid lines
    for py in 0..=ny {
        let y = (py as f64 * ps).min(height as f64 - 1.0);
        let mut line = PathStorage::new();
        line.move_to(0.0, y);
        line.line_to(width as f64, y);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(0.5);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 40));
    }
    for px in 0..=nx {
        let x = (px as f64 * ps).min(width as f64 - 1.0);
        let mut line = PathStorage::new();
        line.move_to(x, 0.0);
        line.line_to(x, height as f64);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(0.5);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 40));
    }

    // Triangle outline
    let mut outline = PathStorage::new();
    outline.move_to(vx0, vy0);
    outline.line_to(vx1, vy1);
    outline.line_to(vx2, vy2);
    outline.close_polygon(0);
    let mut stroke = ConvStroke::new(&mut outline);
    stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 100, 200, 255));

    buf
}

// ============================================================================
// Gamma Correction — matches C++ gamma_correction.cpp
// ============================================================================

/// Render gamma correction visualization with ellipses.
///
/// params[0] = thickness (default 1.0)
/// params[1] = gamma (default 1.0)
pub fn gamma_correction(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let thickness = params.get(0).copied().unwrap_or(1.0).max(0.1);
    let gamma_val = params.get(1).copied().unwrap_or(1.0).max(0.1);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(20, 20, 20, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let w = width as f64;
    let h = height as f64;

    // Concentric stroked ellipses with different colors
    let cx = w * 0.5;
    let cy = h * 0.45;
    let colors: [(u32, u32, u32); 5] = [
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 255),
        (0, 0, 0),
    ];

    for (i, (cr, cg, cb)) in colors.iter().enumerate() {
        let t = i as f64 / colors.len() as f64;
        let rx = (w * 0.4) * (1.0 - t * 0.6);
        let ry = (h * 0.35) * (1.0 - t * 0.6);
        let ell = Ellipse::new(cx, cy, rx, ry, 64, false);
        let mut stroke = ConvStroke::new(ell);
        stroke.set_width(thickness + (1.0 - t) * 3.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(*cr, *cg, *cb, 255));
    }

    // Gamma curve graph at bottom
    let gx = w * 0.05;
    let gy = h * 0.7;
    let gw = w * 0.9;
    let gh = h * 0.25;

    // Graph background
    {
        let mut rect = PathStorage::new();
        rect.move_to(gx, gy);
        rect.line_to(gx + gw, gy);
        rect.line_to(gx + gw, gy + gh);
        rect.line_to(gx, gy + gh);
        rect.close_polygon(0);
        ras.reset();
        ras.add_path(&mut rect, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(40, 40, 40, 255));
    }

    // Gamma curve: y = x^gamma
    {
        let mut curve = PathStorage::new();
        for i in 0..=256 {
            let t = i as f64 / 256.0;
            let gv = t.powf(gamma_val);
            let x = gx + t * gw;
            let y = gy + gh - gv * gh;
            if i == 0 {
                curve.move_to(x, y);
            } else {
                curve.line_to(x, y);
            }
        }
        let mut stroke = ConvStroke::new(&mut curve);
        stroke.set_width(2.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 200, 50, 255));
    }

    // Linear reference line
    {
        let mut line = PathStorage::new();
        line.move_to(gx, gy + gh);
        line.line_to(gx + gw, gy);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(0.5);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(100, 100, 100, 200));
    }

    buf
}

// ============================================================================
// Line Thickness — matches C++ line_thickness.cpp
// ============================================================================

/// Render lines of varying thickness.
///
/// params[0..4] = x0,y0, x1,y1 (line endpoints)
pub fn line_thickness(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let x0 = params.get(0).copied().unwrap_or(w * 0.05);
    let y0 = params.get(1).copied().unwrap_or(h * 0.5);
    let x1 = params.get(2).copied().unwrap_or(w * 0.95);
    let y1 = params.get(3).copied().unwrap_or(h * 0.5);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let n_lines = 20;
    let spacing = h / (n_lines as f64 + 2.0);

    for i in 0..n_lines {
        let t = i as f64 / n_lines as f64;
        let lw = 0.1 + t * 5.0;
        let ly = spacing * (i as f64 + 1.0);
        let dy = (y1 - y0) * (ly / h - 0.5) * 0.2;

        let mut path = PathStorage::new();
        path.move_to(x0, ly - dy);
        path.line_to(x1, ly + dy);

        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(lw);
        ras.reset();
        ras.add_path(&mut stroke, 0);

        let gray = (50.0 + t * 150.0) as u32;
        render_scanlines_aa_solid(
            &mut ras, &mut sl, &mut rb,
            &Rgba8::new(gray, gray / 2, 0, 255),
        );
    }

    // Vertex markers
    for (vx, vy) in &[(x0, y0), (x1, y1)] {
        let mut ell = Ellipse::new(*vx, *vy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    buf
}

// ============================================================================
// Rasterizers — triangle rendering, based on C++ rasterizers.cpp
// ============================================================================

/// Render a filled and stroked triangle.
///
/// params[0..6] = x0,y0, x1,y1, x2,y2
/// params[6] = gamma (unused), params[7] = alpha 0-1
pub fn rasterizers(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let vx0 = params.get(0).copied().unwrap_or(100.0);
    let vy0 = params.get(1).copied().unwrap_or(60.0);
    let vx1 = params.get(2).copied().unwrap_or(400.0);
    let vy1 = params.get(3).copied().unwrap_or(80.0);
    let vx2 = params.get(4).copied().unwrap_or(250.0);
    let vy2 = params.get(5).copied().unwrap_or(350.0);
    let _gamma = params.get(6).copied().unwrap_or(1.0);
    let alpha = params.get(7).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let a = (alpha.clamp(0.0, 1.0) * 255.0) as u32;

    // Filled triangle
    {
        let mut path = PathStorage::new();
        path.move_to(vx0, vy0);
        path.line_to(vx1, vy1);
        path.line_to(vx2, vy2);
        path.close_polygon(0);
        ras.reset();
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 220, 255, a));
    }

    // Stroked outline
    {
        let mut path = PathStorage::new();
        path.move_to(vx0, vy0);
        path.line_to(vx1, vy1);
        path.line_to(vx2, vy2);
        path.close_polygon(0);
        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(3.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 80, a));
    }

    // Vertex markers
    for (vx, vy) in &[(vx0, vy0), (vx1, vy1), (vx2, vy2)] {
        let mut ell = Ellipse::new(*vx, *vy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    buf
}

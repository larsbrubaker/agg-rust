//! Demo render functions.
//!
//! Each function renders a specific demo into an RGBA pixel buffer.
//! The buffer is width * height * 4 bytes (RGBA order).

use agg_rust::basics::{is_stop, is_vertex, VertexSource, PATH_FLAGS_CW, PATH_FLAGS_CCW};
use agg_rust::curves::Curve4Div;
use agg_rust::bounding_rect::bounding_rect;
use agg_rust::color::Rgba8;
use agg_rust::conv_contour::ConvContour;
use agg_rust::ctrl::{render_ctrl, SliderCtrl, CboxCtrl, RboxCtrl};
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_dash::ConvDash;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLut;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_filters::{
    ImageFilterBilinear, ImageFilterBicubic, ImageFilterSpline16, ImageFilterSpline36,
    ImageFilterHanning, ImageFilterHamming, ImageFilterHermite, ImageFilterKaiser,
    ImageFilterQuadric, ImageFilterCatrom, ImageFilterGaussian, ImageFilterBessel,
    ImageFilterMitchell, ImageFilterSinc, ImageFilterLanczos, ImageFilterBlackman,
    ImageFilterFunction,
};
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
    GradientConic, GradientDiamond, GradientRadial, GradientRadialFocus,
    GradientReflectAdaptor, GradientSqrtXY, GradientX, GradientXY,
    SpanGradient,
};
use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_bilinear::TransBilinear;
use agg_rust::trans_perspective::TransPerspective;

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

    // Render AGG slider control — matching C++ lion.cpp
    let mut s_alpha = SliderCtrl::new(5.0, 5.0, 507.0, 12.0);
    s_alpha.label("Alpha%3.3f");
    s_alpha.set_value(alpha as f64 / 255.0);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_alpha);

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

    // AGG controls matching C++ gouraud.cpp
    let mut s_dilation = SliderCtrl::new(5.0, 5.0, 395.0, 11.0);
    s_dilation.label("Dilation=%3.2f");
    s_dilation.set_value(d);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_dilation);

    let mut s_gamma = SliderCtrl::new(5.0, 20.0, 395.0, 26.0);
    s_gamma.label("Linear gamma=%3.2f");
    s_gamma.set_value(_gamma);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);

    let mut s_alpha = SliderCtrl::new(5.0, 35.0, 395.0, 41.0);
    s_alpha.label("Opacity=%3.2f");
    s_alpha.set_value(alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_alpha);

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

    // AGG controls matching C++ conv_stroke.cpp
    let mut r_join = RboxCtrl::new(10.0, 10.0, 133.0, 80.0);
    r_join.add_item("Miter Join");
    r_join.add_item("Miter Join Revert");
    r_join.add_item("Round Join");
    r_join.add_item("Bevel Join");
    r_join.set_cur_item(join_idx);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_join);

    let mut r_cap = RboxCtrl::new(10.0, 90.0, 133.0, 160.0);
    r_cap.add_item("Butt Cap");
    r_cap.add_item("Square Cap");
    r_cap.add_item("Round Cap");
    r_cap.set_cur_item(cap_idx);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_cap);

    let mut s_width = SliderCtrl::new(140.0, 14.0, 490.0, 22.0);
    s_width.label("Width=%1.2f");
    s_width.range(3.0, 40.0);
    s_width.set_value(sw);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut s_miter = SliderCtrl::new(140.0, 34.0, 490.0, 42.0);
    s_miter.label("Miter Limit=%1.2f");
    s_miter.range(1.0, 10.0);
    s_miter.set_value(miter_limit);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_miter);

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

    // Render AGG controls — matching C++ bezier_div.cpp
    let mut s_width = SliderCtrl::new(245.0, 5.0, 495.0, 12.0);
    s_width.label("Width=%.2f");
    s_width.range(-50.0, 100.0);
    s_width.set_value(sw);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut c_pts = CboxCtrl::new(250.0, 20.0, "Show Points");
    c_pts.set_status(show_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_pts);

    let mut c_outline = CboxCtrl::new(250.0, 35.0, "Show Stroke Outline");
    c_outline.set_status(show_outline);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_outline);

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
/// params[4] = corner radius (default 25)
/// params[5] = subpixel offset (default 0)
/// params[6] = white_on_black (0 or 1, default 0)
pub fn rounded_rect_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let x1 = params.get(0).copied().unwrap_or(100.0);
    let y1 = params.get(1).copied().unwrap_or(80.0);
    let x2 = params.get(2).copied().unwrap_or(400.0);
    let y2 = params.get(3).copied().unwrap_or(280.0);
    let radius = params.get(4).copied().unwrap_or(25.0);
    let offset = params.get(5).copied().unwrap_or(0.0);
    let white_on_black = params.get(6).copied().unwrap_or(0.0) > 0.5;

    let (bg, fg) = if white_on_black {
        (Rgba8::new(0, 0, 0, 255), Rgba8::new(255, 255, 255, 255))
    } else {
        (Rgba8::new(255, 255, 255, 255), Rgba8::new(0, 0, 0, 255))
    };

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&bg);

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Apply subpixel offset
    let ox1 = x1 + offset;
    let oy1 = y1 + offset;
    let ox2 = x2 + offset;
    let oy2 = y2 + offset;

    // Fill
    let fill_color = if white_on_black {
        Rgba8::new(40, 40, 60, 255)
    } else {
        Rgba8::new(255, 230, 200, 255)
    };
    let mut rrect = RoundedRect::new(ox1, oy1, ox2, oy2, radius);
    ras.reset();
    ras.add_path(&mut rrect, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &fill_color);

    // Stroke
    let mut rrect2 = RoundedRect::new(ox1, oy1, ox2, oy2, radius);
    let mut stroke = ConvStroke::new(&mut rrect2);
    stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &fg);

    // Inscribed circle
    {
        let cx = (ox1 + ox2) / 2.0;
        let cy = (oy1 + oy2) / 2.0;
        let r = ((ox2 - ox1).min(oy2 - oy1) / 2.0 - radius).max(10.0);
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

    // Render AGG controls — matching C++ rounded_rect.cpp
    let mut s_radius = SliderCtrl::new(10.0, 10.0, 590.0, 19.0);
    s_radius.label("radius=%4.3f");
    s_radius.range(0.0, 50.0);
    s_radius.set_value(radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);

    let mut s_offset = SliderCtrl::new(10.0, 30.0, 590.0, 39.0);
    s_offset.label("subpixel offset=%4.3f");
    s_offset.range(-2.0, 3.0);
    s_offset.set_value(offset);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_offset);

    let mut c_wob = CboxCtrl::new(10.0, 50.0, "White on black");
    c_wob.set_status(white_on_black);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_wob);

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

    // AGG control matching C++ aa_demo.cpp
    let mut s_pixel = SliderCtrl::new(80.0, 10.0, (width as f64) - 10.0, 19.0);
    s_pixel.label("Pixel size=%1.0f");
    s_pixel.range(8.0, 100.0);
    s_pixel.num_steps(23);
    s_pixel.set_value(pixel_size);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_pixel);

    buf
}

// ============================================================================
// Gamma Correction — matches C++ gamma_correction.cpp
// ============================================================================

/// Render gamma correction visualization with ellipses.
///
/// params[0] = thickness (default 1.0)
/// params[1] = contrast (default 1.0)
/// params[2] = gamma (default 1.0)
pub fn gamma_correction(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let thickness = params.get(0).copied().unwrap_or(1.0).max(0.1);
    let _contrast = params.get(1).copied().unwrap_or(1.0);
    let gamma_val = params.get(2).copied().unwrap_or(1.0).max(0.1);

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

    // Render AGG slider controls — matching C++ gamma_correction.cpp
    let mut s_thick = SliderCtrl::new(5.0, 5.0, 395.0, 11.0);
    s_thick.label("Thickness=%3.2f");
    s_thick.range(0.0, 3.0);
    s_thick.set_value(thickness);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_thick);

    let mut s_contrast = SliderCtrl::new(5.0, 20.0, 395.0, 26.0);
    s_contrast.label("Contrast");
    s_contrast.range(0.0, 1.0);
    s_contrast.set_value(_contrast);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_contrast);

    let mut s_gamma = SliderCtrl::new(5.0, 35.0, 395.0, 41.0);
    s_gamma.label("Gamma=%3.2f");
    s_gamma.range(0.5, 3.0);
    s_gamma.set_value(gamma_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);

    buf
}

// ============================================================================
// Line Thickness — matches C++ line_thickness.cpp
// ============================================================================

/// Render lines of varying thickness — matching C++ line_thickness.cpp.
///
/// params[0..4] = x0,y0, x1,y1 (line endpoints)
/// params[4] = line thickness (default 1.0)
/// params[5] = blur radius (default 1.5, display only)
/// params[6] = monochrome (0 or 1, default 1, display only)
/// params[7] = invert (0 or 1, default 0, display only)
pub fn line_thickness(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let x0 = params.get(0).copied().unwrap_or(w * 0.05);
    let y0 = params.get(1).copied().unwrap_or(h * 0.5);
    let x1 = params.get(2).copied().unwrap_or(w * 0.95);
    let y1 = params.get(3).copied().unwrap_or(h * 0.5);
    let thickness = params.get(4).copied().unwrap_or(1.0);
    let _blur = params.get(5).copied().unwrap_or(1.5);
    let _monochrome = params.get(6).copied().unwrap_or(1.0) > 0.5;
    let _invert = params.get(7).copied().unwrap_or(0.0) > 0.5;

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

    // AGG controls matching C++ line_thickness.cpp
    let mut s_thick = SliderCtrl::new(10.0, 10.0, 630.0, 19.0);
    s_thick.label("Line thickness=%1.2f");
    s_thick.range(0.0, 5.0);
    s_thick.set_value(thickness);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_thick);

    let mut s_blur = SliderCtrl::new(10.0, 30.0, 630.0, 39.0);
    s_blur.label("Blur radius=%1.2f");
    s_blur.range(0.0, 2.0);
    s_blur.set_value(_blur);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_blur);

    let mut c_mono = CboxCtrl::new(10.0, 50.0, "Monochrome");
    c_mono.set_status(_monochrome);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_mono);

    let mut c_invert = CboxCtrl::new(10.0, 70.0, "Invert");
    c_invert.set_status(_invert);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_invert);

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
    let vx0 = params.get(0).copied().unwrap_or(157.0);
    let vy0 = params.get(1).copied().unwrap_or(60.0);
    let vx1 = params.get(2).copied().unwrap_or(369.0);
    let vy1 = params.get(3).copied().unwrap_or(170.0);
    let vx2 = params.get(4).copied().unwrap_or(243.0);
    let vy2 = params.get(5).copied().unwrap_or(310.0);
    let _gamma = params.get(6).copied().unwrap_or(0.5);
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

    // AGG controls matching C++ rasterizers.cpp
    let mut s_gamma = SliderCtrl::new(140.0, 14.0, 280.0, 22.0);
    s_gamma.label("Gamma=%1.2f");
    s_gamma.range(0.0, 1.0);
    s_gamma.set_value(_gamma);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);

    let mut s_alpha = SliderCtrl::new(290.0, 14.0, 490.0, 22.0);
    s_alpha.label("Alpha=%1.2f");
    s_alpha.range(0.0, 1.0);
    s_alpha.set_value(alpha);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_alpha);

    buf
}

// ============================================================================
// Conv Contour — matches C++ conv_contour.cpp
// ============================================================================

/// Render contour demo — letter "A" shape with adjustable contour width.
///
/// params[0] = close mode (0=close, 1=close_cw, 2=close_ccw)
/// params[1] = contour width [-100, 100] (default 0)
/// params[2] = auto_detect orientation (0 or 1, default 1)
pub fn conv_contour_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let close_mode = params.get(0).copied().unwrap_or(0.0) as i32;
    let contour_w = params.get(1).copied().unwrap_or(0.0);
    let auto_detect = params.get(2).copied().unwrap_or(1.0) > 0.5;

    let flag = match close_mode {
        1 => PATH_FLAGS_CW,
        2 => PATH_FLAGS_CCW,
        _ => 0,
    };

    // Build the letter "A" path, matching C++ conv_contour.cpp compose_path()
    let mut path = PathStorage::new();

    // Outer contour
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
    path.close_polygon(flag);

    // Inner contour (hole in the "A")
    path.move_to(28.47, 9.62);
    path.line_to(28.47, 26.66);
    path.curve3(21.09, 23.73, 18.95, 22.51);
    path.curve3(15.09, 20.36, 13.43, 18.02);
    path.curve3(11.77, 15.67, 11.77, 12.89);
    path.curve3(11.77, 9.38, 13.87, 7.06);
    path.curve3(15.97, 4.74, 18.70, 4.74);
    path.curve3(22.41, 4.74, 28.47, 9.62);
    path.close_polygon(flag);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Transform: scale 4x, translate to center of canvas (matching C++ scale=4.0, translate(150,100))
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_scaling_uniform(4.0));
    mtx.multiply(&TransAffine::new_translation(150.0, 100.0));

    // Pipeline: path → conv_transform → conv_curve → conv_contour
    let trans = ConvTransform::new(&mut path, mtx);
    let curve = ConvCurve::new(trans);
    let mut contour = ConvContour::new(curve);
    contour.set_width(contour_w);
    contour.set_auto_detect_orientation(auto_detect);

    ras.reset();
    ras.add_path(&mut contour, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Render AGG controls — matching C++ conv_contour.cpp
    let mut r_close = RboxCtrl::new(10.0, 10.0, 130.0, 80.0);
    r_close.add_item("Close");
    r_close.add_item("Close CW");
    r_close.add_item("Close CCW");
    r_close.set_cur_item(close_mode);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_close);

    let mut s_width = SliderCtrl::new(140.0, 14.0, 430.0, 22.0);
    s_width.label("Width=%1.2f");
    s_width.range(-100.0, 100.0);
    s_width.set_value(contour_w);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut c_auto = CboxCtrl::new(140.0, 30.0, "Autodetect orientation if not defined");
    c_auto.set_status(auto_detect);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_auto);

    buf
}

// ============================================================================
// Conv Dash — triangle with dash patterns and cap styles
// ============================================================================

/// Render a triangle with dashed stroke, matching core of C++ conv_dash_marker.cpp.
///
/// params[0..6] = x0,y0, x1,y1, x2,y2 (3 vertex positions)
/// params[6] = cap type (0=butt, 1=square, 2=round)
/// params[7] = stroke width (default 3.0)
/// params[8] = close polygon (0 or 1, default 0)
/// params[9] = even_odd fill (0 or 1, default 0)
/// params[10] = smooth value (default 1.0, display only)
pub fn conv_dash_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let vx0 = params.get(0).copied().unwrap_or(157.0);
    let vy0 = params.get(1).copied().unwrap_or(60.0);
    let vx1 = params.get(2).copied().unwrap_or(469.0);
    let vy1 = params.get(3).copied().unwrap_or(170.0);
    let vx2 = params.get(4).copied().unwrap_or(243.0);
    let vy2 = params.get(5).copied().unwrap_or(310.0);
    let cap_idx = params.get(6).copied().unwrap_or(0.0) as i32;
    let sw = params.get(7).copied().unwrap_or(3.0).max(0.5);
    let close = params.get(8).copied().unwrap_or(0.0) > 0.5;
    let even_odd = params.get(9).copied().unwrap_or(0.0) > 0.5;
    let smooth = params.get(10).copied().unwrap_or(1.0);

    let cap = match cap_idx {
        1 => LineCap::Square,
        2 => LineCap::Round,
        _ => LineCap::Butt,
    };

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build triangle path
    let mut path = PathStorage::new();
    path.move_to(vx0, vy0);
    path.line_to(vx1, vy1);
    path.line_to(vx2, vy2);
    if close {
        path.close_polygon(0);
    }

    // Layer 1: Filled triangle (semi-transparent)
    {
        ras.reset();
        if even_odd {
            ras.filling_rule(agg_rust::basics::FillingRule::EvenOdd);
        }
        ras.add_path(&mut path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(178, 128, 25, 128));
        ras.filling_rule(agg_rust::basics::FillingRule::NonZero);
    }

    // Layer 2: Solid stroke outline
    {
        let mut stroke = ConvStroke::new(&mut path);
        stroke.set_width(sw);
        stroke.set_line_cap(cap);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 153, 0, 204));
    }

    // Layer 3: Dashed stroke overlay
    {
        let mut dash = ConvDash::new(&mut path);
        dash.add_dash(20.0, 5.0);
        dash.add_dash(5.0, 5.0);
        dash.dash_start(10.0);
        let mut stroke = ConvStroke::new(dash);
        stroke.set_width(sw);
        stroke.set_line_cap(cap);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Vertex markers
    for (vx, vy) in &[(vx0, vy0), (vx1, vy1), (vx2, vy2)] {
        let mut ell = Ellipse::new(*vx, *vy, 5.0, 5.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    // Render AGG controls — matching C++ conv_dash_marker.cpp
    let mut r_cap = RboxCtrl::new(10.0, 10.0, 130.0, 80.0);
    r_cap.add_item("Butt Cap");
    r_cap.add_item("Square Cap");
    r_cap.add_item("Round Cap");
    r_cap.set_cur_item(cap_idx);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_cap);

    let mut s_width = SliderCtrl::new(140.0, 14.0, 280.0, 22.0);
    s_width.label("Width=%1.2f");
    s_width.range(0.0, 10.0);
    s_width.set_value(sw);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut s_smooth = SliderCtrl::new(290.0, 14.0, 490.0, 22.0);
    s_smooth.label("Smooth=%1.2f");
    s_smooth.range(0.0, 2.0);
    s_smooth.set_value(smooth);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_smooth);

    let mut c_close = CboxCtrl::new(140.0, 30.0, "Close Polygons");
    c_close.set_status(close);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_close);

    let mut c_eo = CboxCtrl::new(300.0, 30.0, "Even-Odd Fill");
    c_eo.set_status(even_odd);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_eo);

    buf
}

// ============================================================================
// GSV Text — text rendering demo
// ============================================================================

/// Render text using the built-in GSV text engine.
///
/// params[0] = text size (default 24)
/// params[1] = stroke width (default 1.0)
/// params[2] = x offset (default 20)
/// params[3] = y offset (default 40)
pub fn gsv_text_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let text_size = params.get(0).copied().unwrap_or(24.0).max(4.0);
    let stroke_w = params.get(1).copied().unwrap_or(1.0).max(0.1);
    let x_off = params.get(2).copied().unwrap_or(20.0);
    let y_off = params.get(3).copied().unwrap_or(40.0);

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

    // Title
    {
        let mut txt = GsvText::new();
        txt.size(text_size * 1.5, 0.0);
        txt.start_point(x_off, y_off);
        txt.text("AGG for Rust - GSV Text");
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(stroke_w * 1.5);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 50, 120, 255));
    }

    // Subtitle
    {
        let mut txt = GsvText::new();
        txt.size(text_size * 0.7, 0.0);
        txt.start_point(x_off, y_off + text_size * 2.0);
        txt.text("Built-in vector font — no dependencies");
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(stroke_w * 0.7);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(100, 100, 100, 255));
    }

    // Sample text lines at different sizes
    let samples = [
        ("ABCDEFGHIJKLM", Rgba8::new(200, 0, 0, 255)),
        ("NOPQRSTUVWXYZ", Rgba8::new(0, 150, 0, 255)),
        ("abcdefghijklm", Rgba8::new(0, 0, 200, 255)),
        ("nopqrstuvwxyz", Rgba8::new(150, 100, 0, 255)),
        ("0123456789 !@#$%", Rgba8::new(0, 100, 150, 255)),
    ];

    let base_y = y_off + text_size * 4.0;
    for (i, (text, color)) in samples.iter().enumerate() {
        let y = base_y + i as f64 * (text_size * 1.5);
        if y + text_size > h {
            break;
        }
        let mut txt = GsvText::new();
        txt.size(text_size, 0.0);
        txt.start_point(x_off, y);
        txt.text(text);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(stroke_w);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, color);
    }

    // Paragraph at bottom — large text
    let large_y = base_y + samples.len() as f64 * (text_size * 1.5) + text_size;
    if large_y + text_size * 3.0 < h {
        let mut txt = GsvText::new();
        txt.size(text_size * 2.5, 0.0);
        txt.start_point(x_off, large_y);
        txt.text("Aa Bb Cc");
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(stroke_w * 2.0);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(30, 30, 30, 255));
    }

    // Size label at top-right
    {
        let label = format!("Size: {:.0}px  Stroke: {:.1}", text_size, stroke_w);
        let mut txt = GsvText::new();
        txt.size(12.0, 0.0);
        txt.start_point(w - 200.0, 20.0);
        txt.text(&label);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(0.8);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(140, 140, 140, 200));
    }

    buf
}

// ============================================================================
// Perspective — lion with bilinear/perspective quad transform
// ============================================================================

/// Render the lion with perspective or bilinear transformation.
/// Matches C++ perspective.cpp — 4 draggable quad corners.
///
/// params[0..8] = q0x,q0y, q1x,q1y, q2x,q2y, q3x,q3y (quad corners)
/// params[8] = transform type (0=bilinear, 1=perspective)
pub fn perspective_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    // Compute bounding rect of lion paths
    let path_ids: Vec<u32> = path_idx.iter().map(|&i| i as u32).collect();
    let bbox = bounding_rect(&mut path, &path_ids, 0, npaths).unwrap_or(
        agg_rust::basics::RectD::new(0.0, 0.0, 250.0, 400.0),
    );

    // Default quad = bounding rect corners (possibly offset to center)
    let w = width as f64;
    let h = height as f64;
    let ox = (w - (bbox.x2 - bbox.x1)) / 2.0 - bbox.x1;
    let oy = (h - (bbox.y2 - bbox.y1)) / 2.0 - bbox.y1;

    let q0x = params.get(0).copied().unwrap_or(bbox.x1 + ox);
    let q0y = params.get(1).copied().unwrap_or(bbox.y1 + oy);
    let q1x = params.get(2).copied().unwrap_or(bbox.x2 + ox);
    let q1y = params.get(3).copied().unwrap_or(bbox.y1 + oy);
    let q2x = params.get(4).copied().unwrap_or(bbox.x2 + ox);
    let q2y = params.get(5).copied().unwrap_or(bbox.y2 + oy);
    let q3x = params.get(6).copied().unwrap_or(bbox.x1 + ox);
    let q3y = params.get(7).copied().unwrap_or(bbox.y2 + oy);
    let trans_type = params.get(8).copied().unwrap_or(0.0) as i32;

    let quad = [q0x, q0y, q1x, q1y, q2x, q2y, q3x, q3y];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Create transform function (rect → quad)
    enum TransformKind {
        Bilinear(TransBilinear),
        Perspective(TransPerspective),
    }

    let transform = if trans_type == 0 {
        TransformKind::Bilinear(TransBilinear::new_rect_to_quad(
            bbox.x1, bbox.y1, bbox.x2, bbox.y2, &quad,
        ))
    } else {
        let mut tp = TransPerspective::new();
        tp.rect_to_quad(bbox.x1, bbox.y1, bbox.x2, bbox.y2, &quad);
        TransformKind::Perspective(tp)
    };

    let valid = match &transform {
        TransformKind::Bilinear(tb) => tb.is_valid(),
        TransformKind::Perspective(tp) => tp.is_valid(),
    };

    if valid {
        // Transform all lion vertices in-place
        let n_verts = path.total_vertices();
        for vi in 0..n_verts {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = path.vertex_idx(vi, &mut x, &mut y);
            if is_vertex(cmd) {
                match &transform {
                    TransformKind::Bilinear(tb) => tb.transform(&mut x, &mut y),
                    TransformKind::Perspective(tp) => tp.transform(&mut x, &mut y),
                }
                path.modify_vertex(vi, x, y);
            }
        }

        // Render each colored lion path
        for i in 0..npaths {
            let start = path_idx[i] as u32;
            ras.reset();
            ras.add_path(&mut path, start);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
        }
    }

    // Draw quad outline
    {
        let mut quad_path = PathStorage::new();
        quad_path.move_to(q0x, q0y);
        quad_path.line_to(q1x, q1y);
        quad_path.line_to(q2x, q2y);
        quad_path.line_to(q3x, q3y);
        quad_path.close_polygon(0);
        let mut stroke = ConvStroke::new(&mut quad_path);
        stroke.set_width(2.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(50, 50, 200, 200));
    }

    // Quad corner markers
    for (cx, cy) in &[(q0x, q0y), (q1x, q1y), (q2x, q2y), (q3x, q3y)] {
        let mut ell = Ellipse::new(*cx, *cy, 6.0, 6.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 50, 50, 220));
    }

    // AGG control matching C++ perspective.cpp
    let mut r_type = RboxCtrl::new(420.0, 5.0, 550.0, 55.0);
    r_type.add_item("Bilinear");
    r_type.add_item("Perspective");
    r_type.set_cur_item(trans_type);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_type);

    buf
}

// ============================================================================
// Image Filter Graph — filter kernel visualization
// ============================================================================

/// Render image filter weight function graphs.
/// Matches C++ image_fltr_graph.cpp — select a filter, see its kernel shape.
///
/// params[0] = filter index (0–15, default 0)
/// params[1] = radius for variable-radius filters (default 4.0)
pub fn image_fltr_graph(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let filter_idx = params.get(0).copied().unwrap_or(0.0) as usize;
    let custom_radius = params.get(1).copied().unwrap_or(4.0).clamp(2.0, 8.0);

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

    // Plot area
    let x_start = 10.0;
    let x_end = w - 10.0;
    let y_start = 30.0;
    let y_end = h - 10.0;
    let x_width = x_end - x_start;
    let y_height = y_end - y_start;
    let y_baseline = y_start + y_height / 6.0; // Baseline for weight = 0

    // Get filter function and radius
    let (radius, filter_name): (f64, &str) = match filter_idx {
        0 => (ImageFilterBilinear.radius(), "Bilinear"),
        1 => (ImageFilterBicubic.radius(), "Bicubic"),
        2 => (ImageFilterSpline16.radius(), "Spline16"),
        3 => (ImageFilterSpline36.radius(), "Spline36"),
        4 => (ImageFilterHanning.radius(), "Hanning"),
        5 => (ImageFilterHamming.radius(), "Hamming"),
        6 => (ImageFilterHermite.radius(), "Hermite"),
        7 => (ImageFilterKaiser::new(5.0).radius(), "Kaiser"),
        8 => (ImageFilterQuadric.radius(), "Quadric"),
        9 => (ImageFilterCatrom.radius(), "Catrom"),
        10 => (ImageFilterGaussian.radius(), "Gaussian"),
        11 => (ImageFilterBessel.radius(), "Bessel"),
        12 => (ImageFilterMitchell::new(1.0 / 3.0, 1.0 / 3.0).radius(), "Mitchell"),
        13 => (custom_radius, "Sinc"),
        14 => (custom_radius, "Lanczos"),
        15 => (custom_radius, "Blackman"),
        _ => (ImageFilterBilinear.radius(), "Bilinear"),
    };

    // Grid lines
    let n_grid = 16;
    for i in 0..=n_grid {
        let t = i as f64 / n_grid as f64;
        let x = x_start + t * x_width;
        let mut line = PathStorage::new();
        line.move_to(x, y_start);
        line.line_to(x, y_end);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(if i == n_grid / 2 { 1.0 } else { 0.3 });
        ras.reset();
        ras.add_path(&mut stroke, 0);
        let gray = if i == n_grid / 2 { 80 } else { 180 };
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(gray, gray, gray, 200));
    }

    // Baseline (y = 0 weight)
    {
        let mut line = PathStorage::new();
        line.move_to(x_start, y_baseline);
        line.line_to(x_end, y_baseline);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 200));
    }

    // Plot filter weight curve
    let center_x = x_start + x_width / 2.0;
    let x_scale = x_width / (2.0 * radius);
    let y_scale = -(y_end - y_baseline); // Negative: weight=1 goes up

    let n_points = (x_width as usize).max(200);
    let mut curve_path = PathStorage::new();

    // Macro to plot any filter function
    macro_rules! plot_filter {
        ($filter:expr) => {{
            for i in 0..=n_points {
                let t = i as f64 / n_points as f64;
                let x_screen = x_start + t * x_width;
                let x_filter = (x_screen - center_x) / x_scale; // Map to filter coords [-radius, radius]
                let weight = if x_filter.abs() <= radius {
                    $filter.calc_weight(x_filter)
                } else {
                    0.0
                };
                let y_screen = y_baseline + weight * y_scale;
                if i == 0 {
                    curve_path.move_to(x_screen, y_screen);
                } else {
                    curve_path.line_to(x_screen, y_screen);
                }
            }
        }};
    }

    match filter_idx {
        0 => plot_filter!(ImageFilterBilinear),
        1 => plot_filter!(ImageFilterBicubic),
        2 => plot_filter!(ImageFilterSpline16),
        3 => plot_filter!(ImageFilterSpline36),
        4 => plot_filter!(ImageFilterHanning),
        5 => plot_filter!(ImageFilterHamming),
        6 => plot_filter!(ImageFilterHermite),
        7 => plot_filter!(ImageFilterKaiser::new(5.0)),
        8 => plot_filter!(ImageFilterQuadric),
        9 => plot_filter!(ImageFilterCatrom),
        10 => plot_filter!(ImageFilterGaussian),
        11 => plot_filter!(ImageFilterBessel),
        12 => plot_filter!(ImageFilterMitchell::new(1.0 / 3.0, 1.0 / 3.0)),
        13 => plot_filter!(ImageFilterSinc::new(custom_radius)),
        14 => plot_filter!(ImageFilterLanczos::new(custom_radius)),
        15 => plot_filter!(ImageFilterBlackman::new(custom_radius)),
        _ => plot_filter!(ImageFilterBilinear),
    }

    // Render the curve
    {
        let mut stroke = ConvStroke::new(&mut curve_path);
        stroke.set_width(2.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(200, 0, 0, 255));
    }

    // Filter name label
    {
        let label = format!("{} (r={:.1})", filter_name, radius);
        let mut txt = GsvText::new();
        txt.size(16.0, 0.0);
        txt.start_point(x_start + 5.0, y_start + 2.0);
        txt.text(&label);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(1.2);
        stroke.set_line_cap(LineCap::Round);
        stroke.set_line_join(LineJoin::Round);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 120, 255));
    }

    // AGG control matching C++ image_fltr_graph.cpp
    let mut s_radius = SliderCtrl::new(5.0, 5.0, w - 5.0, 10.0);
    s_radius.label("Radius=%.3f");
    s_radius.range(2.0, 8.0);
    s_radius.set_value(custom_radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);

    buf
}

// ============================================================================
// Image1 — Image Affine Transformations with Bilinear Filtering
// ============================================================================

/// Original AGG "spheres.bmp" embedded at compile time (320x300, 24-bit BGR).
static SPHERES_BMP: &[u8] = include_bytes!("spheres.bmp");

/// Decode the embedded spheres BMP into RGBA pixels. Returns (width, height, rgba_data).
fn load_spheres_image() -> (u32, u32, Vec<u8>) {
    let d = SPHERES_BMP;
    let offset = u32::from_le_bytes([d[10], d[11], d[12], d[13]]) as usize;
    let w = i32::from_le_bytes([d[18], d[19], d[20], d[21]]) as u32;
    let h = i32::from_le_bytes([d[22], d[23], d[24], d[25]]) as u32;
    let bpp = u16::from_le_bytes([d[28], d[29]]) as usize;
    let bytes_per_pixel = bpp / 8;
    let row_size = ((w as usize * bytes_per_pixel + 3) / 4) * 4; // BMP rows are 4-byte aligned

    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        // BMP stores rows bottom-to-top; row 0 in BMP = bottom of image
        let src_row = offset + y * row_size;
        let dst_row = y * w as usize * 4;
        for x in 0..w as usize {
            let si = src_row + x * bytes_per_pixel;
            let di = dst_row + x * 4;
            rgba[di] = d[si + 2];     // R (BMP is BGR)
            rgba[di + 1] = d[si + 1]; // G
            rgba[di + 2] = d[si];     // B
            rgba[di + 3] = 255;       // A
        }
    }
    (w, h, rgba)
}

/// Render image1 demo: image affine transformations with bilinear filtering.
///
/// Matches C++ `image1.cpp`:
/// - Procedural source image rotated/scaled through an ellipse clip
/// - SpanImageFilterRgbaBilinearClip for smooth bilinear interpolation
/// - Two transform matrices: one for the ellipse shape, one for the image sampling
///
/// params: [angle_deg, scale]
pub fn image1(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_deg = params.get(0).copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.1);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);

    // Clear to white
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        rb.clear(&Rgba8::new(255, 255, 255, 255));
    }

    // Load the original AGG spheres.bmp (320x300)
    let (img_w, img_h, mut img_data) = load_spheres_image();
    let img_stride = (img_w * 4) as i32;
    let mut img_ra = RowAccessor::new();
    unsafe { img_ra.attach(img_data.as_mut_ptr(), img_w, img_h, img_stride) };

    let iw = width as f64;
    let ih = height as f64;
    let angle_rad = angle_deg * std::f64::consts::PI / 180.0;

    // Source (ellipse) transform — matching C++ image1.cpp src_mtx
    let mut src_mtx = TransAffine::new();
    src_mtx.multiply(&TransAffine::new_translation(-iw / 2.0 - 10.0, -ih / 2.0 - 20.0 - 10.0));
    src_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    src_mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    src_mtx.multiply(&TransAffine::new_translation(iw / 2.0, ih / 2.0 + 20.0));

    // Image transform — matching C++ image1.cpp img_mtx (inverted for sampling)
    let mut img_mtx = TransAffine::new();
    img_mtx.multiply(&TransAffine::new_translation(-iw / 2.0 + 10.0, -ih / 2.0 + 20.0 + 10.0));
    img_mtx.multiply(&TransAffine::new_rotation(angle_rad));
    img_mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    img_mtx.multiply(&TransAffine::new_translation(iw / 2.0, ih / 2.0 + 20.0));
    img_mtx.invert();

    // Span interpolator with the inverted image transform
    let mut interpolator = SpanInterpolatorLinear::new(img_mtx);

    // Bilinear filter with semi-transparent green background (matching C++ rgba_pre(0, 0.4, 0, 0.5))
    let mut sg = SpanImageFilterRgbaBilinearClip::new(
        &img_ra,
        Rgba8::new(0, 102, 0, 128),
        &mut interpolator,
    );

    // Rasterize an ellipse clipping region — matching C++ image1.cpp
    let r = iw.min(ih - 60.0);
    let mut ell = Ellipse::new(
        iw / 2.0 + 10.0,
        ih / 2.0 + 20.0 + 10.0,
        r / 2.0 + 16.0,
        r / 2.0 + 16.0,
        200,
        false,
    );
    let mut tr = ConvTransform::new(&mut ell, src_mtx);

    let mut ras = RasterizerScanlineAa::new();
    ras.clip_box(0.0, 0.0, iw, ih);
    ras.add_path(&mut tr, 0);

    let mut sl = ScanlineU8::new();
    let mut alloc = SpanAllocator::<Rgba8>::new();

    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
    }

    // Render AGG slider controls on canvas — matching C++ image1.cpp layout
    {
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);

        let mut s_angle = SliderCtrl::new(5.0, 5.0, 300.0, 12.0);
        s_angle.label("Angle=%3.2f");
        s_angle.range(-180.0, 180.0);
        s_angle.set_value(angle_deg);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_angle);

        let mut s_scale = SliderCtrl::new(5.0, 5.0 + 15.0, 300.0, 12.0 + 15.0);
        s_scale.label("Scale=%3.2f");
        s_scale.range(0.1, 5.0);
        s_scale.set_value(scale);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_scale);
    }

    buf
}

// ============================================================================
// Gradient Focal — radial gradient with movable focal point
// ============================================================================

/// Render a radial gradient with movable focal point, matching C++ gradient_focal.cpp.
///
/// params[0] = focal_x (default width/2)
/// params[1] = focal_y (default height/2)
/// params[2] = gamma (default 1.0)
pub fn gradient_focal(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let focal_x = params.get(0).copied().unwrap_or(cx);
    let focal_y = params.get(1).copied().unwrap_or(cy);
    let _gamma = params.get(2).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    // Build gradient LUT with 4 color stops, matching C++ gradient_focal.cpp
    let mut lut = GradientLut::new(1024);
    lut.add_color(0.0, Rgba8::new(0, 255, 0, 255));      // Green
    lut.add_color(0.2, Rgba8::new(120, 0, 0, 255));       // Dark red
    lut.add_color(0.7, Rgba8::new(120, 120, 0, 255));     // Yellow-brown
    lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));       // Blue
    lut.build_lut();

    // Gradient setup: radial focus with reflect adaptor
    let r = 100.0;
    let fx = focal_x - cx;
    let fy = focal_y - cy;
    let grad_func = GradientRadialFocus::new(r, fx, fy);
    let grad_adaptor = GradientReflectAdaptor::new(grad_func);

    // Transform: translate to center, invert for sampling
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(cx, cy));
    mtx.invert();

    let interp = SpanInterpolatorLinear::new(mtx);
    let mut grad = SpanGradient::new(interp, grad_adaptor, &lut, 0.0, r);

    // Full-screen rectangle
    let mut path = PathStorage::new();
    path.move_to(0.0, 0.0);
    path.line_to(w, 0.0);
    path.line_to(w, h);
    path.line_to(0.0, h);
    path.close_polygon(0);
    ras.reset();
    ras.add_path(&mut path, 0);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut grad);

    // Draw gradient boundary circle (white, stroked)
    {
        let ell = Ellipse::new(cx, cy, r, r, 64, false);
        let mut stroke = ConvStroke::new(ell);
        stroke.set_width(2.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 200));
    }

    // Draw focal point marker
    {
        let mut ell = Ellipse::new(focal_x, focal_y, 4.0, 4.0, 16, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 255));
    }

    // Render AGG control — matching C++ gradient_focal.cpp
    let mut s_gamma = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    s_gamma.label("Gamma = %.3f");
    s_gamma.range(0.5, 2.5);
    s_gamma.set_value(_gamma);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_gamma);

    buf
}

// ============================================================================
// Idea — rotating light bulb icon, matches C++ idea.cpp
// ============================================================================

/// Render the "idea" light bulb icon with rotation.
///
/// params[0] = rotation angle in degrees (default 0)
/// params[1] = even_odd fill (0 or 1, default 0)
/// params[2] = draft mode (0 or 1, default 0)
/// params[3] = roundoff (0 or 1, default 0)
/// params[4] = angle delta (default 0.01, for display)
/// params[5] = rotate enabled (0 or 1, default 0)
pub fn idea(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_deg = params.get(0).copied().unwrap_or(0.0);
    let even_odd = params.get(1).copied().unwrap_or(0.0) > 0.5;
    let _draft = params.get(2).copied().unwrap_or(0.0) > 0.5;
    let roundoff = params.get(3).copied().unwrap_or(0.0) > 0.5;
    let angle_delta = params.get(4).copied().unwrap_or(0.01);
    let rotate = params.get(5).copied().unwrap_or(0.0) > 0.5;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    if even_odd {
        ras.filling_rule(agg_rust::basics::FillingRule::EvenOdd);
    }

    let w = width as f64;
    let h = height as f64;

    // Path attributes for each layer
    struct PathAttr {
        fill: Rgba8,
        stroke: Rgba8,
        stroke_width: f64,
    }

    let attrs = [
        PathAttr { fill: Rgba8::new(255, 255, 0, 255), stroke: Rgba8::new(0, 0, 0, 255), stroke_width: 1.0 },
        PathAttr { fill: Rgba8::new(255, 255, 200, 255), stroke: Rgba8::new(90, 0, 0, 255), stroke_width: 0.7 },
        PathAttr { fill: Rgba8::new(0, 0, 0, 255), stroke: Rgba8::new(0, 0, 0, 255), stroke_width: 0.0 },
    ];

    // Bulb outline
    let poly_bulb: &[(f64, f64)] = &[
        (-29.0, -26.0), (-9.0, -33.0), (0.0, -44.0), (9.0, -33.0), (29.0, -26.0),
        (36.0, -9.0), (17.0, 0.0), (36.0, 9.0), (29.0, 26.0), (9.0, 33.0),
        (0.0, 44.0), (-9.0, 33.0), (-29.0, 26.0), (-36.0, 9.0), (-17.0, 0.0),
        (-36.0, -9.0), (-29.0, -26.0), (-9.0, -33.0), (0.0, -44.0), (9.0, -33.0),
    ];

    // Light beams
    let poly_beam1: &[(f64, f64)] = &[(-48.0, -6.0), (-48.0, 6.0), (-56.0, 6.0), (-56.0, -6.0)];
    let poly_beam2: &[(f64, f64)] = &[(6.0, -48.0), (-6.0, -48.0), (-6.0, -56.0), (6.0, -56.0)];
    let poly_beam3: &[(f64, f64)] = &[(48.0, 6.0), (48.0, -6.0), (56.0, -6.0), (56.0, 6.0)];
    let poly_beam4: &[(f64, f64)] = &[(-6.0, 48.0), (6.0, 48.0), (6.0, 56.0), (-6.0, 56.0)];

    // Figure parts
    let poly_fig1: &[(f64, f64)] = &[(-6.0, -7.0), (-14.0, -10.0), (-14.0, 92.0), (-6.0, 95.0)];
    let poly_fig2: &[(f64, f64)] = &[(6.0, -7.0), (14.0, -10.0), (14.0, 92.0), (6.0, 95.0)];
    let poly_fig3: &[(f64, f64)] = &[(-14.0, 92.0), (-14.0, 100.0), (14.0, 100.0), (14.0, 92.0)];
    let poly_fig4: &[(f64, f64)] = &[(-18.0, 100.0), (-18.0, 108.0), (18.0, 108.0), (18.0, 100.0)];
    let poly_fig5: &[(f64, f64)] = &[(-21.0, 108.0), (-21.0, 113.0), (21.0, 113.0), (21.0, 108.0)];
    let poly_fig6: &[(f64, f64)] = &[(-24.0, 113.0), (-24.0, 118.0), (24.0, 118.0), (24.0, 113.0)];

    // Group paths by attribute: (polygons, attr_index)
    let groups: [(&[&[(f64, f64)]], usize); 3] = [
        (&[poly_bulb], 0),
        (&[poly_beam1, poly_beam2, poly_beam3, poly_beam4], 1),
        (&[poly_fig1, poly_fig2, poly_fig3, poly_fig4, poly_fig5, poly_fig6], 2),
    ];

    // Transform: rotate, scale to fit window, translate to center
    let scale = (w.min(h) / 300.0).min(2.0);
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_rotation(angle_deg * std::f64::consts::PI / 180.0));
    mtx.multiply(&TransAffine::new_scaling_uniform(scale));
    mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0 + 10.0));

    for (polys, attr_idx) in &groups {
        let attr = &attrs[*attr_idx];

        for poly in *polys {
            let mut path = PathStorage::new();
            for (i, (px, py)) in poly.iter().enumerate() {
                let (x, y) = if roundoff {
                    let mut tx = *px;
                    let mut ty = *py;
                    mtx.transform(&mut tx, &mut ty);
                    ((tx + 0.5).floor(), (ty + 0.5).floor())
                } else {
                    (*px, *py)
                };
                if i == 0 { path.move_to(x, y); } else { path.line_to(x, y); }
            }
            path.close_polygon(0);

            // Fill
            ras.reset();
            if roundoff {
                ras.add_path(&mut path, 0);
            } else {
                let mut transformed = ConvTransform::new(&mut path, mtx);
                ras.add_path(&mut transformed, 0);
            }
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &attr.fill);

            // Stroke (if width > 0)
            if attr.stroke_width > 0.01 {
                let mut path2 = PathStorage::new();
                for (i, (px, py)) in poly.iter().enumerate() {
                    if i == 0 { path2.move_to(*px, *py); } else { path2.line_to(*px, *py); }
                }
                path2.close_polygon(0);

                let transformed = ConvTransform::new(&mut path2, mtx);
                let mut stroke = ConvStroke::new(transformed);
                stroke.set_width(attr.stroke_width);
                ras.reset();
                ras.add_path(&mut stroke, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &attr.stroke);
            }
        }
    }

    ras.filling_rule(agg_rust::basics::FillingRule::NonZero);

    // Render AGG controls — matching C++ idea.cpp
    let mut c_rotate = CboxCtrl::new(10.0, 3.0, "Rotate");
    c_rotate.set_status(rotate);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_rotate);

    let mut c_even_odd = CboxCtrl::new(60.0, 3.0, "Even-Odd");
    c_even_odd.set_status(even_odd);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_even_odd);

    let mut c_draft = CboxCtrl::new(130.0, 3.0, "Draft");
    c_draft.set_status(_draft);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_draft);

    let mut c_roundoff = CboxCtrl::new(175.0, 3.0, "Roundoff");
    c_roundoff.set_status(roundoff);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_roundoff);

    let mut s_angle = SliderCtrl::new(10.0, 21.0, 240.0, 27.0);
    s_angle.label("Step=%4.3f degree");
    s_angle.set_value(angle_delta);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_angle);

    buf
}

// ============================================================================
// Graph Test — random graph with various edge rendering modes
// Matches C++ graph_test.cpp (simplified: no arrowheads, requires conv_marker_adaptor)
// ============================================================================

/// Simple portable PRNG matching C's srand(100)/rand() pattern.
struct SimpleRng(u32);
impl SimpleRng {
    fn new(seed: u32) -> Self { SimpleRng(seed) }
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 >> 16) & 0x7FFF
    }
    fn next_f64(&mut self) -> f64 {
        self.next() as f64 / 32767.0
    }
}

struct GraphNode { x: f64, y: f64 }
struct GraphEdge { n1: usize, n2: usize }

/// Generate a random graph with seeded PRNG.
fn generate_graph(num_nodes: usize, num_edges: usize) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut rng = SimpleRng::new(100);
    let nodes: Vec<GraphNode> = (0..num_nodes).map(|_| GraphNode {
        x: rng.next_f64() * 0.75 + 0.2,
        y: rng.next_f64() * 0.85 + 0.1,
    }).collect();

    let mut edges = Vec::with_capacity(num_edges);
    while edges.len() < num_edges {
        let n1 = rng.next() as usize % num_nodes;
        let n2 = rng.next() as usize % num_nodes;
        if n1 != n2 {
            edges.push(GraphEdge { n1, n2 });
        }
    }
    (nodes, edges)
}

/// Render graph_test demo.
///
/// params[0] = edge type (0=solid, 1=bezier, 2=dashed, 3=polygonsAA, 4=polygonsBin)
/// params[1] = stroke width (0-5, default 2)
/// params[2] = draw_nodes (0 or 1, default 1)
/// params[3] = draw_edges (0 or 1, default 1)
/// params[4] = draft mode (0 or 1, default 0)
/// params[5] = translucent (0 or 1, default 0)
pub fn graph_test(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let edge_type = params.get(0).copied().unwrap_or(0.0) as usize;
    let stroke_width = params.get(1).copied().unwrap_or(2.0);
    let draw_nodes = params.get(2).copied().unwrap_or(1.0) > 0.5;
    let draw_edges = params.get(3).copied().unwrap_or(1.0) > 0.5;
    let _draft = params.get(4).copied().unwrap_or(0.0) > 0.5;
    let translucent = params.get(5).copied().unwrap_or(0.0) > 0.5;

    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    // Build gradient LUT for nodes (yellow→blue, 256 colors)
    let mut grad_colors = Vec::with_capacity(256);
    for i in 0..256 {
        let t = i as f64 / 255.0;
        let r = ((1.0 - t) * 255.0) as u32;
        let g = ((1.0 - t) * 255.0) as u32;
        let b = (t * 255.0) as u32;
        let a = ((1.0 - t) * 0.25 * 255.0 + t * 255.0) as u32;
        grad_colors.push(Rgba8::new(r, g, b, a));
    }

    let (nodes, edges) = generate_graph(200, 100);

    // Deterministic random colors for edges
    let mut color_rng = SimpleRng::new(100);
    // Skip the node generation RNG calls to stay in sync
    for _ in 0..(200 * 2 + 100 * 2) { color_rng.next(); }

    let alpha: u32 = if translucent { 80 } else { 255 };

    // Draw edges
    if draw_edges {
        let mut edge_rng = SimpleRng::new(42); // separate seed for edge colors
        for edge in &edges {
            let n1 = &nodes[edge.n1];
            let n2 = &nodes[edge.n2];
            let x1 = n1.x * w;
            let y1 = n1.y * h;
            let x2 = n2.x * w;
            let y2 = n2.y * h;

            let r = edge_rng.next() & 0x7F;
            let g = edge_rng.next() & 0x7F;
            let b = edge_rng.next() & 0x7F;
            let edge_color = Rgba8::new(r, g, b, alpha as u32);

            match edge_type {
                0 => {
                    // Solid lines
                    let mut path = PathStorage::new();
                    path.move_to(x1, y1);
                    path.line_to(x2, y2);
                    let mut stroke = ConvStroke::new(path);
                    stroke.set_width(stroke_width);
                    ras.reset();
                    ras.add_path(&mut stroke, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &edge_color);
                }
                1 => {
                    // Bezier curves
                    let k = 0.5;
                    let curve = Curve4Div::new_with_points(
                        x1, y1,
                        x1 - (y2 - y1) * k, y1 + (x2 - x1) * k,
                        x2 + (y2 - y1) * k, y2 - (x2 - x1) * k,
                        x2, y2,
                    );
                    let mut stroke = ConvStroke::new(curve);
                    stroke.set_width(stroke_width);
                    ras.reset();
                    ras.add_path(&mut stroke, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &edge_color);
                }
                2 => {
                    // Dashed curves
                    let k = 0.5;
                    let curve = Curve4Div::new_with_points(
                        x1, y1,
                        x1 - (y2 - y1) * k, y1 + (x2 - x1) * k,
                        x2 + (y2 - y1) * k, y2 - (x2 - x1) * k,
                        x2, y2,
                    );
                    let mut dash = ConvDash::new(curve);
                    dash.add_dash(6.0, 3.0);
                    let mut stroke = ConvStroke::new(dash);
                    stroke.set_width(stroke_width);
                    ras.reset();
                    ras.add_path(&mut stroke, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &edge_color);
                }
                3 | 4 => {
                    // Polygons (AA for type 3, threshold for type 4)
                    let k = 0.5;
                    let mut curve = Curve4Div::new_with_points(
                        x1, y1,
                        x1 - (y2 - y1) * k, y1 + (x2 - x1) * k,
                        x2 + (y2 - y1) * k, y2 - (x2 - x1) * k,
                        x2, y2,
                    );
                    ras.reset();
                    ras.add_path(&mut curve, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &edge_color);
                }
                _ => {}
            }
        }
    }

    // Draw nodes — gradient-filled circles matching C++ draw_nodes_fine
    if draw_nodes {
        for node in &nodes {
            let nx = node.x * w;
            let ny = node.y * h;
            let node_size = 5.0 * stroke_width;

            // Outer circle with radial gradient
            let grad_func = GradientRadial;
            let mut mtx = TransAffine::new();
            mtx.multiply(&TransAffine::new_scaling_uniform(stroke_width / 2.0));
            mtx.multiply(&TransAffine::new_translation(nx, ny));
            mtx.invert();
            let interp = SpanInterpolatorLinear::new(mtx);

            let mut lut = GradientLut::new(256);
            lut.add_color(0.0, grad_colors[50]);
            lut.add_color(0.5, grad_colors[147]);
            lut.add_color(1.0, grad_colors[255]);
            lut.build_lut();

            let mut grad = SpanGradient::new(interp, grad_func, &lut, 0.0, 10.0);

            let mut ell = Ellipse::new(nx, ny, node_size, node_size, 32, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut grad);

            // Inner filled circle
            let mut ell2 = Ellipse::new(nx, ny, node_size * 0.4, node_size * 0.4, 16, false);
            ras.reset();
            ras.add_path(&mut ell2, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &grad_colors[50]);
        }
    }

    // Render AGG controls — matching C++ graph_test.cpp layout
    let mut r_type = RboxCtrl::new(5.0, 35.0, 110.0, 110.0);
    r_type.text_size(8.0, 4.0);
    r_type.add_item("Solid lines");
    r_type.add_item("Bezier curves");
    r_type.add_item("Dashed curves");
    r_type.add_item("Polygons AA");
    r_type.add_item("Polygons Bin");
    r_type.set_cur_item(edge_type as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_type);

    let mut s_width = SliderCtrl::new(190.0, 8.0, 390.0, 15.0);
    s_width.label("Width=%1.2f");
    s_width.range(0.0, 5.0);
    s_width.num_steps(20);
    s_width.set_value(stroke_width);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_width);

    let mut c_draw_nodes = CboxCtrl::new(398.0, 21.0, "Draw Nodes");
    c_draw_nodes.text_size(8.0, 4.0);
    c_draw_nodes.set_status(draw_nodes);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_draw_nodes);

    let mut c_draw_edges = CboxCtrl::new(488.0, 21.0, "Draw Edges");
    c_draw_edges.text_size(8.0, 4.0);
    c_draw_edges.set_status(draw_edges);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_draw_edges);

    let mut c_draft = CboxCtrl::new(488.0, 6.0, "Draft Mode");
    c_draft.text_size(8.0, 4.0);
    c_draft.set_status(_draft);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_draft);

    let mut c_translucent = CboxCtrl::new(190.0, 21.0, "Translucent Mode");
    c_translucent.set_status(translucent);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_translucent);

    buf
}

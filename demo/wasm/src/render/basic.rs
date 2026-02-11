//! Basic demo render functions: lion, gradients, gouraud, conv_stroke,
//! bezier_div, circles, rounded_rect, aa_demo, gamma_correction, line_thickness,
//! rasterizers, conv_contour, conv_dash, perspective.

use agg_rust::basics::{is_stop, is_vertex, VertexSource, PATH_FLAGS_CW, PATH_FLAGS_CCW};
use agg_rust::bspline::Bspline;
use agg_rust::bounding_rect::bounding_rect;
use agg_rust::color::Rgba8;
use agg_rust::conv_contour::ConvContour;
use agg_rust::ctrl::{render_ctrl, CboxCtrl, GammaCtrl, RboxCtrl, ScaleCtrl, SliderCtrl, SplineCtrl};
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_dash::ConvDash;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gsv_text::GsvText;
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
    GradientConic, GradientDiamond, GradientRadial, GradientReflectAdaptor,
    GradientSqrtXY, GradientX, GradientXY,
    SpanGradient,
};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_bilinear::TransBilinear;
use agg_rust::trans_perspective::TransPerspective;
use super::setup_renderer;

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
// Gradients
// ============================================================================

/// Render gradient sphere — 6 gradient types, matching C++ gradients.cpp.
///
/// params[0] = center_x (default 350, matching C++ `center_x`)
/// params[1] = center_y (default 280, matching C++ `center_y`)
/// params[2] = angle in radians (default 0)
/// params[3] = scale (default 1.0)
/// params[4] = gradient type (0=radial, 1=diamond, 2=linear, 3=xy, 4=sqrt_xy, 5=conic)
/// params[5] = scale_x (default 1.0)
/// params[6] = scale_y (default 1.0)
/// params[7..10] = gamma spline values (kx1, ky1, kx2, ky2), default 1.0 each
/// params[11..22] = spline_r points x0,y0..x5,y5 (defaults to C++ ramp)
/// params[23..34] = spline_g points x0,y0..x5,y5 (defaults to C++ ramp)
/// params[35..46] = spline_b points x0,y0..x5,y5 (defaults to C++ ramp)
/// params[47..58] = spline_a points x0,y0..x5,y5 (defaults to C++ alpha ramp)
pub fn gradients(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    struct ColorFunctionProfile {
        colors: [Rgba8; 256],
        profile: [u8; 256],
    }

    impl agg_rust::gradient_lut::ColorFunction for ColorFunctionProfile {
        type Color = Rgba8;

        fn size(&self) -> usize {
            256
        }

        fn get(&self, index: usize) -> Rgba8 {
            self.colors[self.profile[index] as usize]
        }
    }

    const CENTER_X: f64 = 350.0;
    const CENTER_Y: f64 = 280.0;
    const INI_SCALE: f64 = 1.0;
    const SPHERE_RADIUS: f64 = 110.0;
    const SPLINE_R_BASE: usize = 11;
    const SPLINE_G_BASE: usize = 23;
    const SPLINE_B_BASE: usize = 35;
    const SPLINE_A_BASE: usize = 47;

    let read_spline_point = |base: usize, idx: usize, default_x: f64, default_y: f64| -> (f64, f64) {
        let x = params
            .get(base + idx * 2)
            .copied()
            .unwrap_or(default_x)
            .clamp(0.0, 1.0);
        let y = params
            .get(base + idx * 2 + 1)
            .copied()
            .unwrap_or(default_y)
            .clamp(0.0, 1.0);
        (x, y)
    };

    let cx = params.get(0).copied().unwrap_or(CENTER_X);
    let cy = params.get(1).copied().unwrap_or(CENTER_Y);
    let angle = params.get(2).copied().unwrap_or(0.0);
    let scale = params.get(3).copied().unwrap_or(1.0).max(0.01);
    let grad_type = params.get(4).copied().unwrap_or(0.0) as i32;
    let scale_x = params.get(5).copied().unwrap_or(1.0).max(0.01);
    let scale_y = params.get(6).copied().unwrap_or(1.0).max(0.01);
    let gamma_kx1 = params.get(7).copied().unwrap_or(1.0);
    let gamma_ky1 = params.get(8).copied().unwrap_or(1.0);
    let gamma_kx2 = params.get(9).copied().unwrap_or(1.0);
    let gamma_ky2 = params.get(10).copied().unwrap_or(1.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(0, 0, 0, 255)); // Black background, matching C++

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut alloc: SpanAllocator<Rgba8> = SpanAllocator::new();

    // Full C++ controls setup: gamma control, 4 spline controls, and gradient type rbox.
    let mut gamma_ctrl = GammaCtrl::new(10.0, 10.0, 200.0, 165.0);
    gamma_ctrl.border_width(2.0, 2.0);
    gamma_ctrl.text_size(8.0, 0.0);
    gamma_ctrl.set_values(gamma_kx1, gamma_ky1, gamma_kx2, gamma_ky2);

    let mut spline_r = SplineCtrl::new(210.0, 10.0, 460.0, 45.0, 6);
    let mut spline_g = SplineCtrl::new(210.0, 50.0, 460.0, 85.0, 6);
    let mut spline_b = SplineCtrl::new(210.0, 90.0, 460.0, 125.0, 6);
    let mut spline_a = SplineCtrl::new(210.0, 130.0, 460.0, 165.0, 6);
    spline_r.background_color(Rgba8::new(255, 204, 204, 255));
    spline_g.background_color(Rgba8::new(204, 255, 204, 255));
    spline_b.background_color(Rgba8::new(204, 204, 255, 255));
    spline_a.background_color(Rgba8::new(255, 255, 255, 255));
    spline_r.border_width(1.0, 2.0);
    spline_g.border_width(1.0, 2.0);
    spline_b.border_width(1.0, 2.0);
    spline_a.border_width(1.0, 2.0);
    for i in 0..6 {
        let x = i as f64 / 5.0;
        let y = 1.0 - x;
        let (xr, yr) = read_spline_point(SPLINE_R_BASE, i, x, y);
        let (xg, yg) = read_spline_point(SPLINE_G_BASE, i, x, y);
        let (xb, yb) = read_spline_point(SPLINE_B_BASE, i, x, y);
        let (xa, ya) = read_spline_point(SPLINE_A_BASE, i, x, 1.0);
        spline_r.point(i, xr, yr);
        spline_g.point(i, xg, yg);
        spline_b.point(i, xb, yb);
        spline_a.point(i, xa, ya);
    }
    spline_r.update_spline();
    spline_g.update_spline();
    spline_b.update_spline();
    spline_a.update_spline();

    let mut rbox = RboxCtrl::new(10.0, 180.0, 200.0, 300.0);
    rbox.border_width(2.0, 2.0);
    rbox.add_item("Circular");
    rbox.add_item("Diamond");
    rbox.add_item("Linear");
    rbox.add_item("XY");
    rbox.add_item("sqrt(XY)");
    rbox.add_item("Conic");
    rbox.set_cur_item(grad_type.clamp(0, 5));

    render_ctrl(&mut ras, &mut sl, &mut rb, &mut gamma_ctrl);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut spline_r);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut spline_g);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut spline_b);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut spline_a);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut rbox);

    let mut color_profile = [Rgba8::default(); 256];
    let gamma_profile = *gamma_ctrl.gamma();
    for (i, c) in color_profile.iter_mut().enumerate() {
        *c = Rgba8::new(
            (spline_r.spline()[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u32,
            (spline_g.spline()[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u32,
            (spline_b.spline()[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u32,
            (spline_a.spline()[i].clamp(0.0, 1.0) * 255.0 + 0.5) as u32,
        );
    }
    let colors = ColorFunctionProfile {
        colors: color_profile,
        profile: gamma_profile,
    };

    // Shape transform: ellipse(0,0,110,110) translated to C++ default center.
    let mut shape_mtx = TransAffine::new();
    shape_mtx.multiply(&TransAffine::new_scaling(INI_SCALE, INI_SCALE));
    shape_mtx.multiply(&TransAffine::new_rotation(0.0));
    shape_mtx.multiply(&TransAffine::new_translation(CENTER_X, CENTER_Y));

    let mut ell = Ellipse::new(0.0, 0.0, SPHERE_RADIUS, SPHERE_RADIUS, 64, false);
    let mut transformed_ellipse = ConvTransform::new(&mut ell, shape_mtx);
    ras.reset();
    ras.add_path(&mut transformed_ellipse, 0);

    // Gradient transform (inverted) matching C++ transform order.
    let mut gradient_mtx = TransAffine::new();
    gradient_mtx.multiply(&TransAffine::new_scaling(INI_SCALE, INI_SCALE));
    gradient_mtx.multiply(&TransAffine::new_scaling(scale, scale));
    gradient_mtx.multiply(&TransAffine::new_scaling(scale_x, scale_y));
    gradient_mtx.multiply(&TransAffine::new_rotation(angle));
    gradient_mtx.multiply(&TransAffine::new_translation(cx, cy));
    gradient_mtx.invert();

    let d1 = 0.0;
    let d2 = 150.0; // Gradient extent, matching C++

    // Dispatch on gradient type using macro to avoid lifetime issues
    macro_rules! do_render {
        ($gf:expr) => {{
            let interp = SpanInterpolatorLinear::new(gradient_mtx);
            let grad_reflect = GradientReflectAdaptor::new($gf);
            let mut grad = SpanGradient::new(interp, grad_reflect, &colors, d1, d2);
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

#[cfg(test)]
mod gradients_tests {
    use super::gradients;

    fn pixel_at(buf: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (y * width + x) * 4;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    #[test]
    fn gradients_uses_cpp_sphere_radius() {
        // C++ gradients.cpp draws a fixed ellipse radius of 110, not a scaled fullscreen disc.
        let w = 600usize;
        let h = 600usize;
        let cx = 300.0;
        let cy = 300.0;
        let img = gradients(w as u32, h as u32, &[cx, cy, 0.0, 1.0, 0.0, 1.0, 1.0]);

        // Clearly inside the C++ sphere (r=110) should be non-background.
        let inside = pixel_at(&img, w, cx as usize + 80, cy as usize);
        assert_ne!(inside[..3], [0, 0, 0], "inside sphere should not be black");

        // Outside r=110 should be untouched background black.
        let outside = pixel_at(&img, w, 580, 20);
        assert_eq!(outside, [0, 0, 0, 255], "outside sphere should be background");
    }
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
/// params[0] = z_min (default 0.0)
/// params[1] = z_max (default 1.0)
/// params[2] = size (default 0.5)
/// params[3] = selectivity (default 0.5)
/// params[4] = seed (default 1)
pub fn circles(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    #[derive(Clone, Copy)]
    struct ScatterPoint {
        x: f64,
        y: f64,
        z: f64,
        r: f64,
        g: f64,
        b: f64,
    }

    fn msvc_rand15(state: &mut u32) -> u32 {
        // Match MSVC's rand() sequence used by AGG demos.
        *state = state.wrapping_mul(214013).wrapping_add(2531011);
        (*state >> 16) & 0x7FFF
    }

    fn random_dbl(state: &mut u32, start: f64, end: f64) -> f64 {
        let r = msvc_rand15(state) as f64;
        r * (end - start) / 32768.0 + start
    }

    fn build_spline(x: &[f64; 6], y: &[f64; 6]) -> Bspline {
        let mut s = Bspline::new();
        s.init(x, y);
        s
    }

    let mut z_min = params.first().copied().unwrap_or(0.3).clamp(0.0, 1.0);
    let mut z_max = params.get(1).copied().unwrap_or(0.7).clamp(0.0, 1.0);
    let size = params.get(2).copied().unwrap_or(0.5).clamp(0.0, 1.0);
    let selectivity = params.get(3).copied().unwrap_or(0.5).clamp(0.0, 1.0);
    let seed = params.get(4).copied().unwrap_or(1.0) as u32;
    if z_min > z_max {
        std::mem::swap(&mut z_min, &mut z_max);
    }
    let num_points = 10_000usize;

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
    let rx = w / 3.5;
    let ry = h / 3.5;
    let circle_radius = size * 5.0;

    let spline_r_x = [0.0, 0.2, 0.4, 0.910484, 0.957258, 1.0];
    let spline_r_y = [1.0, 0.8, 0.6, 0.066667, 0.169697, 0.6];
    let spline_g_x = [0.0, 0.292244, 0.485655, 0.564859, 0.795607, 1.0];
    let spline_g_y = [0.0, 0.607260, 0.964065, 0.892558, 0.435571, 0.0];
    let spline_b_x = [0.0, 0.055045, 0.143034, 0.433082, 0.764859, 1.0];
    let spline_b_y = [0.385480, 0.128493, 0.021416, 0.271507, 0.713974, 1.0];
    let spline_r = build_spline(&spline_r_x, &spline_r_y);
    let spline_g = build_spline(&spline_g_x, &spline_g_y);
    let spline_b = build_spline(&spline_b_x, &spline_b_y);

    let mut rng = seed;
    let mut points = Vec::with_capacity(num_points);
    for _ in 0..num_points {
        let z = random_dbl(&mut rng, 0.0, 1.0);
        let x = (z * std::f64::consts::PI * 2.0).cos() * rx;
        let y = (z * std::f64::consts::PI * 2.0).sin() * ry;
        let dist = random_dbl(&mut rng, 0.0, rx / 2.0);
        let angle = random_dbl(&mut rng, 0.0, std::f64::consts::PI * 2.0);

        points.push(ScatterPoint {
            x: w / 2.0 + x + angle.cos() * dist,
            y: h / 2.0 + y + angle.sin() * dist,
            z,
            r: spline_r.get(z) * 0.8,
            g: spline_g.get(z) * 0.8,
            b: spline_b.get(z) * 0.8,
        });
    }

    let mut n_drawn = 0u32;
    for p in &points {
        let mut alpha = 1.0;
        if p.z < z_min {
            alpha = 1.0 - (z_min - p.z) * selectivity * 100.0;
        }
        if p.z > z_max {
            alpha = 1.0 - (p.z - z_max) * selectivity * 100.0;
        }
        alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            continue;
        }

        let mut ell = Ellipse::new(p.x, p.y, circle_radius, circle_radius, 8, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(
            &mut ras,
            &mut sl,
            &mut rb,
            &Rgba8::new(
                ((p.r.clamp(0.0, 1.0) * 255.0) + 0.5) as u32,
                ((p.g.clamp(0.0, 1.0) * 255.0) + 0.5) as u32,
                ((p.b.clamp(0.0, 1.0) * 255.0) + 0.5) as u32,
                ((alpha * 255.0) + 0.5) as u32,
            ),
        );
        n_drawn += 1;
    }

    // Render draw count text in the bottom-left, matching C++ circles.cpp.
    let mut txt = GsvText::new();
    txt.size(15.0, 0.0);
    txt.text(&format!("{:08}", n_drawn));
    txt.start_point(10.0, h - 20.0);
    let mut txt_stroke = ConvStroke::new(txt);
    txt_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // On-canvas controls: match C++ circles.cpp layout at the bottom.
    let x1 = 5.0;
    let x2 = w - 5.0;

    let mut s_size = SliderCtrl::new(x1, 35.0, x2, 42.0);
    s_size.label("Size");
    s_size.range(0.0, 1.0);
    s_size.set_value(size);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_size);

    let mut s_sel = SliderCtrl::new(x1, 20.0, x2, 27.0);
    s_sel.label("Selectivity");
    s_sel.range(0.0, 1.0);
    s_sel.set_value(selectivity);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_sel);

    let mut s_z = ScaleCtrl::new(x1, 5.0, x2, 12.0);
    s_z.set_value1(z_min);
    s_z.set_value2(z_max);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_z);

    buf
}

#[cfg(test)]
mod circles_tests {
    use super::circles;

    fn non_white_pixels(buf: &[u8]) -> usize {
        buf.chunks_exact(4)
            .filter(|px| px[0] != 255 || px[1] != 255 || px[2] != 255)
            .count()
    }

    #[test]
    fn circles_uses_z_range_params_not_circle_count() {
        // Regression check: C++ circles.cpp uses z-range controls in [0,1].
        // A full range should still render many scatter points.
        let full = circles(400, 400, &[0.0, 1.0, 0.5, 0.5, 1.0]);
        assert!(
            non_white_pixels(&full) > 1_000,
            "expected substantial scatter rendering for full z-range"
        );
    }

    #[test]
    fn circles_narrow_z_range_draws_fewer_points() {
        let full = circles(400, 400, &[0.0, 1.0, 0.5, 0.5, 1.0]);
        let narrow = circles(400, 400, &[0.45, 0.55, 0.5, 0.5, 1.0]);
        assert!(
            non_white_pixels(&narrow) < non_white_pixels(&full),
            "narrow z-range should reduce drawn pixels"
        );
    }
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
    let vx0 = params.get(0).copied().unwrap_or(57.0);
    let vy0 = params.get(1).copied().unwrap_or(100.0);
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
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 150, 160, 200));

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


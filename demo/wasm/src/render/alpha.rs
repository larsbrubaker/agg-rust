//! Alpha/blending demo render functions: bspline, image_perspective, alpha_mask,
//! alpha_gradient, image_alpha, alpha_mask3, image_transforms, mol_view,
//! image_resample, alpha_mask2.

use agg_rust::bounding_rect::bounding_rect;
use agg_rust::basics::{VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP, PATH_FLAGS_CCW};
use agg_rust::conv_curve::ConvCurve;
use agg_rust::bspline::Bspline;
use agg_rust::color::{Gray8, Rgba8};
use agg_rust::ctrl::{render_ctrl, SliderCtrl, CboxCtrl, RboxCtrl, SplineCtrl};
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLut;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_accessors::ImageAccessorClone;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterLut};
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_gray::PixfmtGray8;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_primitives::RendererPrimitives;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_converter::{SpanConverter, SpanConverterFunction};
use agg_rust::span_gradient::{GradientRadial, SpanGradient};
use agg_rust::span_image_filter_rgba::{
    SpanImageFilterRgbaBilinearClip, SpanImageFilterRgbaNn, SpanImageFilterRgba2x2,
    SpanImageResampleRgbaAffine,
};
use agg_rust::span_interpolator_persp::SpanInterpolatorPerspLerp;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::span_interpolator_trans::SpanInterpolatorTrans;
use agg_rust::trans_affine::TransAffine;
use agg_rust::trans_bilinear::TransBilinear;
use agg_rust::trans_perspective::TransPerspective;
use agg_rust::math::calc_orthogonal;
use agg_rust::math_stroke::{LineCap, LineJoin};
use super::gb_poly::{make_arrows, make_gb_poly, Spiral};
use super::{setup_renderer, load_spheres_image};
use std::sync::OnceLock;

#[cfg(target_arch = "wasm32")]
fn measure_with_ms<T, F: FnOnce() -> T>(f: F) -> (T, f64) {
    // std::time::Instant::now() can trap on wasm in some browser/runtime setups.
    (f(), 0.0)
}

#[cfg(not(target_arch = "wasm32"))]
fn measure_with_ms<T, F: FnOnce() -> T>(f: F) -> (T, f64) {
    let t0 = std::time::Instant::now();
    let out = f();
    (out, t0.elapsed().as_secs_f64() * 1000.0)
}


/// BSpline — 6 draggable control points, B-spline curve through them.
/// Matches C++ bspline.cpp.
pub fn bspline_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    // 6 control points
    let px0 = params.get(0).copied().unwrap_or(100.0);
    let py0 = params.get(1).copied().unwrap_or(h - 100.0);
    let px1 = params.get(2).copied().unwrap_or(w - 100.0);
    let py1 = params.get(3).copied().unwrap_or(h - 100.0);
    let px2 = params.get(4).copied().unwrap_or(w - 100.0);
    let py2 = params.get(5).copied().unwrap_or(100.0);
    let px3 = params.get(6).copied().unwrap_or(100.0);
    let py3 = params.get(7).copied().unwrap_or(100.0);
    let px4 = params.get(8).copied().unwrap_or(w / 2.0);
    let py4 = params.get(9).copied().unwrap_or(h / 2.0);
    let px5 = params.get(10).copied().unwrap_or(w / 2.0);
    let py5 = params.get(11).copied().unwrap_or(h / 3.0);
    let num_points = params.get(12).copied().unwrap_or(20.0);
    let close = params.get(13).copied().unwrap_or(0.0) > 0.5;

    let pts_x = [px0, px1, px2, px3, px4, px5];
    let pts_y = [py0, py1, py2, py3, py4, py5];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let n = 6usize;

    // Create separate x and y bsplines parameterized by t = [0..n-1] or [0..n]
    let t_vals: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let mut sx = Bspline::new();
    let mut sy = Bspline::new();

    if close {
        // For closed curve, duplicate first point at end
        let mut tx = t_vals.clone();
        tx.push(n as f64);
        let mut vx = pts_x.to_vec();
        vx.push(pts_x[0]);
        let mut vy = pts_y.to_vec();
        vy.push(pts_y[0]);
        sx.init(&tx, &vx);
        sy.init(&tx, &vy);
    } else {
        sx.init(&t_vals, &pts_x);
        sy.init(&t_vals, &pts_y);
    }

    // Build path by sampling the spline
    let step = 1.0 / num_points.max(1.0);
    let t_max = if close { n as f64 } else { (n - 1) as f64 };
    let mut path = PathStorage::new();
    let mut first = true;
    let mut t = 0.0;
    while t <= t_max + step * 0.5 {
        let x = sx.get(t);
        let y = sy.get(t);
        if first {
            path.move_to(x, y);
            first = false;
        } else {
            path.line_to(x, y);
        }
        t += step;
    }

    // Stroke the bspline curve
    let mut stroke = ConvStroke::new(&mut path);
    stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // Draw control polygon (lines connecting points)
    let mut poly_path = PathStorage::new();
    poly_path.move_to(pts_x[0], pts_y[0]);
    for i in 1..n {
        poly_path.line_to(pts_x[i], pts_y[i]);
    }
    if close {
        poly_path.close_polygon(0);
    }
    let mut poly_stroke = ConvStroke::new(&mut poly_path);
    poly_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut poly_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(0, 76, 127, 153)); // rgba(0, 0.3, 0.5, 0.6)

    // Draw control points as circles
    for i in 0..n {
        let mut ell = Ellipse::new(pts_x[i], pts_y[i], 5.0, 5.0, 20, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
            &Rgba8::new(0, 76, 127, 153));
    }

    // Render controls
    let mut m_num = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    m_num.label("Number of intermediate Points = %.3f");
    m_num.range(1.0, 40.0);
    m_num.set_value(num_points);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_num);

    let mut m_close = CboxCtrl::new(350.0, 5.0, "Close");
    m_close.set_status(close);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_close);

    buf
}

/// Image perspective — spheres image through quad transform.
/// Matches C++ image_perspective.cpp.
pub fn image_perspective_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    let q0x = params.get(0).copied().unwrap_or(100.0);
    let q0y = params.get(1).copied().unwrap_or(100.0);
    let q1x = params.get(2).copied().unwrap_or(w - 100.0);
    let q1y = params.get(3).copied().unwrap_or(100.0);
    let q2x = params.get(4).copied().unwrap_or(w - 100.0);
    let q2y = params.get(5).copied().unwrap_or(h - 100.0);
    let q3x = params.get(6).copied().unwrap_or(100.0);
    let q3y = params.get(7).copied().unwrap_or(h - 100.0);
    let trans_type = params.get(8).copied().unwrap_or(2.0) as i32; // 0=affine, 1=bilinear, 2=perspective

    let quad = [q0x, q0y, q1x, q1y, q2x, q2y, q3x, q3y];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut sa = SpanAllocator::new();

    // Load source image
    let (img_w, img_h, mut img_data) = load_spheres_image();
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_data.as_mut_ptr(), img_w, img_h, img_stride) };

    let g_x1 = 0.0_f64;
    let g_y1 = 0.0_f64;
    let g_x2 = img_w as f64;
    let g_y2 = img_h as f64;

    // For affine mode, force parallelogram
    let mut quad_adj = quad;
    if trans_type == 0 {
        quad_adj[6] = quad_adj[0] + (quad_adj[4] - quad_adj[2]);
        quad_adj[7] = quad_adj[1] + (quad_adj[5] - quad_adj[3]);
    }

    // Rasterize quad area
    ras.reset();
    ras.move_to_d(quad_adj[0], quad_adj[1]);
    ras.line_to_d(quad_adj[2], quad_adj[3]);
    ras.line_to_d(quad_adj[4], quad_adj[5]);
    ras.line_to_d(quad_adj[6], quad_adj[7]);

    let mut source = ImageAccessorClone::<4>::new(&img_ra);

    // Create filter for bilinear/perspective modes
    let mut filter = ImageFilterLut::new();
    filter.calculate(&ImageFilterBilinear, false);

    match trans_type {
        0 => {
            // Affine parallelogram — use TransAffine
            let mut mtx = TransAffine::new();
            // We need quad→quad but just use the parl_to_parl approach
            let src_parl = [g_x1, g_y1, g_x2, g_y1, g_x2, g_y2];
            let dst_parl = [quad_adj[0], quad_adj[1], quad_adj[2], quad_adj[3], quad_adj[4], quad_adj[5]];
            mtx.parl_to_parl(&dst_parl, &src_parl);
            let mut interp = SpanInterpolatorLinear::new(mtx);
            let mut sg = SpanImageFilterRgbaNn::new(&mut source, &mut interp);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
        }
        1 => {
            // Bilinear
            let tb = TransBilinear::new_quad_to_rect(&quad_adj, g_x1, g_y1, g_x2, g_y2);
            if tb.is_valid() {
                let mut interp = SpanInterpolatorTrans::new(tb);
                let mut sg = SpanImageFilterRgba2x2::new(&mut source, &mut interp, &filter);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
        _ => {
            // Perspective
            let mut tp = TransPerspective::new();
            tp.quad_to_rect(&quad_adj, g_x1, g_y1, g_x2, g_y2);
            if tp.is_valid() {
                let mut interp = SpanInterpolatorTrans::new(tp);
                let mut sg = SpanImageFilterRgba2x2::new(&mut source, &mut interp, &filter);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
    }

    // Draw interactive quad overlay exactly like C++ interactive_polygon:
    // a stroked outline plus vertex circles (no filled quad).
    let mut quad_path = PathStorage::new();
    quad_path.move_to(quad_adj[0], quad_adj[1]);
    quad_path.line_to(quad_adj[2], quad_adj[3]);
    quad_path.line_to(quad_adj[4], quad_adj[5]);
    quad_path.line_to(quad_adj[6], quad_adj[7]);
    quad_path.close_polygon(0);
    ras.reset();
    let mut quad_stroke = ConvStroke::new(&mut quad_path);
    quad_stroke.set_width(1.0);
    ras.add_path(&mut quad_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(0, 76, 127, 153)); // rgba(0, 0.3, 0.5, 0.6)
    for i in 0..4 {
        let vx = quad_adj[i * 2];
        let vy = quad_adj[i * 2 + 1];
        let mut vtx = Ellipse::new(vx, vy, 5.0, 5.0, 32, false);
        ras.reset();
        ras.add_path(&mut vtx, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 76, 127, 153));
    }

    // Render controls
    let mut m_trans = RboxCtrl::new(420.0, 5.0, 590.0, 65.0);
    m_trans.add_item("Affine Parallelogram");
    m_trans.add_item("Bilinear");
    m_trans.add_item("Perspective");
    m_trans.set_cur_item(trans_type);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_trans);

    buf
}

/// Alpha mask — lion with elliptical alpha mask.
/// Ported from C++ alpha_mask.cpp.
pub fn alpha_mask_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_rad = params.first().copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(2).copied().unwrap_or(0.0);
    let skew_y = params.get(3).copied().unwrap_or(0.0);

    let w = width as f64;
    let h = height as f64;

    // C++ alpha_mask.cpp uses std::rand() with no explicit seed.
    // Use MSVC's rand() sequence so the demo visually matches the original on Windows.
    fn msvc_rand(seed: &mut u32) -> u32 {
        *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
        (*seed >> 16) & 0x7fff
    }

    // Generate gray8 alpha mask with 10 random ellipses.
    let mut mask_buf = vec![0u8; (width * height) as usize];
    {
        let mut mask_ra = RowAccessor::new();
        unsafe { mask_ra.attach(mask_buf.as_mut_ptr(), width, height, width as i32) };
        let mask_pf = PixfmtGray8::new(&mut mask_ra);
        let mut mask_rb = RendererBase::new(mask_pf);
        let mut mask_ras = RasterizerScanlineAa::new();
        let mut mask_sl = ScanlineU8::new();
        mask_rb.clear(&Gray8::new(0, 255));

        let mut seed = 1u32;
        for _ in 0..10 {
            let cx = (msvc_rand(&mut seed) % width.max(1)) as f64;
            let cy = (msvc_rand(&mut seed) % height.max(1)) as f64;
            let rx = (msvc_rand(&mut seed) % 100 + 20) as f64;
            let ry = (msvc_rand(&mut seed) % 100 + 20) as f64;
            let gray = (msvc_rand(&mut seed) & 0xFF) as u32;
            let alpha = (msvc_rand(&mut seed) & 0xFF) as u32;

            let mut ell = Ellipse::new(cx, cy, rx, ry, 100, false);
            mask_ras.reset();
            mask_ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(
                &mut mask_ras,
                &mut mask_sl,
                &mut mask_rb,
                &Gray8::new(gray, alpha),
            );
        }
    }

    // Render lion to a transparent temporary RGBA buffer.
    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let path_ids: Vec<u32> = path_idx.iter().map(|&i| i as u32).collect();
    let bbox = bounding_rect(&mut path, &path_ids, 0, npaths).unwrap_or(
        agg_rust::basics::RectD::new(0.0, 0.0, 250.0, 400.0),
    );
    let base_dx = (bbox.x2 - bbox.x1) / 2.0;
    let base_dy = (bbox.y2 - bbox.y1) / 2.0;

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling(scale, scale));
    mtx.multiply(&TransAffine::new_rotation(angle_rad + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_skewing(skew_x / 1000.0, skew_y / 1000.0));
    mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));

    let mut lion_buf = vec![0u8; (width * height * 4) as usize];
    {
        let mut lion_ra = RowAccessor::new();
        unsafe { lion_ra.attach(lion_buf.as_mut_ptr(), width, height, (width * 4) as i32) };
        let lion_pf = PixfmtRgba32::new(&mut lion_ra);
        let mut lion_rb = RendererBase::new(lion_pf);
        lion_rb.clear(&Rgba8::new(0, 0, 0, 0));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut transformed = ConvTransform::new(&mut path, mtx);
        for i in 0..npaths {
            ras.reset();
            ras.add_path(&mut transformed, path_idx[i] as u32);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut lion_rb, &colors[i]);
        }
    }

    // Composite the masked lion over a white background.
    let mut buf = vec![255u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let i = y * width as usize + x;
            let pi = i * 4;
            let sr = lion_buf[pi] as u32;
            let sg = lion_buf[pi + 1] as u32;
            let sb = lion_buf[pi + 2] as u32;
            let sa = lion_buf[pi + 3] as u32;
            if sa == 0 {
                continue;
            }
            let m = mask_buf[i] as u32;
            let a = (255 + sa * m) >> 8;
            if a == 0 {
                continue;
            }
            let inv_a = 255 - a;
            // lion_buf RGB is already premultiplied by lion alpha; apply only mask.
            let src_r = (255 + sr * m) >> 8;
            let src_g = (255 + sg * m) >> 8;
            let src_b = (255 + sb * m) >> 8;
            buf[pi] = (src_r + ((255 + buf[pi] as u32 * inv_a) >> 8)).min(255) as u8;
            buf[pi + 1] = (src_g + ((255 + buf[pi + 1] as u32 * inv_a) >> 8)).min(255) as u8;
            buf[pi + 2] = (src_b + ((255 + buf[pi + 2] as u32 * inv_a) >> 8)).min(255) as u8;
            buf[pi + 3] = 255;
        }
    }

    buf
}

/// Alpha gradient — color gradient with alpha modulation.
/// Simplified from C++ alpha_gradient.cpp.
/// Params: [x0,y0, x1,y1, x2,y2, a0,a1,a2,a3,a4,a5]
pub fn alpha_gradient(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    let x0 = params.get(0).copied().unwrap_or(257.0);
    let y0 = params.get(1).copied().unwrap_or(60.0);
    let x1 = params.get(2).copied().unwrap_or(369.0);
    let y1 = params.get(3).copied().unwrap_or(170.0);
    let x2 = params.get(4).copied().unwrap_or(143.0);
    let y2 = params.get(5).copied().unwrap_or(310.0);

    let a: Vec<f64> = (0..6)
        .map(|i| params.get(6 + i).copied().unwrap_or(i as f64 / 5.0))
        .collect();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Background: 100 random semi-transparent ellipses
    let mut seed = 1234u32;
    let rng = |s: &mut u32| -> f64 {
        *s = s.wrapping_mul(1103515245).wrapping_add(12345);
        ((*s >> 16) & 0x7FFF) as f64 / 32767.0
    };

    for _ in 0..100 {
        let ex = rng(&mut seed) * w;
        let ey = rng(&mut seed) * h;
        let erx = rng(&mut seed) * 60.0 + 5.0;
        let ery = rng(&mut seed) * 60.0 + 5.0;
        let mut ell = Ellipse::new(ex, ey, erx, ery, 50, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        let r = (rng(&mut seed) * 127.0 + 127.0) as u32;
        let g = (rng(&mut seed) * 127.0 + 127.0) as u32;
        let b = (rng(&mut seed) * 127.0 + 127.0) as u32;
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(r, g, b, 100));
    }

    // C++ uses spline_ctrl with fixed X points and draggable Y values.
    // Build the same control so the rendered widget and alpha LUT match.
    let mut alpha_ctrl = SplineCtrl::new(2.0, 2.0, 200.0, 30.0, 6);
    for i in 0..6 {
        alpha_ctrl.point(i, i as f64 / 5.0, a[i]);
    }
    alpha_ctrl.update_spline();
    let alpha_lut = alpha_ctrl.spline8().to_vec();

    // Build color gradient LUT: teal -> yellow-green -> dark red
    let mut color_lut = GradientLut::new_default();
    color_lut.add_color(0.0, Rgba8::new(0, 48, 48, 255));
    color_lut.add_color(0.5, Rgba8::new(178, 178, 48, 255));
    color_lut.add_color(1.0, Rgba8::new(79, 0, 0, 255));
    color_lut.build_lut();

    // Color gradient: radial, transformed
    let mut grad_mtx = TransAffine::new();
    grad_mtx.multiply(&TransAffine::new_scaling(0.75, 1.2));
    grad_mtx.multiply(&TransAffine::new_rotation(-std::f64::consts::PI / 3.0));
    grad_mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));
    grad_mtx.invert();

    let interp = SpanInterpolatorLinear::new(grad_mtx);
    let grad_func = GradientRadial;
    let sg = SpanGradient::new(interp, grad_func, &color_lut, 0.0, 150.0);

    // Alpha modulation via SpanConverter
    let mut alpha_mtx = TransAffine::new();
    let parl = [x0, y0, x1, y1, x2, y2];
    let rect = [-100.0, -100.0, 100.0, -100.0, 100.0, 100.0];
    alpha_mtx.parl_to_parl(&parl, &rect);

    struct AlphaConverter {
        mtx: TransAffine,
        lut: Vec<u8>,
    }
    impl SpanConverterFunction for AlphaConverter {
        type Color = Rgba8;
        fn convert(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
            for i in 0..len as usize {
                let mut px = (x + i as i32) as f64;
                let mut py = y as f64;
                self.mtx.transform(&mut px, &mut py);
                let d = (px.abs() * py.abs()).sqrt();
                let idx = ((d / 100.0).clamp(0.0, 1.0) * 255.0) as usize;
                let alpha = self.lut[idx.min(255)];
                span[i].a = ((span[i].a as u32 * alpha as u32) / 255) as u8;
            }
        }
    }

    let alpha_conv = AlphaConverter { mtx: alpha_mtx, lut: alpha_lut };
    let mut pipeline = SpanConverter::new(sg, alpha_conv);

    let mut ell = Ellipse::new(w / 2.0, h / 2.0, 150.0, 150.0, 100, false);
    ras.reset();
    ras.add_path(&mut ell, 0);
    let mut sa = SpanAllocator::new();
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut pipeline);

    // Draw control points and full parallelogram (matches C++ interactive polygon)
    let x3 = x0 + x2 - x1;
    let y3 = y0 + y2 - y1;
    let mut para_path = PathStorage::new();
    para_path.move_to(x0, y0);
    para_path.line_to(x1, y1);
    para_path.line_to(x2, y2);
    para_path.line_to(x3, y3);
    para_path.close_polygon(0);
    let mut stroke = ConvStroke::new(&mut para_path);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    for &(vx, vy) in &[(x0, y0), (x1, y1), (x2, y2)] {
        let mut c = Ellipse::new(vx, vy, 5.0, 5.0, 20, false);
        ras.reset();
        ras.add_path(&mut c, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 200, 200, 255));
    }

    render_ctrl(&mut ras, &mut sl, &mut rb, &mut alpha_ctrl);

    buf
}

/// Image alpha — image with brightness-to-alpha function.
/// Simplified from C++ image_alpha.cpp.
/// Params: [a0,a1,a2,a3,a4,a5]
pub fn image_alpha(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    let a: Vec<f64> = (0..6).map(|i| params.get(i).copied().unwrap_or(
        match i { 3 => 0.5, 4 => 0.5, _ => 1.0 }
    )).collect();

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Background: 50 random colored ellipses
    let mut seed = 5678u32;
    let rng = |s: &mut u32| -> f64 {
        *s = s.wrapping_mul(1103515245).wrapping_add(12345);
        ((*s >> 16) & 0x7FFF) as f64 / 32767.0
    };

    for _ in 0..50 {
        let ex = rng(&mut seed) * w;
        let ey = rng(&mut seed) * h;
        let erx = rng(&mut seed) * 60.0 + 5.0;
        let ery = rng(&mut seed) * 60.0 + 5.0;
        let mut ell = Ellipse::new(ex, ey, erx, ery, 50, false);
        ras.reset();
        ras.add_path(&mut ell, 0);
        let r = (rng(&mut seed) * 255.0) as u32;
        let g = (rng(&mut seed) * 255.0) as u32;
        let b = (rng(&mut seed) * 255.0) as u32;
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(r, g, b, 255));
    }

    // Build brightness-to-alpha LUT from 6 spline values
    let alpha_lut: Vec<u8> = {
        let t_vals: Vec<f64> = (0..6).map(|i| i as f64 / 5.0).collect();
        let mut spline = Bspline::new();
        spline.init(&t_vals, &a);
        (0..768).map(|i| {
            let t = i as f64 / 768.0;
            let v = spline.get(t).clamp(0.0, 1.0);
            (v * 255.0) as u8
        }).collect()
    };

    // Load source image
    let (img_w, img_h, mut img_data) = load_spheres_image();
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_data.as_mut_ptr(), img_w, img_h, img_stride) };

    // Transform: center, rotate 10 deg, center back
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-(img_w as f64) / 2.0, -(img_h as f64) / 2.0));
    mtx.multiply(&TransAffine::new_rotation(10.0_f64.to_radians()));
    mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));
    let inv_mtx = {
        let mut m = mtx;
        m.invert();
        m
    };

    let erx = w / 1.9;
    let ery = h / 1.9;
    let mut ell = Ellipse::new(w / 2.0, h / 2.0, erx, ery, 200, false);

    let mut interp = SpanInterpolatorLinear::new(inv_mtx);
    let bg = Rgba8::new(0, 0, 0, 0);
    let sg_img = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg, &mut interp);

    struct BrightnessAlpha {
        lut: Vec<u8>,
    }
    impl SpanConverterFunction for BrightnessAlpha {
        type Color = Rgba8;
        fn convert(&mut self, span: &mut [Rgba8], _x: i32, _y: i32, len: u32) {
            for pixel in span.iter_mut().take(len as usize) {
                let brightness = pixel.r as usize + pixel.g as usize + pixel.b as usize;
                pixel.a = self.lut[brightness.min(767)];
            }
        }
    }

    let alpha_conv = BrightnessAlpha { lut: alpha_lut };
    let mut pipeline = SpanConverter::new(sg_img, alpha_conv);

    ras.reset();
    ras.add_path(&mut ell, 0);
    let mut sa = SpanAllocator::new();
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut pipeline);

    // Draw alpha curve visualization
    let ctrl_w = 200.0;
    let ctrl_h = 30.0;
    let ctrl_x = 2.0;
    let ctrl_y = 2.0;
    let mut ctrl_bg = PathStorage::new();
    ctrl_bg.move_to(ctrl_x, ctrl_y);
    ctrl_bg.line_to(ctrl_x + ctrl_w, ctrl_y);
    ctrl_bg.line_to(ctrl_x + ctrl_w, ctrl_y + ctrl_h);
    ctrl_bg.line_to(ctrl_x, ctrl_y + ctrl_h);
    ctrl_bg.close_polygon(0);
    ras.reset();
    ras.add_path(&mut ctrl_bg, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 200));

    let mut spline_path = PathStorage::new();
    let t_vals: Vec<f64> = (0..6).map(|j| j as f64 / 5.0).collect();
    let mut sp = Bspline::new();
    sp.init(&t_vals, &a);
    for i in 0..=100 {
        let t = i as f64 / 100.0;
        let v = sp.get(t).clamp(0.0, 1.0);
        let px = ctrl_x + t * ctrl_w;
        let py = ctrl_y + ctrl_h - v * ctrl_h;
        if i == 0 { spline_path.move_to(px, py); } else { spline_path.line_to(px, py); }
    }
    let mut sp_stroke = ConvStroke::new(&mut spline_path);
    sp_stroke.set_width(1.5);
    ras.reset();
    ras.add_path(&mut sp_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 200, 255));

    buf
}

/// Alpha mask 3 — alpha mask polygon clipping (AND/SUB).
/// Ported from C++ alpha_mask3.cpp.
/// Params: [scenario, operation, mouse_x, mouse_y]
pub fn alpha_mask3(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    fn generate_alpha_mask<V: VertexSource>(
        width: u32,
        height: u32,
        operation: i32,
        path: &mut V,
    ) -> Vec<u8> {
        let mut mask_buf = vec![0u8; (width * height) as usize];
        let mut mask_ra = RowAccessor::new();
        unsafe { mask_ra.attach(mask_buf.as_mut_ptr(), width, height, width as i32) };
        let mask_pf = PixfmtGray8::new(&mut mask_ra);
        let mut mask_rb = RendererBase::new(mask_pf);
        let mut mask_ras = RasterizerScanlineAa::new();
        let mut mask_sl = ScanlineU8::new();

        if operation == 0 {
            mask_rb.clear(&Gray8::new(0, 255));
            mask_ras.reset();
            mask_ras.add_path(path, 0);
            render_scanlines_aa_solid(
                &mut mask_ras,
                &mut mask_sl,
                &mut mask_rb,
                &Gray8::new(255, 255),
            );
        } else {
            mask_rb.clear(&Gray8::new(255, 255));
            mask_ras.reset();
            mask_ras.add_path(path, 0);
            render_scanlines_aa_solid(
                &mut mask_ras,
                &mut mask_sl,
                &mut mask_rb,
                &Gray8::new(0, 255),
            );
        }

        mask_buf
    }

    fn render_with_alpha_mask<V: VertexSource>(
        width: u32,
        height: u32,
        path: &mut V,
        mask: &[u8],
        color: Rgba8,
        dst: &mut [u8],
    ) {
        let mut temp_buf = vec![0u8; (width * height * 4) as usize];
        let mut temp_ra = RowAccessor::new();
        unsafe { temp_ra.attach(temp_buf.as_mut_ptr(), width, height, (width * 4) as i32) };
        let temp_pf = PixfmtRgba32::new(&mut temp_ra);
        let mut temp_rb = RendererBase::new(temp_pf);
        temp_rb.clear(&Rgba8::new(0, 0, 0, 0));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        ras.reset();
        ras.add_path(path, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut temp_rb, &color);

        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = y * width as usize + x;
                let si = i * 4;
                let mask_val = mask[i] as u32;
                let src_a = temp_buf[si + 3] as u32;
                let a = (255 + src_a * mask_val) >> 8;
                if a == 0 {
                    continue;
                }
                let inv = 255 - a;
                let sr = temp_buf[si] as u32;
                let sg = temp_buf[si + 1] as u32;
                let sb = temp_buf[si + 2] as u32;
                // temp_buf RGB is premultiplied by src_a; modulate by mask only.
                let src_r = (255 + sr * mask_val) >> 8;
                let src_g = (255 + sg * mask_val) >> 8;
                let src_b = (255 + sb * mask_val) >> 8;
                dst[si] = (src_r + ((255 + dst[si] as u32 * inv) >> 8)).min(255) as u8;
                dst[si + 1] = (src_g + ((255 + dst[si + 1] as u32 * inv) >> 8)).min(255) as u8;
                dst[si + 2] = (src_b + ((255 + dst[si + 2] as u32 * inv) >> 8)).min(255) as u8;
                dst[si + 3] = (a + ((255 + dst[si + 3] as u32 * inv) >> 8)).min(255) as u8;
            }
        }
    }

    let w = width as f64;
    let h = height as f64;
    let scenario = (params.first().copied().unwrap_or(3.0) as i32).clamp(0, 4) as usize;
    let operation = if (params.get(1).copied().unwrap_or(0.0) as i32) == 0 { 0 } else { 1 };
    let mx = params.get(2).copied().unwrap_or(w / 2.0);
    let my = params.get(3).copied().unwrap_or(h / 2.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    macro_rules! draw_text {
        ($x:expr, $y:expr, $text:expr) => {{
            let mut txt = GsvText::new();
            txt.size(10.0, 0.0);
            txt.start_point($x, $y);
            txt.text($text);
            let mut txt_stroke = ConvStroke::new(&mut txt);
            txt_stroke.set_width(1.5);
            txt_stroke.set_line_cap(LineCap::Round);
            ras.reset();
            ras.add_path(&mut txt_stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }};
    }

    match scenario {
        0 => {
            let mut ps1 = PathStorage::new();
            let mut ps2 = PathStorage::new();

            let x = mx - w / 2.0 + 100.0;
            let y = my - h / 2.0 + 100.0;
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
            ps1.line_to(x + 268.0, y + 309.0);
            ps1.line_to(x + 325.0, y + 261.0);

            ps1.move_to(x + 259.0, y + 259.0);
            ps1.line_to(x + 273.0, y + 288.0);
            ps1.line_to(x + 298.0, y + 266.0);

            ps2.move_to(132.0, 177.0);
            ps2.line_to(573.0, 363.0);
            ps2.line_to(451.0, 390.0);
            ps2.line_to(454.0, 474.0);

            ras.reset();
            ras.add_path(&mut ps1, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 26));
            ras.reset();
            ras.add_path(&mut ps2, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 153, 0, 26));

            let (mask, gen_ms) = measure_with_ms(|| generate_alpha_mask(width, height, operation, &mut ps1));
            draw_text!(250.0, 20.0, &format!("Generate AlphaMask: {:.3}ms", gen_ms));

            let (_, render_ms) = measure_with_ms(|| {
                render_with_alpha_mask(
                    width,
                    height,
                    &mut ps2,
                    &mask,
                    Rgba8::new(127, 0, 0, 127),
                    &mut buf,
                );
            });
            draw_text!(250.0, 5.0, &format!("Render with AlphaMask: {:.3}ms", render_ms));
        }
        1 => {
            let mut ps1 = PathStorage::new();
            let mut ps2 = PathStorage::new();

            let x = mx - w / 2.0 + 100.0;
            let y = my - h / 2.0 + 100.0;
            ps1.move_to(x + 140.0, y + 145.0);
            ps1.line_to(x + 225.0, y + 44.0);
            ps1.line_to(x + 296.0, y + 219.0);
            ps1.close_polygon(0);

            ps1.line_to(x + 226.0, y + 289.0);
            ps1.line_to(x + 82.0, y + 292.0);

            ps1.move_to(x + 170.0, y + 222.0);
            ps1.line_to(x + 215.0, y + 331.0);
            ps1.line_to(x + 313.0, y + 249.0);
            ps1.close_polygon(PATH_FLAGS_CCW);

            ps2.move_to(132.0, 177.0);
            ps2.line_to(573.0, 363.0);
            ps2.line_to(451.0, 390.0);
            ps2.line_to(454.0, 474.0);
            ps2.close_polygon(0);

            let mut stroke = ConvStroke::new(&mut ps2);
            stroke.set_width(10.0);

            ras.reset();
            ras.add_path(&mut ps1, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 26));
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 153, 0, 26));

            let (mask, gen_ms) = measure_with_ms(|| generate_alpha_mask(width, height, operation, &mut ps1));
            draw_text!(250.0, 20.0, &format!("Generate AlphaMask: {:.3}ms", gen_ms));

            let mut ps2_for_masked = PathStorage::new();
            ps2_for_masked.move_to(132.0, 177.0);
            ps2_for_masked.line_to(573.0, 363.0);
            ps2_for_masked.line_to(451.0, 390.0);
            ps2_for_masked.line_to(454.0, 474.0);
            ps2_for_masked.close_polygon(0);
            let mut stroke_for_masked = ConvStroke::new(&mut ps2_for_masked);
            stroke_for_masked.set_width(10.0);

            let (_, render_ms) = measure_with_ms(|| {
                render_with_alpha_mask(
                    width,
                    height,
                    &mut stroke_for_masked,
                    &mask,
                    Rgba8::new(127, 0, 0, 127),
                    &mut buf,
                );
            });
            draw_text!(250.0, 5.0, &format!("Render with AlphaMask: {:.3}ms", render_ms));
        }
        2 => {
            let mut gb_poly = PathStorage::new();
            let mut arrows = PathStorage::new();
            make_gb_poly(&mut gb_poly);
            make_arrows(&mut arrows);

            let mut mtx1 = TransAffine::new();
            mtx1.multiply(&TransAffine::new_translation(-1150.0, -1150.0));
            mtx1.multiply(&TransAffine::new_scaling(2.0, 2.0));

            let mut mtx2 = mtx1;
            mtx2.multiply(&TransAffine::new_translation(mx - w / 2.0, my - h / 2.0));

            {
                let mut trans_gb = ConvTransform::new(&mut gb_poly, mtx1);
                ras.reset();
                ras.add_path(&mut trans_gb, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(127, 127, 0, 26));
            }
            {
                let mut trans_gb = ConvTransform::new(&mut gb_poly, mtx1);
                let mut stroke_gb = ConvStroke::new(&mut trans_gb);
                stroke_gb.set_width(0.1);
                ras.reset();
                ras.add_path(&mut stroke_gb, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
            }
            {
                let mut trans_arrows = ConvTransform::new(&mut arrows, mtx2);
                ras.reset();
                ras.add_path(&mut trans_arrows, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 127, 127, 26));
            }

            let (mask, gen_ms) = measure_with_ms(|| {
                let mut trans_gb_for_mask = ConvTransform::new(&mut gb_poly, mtx1);
                generate_alpha_mask(width, height, operation, &mut trans_gb_for_mask)
            });
            draw_text!(250.0, 20.0, &format!("Generate AlphaMask: {:.3}ms", gen_ms));

            let (_, render_ms) = measure_with_ms(|| {
                let mut trans_arrows_for_masked = ConvTransform::new(&mut arrows, mtx2);
                render_with_alpha_mask(
                    width,
                    height,
                    &mut trans_arrows_for_masked,
                    &mask,
                    Rgba8::new(127, 0, 0, 127),
                    &mut buf,
                );
            });
            draw_text!(250.0, 5.0, &format!("Render with AlphaMask: {:.3}ms", render_ms));
        }
        3 => {
            let mut gb_poly = PathStorage::new();
            make_gb_poly(&mut gb_poly);

            let mut mtx = TransAffine::new();
            mtx.multiply(&TransAffine::new_translation(-1150.0, -1150.0));
            mtx.multiply(&TransAffine::new_scaling(2.0, 2.0));

            let mut sp = Spiral::new(mx, my, 10.0, 150.0, 30.0, 0.0);
            let mut stroke = ConvStroke::new(&mut sp);
            stroke.set_width(15.0);

            {
                let mut trans_gb = ConvTransform::new(&mut gb_poly, mtx);
                ras.reset();
                ras.add_path(&mut trans_gb, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(127, 127, 0, 26));
            }
            {
                let mut trans_gb = ConvTransform::new(&mut gb_poly, mtx);
                let mut stroke_gb = ConvStroke::new(&mut trans_gb);
                stroke_gb.set_width(0.1);
                ras.reset();
                ras.add_path(&mut stroke_gb, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
            }
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 127, 127, 26));

            let (mask, gen_ms) = measure_with_ms(|| {
                let mut trans_gb_for_mask = ConvTransform::new(&mut gb_poly, mtx);
                generate_alpha_mask(width, height, operation, &mut trans_gb_for_mask)
            });
            draw_text!(250.0, 20.0, &format!("Generate AlphaMask: {:.3}ms", gen_ms));

            let mut sp2 = Spiral::new(mx, my, 10.0, 150.0, 30.0, 0.0);
            let mut stroke_for_masked = ConvStroke::new(&mut sp2);
            stroke_for_masked.set_width(15.0);
            let (_, render_ms) = measure_with_ms(|| {
                render_with_alpha_mask(
                    width,
                    height,
                    &mut stroke_for_masked,
                    &mask,
                    Rgba8::new(127, 0, 0, 127),
                    &mut buf,
                );
            });
            draw_text!(250.0, 5.0, &format!("Render with AlphaMask: {:.3}ms", render_ms));
        }
        _ => {
            let mut sp = Spiral::new(mx, my, 10.0, 150.0, 30.0, 0.0);
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

            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 26));
            ras.reset();
            ras.add_path(&mut curve, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 153, 0, 26));

            let mut sp_for_mask = Spiral::new(mx, my, 10.0, 150.0, 30.0, 0.0);
            let mut stroke_for_mask = ConvStroke::new(&mut sp_for_mask);
            stroke_for_mask.set_width(15.0);
            let (mask, gen_ms) = measure_with_ms(|| generate_alpha_mask(width, height, operation, &mut stroke_for_mask));
            draw_text!(250.0, 20.0, &format!("Generate AlphaMask: {:.3}ms", gen_ms));

            let mut glyph_for_masked = PathStorage::new();
            glyph_for_masked.move_to(28.47, 6.45);
            glyph_for_masked.curve3(21.58, 1.12, 19.82, 0.29);
            glyph_for_masked.curve3(17.19, -0.93, 14.21, -0.93);
            glyph_for_masked.curve3(9.57, -0.93, 6.57, 2.25);
            glyph_for_masked.curve3(3.56, 5.42, 3.56, 10.60);
            glyph_for_masked.curve3(3.56, 13.87, 5.03, 16.26);
            glyph_for_masked.curve3(7.03, 19.58, 11.99, 22.51);
            glyph_for_masked.curve3(16.94, 25.44, 28.47, 29.64);
            glyph_for_masked.line_to(28.47, 31.40);
            glyph_for_masked.curve3(28.47, 38.09, 26.34, 40.58);
            glyph_for_masked.curve3(24.22, 43.07, 20.17, 43.07);
            glyph_for_masked.curve3(17.09, 43.07, 15.28, 41.41);
            glyph_for_masked.curve3(13.43, 39.75, 13.43, 37.60);
            glyph_for_masked.line_to(13.53, 34.77);
            glyph_for_masked.curve3(13.53, 32.52, 12.38, 31.30);
            glyph_for_masked.curve3(11.23, 30.08, 9.38, 30.08);
            glyph_for_masked.curve3(7.57, 30.08, 6.42, 31.35);
            glyph_for_masked.curve3(5.27, 32.62, 5.27, 34.81);
            glyph_for_masked.curve3(5.27, 39.01, 9.57, 42.53);
            glyph_for_masked.curve3(13.87, 46.04, 21.63, 46.04);
            glyph_for_masked.curve3(27.59, 46.04, 31.40, 44.04);
            glyph_for_masked.curve3(34.28, 42.53, 35.64, 39.31);
            glyph_for_masked.curve3(36.52, 37.21, 36.52, 30.71);
            glyph_for_masked.line_to(36.52, 15.53);
            glyph_for_masked.curve3(36.52, 9.13, 36.77, 7.69);
            glyph_for_masked.curve3(37.01, 6.25, 37.57, 5.76);
            glyph_for_masked.curve3(38.13, 5.27, 38.87, 5.27);
            glyph_for_masked.curve3(39.65, 5.27, 40.23, 5.62);
            glyph_for_masked.curve3(41.26, 6.25, 44.19, 9.18);
            glyph_for_masked.line_to(44.19, 6.45);
            glyph_for_masked.curve3(38.72, -0.88, 33.74, -0.88);
            glyph_for_masked.curve3(31.35, -0.88, 29.93, 0.78);
            glyph_for_masked.curve3(28.52, 2.44, 28.47, 6.45);
            glyph_for_masked.close_polygon(0);
            glyph_for_masked.move_to(28.47, 9.62);
            glyph_for_masked.line_to(28.47, 26.66);
            glyph_for_masked.curve3(21.09, 23.73, 18.95, 22.51);
            glyph_for_masked.curve3(15.09, 20.36, 13.43, 18.02);
            glyph_for_masked.curve3(11.77, 15.67, 11.77, 12.89);
            glyph_for_masked.curve3(11.77, 9.38, 13.87, 7.06);
            glyph_for_masked.curve3(15.97, 4.74, 18.70, 4.74);
            glyph_for_masked.curve3(22.41, 4.74, 28.47, 9.62);
            glyph_for_masked.close_polygon(0);

            let mut mtx2 = TransAffine::new();
            mtx2.multiply(&TransAffine::new_scaling(4.0, 4.0));
            mtx2.multiply(&TransAffine::new_translation(220.0, 200.0));
            let mut trans2 = ConvTransform::new(&mut glyph_for_masked, mtx2);
            let mut curve_for_masked = ConvCurve::new(&mut trans2);
            let (_, render_ms) = measure_with_ms(|| {
                render_with_alpha_mask(
                    width,
                    height,
                    &mut curve_for_masked,
                    &mask,
                    Rgba8::new(127, 0, 0, 127),
                    &mut buf,
                );
            });
            draw_text!(250.0, 5.0, &format!("Render with AlphaMask: {:.3}ms", render_ms));
        }
    }

    let mut m_polygons = RboxCtrl::new(5.0, 5.0, 210.0, 110.0);
    m_polygons.add_item("Two Simple Paths");
    m_polygons.add_item("Closed Stroke");
    m_polygons.add_item("Great Britain and Arrows");
    m_polygons.add_item("Great Britain and Spiral");
    m_polygons.add_item("Spiral and Glyph");
    m_polygons.set_cur_item(scenario as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_polygons);

    let mut m_operation = RboxCtrl::new(555.0, 5.0, 635.0, 55.0);
    m_operation.add_item("AND");
    m_operation.add_item("SUB");
    m_operation.set_cur_item(operation);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_operation);

    buf
}

/// Image transforms — star polygon textured with image through 7 transform modes.
/// Matches C++ image_transforms.cpp behavior.
/// Params:
/// [poly_angle, poly_scale, img_angle, img_scale, rotate_polygon, rotate_image,
///  example_idx, img_cx, img_cy, poly_cx, poly_cy]
pub fn image_transforms_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    let poly_angle = params.get(0).copied().unwrap_or(0.0);
    let poly_scale = params.get(1).copied().unwrap_or(1.0);
    let img_angle = params.get(2).copied().unwrap_or(0.0);
    let img_scale = params.get(3).copied().unwrap_or(1.0);
    let rotate_polygon = params.get(4).copied().unwrap_or(0.0) > 0.5;
    let rotate_image = params.get(5).copied().unwrap_or(0.0) > 0.5;
    let example_idx = (params.get(6).copied().unwrap_or(0.0) as i32).clamp(0, 6);
    let img_cx = params.get(7).copied().unwrap_or(w / 2.0);
    let img_cy = params.get(8).copied().unwrap_or(h / 2.0);
    let poly_cx = params.get(9).copied().unwrap_or(w / 2.0);
    let poly_cy = params.get(10).copied().unwrap_or(h / 2.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut sa = SpanAllocator::new();

    let (img_w, img_h, mut img_data) = load_spheres_image();
    let mut img_ra = RowAccessor::new();
    let img_stride = (img_w * 4) as i32;
    unsafe { img_ra.attach(img_data.as_mut_ptr(), img_w, img_h, img_stride) };

    // Build the same 14-point star as C++, centered at polygon center.
    let mut r = w;
    if h < r {
        r = h;
    }
    let r1 = r / 3.0 - 8.0;
    let r2 = r1 / 1.45;
    let mut star = PathStorage::new();
    for i in 0..14 {
        let a = std::f64::consts::PI * 2.0 * i as f64 / 14.0 - std::f64::consts::PI / 2.0;
        let dx = a.cos();
        let dy = a.sin();
        if i % 2 == 1 {
            star.line_to(poly_cx + dx * r1, poly_cy + dy * r1);
        } else if i == 0 {
            star.move_to(poly_cx + dx * r2, poly_cy + dy * r2);
        } else {
            star.line_to(poly_cx + dx * r2, poly_cy + dy * r2);
        }
    }
    star.close_polygon(PATH_FLAGS_CCW as u32);

    let pa = poly_angle.to_radians();
    let mut poly_mtx = TransAffine::new();
    poly_mtx.multiply(&TransAffine::new_translation(-poly_cx, -poly_cy));
    poly_mtx.multiply(&TransAffine::new_rotation(pa));
    poly_mtx.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
    poly_mtx.multiply(&TransAffine::new_translation(poly_cx, poly_cy));

    let image_center_x = img_w as f64 / 2.0;
    let image_center_y = img_h as f64 / 2.0;
    let mut image_mtx = TransAffine::new();
    match example_idx {
        0 => {}
        1 => {
            image_mtx.multiply(&TransAffine::new_translation(-image_center_x, -image_center_y));
            image_mtx.multiply(&TransAffine::new_rotation(pa));
            image_mtx.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
            image_mtx.multiply(&TransAffine::new_translation(poly_cx, poly_cy));
            image_mtx.invert();
        }
        2 => {
            let ia = img_angle.to_radians();
            image_mtx.multiply(&TransAffine::new_translation(-image_center_x, -image_center_y));
            image_mtx.multiply(&TransAffine::new_rotation(ia));
            image_mtx.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            image_mtx.multiply(&TransAffine::new_translation(img_cx, img_cy));
            image_mtx.invert();
        }
        3 => {
            let ia = img_angle.to_radians();
            image_mtx.multiply(&TransAffine::new_translation(-image_center_x, -image_center_y));
            image_mtx.multiply(&TransAffine::new_rotation(ia));
            image_mtx.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            image_mtx.multiply(&TransAffine::new_translation(poly_cx, poly_cy));
            image_mtx.invert();
        }
        4 => {
            image_mtx.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            image_mtx.multiply(&TransAffine::new_rotation(pa));
            image_mtx.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
            image_mtx.multiply(&TransAffine::new_translation(poly_cx, poly_cy));
            image_mtx.invert();
        }
        5 => {
            let ia = img_angle.to_radians();
            image_mtx.multiply(&TransAffine::new_translation(-image_center_x, -image_center_y));
            image_mtx.multiply(&TransAffine::new_rotation(ia));
            image_mtx.multiply(&TransAffine::new_rotation(pa));
            image_mtx.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            image_mtx.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
            image_mtx.multiply(&TransAffine::new_translation(img_cx, img_cy));
            image_mtx.invert();
        }
        6 => {
            let ia = img_angle.to_radians();
            image_mtx.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            image_mtx.multiply(&TransAffine::new_rotation(ia));
            image_mtx.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            image_mtx.multiply(&TransAffine::new_translation(img_cx, img_cy));
            image_mtx.invert();
        }
        _ => {}
    }

    let mut transformed_star = ConvTransform::new(&mut star, poly_mtx);
    ras.reset();
    ras.add_path(&mut transformed_star, 0);

    let mut interp = SpanInterpolatorLinear::new(image_mtx);
    let bg = Rgba8::new(255, 255, 255, 255);
    let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg, &mut interp);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

    // Image center marker.
    let mut e1 = Ellipse::new(img_cx, img_cy, 5.0, 5.0, 20, false);
    ras.reset();
    ras.add_path(&mut e1, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(179, 204, 0, 255));

    let mut e1_stroke_src = Ellipse::new(img_cx, img_cy, 5.0, 5.0, 20, false);
    let mut e1_stroke = ConvStroke::new(&mut e1_stroke_src);
    e1_stroke.set_width(1.0);
    ras.reset();
    ras.add_path(&mut e1_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    let mut e2 = Ellipse::new(img_cx, img_cy, 2.0, 2.0, 20, false);
    ras.reset();
    ras.add_path(&mut e2, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    // C++-style controls.
    let mut sl_pa = SliderCtrl::new(5.0, 5.0, 145.0, 11.0);
    sl_pa.label("Polygon Angle=%3.2f");
    sl_pa.range(-180.0, 180.0);
    sl_pa.set_value(poly_angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_pa);

    let mut sl_ps = SliderCtrl::new(5.0, 19.0, 145.0, 26.0);
    sl_ps.label("Polygon Scale=%3.2f");
    sl_ps.range(0.1, 5.0);
    sl_ps.set_value(poly_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_ps);

    let mut sl_ia = SliderCtrl::new(155.0, 5.0, 300.0, 12.0);
    sl_ia.label("Image Angle=%3.2f");
    sl_ia.range(-180.0, 180.0);
    sl_ia.set_value(img_angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_ia);

    let mut sl_is = SliderCtrl::new(155.0, 19.0, 300.0, 26.0);
    sl_is.label("Image Scale=%3.2f");
    sl_is.range(0.1, 5.0);
    sl_is.set_value(img_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_is);

    let mut cb_rotate_polygon = CboxCtrl::new(5.0, 33.0, "Rotate Polygon");
    cb_rotate_polygon.set_status(rotate_polygon);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_rotate_polygon);

    let mut cb_rotate_image = CboxCtrl::new(5.0, 47.0, "Rotate Image");
    cb_rotate_image.set_status(rotate_image);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb_rotate_image);

    let mut m_example = RboxCtrl::new(5.0, 56.0, 40.0, 190.0);
    m_example.background_color(Rgba8::new(255, 255, 255, 255));
    m_example.add_item("0");
    m_example.add_item("1");
    m_example.add_item("2");
    m_example.add_item("3");
    m_example.add_item("4");
    m_example.add_item("5");
    m_example.add_item("6");
    m_example.set_cur_item(example_idx);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_example);

    buf
}

const MOL_VIEW_SDF: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../cpp-references/agg-src/examples/X11/1.sdf"
));

const ATOM_COLOR_GENERAL: usize = 0;
const ATOM_COLOR_N: usize = 1;
const ATOM_COLOR_O: usize = 2;
const ATOM_COLOR_S: usize = 3;
const ATOM_COLOR_P: usize = 4;
const ATOM_COLOR_HALOGEN: usize = 5;
const ATOM_COLOR_COUNT: usize = 6;

#[derive(Clone)]
struct MolAtom {
    x: f64,
    y: f64,
    label: String,
    charge: i32,
    color_idx: usize,
}

#[derive(Clone, Copy)]
struct MolBond {
    idx1: usize,
    idx2: usize,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    order: u32,
    stereo: i32,
    topology: i32,
}

#[derive(Clone)]
struct Molecule {
    name: String,
    atoms: Vec<MolAtom>,
    bonds: Vec<MolBond>,
    avr_len: f64,
}

fn get_field<'a>(line: &'a str, pos: usize, len: usize) -> &'a str {
    let start = pos.saturating_sub(1);
    if start >= line.len() {
        return "";
    }
    let end = (start + len).min(line.len());
    &line[start..end]
}

fn get_int(line: &str, pos: usize, len: usize) -> i32 {
    let token = get_field(line, pos, len)
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("0");
    token.parse::<i32>().unwrap_or(0)
}

fn get_dbl(line: &str, pos: usize, len: usize) -> f64 {
    let token = get_field(line, pos, len)
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("0");
    token.parse::<f64>().unwrap_or(0.0)
}

fn get_str(line: &str, pos: usize, len: usize) -> String {
    get_field(line, pos, len)
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

fn atom_color_idx(label: &str) -> usize {
    match label {
        "N" => ATOM_COLOR_N,
        "O" => ATOM_COLOR_O,
        "S" => ATOM_COLOR_S,
        "P" => ATOM_COLOR_P,
        "F" | "Cl" | "Br" | "I" => ATOM_COLOR_HALOGEN,
        _ => ATOM_COLOR_GENERAL,
    }
}

fn parse_molecules_from_sdf(src: &str, max_molecules: usize) -> Vec<Molecule> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0usize;

    while i + 3 < lines.len() && out.len() < max_molecules {
        let name = lines[i].to_string();
        i += 1;
        if i + 2 >= lines.len() {
            break;
        }
        i += 2;

        let count_line = lines[i];
        i += 1;
        let num_atoms = get_int(count_line, 1, 3).max(0) as usize;
        let num_bonds = get_int(count_line, 4, 3).max(0) as usize;
        if num_atoms == 0 || num_bonds == 0 {
            break;
        }
        if i + num_atoms + num_bonds > lines.len() {
            break;
        }

        let mut atoms = Vec::with_capacity(num_atoms);
        for _ in 0..num_atoms {
            let atom_line = lines[i];
            i += 1;
            let label = get_str(atom_line, 32, 3);
            let mut charge = get_int(atom_line, 39, 1);
            if charge != 0 {
                charge = 4 - charge;
            }
            atoms.push(MolAtom {
                x: get_dbl(atom_line, 1, 10),
                y: get_dbl(atom_line, 11, 10),
                label: label.clone(),
                charge,
                color_idx: atom_color_idx(&label),
            });
        }

        let mut bonds = Vec::with_capacity(num_bonds);
        let mut avr_len = 0.0f64;
        for _ in 0..num_bonds {
            let bond_line = lines[i];
            i += 1;
            let idx1 = get_int(bond_line, 1, 3) - 1;
            let idx2 = get_int(bond_line, 4, 3) - 1;
            if idx1 < 0 || idx2 < 0 {
                continue;
            }
            let idx1 = idx1 as usize;
            let idx2 = idx2 as usize;
            if idx1 >= atoms.len() || idx2 >= atoms.len() {
                continue;
            }
            let x1 = atoms[idx1].x;
            let y1 = atoms[idx1].y;
            let x2 = atoms[idx2].x;
            let y2 = atoms[idx2].y;
            let dx = x1 - x2;
            let dy = y1 - y2;
            avr_len += (dx * dx + dy * dy).sqrt();
            bonds.push(MolBond {
                idx1,
                idx2,
                x1,
                y1,
                x2,
                y2,
                order: get_int(bond_line, 7, 3).max(0) as u32,
                stereo: get_int(bond_line, 10, 3),
                topology: get_int(bond_line, 13, 3),
            });
        }
        if !bonds.is_empty() {
            avr_len /= bonds.len() as f64;
        } else {
            avr_len = 1.0;
        }

        while i < lines.len() {
            let line = lines[i];
            i += 1;
            if line.starts_with('$') {
                break;
            }
        }

        out.push(Molecule {
            name,
            atoms,
            bonds,
            avr_len,
        });
    }

    out
}

fn mol_data() -> &'static [Molecule] {
    static DATA: OnceLock<Vec<Molecule>> = OnceLock::new();
    DATA.get_or_init(|| parse_molecules_from_sdf(MOL_VIEW_SDF, 100))
        .as_slice()
}

struct BondLine {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    dx: f64,
    dy: f64,
    thickness: f64,
    vertex: u32,
}

impl BondLine {
    fn new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 0.0,
            dx: 0.0,
            dy: 0.0,
            thickness: 0.1,
            vertex: 0,
        }
    }
    fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;
    }
    fn thickness(&mut self, th: f64) {
        self.thickness = th;
    }
}

impl VertexSource for BondLine {
    fn rewind(&mut self, _path_id: u32) {
        let (dx, dy) = calc_orthogonal(
            self.thickness * 0.5,
            self.x1,
            self.y1,
            self.x2,
            self.y2,
        );
        self.dx = dx;
        self.dy = dy;
        self.vertex = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        match self.vertex {
            0 => {
                *x = self.x1 - self.dx;
                *y = self.y1 - self.dy;
                self.vertex += 1;
                PATH_CMD_MOVE_TO
            }
            1 => {
                *x = self.x2 - self.dx;
                *y = self.y2 - self.dy;
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
            2 => {
                *x = self.x2 + self.dx;
                *y = self.y2 + self.dy;
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
            3 => {
                *x = self.x1 + self.dx;
                *y = self.y1 + self.dy;
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
            _ => PATH_CMD_STOP,
        }
    }
}

struct SolidWedge {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    dx: f64,
    dy: f64,
    thickness: f64,
    vertex: u32,
}

impl SolidWedge {
    fn new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 0.0,
            dx: 0.0,
            dy: 0.0,
            thickness: 0.1,
            vertex: 0,
        }
    }
    fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;
    }
    fn thickness(&mut self, th: f64) {
        self.thickness = th;
    }
}

impl VertexSource for SolidWedge {
    fn rewind(&mut self, _path_id: u32) {
        let (dx, dy) = calc_orthogonal(
            self.thickness * 2.0,
            self.x1,
            self.y1,
            self.x2,
            self.y2,
        );
        self.dx = dx;
        self.dy = dy;
        self.vertex = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        match self.vertex {
            0 => {
                *x = self.x1;
                *y = self.y1;
                self.vertex += 1;
                PATH_CMD_MOVE_TO
            }
            1 => {
                *x = self.x2 - self.dx;
                *y = self.y2 - self.dy;
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
            2 => {
                *x = self.x2 + self.dx;
                *y = self.y2 + self.dy;
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
            _ => PATH_CMD_STOP,
        }
    }
}

struct DashedWedge {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    xt2: f64,
    yt2: f64,
    xt3: f64,
    yt3: f64,
    xd: [f64; 4],
    yd: [f64; 4],
    thickness: f64,
    num_dashes: u32,
    vertex: u32,
}

impl DashedWedge {
    fn new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 0.0,
            xt2: 0.0,
            yt2: 0.0,
            xt3: 0.0,
            yt3: 0.0,
            xd: [0.0; 4],
            yd: [0.0; 4],
            thickness: 0.1,
            num_dashes: 8,
            vertex: 0,
        }
    }

    fn init(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        // Matches C++: dashed wedge is reversed.
        self.x1 = x2;
        self.y1 = y2;
        self.x2 = x1;
        self.y2 = y1;
    }

    fn thickness(&mut self, th: f64) {
        self.thickness = th;
    }
}

impl VertexSource for DashedWedge {
    fn rewind(&mut self, _path_id: u32) {
        let (dx, dy) = calc_orthogonal(
            self.thickness * 2.0,
            self.x1,
            self.y1,
            self.x2,
            self.y2,
        );
        self.xt2 = self.x2 - dx;
        self.yt2 = self.y2 - dy;
        self.xt3 = self.x2 + dx;
        self.yt3 = self.y2 + dy;
        self.vertex = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex < self.num_dashes * 4 {
            if (self.vertex % 4) == 0 {
                let k1 = (self.vertex / 4) as f64 / self.num_dashes as f64;
                let k2 = k1 + 0.4 / self.num_dashes as f64;
                self.xd[0] = self.x1 + (self.xt2 - self.x1) * k1;
                self.yd[0] = self.y1 + (self.yt2 - self.y1) * k1;
                self.xd[1] = self.x1 + (self.xt2 - self.x1) * k2;
                self.yd[1] = self.y1 + (self.yt2 - self.y1) * k2;
                self.xd[2] = self.x1 + (self.xt3 - self.x1) * k2;
                self.yd[2] = self.y1 + (self.yt3 - self.y1) * k2;
                self.xd[3] = self.x1 + (self.xt3 - self.x1) * k1;
                self.yd[3] = self.y1 + (self.yt3 - self.y1) * k1;
                *x = self.xd[0];
                *y = self.yd[0];
                self.vertex += 1;
                PATH_CMD_MOVE_TO
            } else {
                let idx = (self.vertex % 4) as usize;
                *x = self.xd[idx];
                *y = self.yd[idx];
                self.vertex += 1;
                PATH_CMD_LINE_TO
            }
        } else {
            PATH_CMD_STOP
        }
    }
}

#[derive(Clone, Copy)]
enum BondStyle {
    Single,
    WedgedSolid,
    WedgedDashed,
    Double,
    DoubleLeft,
    DoubleRight,
    Triple,
}

struct BondVertexGenerator<'a> {
    bond: &'a MolBond,
    thickness: f64,
    style: BondStyle,
    line1: BondLine,
    line2: BondLine,
    solid_wedge: SolidWedge,
    dashed_wedge: DashedWedge,
    status: u32,
}

impl<'a> BondVertexGenerator<'a> {
    fn new(bond: &'a MolBond, thickness: f64) -> Self {
        let mut style = BondStyle::Single;
        if bond.order == 1 {
            if bond.stereo == 1 {
                style = BondStyle::WedgedSolid;
            }
            if bond.stereo == 6 {
                style = BondStyle::WedgedDashed;
            }
        }
        if bond.order == 2 {
            style = BondStyle::Double;
            if bond.topology == 1 {
                style = BondStyle::DoubleLeft;
            }
            if bond.topology == 2 {
                style = BondStyle::DoubleRight;
            }
        }
        if bond.order == 3 {
            style = BondStyle::Triple;
        }

        let mut line1 = BondLine::new();
        let mut line2 = BondLine::new();
        let mut solid_wedge = SolidWedge::new();
        let mut dashed_wedge = DashedWedge::new();
        line1.thickness(thickness);
        line2.thickness(thickness);
        solid_wedge.thickness(thickness);
        dashed_wedge.thickness(thickness);

        Self {
            bond,
            thickness,
            style,
            line1,
            line2,
            solid_wedge,
            dashed_wedge,
            status: 0,
        }
    }
}

impl VertexSource for BondVertexGenerator<'_> {
    fn rewind(&mut self, _path_id: u32) {
        match self.style {
            BondStyle::WedgedSolid => {
                self.solid_wedge
                    .init(self.bond.x1, self.bond.y1, self.bond.x2, self.bond.y2);
                self.solid_wedge.rewind(0);
            }
            BondStyle::WedgedDashed => {
                self.dashed_wedge
                    .init(self.bond.x1, self.bond.y1, self.bond.x2, self.bond.y2);
                self.dashed_wedge.rewind(0);
            }
            BondStyle::Double | BondStyle::DoubleLeft | BondStyle::DoubleRight => {
                let (dx, dy) = calc_orthogonal(
                    self.thickness,
                    self.bond.x1,
                    self.bond.y1,
                    self.bond.x2,
                    self.bond.y2,
                );
                let dx1 = dx;
                let dy1 = dy;
                let dx2 = dx;
                let dy2 = dy;
                self.line1.init(
                    self.bond.x1 - dx1,
                    self.bond.y1 - dy1,
                    self.bond.x2 - dx1,
                    self.bond.y2 - dy1,
                );
                self.line1.rewind(0);
                self.line2.init(
                    self.bond.x1 + dx2,
                    self.bond.y1 + dy2,
                    self.bond.x2 + dx2,
                    self.bond.y2 + dy2,
                );
                self.line2.rewind(0);
                self.status = 0;
            }
            BondStyle::Triple | BondStyle::Single => {
                self.line1
                    .init(self.bond.x1, self.bond.y1, self.bond.x2, self.bond.y2);
                self.line1.rewind(0);
            }
        }
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        match self.style {
            BondStyle::WedgedSolid => self.solid_wedge.vertex(x, y),
            BondStyle::WedgedDashed => self.dashed_wedge.vertex(x, y),
            BondStyle::Double | BondStyle::DoubleLeft | BondStyle::DoubleRight => {
                let mut flag = PATH_CMD_STOP;
                if self.status == 0 {
                    flag = self.line1.vertex(x, y);
                    if flag == PATH_CMD_STOP {
                        self.status = 1;
                    }
                }
                if self.status == 1 {
                    flag = self.line2.vertex(x, y);
                }
                flag
            }
            BondStyle::Triple | BondStyle::Single => self.line1.vertex(x, y),
        }
    }
}

/// Molecular structure viewer.
/// Ported from C++ `mol_view.cpp`.
/// Params: [mol_idx, thickness, text_size, angle, scale, cx, cy]
pub fn mol_view(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let mol_idx = params.get(0).copied().unwrap_or(0.0).max(0.0) as usize;
    let thickness_ctrl = params.get(1).copied().unwrap_or(0.5);
    let text_size_ctrl = params.get(2).copied().unwrap_or(0.5);
    let angle = params.get(3).copied().unwrap_or(0.0);
    let scale = params.get(4).copied().unwrap_or(1.0);
    let cx = params.get(5).copied().unwrap_or(w / 2.0);
    let cy = params.get(6).copied().unwrap_or(h / 2.0);

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let molecules = mol_data();
    if molecules.is_empty() {
        return buf;
    }
    let mol = &molecules[mol_idx.min(molecules.len() - 1)];

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = -f64::INFINITY;
    let mut max_y = -f64::INFINITY;
    for atom in &mol.atoms {
        min_x = min_x.min(atom.x);
        min_y = min_y.min(atom.y);
        max_x = max_x.max(atom.x);
        max_y = max_y.max(atom.y);
    }

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(
        -(max_x + min_x) * 0.5,
        -(max_y + min_y) * 0.5,
    ));

    let mut fit_scale = w / (max_x - min_x);
    let fit_h = h / (max_y - min_y);
    if fit_scale > fit_h {
        fit_scale = fit_h;
    }

    let mut text_size = mol.avr_len * text_size_ctrl / 4.0;
    let thickness = mol.avr_len / scale.max(0.0001).sqrt() / 8.0;

    mtx.multiply(&TransAffine::new_scaling(fit_scale * 0.80, fit_scale * 0.80));
    mtx.multiply(&TransAffine::new_rotation(angle));
    mtx.multiply(&TransAffine::new_scaling(scale, scale));
    mtx.multiply(&TransAffine::new_translation(cx, cy));

    let black = Rgba8::new(0, 0, 0, 255);
    for bond in &mol.bonds {
        let _ = (bond.idx1, bond.idx2);
        let mut bond_vs = BondVertexGenerator::new(bond, thickness_ctrl * thickness);
        let mut tr = ConvTransform::new(&mut bond_vs, mtx);
        ras.reset();
        ras.add_path(&mut tr, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &black);
    }

    for atom in &mol.atoms {
        if atom.label != "C" {
            let _ = atom.charge;
            let mut ell = Ellipse::new(atom.x, atom.y, text_size * 2.5, text_size * 2.5, 20, false);
            let mut tr = ConvTransform::new(&mut ell, mtx);
            ras.reset();
            ras.add_path(&mut tr, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 255));
        }
    }

    text_size *= 3.0;
    let atom_colors = [
        Rgba8::new(0, 0, 0, 255),
        Rgba8::new(0, 0, 120, 255),
        Rgba8::new(200, 0, 0, 255),
        Rgba8::new(120, 120, 0, 255),
        Rgba8::new(80, 50, 0, 255),
        Rgba8::new(0, 200, 0, 255),
    ];

    let mut label_stroke = ConvStroke::new(GsvText::new());
    label_stroke.set_line_join(LineJoin::Round);
    label_stroke.set_line_cap(LineCap::Round);
    label_stroke.set_approximation_scale(mtx.get_scale());
    for atom in &mol.atoms {
        if atom.label != "C" {
            label_stroke.set_width(thickness_ctrl * thickness);
            let label = label_stroke.source_mut();
            label.text(&atom.label);
            label.start_point(atom.x - text_size * 0.5, atom.y - text_size * 0.5);
            label.size(text_size, 0.0);

            let mut tr = ConvTransform::new(&mut label_stroke, mtx);
            ras.reset();
            ras.add_path(&mut tr, 0);
            let color = atom_colors[atom.color_idx.min(ATOM_COLOR_COUNT - 1)];
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &color);
        }
    }

    label_stroke.set_approximation_scale(1.0);
    label_stroke.set_width(1.5);
    {
        let label = label_stroke.source_mut();
        label.text(&mol.name);
        label.size(10.0, 0.0);
        label.start_point(10.0, h - 20.0);
    }
    ras.reset();
    ras.add_path(&mut label_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &black);

    let mut sl_thickness = SliderCtrl::new(5.0, 5.0, w - 5.0, 12.0);
    sl_thickness.label("Thickness=%3.2f");
    sl_thickness.set_value(thickness_ctrl);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_thickness);

    let mut sl_text = SliderCtrl::new(5.0, 20.0, w - 5.0, 27.0);
    sl_text.label("Label Size=%3.2f");
    sl_text.set_value(text_size_ctrl);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_text);

    buf
}


// ============================================================================
// Image Resample — image affine/perspective transforms with resampling
// ============================================================================

/// Image resampling with multiple transform modes.
/// Adapted from C++ image_resample.cpp.
///
/// params[0] = mode (0-3): 0=affine 2x2, 1=affine resample, 2=persp lerp, 3=persp exact
/// params[1] = blur factor (0.5-2.0, default 1.0)
/// params[2..9] = quad corners (x0,y0,x1,y1,x2,y2,x3,y3)
pub fn image_resample_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let mode = params.get(0).copied().unwrap_or(0.0) as u32;
    let blur = params.get(1).copied().unwrap_or(1.0).clamp(0.5, 2.0);

    let w = width as f64;
    let h = height as f64;

    let (img_w, img_h, mut img_data) = load_spheres_image();
    let iw = img_w as f64;
    let ih = img_h as f64;

    // Source image rectangle
    let g_x1 = 0.0;
    let g_y1 = 0.0;
    let g_x2 = iw;
    let g_y2 = ih;

    // Default quad corners (slightly inset and rotated)
    let cx = w / 2.0;
    let cy = h / 2.0;
    let hw = iw * 0.45;
    let hh = ih * 0.45;
    let quad = [
        params.get(2).copied().unwrap_or(cx - hw),     // x0 top-left
        params.get(3).copied().unwrap_or(cy - hh),     // y0
        params.get(4).copied().unwrap_or(cx + hw),     // x1 top-right
        params.get(5).copied().unwrap_or(cy - hh),     // y1
        params.get(6).copied().unwrap_or(cx + hw),     // x2 bottom-right
        params.get(7).copied().unwrap_or(cy + hh),     // y2
        params.get(8).copied().unwrap_or(cx - hw),     // x3 bottom-left
        params.get(9).copied().unwrap_or(cy + hh),     // y3
    ];

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Setup source image accessor
    let img_stride = (img_w * 4) as i32;
    let mut img_ra = RowAccessor::new();
    unsafe { img_ra.attach(img_data.as_mut_ptr(), img_w, img_h, img_stride) };
    let mut source = ImageAccessorClone::<4>::new(&img_ra);

    // Create filter
    let filter_gen = ImageFilterBilinear {};
    let mut filter = ImageFilterLut::new();
    filter.calculate(&filter_gen, true);

    // Rasterize the quad outline as the clipping shape
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut sa = SpanAllocator::new();

    ras.move_to_d(quad[0], quad[1]);
    ras.line_to_d(quad[2], quad[3]);
    ras.line_to_d(quad[4], quad[5]);
    ras.line_to_d(quad[6], quad[7]);

    match mode {
        0 => {
            // Mode 0: Affine + 2x2 filter (no resample)
            let mut tr = TransAffine::new();
            let dst_parl = [quad[0], quad[1], quad[2], quad[3], quad[4], quad[5]];
            let src_parl = [g_x1, g_y1, g_x2, g_y1, g_x2, g_y2];
            tr.parl_to_parl(&dst_parl, &src_parl);
            let mut interp = SpanInterpolatorLinear::new(tr);
            let mut sg = SpanImageFilterRgba2x2::new(&mut source, &mut interp, &filter);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
        }
        1 => {
            // Mode 1: Affine + resampling
            let mut tr = TransAffine::new();
            let dst_parl = [quad[0], quad[1], quad[2], quad[3], quad[4], quad[5]];
            let src_parl = [g_x1, g_y1, g_x2, g_y1, g_x2, g_y2];
            tr.parl_to_parl(&dst_parl, &src_parl);
            let mut interp = SpanInterpolatorLinear::new(tr);
            let mut sg = SpanImageResampleRgbaAffine::new(&mut source, &mut interp, &filter);
            sg.resample_base_mut().set_blur(blur);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
        }
        2 => {
            // Mode 2: Perspective (lerp) + 2x2 filter
            let mut interp = SpanInterpolatorPerspLerp::new_quad_to_rect(
                &quad, g_x1, g_y1, g_x2, g_y2);
            if interp.is_valid() {
                let mut sg = SpanImageFilterRgba2x2::new(&mut source, &mut interp, &filter);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
        _ => {
            // Mode 3: Perspective (exact via SpanInterpolatorTrans) + 2x2 filter
            let mut tr = TransPerspective::new();
            tr.quad_to_rect(&quad, g_x1, g_y1, g_x2, g_y2);
            if tr.is_valid() {
                let mut interp = SpanInterpolatorTrans::new(tr);
                let mut sg = SpanImageFilterRgba2x2::new(&mut source, &mut interp, &filter);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
            }
        }
    }

    // Draw quad outline
    let mut path = PathStorage::new();
    path.move_to(quad[0], quad[1]);
    path.line_to(quad[2], quad[3]);
    path.line_to(quad[4], quad[5]);
    path.line_to(quad[6], quad[7]);
    path.close_polygon(0);
    let mut stroke = ConvStroke::new(&mut path);
    stroke.set_width(2.0);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 200));

    // Mode label
    let mode_label = match mode {
        0 => "Affine (2x2 filter)",
        1 => "Affine (resample)",
        2 => "Perspective LERP",
        _ => "Perspective Exact",
    };
    let label = format!("Mode {}: {} — blur={:.2}", mode, mode_label, blur);
    let mut txt = GsvText::new();
    txt.size(8.0, 0.0);
    txt.start_point(5.0, h - 15.0);
    txt.text(&label);
    let mut txt_stroke = ConvStroke::new(&mut txt);
    txt_stroke.set_width(0.8);
    ras.reset();
    ras.add_path(&mut txt_stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

    buf
}

// ============================================================================
// Alpha Mask 2 — alpha mask with random ellipses modulating rendering
// ============================================================================

/// Alpha mask demo with random ellipses creating a mask pattern.
/// Ported from C++ `alpha_mask2.cpp`.
///
/// params[0] = num_mask_ellipses (5-100, default 10)
/// params[1] = lion angle (radians)
/// params[2] = lion scale
/// params[3] = skew_x
/// params[4] = skew_y
pub fn alpha_mask2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    fn msvc_rand15(state: &mut u32) -> u32 {
        // Match MSVC's rand() sequence used by AGG demos on Windows.
        *state = state.wrapping_mul(214013).wrapping_add(2531011);
        (*state >> 16) & 0x7FFF
    }

    let num_ellipses = params.get(0).copied().unwrap_or(10.0).clamp(5.0, 100.0) as u32;
    let angle = params.get(1).copied().unwrap_or(0.0);
    let scale = params.get(2).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(3).copied().unwrap_or(0.0);
    let skew_y = params.get(4).copied().unwrap_or(0.0);

    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Step 1: Generate alpha mask buffer (Gray8), matching C++ sequence.
    let mask_size = (width * height) as usize;
    let mut mask_gray = vec![0u8; mask_size];
    let mut rng_state = 1432u32;
    {
        let mut mask_ra = RowAccessor::new();
        let mask_stride = width as i32;
        unsafe { mask_ra.attach(mask_gray.as_mut_ptr(), width, height, mask_stride) };
        let mask_pf = PixfmtGray8::new(&mut mask_ra);
        let mut mask_rb = RendererBase::new(mask_pf);
        mask_rb.clear(&Gray8::new(0, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        for _ in 0..num_ellipses {
            let cx = (msvc_rand15(&mut rng_state) % width) as f64;
            let cy = (msvc_rand15(&mut rng_state) % height) as f64;
            let rx = (msvc_rand15(&mut rng_state) % 100 + 20) as f64;
            let ry = (msvc_rand15(&mut rng_state) % 100 + 20) as f64;
            let v = ((msvc_rand15(&mut rng_state) & 127) + 128) as u32;
            let a = ((msvc_rand15(&mut rng_state) & 127) + 128) as u32;

            let mut ell = Ellipse::new(cx, cy, rx, ry, 100, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut mask_rb,
                &Gray8::new(v, a));
        }
    }

    // Step 2: Render lion and random primitives into a temporary buffer.
    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let mut temp_buf = vec![0u8; (width * height * 4) as usize];
    {
        let mut temp_ra = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { temp_ra.attach(temp_buf.as_mut_ptr(), width, height, stride) };
        let temp_pf = PixfmtRgba32::new(&mut temp_ra);
        let mut temp_rb = RendererBase::new(temp_pf);
        temp_rb.clear(&Rgba8::new(0, 0, 0, 0)); // transparent

        let path_ids: Vec<u32> = path_idx.iter().map(|&i| i as u32).collect();
        let npaths = path_idx.len();
        let bbox = bounding_rect(&mut path, &path_ids, 0, npaths).unwrap_or(
            agg_rust::basics::RectD::new(0.0, 0.0, 250.0, 400.0),
        );
        let base_dx = (bbox.x2 - bbox.x1) / 2.0;
        let base_dy = (bbox.y2 - bbox.y1) / 2.0;

        let mut mtx = TransAffine::new();
        mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
        mtx.multiply(&TransAffine::new_scaling(scale, scale));
        mtx.multiply(&TransAffine::new_rotation(angle + std::f64::consts::PI));
        mtx.multiply(&TransAffine::new_skewing(skew_x / 1000.0, skew_y / 1000.0));
        mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));

        let mut conv = ConvTransform::new(&mut path, mtx);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        for i in 0..npaths {
            ras.reset();
            ras.add_path(&mut conv, path_idx[i] as u32);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut temp_rb, &colors[i]);
        }
        // Random Bresenham lines + simple filled markers.
        {
            let mut prim = RendererPrimitives::new(&mut temp_rb);
            for _ in 0..50 {
                let line_color = Rgba8::new(
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    ((msvc_rand15(&mut rng_state) & 0x7F) + 0x7F) as u32,
                );
                let fill_color = Rgba8::new(
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                    ((msvc_rand15(&mut rng_state) & 0x7F) + 0x7F) as u32,
                );
                prim.set_line_color(line_color);
                prim.set_fill_color(fill_color);
                prim.line(
                    RendererPrimitives::<PixfmtRgba32>::coord((msvc_rand15(&mut rng_state) % width) as f64),
                    RendererPrimitives::<PixfmtRgba32>::coord((msvc_rand15(&mut rng_state) % height) as f64),
                    RendererPrimitives::<PixfmtRgba32>::coord((msvc_rand15(&mut rng_state) % width) as f64),
                    RendererPrimitives::<PixfmtRgba32>::coord((msvc_rand15(&mut rng_state) % height) as f64),
                    true,
                );

                let mx = (msvc_rand15(&mut rng_state) % width) as i32;
                let my = (msvc_rand15(&mut rng_state) % height) as i32;
                let mr = (msvc_rand15(&mut rng_state) % 10 + 5) as i32;
                prim.solid_ellipse(mx, my, mr, mr);
            }
        }

        // Random anti-aliased lines.
        for _ in 0..50 {
            let mut line_path = PathStorage::new();
            line_path.move_to(
                (msvc_rand15(&mut rng_state) % width) as f64,
                (msvc_rand15(&mut rng_state) % height) as f64,
            );
            line_path.line_to(
                (msvc_rand15(&mut rng_state) % width) as f64,
                (msvc_rand15(&mut rng_state) % height) as f64,
            );
            let mut stroke = ConvStroke::new(&mut line_path);
            stroke.set_width(5.0);
            stroke.set_line_cap(LineCap::Round);
            let color = Rgba8::new(
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                ((msvc_rand15(&mut rng_state) & 0x7F) + 0x7F) as u32,
            );
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut temp_rb, &color);
        }

        // Random circles (C++ uses gradient circles; this keeps the same structure/count).
        for _ in 0..50 {
            let x = (msvc_rand15(&mut rng_state) % width) as f64;
            let y = (msvc_rand15(&mut rng_state) % height) as f64;
            let r = (msvc_rand15(&mut rng_state) % 10 + 5) as f64;
            let color = Rgba8::new(
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                (msvc_rand15(&mut rng_state) & 0x7F) as u32,
                255,
            );
            let mut ell = Ellipse::new(x, y, r, r, 32, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut temp_rb, &color);
        }
    }

    // Step 3: Composite masked temp buffer onto main buffer.
    for y in 0..height as usize {
        for x in 0..width as usize {
            let idx = y * width as usize + x;
            let pi = idx * 4;
            let mask_val = mask_gray[idx] as u32;
            let sa = temp_buf[pi + 3] as u32;
            if sa > 0 && mask_val > 0 {
                let a = (255 + sa * mask_val) >> 8;
                let inv_a = 255 - a;
                let sr = temp_buf[pi] as u32;
                let sg = temp_buf[pi + 1] as u32;
                let sb = temp_buf[pi + 2] as u32;
                let src_r = (255 + sr * mask_val) >> 8;
                let src_g = (255 + sg * mask_val) >> 8;
                let src_b = (255 + sb * mask_val) >> 8;
                buf[pi] = (src_r + ((255 + buf[pi] as u32 * inv_a) >> 8)).min(255) as u8;
                buf[pi + 1] = (src_g + ((255 + buf[pi + 1] as u32 * inv_a) >> 8)).min(255) as u8;
                buf[pi + 2] = (src_b + ((255 + buf[pi + 2] as u32 * inv_a) >> 8)).min(255) as u8;
                buf[pi + 3] = (a + ((255 + buf[pi + 3] as u32 * inv_a) >> 8)).min(255) as u8;
            }
        }
    }

    drop(rb);

    // Step 4: Render C++ slider control on top.
    {
        let pf2 = PixfmtRgba32::new(&mut ra);
        let mut rb2 = RendererBase::new(pf2);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut m_num = SliderCtrl::new(5.0, 5.0, 150.0, 12.0);
        m_num.range(5.0, 100.0);
        m_num.set_value(num_ellipses as f64);
        m_num.label("N=%.2f");
        render_ctrl(&mut ras, &mut sl, &mut rb2, &mut m_num);
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mol_view_sdf_parses_reference_molecule() {
        let molecules = parse_molecules_from_sdf(MOL_VIEW_SDF, 100);
        assert!(!molecules.is_empty());
        let first = &molecules[0];
        assert_eq!(first.name, "MFCD00133935");
        assert_eq!(first.atoms.len(), 89);
        assert_eq!(first.bonds.len(), 94);
        assert!(first.avr_len > 0.0);
    }

    #[test]
    fn mol_view_renders_non_empty_scene() {
        let img = mol_view(400, 400, &[0.0, 0.5, 0.5, 0.0, 1.0, 200.0, 200.0]);
        assert_eq!(img.len(), 400 * 400 * 4);
        // At least one pixel should differ from the white clear color.
        assert!(img
            .chunks_exact(4)
            .any(|px| px[0] != 255 || px[1] != 255 || px[2] != 255 || px[3] != 255));
    }

    #[test]
    fn alpha_mask3_default_scene_renders() {
        // Match C++ startup state: scenario=3 (GB + Spiral), operation=AND.
        let img = alpha_mask3(640, 520, &[3.0, 0.0, 320.0, 260.0]);
        assert_eq!(img.len(), 640 * 520 * 4);
        assert!(img
            .chunks_exact(4)
            .any(|px| px[0] != 255 || px[1] != 255 || px[2] != 255 || px[3] != 255));
    }
}

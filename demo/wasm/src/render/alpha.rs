//! Alpha/blending demo render functions: bspline, image_perspective, alpha_mask,
//! alpha_gradient, image_alpha, alpha_mask3, image_transforms, mol_view,
//! image_resample, alpha_mask2.

use agg_rust::bounding_rect::bounding_rect;
use agg_rust::basics::{VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
use agg_rust::bspline::Bspline;
use agg_rust::color::Rgba8;
use agg_rust::ctrl::{render_ctrl, SliderCtrl, CboxCtrl, RboxCtrl};
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLut;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_accessors::ImageAccessorClone;
use agg_rust::image_filters::{ImageFilterBilinear, ImageFilterLut};
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
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
use super::{setup_renderer, load_spheres_image};
use std::sync::OnceLock;


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

    // Draw quad overlay
    let mut quad_path = PathStorage::new();
    quad_path.move_to(quad_adj[0], quad_adj[1]);
    quad_path.line_to(quad_adj[2], quad_adj[3]);
    quad_path.line_to(quad_adj[4], quad_adj[5]);
    quad_path.line_to(quad_adj[6], quad_adj[7]);
    quad_path.close_polygon(0);
    ras.reset();
    ras.add_path(&mut quad_path, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(0, 76, 127, 153)); // rgba(0, 0.3, 0.5, 0.6)

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
/// Simplified from C++ alpha_mask.cpp (manual compositing instead of ScanlineU8Am).
pub fn alpha_mask_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let angle_rad = params.get(0).copied().unwrap_or(0.0);
    let scale = params.get(1).copied().unwrap_or(1.0).max(0.01);
    let skew_x = params.get(2).copied().unwrap_or(0.0);
    let skew_y = params.get(3).copied().unwrap_or(0.0);

    let w = width as f64;
    let h = height as f64;

    // Generate alpha mask: 10 random ellipses in grayscale
    let mask_size = (width * height) as usize;
    let mut mask = vec![0u8; mask_size];
    {
        // Simple deterministic "random" based on seed
        let mut seed = 1234u32;
        let rng = |s: &mut u32| -> u32 {
            *s = s.wrapping_mul(1103515245).wrapping_add(12345);
            (*s >> 16) & 0x7FFF
        };

        // Render 10 random ellipses into the mask
        // Each pixel in mask stores max alpha from any overlapping ellipse
        for _e in 0..10 {
            let cx = (rng(&mut seed) as f64 / 32767.0) * w;
            let cy = (rng(&mut seed) as f64 / 32767.0) * h;
            let rx = (rng(&mut seed) as f64 / 32767.0) * 100.0 + 20.0;
            let ry = (rng(&mut seed) as f64 / 32767.0) * 100.0 + 20.0;
            let alpha = (rng(&mut seed) & 0xFF) as u8;

            // Rasterize ellipse to mask using simple coverage
            let x_min = ((cx - rx) as i32).max(0);
            let x_max = ((cx + rx) as i32 + 1).min(width as i32 - 1);
            let y_min = ((cy - ry) as i32).max(0);
            let y_max = ((cy + ry) as i32 + 1).min(height as i32 - 1);

            for y in y_min..=y_max {
                for x in x_min..=x_max {
                    let dx = (x as f64 - cx) / rx;
                    let dy = (y as f64 - cy) / ry;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= 1.0 {
                        let idx = (y as u32 * width + x as u32) as usize;
                        let val = mask[idx] as u32 + alpha as u32;
                        mask[idx] = val.min(255) as u8;
                    }
                }
            }
        }
    }

    // Render lion to a temp buffer
    let (mut path, colors, path_idx) = crate::lion_data::parse_lion();
    let npaths = colors.len();

    let path_ids: Vec<u32> = path_idx.iter().map(|&i| i as u32).collect();
    let bbox = bounding_rect(&mut path, &path_ids, 0, npaths).unwrap_or(
        agg_rust::basics::RectD::new(0.0, 0.0, 250.0, 400.0),
    );
    let base_dx = (bbox.x1 + bbox.x2) / 2.0;
    let base_dy = (bbox.y1 + bbox.y2) / 2.0;

    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_translation(-base_dx, -base_dy));
    mtx.multiply(&TransAffine::new_scaling(scale, scale));
    mtx.multiply(&TransAffine::new_rotation(angle_rad + std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_skewing(skew_x / 1000.0, skew_y / 1000.0));
    mtx.multiply(&TransAffine::new_translation(w / 2.0, h / 2.0));

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Render lion with transform
    let mut transformed = ConvTransform::new(&mut path, mtx);
    for i in 0..npaths {
        let start = path_idx[i] as u32;
        ras.reset();
        ras.add_path(&mut transformed, start);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &colors[i]);
    }

    // Apply alpha mask: multiply each pixel's alpha by mask value
    for y in 0..height {
        for x in 0..width {
            let mask_val = mask[(y * width + x) as usize] as u32;
            let idx = ((y * width + x) * 4) as usize;
            // Multiply RGB by mask/255 (pre-multiply alpha effect)
            buf[idx] = ((buf[idx] as u32 * mask_val) / 255) as u8;
            buf[idx + 1] = ((buf[idx + 1] as u32 * mask_val) / 255) as u8;
            buf[idx + 2] = ((buf[idx + 2] as u32 * mask_val) / 255) as u8;
            buf[idx + 3] = ((buf[idx + 3] as u32 * mask_val) / 255) as u8;
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

    let a: Vec<f64> = (0..6).map(|i| params.get(6 + i).copied().unwrap_or(i as f64 / 5.0)).collect();

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

    // Build alpha LUT from 6 spline values using Bspline
    let alpha_lut: Vec<u8> = {
        let t_vals: Vec<f64> = (0..6).map(|i| i as f64 / 5.0).collect();
        let mut spline = Bspline::new();
        spline.init(&t_vals, &a);
        (0..256).map(|i| {
            let t = i as f64 / 255.0;
            let v = spline.get(t).clamp(0.0, 1.0);
            (v * 255.0) as u8
        }).collect()
    };

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

    // Draw parallelogram outline + vertex circles
    let mut para_path = PathStorage::new();
    para_path.move_to(x0, y0);
    para_path.line_to(x1, y1);
    para_path.line_to(x2, y2);
    para_path.close_polygon(0);
    let mut stroke = ConvStroke::new(&mut para_path);
    stroke.set_width(1.5);
    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 200));

    for &(vx, vy) in &[(x0, y0), (x1, y1), (x2, y2)] {
        let mut c = Ellipse::new(vx, vy, 5.0, 5.0, 20, false);
        ras.reset();
        ras.add_path(&mut c, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 200, 200, 255));
    }

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
/// Simplified from C++ alpha_mask3.cpp.
/// Params: [scenario, operation, mouse_x, mouse_y]
pub fn alpha_mask3(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;
    let scenario = params.get(0).copied().unwrap_or(0.0) as usize;
    let operation = params.get(1).copied().unwrap_or(0.0) as usize;
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

    let (mut path1, mut path2) = match scenario {
        0 => {
            let mut p1 = PathStorage::new();
            p1.move_to(100.0, 50.0);
            p1.line_to(300.0, 150.0);
            p1.line_to(50.0, 300.0);
            p1.close_polygon(0);
            p1.move_to(350.0, 50.0);
            p1.line_to(500.0, 200.0);
            p1.line_to(250.0, 350.0);
            p1.close_polygon(0);

            let mut p2 = PathStorage::new();
            p2.move_to(mx - 100.0, my - 80.0);
            p2.line_to(mx + 100.0, my - 80.0);
            p2.line_to(mx, my + 100.0);
            p2.close_polygon(0);
            (p1, p2)
        }
        1 => {
            let mut p1 = PathStorage::new();
            p1.move_to(100.0, 50.0);
            p1.line_to(500.0, 150.0);
            p1.line_to(200.0, 400.0);
            p1.close_polygon(0);

            let mut p2 = PathStorage::new();
            p2.move_to(mx - 80.0, my - 60.0);
            p2.line_to(mx + 80.0, my);
            p2.line_to(mx, my + 80.0);
            p2.close_polygon(0);
            (p1, p2)
        }
        2 => {
            let mut p1 = PathStorage::new();
            let cx1 = 250.0;
            let cy1 = 250.0;
            for i in 0..10 {
                let angle = std::f64::consts::PI * 2.0 * i as f64 / 10.0 - std::f64::consts::PI / 2.0;
                let r = if i % 2 == 0 { 150.0 } else { 60.0 };
                let px = cx1 + angle.cos() * r;
                let py = cy1 + angle.sin() * r;
                if i == 0 { p1.move_to(px, py); } else { p1.line_to(px, py); }
            }
            p1.close_polygon(0);

            let mut p2 = PathStorage::new();
            p2.move_to(mx - 120.0, my - 80.0);
            p2.line_to(mx + 120.0, my - 80.0);
            p2.line_to(mx + 120.0, my + 80.0);
            p2.line_to(mx - 120.0, my + 80.0);
            p2.close_polygon(0);
            (p1, p2)
        }
        3 => {
            let mut p1 = PathStorage::new();
            for i in 0..14 {
                let angle = std::f64::consts::PI * 2.0 * i as f64 / 14.0 - std::f64::consts::PI / 2.0;
                let r = if i % 2 == 0 { 180.0 } else { 70.0 };
                let px = 300.0 + angle.cos() * r;
                let py = 250.0 + angle.sin() * r;
                if i == 0 { p1.move_to(px, py); } else { p1.line_to(px, py); }
            }
            p1.close_polygon(0);

            let mut p2 = PathStorage::new();
            let n = 100;
            for i in 0..n {
                let t = i as f64 / n as f64;
                let angle = t * std::f64::consts::PI * 8.0;
                let r = 10.0 + t * 140.0;
                let px = mx + angle.cos() * r;
                let py = my + angle.sin() * r;
                if i == 0 { p2.move_to(px, py); } else { p2.line_to(px, py); }
            }
            (p1, p2)
        }
        _ => {
            let mut p1 = PathStorage::new();
            for i in 0..5 {
                let angle = std::f64::consts::PI * 2.0 * i as f64 / 5.0 - std::f64::consts::PI / 2.0;
                let px = 250.0 + angle.cos() * 150.0;
                let py = 250.0 + angle.sin() * 150.0;
                if i == 0 { p1.move_to(px, py); } else { p1.line_to(px, py); }
            }
            p1.close_polygon(0);

            let mut p2 = PathStorage::new();
            for i in 0..6 {
                let angle = std::f64::consts::PI * 2.0 * i as f64 / 6.0;
                let px = mx + angle.cos() * 100.0;
                let py = my + angle.sin() * 100.0;
                if i == 0 { p2.move_to(px, py); } else { p2.line_to(px, py); }
            }
            p2.close_polygon(0);
            (p1, p2)
        }
    };

    // Render alpha mask manually
    let mask_size = (width * height) as usize;
    let mut mask = vec![if operation == 0 { 0u8 } else { 255u8 }; mask_size];

    {
        let mut mask_buf: Vec<u8> = vec![0; (width * height * 4) as usize];
        let mut mask_ra = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { mask_ra.attach(mask_buf.as_mut_ptr(), width, height, stride) };
        let mask_pf = PixfmtRgba32::new(&mut mask_ra);
        let mut mask_rb = RendererBase::new(mask_pf);
        mask_rb.clear(&Rgba8::new(0, 0, 0, 0));

        let mut mask_ras = RasterizerScanlineAa::new();
        let mut mask_sl = ScanlineU8::new();
        mask_ras.add_path(&mut path1, 0);
        render_scanlines_aa_solid(&mut mask_ras, &mut mask_sl, &mut mask_rb,
            &Rgba8::new(255, 255, 255, 255));

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4 + 3) as usize;
                let coverage = mask_buf[idx];
                let mi = (y * width + x) as usize;
                if operation == 0 {
                    mask[mi] = coverage;
                } else {
                    mask[mi] = 255u8.saturating_sub(coverage);
                }
            }
        }
    }

    {
        let mut temp_buf: Vec<u8> = vec![0; (width * height * 4) as usize];
        let mut temp_ra = RowAccessor::new();
        let stride = (width * 4) as i32;
        unsafe { temp_ra.attach(temp_buf.as_mut_ptr(), width, height, stride) };
        let temp_pf = PixfmtRgba32::new(&mut temp_ra);
        let mut temp_rb = RendererBase::new(temp_pf);
        temp_rb.clear(&Rgba8::new(0, 0, 0, 0));

        let mut temp_ras = RasterizerScanlineAa::new();
        let mut temp_sl = ScanlineU8::new();
        temp_ras.add_path(&mut path2, 0);
        render_scanlines_aa_solid(&mut temp_ras, &mut temp_sl, &mut temp_rb,
            &Rgba8::new(127, 0, 0, 127));

        for y in 0..height {
            for x in 0..width {
                let mi = (y * width + x) as usize;
                let mask_val = mask[mi] as u32;
                let si = mi * 4;
                let sr = temp_buf[si] as u32;
                let sgreen = temp_buf[si + 1] as u32;
                let sb = temp_buf[si + 2] as u32;
                let salpha = (temp_buf[si + 3] as u32 * mask_val) / 255;

                if salpha > 0 {
                    let di = si;
                    let da = 255 - salpha;
                    buf[di] = ((sr * salpha + buf[di] as u32 * da) / 255) as u8;
                    buf[di + 1] = ((sgreen * salpha + buf[di + 1] as u32 * da) / 255) as u8;
                    buf[di + 2] = ((sb * salpha + buf[di + 2] as u32 * da) / 255) as u8;
                    buf[di + 3] = (salpha + (buf[di + 3] as u32 * da) / 255).min(255) as u8;
                }
            }
        }
    }

    // Draw path outlines
    let mut stroke1 = ConvStroke::new(&mut path1);
    stroke1.set_width(1.0);
    ras.reset();
    ras.add_path(&mut stroke1, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(127, 127, 0, 60));

    let mut stroke2 = ConvStroke::new(&mut path2);
    stroke2.set_width(1.0);
    ras.reset();
    ras.add_path(&mut stroke2, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 127, 127, 60));

    let mut m_polygons = RboxCtrl::new(5.0, 5.0, 210.0, 110.0);
    m_polygons.add_item("Two Simple Paths");
    m_polygons.add_item("Closed Stroke");
    m_polygons.add_item("Star + Rectangle");
    m_polygons.add_item("Star + Spiral");
    m_polygons.add_item("Pentagon + Hexagon");
    m_polygons.set_cur_item(scenario as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_polygons);

    let mut m_operation = RboxCtrl::new(w - 85.0, 5.0, w - 5.0, 55.0);
    m_operation.add_item("AND");
    m_operation.add_item("SUB");
    m_operation.set_cur_item(operation as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_operation);

    buf
}

/// Image transforms — star polygon textured with image through 7 transform modes.
/// Simplified from C++ image_transforms.cpp.
/// Params: [poly_angle, poly_scale, img_angle, img_scale, example_idx, img_cx, img_cy, poly_cx, poly_cy]
pub fn image_transforms_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let w = width as f64;
    let h = height as f64;

    let poly_angle = params.get(0).copied().unwrap_or(0.0);
    let poly_scale = params.get(1).copied().unwrap_or(1.0);
    let img_angle = params.get(2).copied().unwrap_or(0.0);
    let img_scale = params.get(3).copied().unwrap_or(1.0);
    let example_idx = params.get(4).copied().unwrap_or(1.0) as usize;
    let img_cx = params.get(5).copied().unwrap_or(w / 2.0);
    let img_cy = params.get(6).copied().unwrap_or(h / 2.0);
    let poly_cx = params.get(7).copied().unwrap_or(w / 2.0);
    let poly_cy = params.get(8).copied().unwrap_or(h / 2.0);

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

    // Build 14-point star polygon
    let r1 = 100.0;
    let r2 = 50.0;
    let mut star = PathStorage::new();
    for i in 0..14 {
        let angle = std::f64::consts::PI * 2.0 * i as f64 / 14.0 - std::f64::consts::PI / 2.0;
        let r = if i % 2 == 0 { r1 } else { r2 };
        let px = angle.cos() * r;
        let py = angle.sin() * r;
        if i == 0 { star.move_to(px, py); } else { star.line_to(px, py); }
    }
    star.close_polygon(0);

    let pa = poly_angle.to_radians();
    let mut poly_mtx = TransAffine::new();
    poly_mtx.multiply(&TransAffine::new_rotation(pa));
    poly_mtx.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
    poly_mtx.multiply(&TransAffine::new_translation(poly_cx, poly_cy));

    let mut image_mtx = match example_idx {
        0 => TransAffine::new(),
        1 => {
            let ia = poly_angle.to_radians();
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-poly_cx, -poly_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(poly_scale, poly_scale));
            m.multiply(&TransAffine::new_translation(poly_cx, poly_cy));
            m
        }
        2 => {
            let ia = img_angle.to_radians();
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            m.multiply(&TransAffine::new_translation(img_cx, img_cy));
            m
        }
        3 => {
            let ia = img_angle.to_radians();
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-poly_cx, -poly_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            m.multiply(&TransAffine::new_translation(poly_cx, poly_cy));
            m
        }
        4 => {
            let ia = img_angle.to_radians();
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            m.multiply(&TransAffine::new_translation(img_cx, img_cy));
            m
        }
        5 => {
            let ia = (poly_angle + img_angle).to_radians();
            let is = poly_scale * img_scale;
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(is, is));
            m.multiply(&TransAffine::new_translation(img_cx, img_cy));
            m
        }
        _ => {
            let ia = img_angle.to_radians();
            let mut m = TransAffine::new();
            m.multiply(&TransAffine::new_translation(-img_cx, -img_cy));
            m.multiply(&TransAffine::new_rotation(ia));
            m.multiply(&TransAffine::new_scaling(img_scale, img_scale));
            m.multiply(&TransAffine::new_translation(img_cx, img_cy));
            m
        }
    };
    image_mtx.invert();

    let mut transformed_star = ConvTransform::new(&mut star, poly_mtx);
    ras.reset();
    ras.add_path(&mut transformed_star, 0);

    let mut interp = SpanInterpolatorLinear::new(image_mtx);
    let bg = Rgba8::new(0, 0, 0, 0);
    let mut sg = SpanImageFilterRgbaBilinearClip::new(&img_ra, bg, &mut interp);
    render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

    let mut ic = Ellipse::new(img_cx, img_cy, 5.0, 5.0, 20, false);
    ras.reset();
    ras.add_path(&mut ic, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 0, 255));

    let mut pc = Ellipse::new(poly_cx, poly_cy, 3.0, 3.0, 20, false);
    ras.reset();
    ras.add_path(&mut pc, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 200, 200, 255));

    // Render controls
    let mut sl_pa = SliderCtrl::new(5.0, 5.0, 195.0, 12.0);
    sl_pa.label("Poly Angle=%.0f");
    sl_pa.range(-180.0, 180.0);
    sl_pa.set_value(poly_angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_pa);

    let mut sl_ps = SliderCtrl::new(5.0, 17.0, 195.0, 24.0);
    sl_ps.label("Poly Scale=%.2f");
    sl_ps.range(0.1, 5.0);
    sl_ps.set_value(poly_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_ps);

    let mut sl_ia = SliderCtrl::new(5.0, 29.0, 195.0, 36.0);
    sl_ia.label("Img Angle=%.0f");
    sl_ia.range(-180.0, 180.0);
    sl_ia.set_value(img_angle);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_ia);

    let mut sl_is = SliderCtrl::new(5.0, 41.0, 195.0, 48.0);
    sl_is.label("Img Scale=%.2f");
    sl_is.range(0.1, 5.0);
    sl_is.set_value(img_scale);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut sl_is);

    let mut m_example = RboxCtrl::new(w - 145.0, 5.0, w - 5.0, 100.0);
    m_example.add_item("No Image");
    m_example.add_item("Follow Polygon");
    m_example.add_item("Independent");
    m_example.add_item("Img to Poly Ctr");
    m_example.add_item("Both Indep");
    m_example.add_item("Double Rot+Scale");
    m_example.add_item("Around Img Ctr");
    m_example.set_cur_item(example_idx as i32);
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
/// Adapted from C++ alpha_mask2.cpp.
///
/// params[0] = num_mask_ellipses (5-200, default 50)
/// params[1] = rotation angle for lion (degrees)
/// params[2] = scale factor
pub fn alpha_mask2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let num_ellipses = params.get(0).copied().unwrap_or(50.0).clamp(5.0, 200.0) as u32;
    let angle = params.get(1).copied().unwrap_or(0.0);
    let scale = params.get(2).copied().unwrap_or(1.0).clamp(0.3, 3.0);

    let w = width as f64;
    let h = height as f64;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Step 1: Generate alpha mask buffer (grayscale)
    let mask_size = (width * height) as usize;
    let mut mask_buf = vec![0u8; mask_size * 4]; // RGBA for rendering
    {
        let mut mask_ra = RowAccessor::new();
        let mask_stride = (width * 4) as i32;
        unsafe { mask_ra.attach(mask_buf.as_mut_ptr(), width, height, mask_stride) };
        let mask_pf = PixfmtRgba32::new(&mut mask_ra);
        let mut mask_rb = RendererBase::new(mask_pf);
        mask_rb.clear(&Rgba8::new(0, 0, 0, 255));

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // Render random ellipses into mask
        for i in 0..num_ellipses {
            let hash = |seed: u32, off: u32| -> f64 {
                let v = (seed.wrapping_mul(2654435761).wrapping_add(off.wrapping_mul(2246822519))) >> 16;
                (v & 0xFFFF) as f64 / 65536.0
            };
            let cx = hash(i, 0) * w;
            let cy = hash(i, 1) * h;
            let rx = 10.0 + hash(i, 2) * 80.0;
            let ry = 10.0 + hash(i, 3) * 80.0;
            let alpha = (80 + (hash(i, 4) * 175.0) as u32).min(255);

            let mut ell = Ellipse::new(cx, cy, rx, ry, 32, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut mask_rb,
                &Rgba8::new(alpha, alpha, alpha, 255));
        }
    }

    // Extract grayscale mask from the rendered mask buffer
    let mut mask_gray = vec![0u8; mask_size];
    for i in 0..mask_size {
        mask_gray[i] = mask_buf[i * 4]; // R channel = grayscale value
    }

    // Step 2: Render lion into a temporary buffer
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
        let npaths = path_ids.len();
        let bbox = bounding_rect(&mut path, &path_ids, 0, npaths).unwrap_or(
            agg_rust::basics::RectD::new(0.0, 0.0, 250.0, 400.0),
        );
        let lion_cx = (bbox.x1 + bbox.x2) / 2.0;
        let lion_cy = (bbox.y1 + bbox.y2) / 2.0;

        let mut mtx = TransAffine::new();
        mtx.translate(-lion_cx, -lion_cy);
        mtx.scale(scale, scale);
        mtx.rotate(angle.to_radians());
        mtx.translate(w / 2.0, h / 2.0);

        let mut conv = ConvTransform::new(&mut path, mtx);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        for i in 0..path_idx.len() {
            ras.reset();
            ras.add_path(&mut conv, path_idx[i] as u32);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut temp_rb, &colors[i]);
        }
    }

    // Step 3: Composite lion onto main buffer using alpha mask
    for y in 0..height as usize {
        for x in 0..width as usize {
            let idx = y * width as usize + x;
            let pi = idx * 4;
            let mask_val = mask_gray[idx] as u32;
            let sr = temp_buf[pi] as u32;
            let sg = temp_buf[pi + 1] as u32;
            let sb = temp_buf[pi + 2] as u32;
            let sa = temp_buf[pi + 3] as u32;
            if sa > 0 && mask_val > 0 {
                // Modulate source alpha by mask
                let a = (sa * mask_val) / 255;
                let inv_a = 255 - a;
                buf[pi]     = ((sr * a + buf[pi] as u32 * inv_a) / 255) as u8;
                buf[pi + 1] = ((sg * a + buf[pi + 1] as u32 * inv_a) / 255) as u8;
                buf[pi + 2] = ((sb * a + buf[pi + 2] as u32 * inv_a) / 255) as u8;
                buf[pi + 3] = (a + (buf[pi + 3] as u32 * inv_a) / 255).min(255) as u8;
            }
        }
    }

    // Drop the first renderer to release borrow on ra
    drop(rb);

    // Step 4: Render gradient circles + label
    {
        let pf2 = PixfmtRgba32::new(&mut ra);
        let mut rb2 = RendererBase::new(pf2);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        for i in 0..5u32 {
            let hash_f = |seed: u32, off: u32| -> f64 {
                let v = (seed.wrapping_mul(2654435761).wrapping_add(off.wrapping_mul(374761393))) >> 16;
                (v & 0xFFFF) as f64 / 65536.0
            };
            let cx = 50.0 + hash_f(i + 100, 0) * (w - 100.0);
            let cy = 50.0 + hash_f(i + 100, 1) * (h - 100.0);
            let r = 30.0 + hash_f(i + 100, 2) * 60.0;

            let mut ell = Ellipse::new(cx, cy, r, r, 32, false);
            ras.reset();
            ras.add_path(&mut ell, 0);
            let color = Rgba8::new(
                (hash_f(i + 100, 3) * 255.0) as u32,
                (hash_f(i + 100, 4) * 255.0) as u32,
                (hash_f(i + 100, 5) * 255.0) as u32,
                180,
            );
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb2, &color);
        }

        // Draw label
        let label = format!("Alpha Mask 2: {} ellipses, angle={:.0}, scale={:.2}",
            num_ellipses, angle, scale);
        let mut txt = GsvText::new();
        txt.size(8.0, 0.0);
        txt.start_point(5.0, h - 15.0);
        txt.text(&label);
        let mut txt_stroke = ConvStroke::new(&mut txt);
        txt_stroke.set_width(0.8);
        ras.reset();
        ras.add_path(&mut txt_stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb2, &Rgba8::new(0, 0, 0, 255));
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
}

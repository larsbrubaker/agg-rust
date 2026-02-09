//! Image-related demo render functions: image_fltr_graph, image1, image_filters,
//! gradient_focal, idea, graph_test, gamma_tuner, image_filters2, conv_dash_marker,
//! aa_test.

use agg_rust::color::Rgba8;
use agg_rust::curves::Curve4Div;
use agg_rust::ctrl::{render_ctrl, SliderCtrl, CboxCtrl, RboxCtrl};
use agg_rust::conv_curve::ConvCurve;
use agg_rust::conv_dash::ConvDash;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::conv_transform::ConvTransform;
use agg_rust::ellipse::Ellipse;
use agg_rust::gradient_lut::GradientLut;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_accessors::ImageAccessorClone;
use agg_rust::image_filters::{
    ImageFilterBilinear, ImageFilterBicubic, ImageFilterSpline16, ImageFilterSpline36,
    ImageFilterHanning, ImageFilterHamming, ImageFilterHermite, ImageFilterKaiser,
    ImageFilterQuadric, ImageFilterCatrom, ImageFilterGaussian, ImageFilterBessel,
    ImageFilterMitchell, ImageFilterSinc, ImageFilterLanczos, ImageFilterBlackman,
    ImageFilterFunction, ImageFilterLut, IMAGE_FILTER_SCALE,
};
use agg_rust::math_stroke::LineCap;
use agg_rust::path_storage::PathStorage;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid, SpanGenerator};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_gouraud_rgba::SpanGouraudRgba;
use agg_rust::span_gradient::{
    GradientRadial, GradientRadialFocus, GradientReflectAdaptor, GradientX,
    SpanGradient,
};
use agg_rust::span_image_filter_rgba::{
    SpanImageFilterRgbaBilinearClip, SpanImageFilterRgbaNn,
    SpanImageFilterRgbaGen,
};
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;
use super::{setup_renderer, load_spheres_image};

// ============================================================================
// Image Filter Graph — filter kernel visualization
// ============================================================================

/// Render image filter weight function graphs.
/// Matches C++ image_fltr_graph.cpp — select a filter, see its kernel shape.
///
/// params[0] = filter index (0–15, default 0)
/// params[1] = radius for variable-radius filters (default 4.0)
pub fn image_fltr_graph(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    // params[0] = radius (2.0-8.0, default 4.0)
    // params[1..17] = 16 checkbox states (0 or 1)
    let radius = params.get(0).copied().unwrap_or(4.0).clamp(2.0, 8.0);
    let mut enabled = [false; 16];
    for i in 0..16 {
        enabled[i] = params.get(1 + i).copied().unwrap_or(0.0) > 0.5;
    }

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Graph area matching C++ exactly
    let x_start = 125.0_f64;
    let x_end = 780.0 - 15.0; // 765.0
    let y_start = 10.0_f64;
    let y_end = 300.0 - 10.0; // 290.0
    let ys = y_start + (y_end - y_start) / 6.0;
    let dy = y_end - ys;

    // Grid lines — 17 vertical lines (16 divisions)
    for i in 0..=16 {
        let x = x_start + (x_end - x_start) * i as f64 / 16.0;
        let mut line = PathStorage::new();
        line.move_to(x, y_start);
        line.line_to(x, y_end);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        let alpha = if i == 8 { 255u32 } else { 100u32 };
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, alpha));
    }

    // Horizontal baseline
    {
        let mut line = PathStorage::new();
        line.move_to(x_start, ys);
        line.line_to(x_end, ys);
        let mut stroke = ConvStroke::new(&mut line);
        stroke.set_width(1.0);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Filter names for checkboxes
    let filter_names = [
        "bilinear", "bicubic", "spline16", "spline36",
        "hanning", "hamming", "hermite", "kaiser",
        "quadric", "catrom", "gaussian", "bessel",
        "mitchell", "sinc", "lanczos", "blackman",
    ];

    // Helper: plot weight, cumulative, and normalized LUT curves for a filter
    macro_rules! plot_filter_curves {
        ($filter:expr, $filt_radius:expr) => {{
            let r = $filt_radius;

            // Curve 1: Weight function (dark red)
            {
                let n = (r * 256.0 * 2.0) as usize;
                let xs = (x_end + x_start) / 2.0 - (r * (x_end - x_start) / 16.0);
                let dx = (x_end - x_start) * r / 8.0;
                let mut path = PathStorage::new();
                let w0 = $filter.calc_weight(-r);
                path.move_to(xs + 0.5, ys + dy * w0);
                for j in 1..n {
                    let xf = j as f64 / 256.0 - r;
                    let w = $filter.calc_weight(xf);
                    path.line_to(xs + dx * j as f64 / n as f64 + 0.5, ys + dy * w);
                }
                let mut stroke = ConvStroke::new(&mut path);
                stroke.set_width(1.5);
                ras.reset();
                ras.add_path(&mut stroke, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(128, 0, 0, 255));
            }

            // Curve 2: Cumulative/integrated (dark green)
            {
                let ir = (r.ceil() + 0.1) as i32;
                let mut path = PathStorage::new();
                let x_center = (x_start + x_end) / 2.0;
                for xint in 0..=255usize {
                    let mut sum = 0.0_f64;
                    for xfract in (-ir)..ir {
                        let xf = xint as f64 / 256.0 + xfract as f64;
                        if xf >= -r && xf <= r {
                            sum += $filter.calc_weight(xf);
                        }
                    }
                    let x = x_center + ((-128.0 + xint as f64) / 128.0) * r * (x_end - x_start) / 16.0;
                    let y = ys + sum * 256.0 - 256.0;
                    if xint == 0 {
                        path.move_to(x, y);
                    } else {
                        path.line_to(x, y);
                    }
                }
                let mut stroke = ConvStroke::new(&mut path);
                stroke.set_width(1.5);
                ras.reset();
                ras.add_path(&mut stroke, 0);
                render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 128, 0, 255));
            }

            // Curve 3: Normalized LUT (dark blue)
            {
                let mut lut = ImageFilterLut::new();
                lut.calculate(&$filter, true);
                let weights = lut.weight_array();
                let nn = lut.diameter() as usize * 256;
                let lut_r = lut.diameter() as f64 / 2.0;
                let xs = (x_end + x_start) / 2.0 - (lut_r * (x_end - x_start) / 16.0);
                let dx = (x_end - x_start) * lut_r / 8.0;
                let mut path = PathStorage::new();
                let scale = IMAGE_FILTER_SCALE as f64;
                if !weights.is_empty() && nn > 0 {
                    path.move_to(xs + 0.5, ys + dy * weights[0] as f64 / scale);
                    let actual_nn = nn.min(weights.len());
                    for j in 1..actual_nn {
                        let w = weights[j] as f64 / scale;
                        path.line_to(xs + dx * j as f64 / nn as f64 + 0.5, ys + dy * w);
                    }
                    let mut stroke = ConvStroke::new(&mut path);
                    stroke.set_width(1.5);
                    ras.reset();
                    ras.add_path(&mut stroke, 0);
                    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 128, 255));
                }
            }
        }};
    }

    // For each enabled filter, plot its 3 curves
    for i in 0..16 {
        if !enabled[i] {
            continue;
        }
        match i {
            0 => plot_filter_curves!(ImageFilterBilinear, ImageFilterBilinear.radius()),
            1 => plot_filter_curves!(ImageFilterBicubic, ImageFilterBicubic.radius()),
            2 => plot_filter_curves!(ImageFilterSpline16, ImageFilterSpline16.radius()),
            3 => plot_filter_curves!(ImageFilterSpline36, ImageFilterSpline36.radius()),
            4 => plot_filter_curves!(ImageFilterHanning, ImageFilterHanning.radius()),
            5 => plot_filter_curves!(ImageFilterHamming, ImageFilterHamming.radius()),
            6 => plot_filter_curves!(ImageFilterHermite, ImageFilterHermite.radius()),
            7 => plot_filter_curves!(ImageFilterKaiser::new(5.0), ImageFilterKaiser::new(5.0).radius()),
            8 => plot_filter_curves!(ImageFilterQuadric, ImageFilterQuadric.radius()),
            9 => plot_filter_curves!(ImageFilterCatrom, ImageFilterCatrom.radius()),
            10 => plot_filter_curves!(ImageFilterGaussian, ImageFilterGaussian.radius()),
            11 => plot_filter_curves!(ImageFilterBessel, ImageFilterBessel.radius()),
            12 => {
                let f = ImageFilterMitchell::new(1.0 / 3.0, 1.0 / 3.0);
                plot_filter_curves!(f, f.radius());
            }
            13 => {
                let f = ImageFilterSinc::new(radius);
                plot_filter_curves!(f, radius);
            }
            14 => {
                let f = ImageFilterLanczos::new(radius);
                plot_filter_curves!(f, radius);
            }
            15 => {
                let f = ImageFilterBlackman::new(radius);
                plot_filter_curves!(f, radius);
            }
            _ => {}
        }
    }

    // Render AGG controls matching C++ image_fltr_graph.cpp
    // Slider at top
    let mut s_radius = SliderCtrl::new(5.0, 5.0, 775.0, 15.0);
    s_radius.label("Radius=%.3f");
    s_radius.range(2.0, 8.0);
    s_radius.set_value(radius);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);

    // 16 checkboxes along the left
    for i in 0..16 {
        let y = 30.0 + 15.0 * i as f64;
        let mut cb = CboxCtrl::new(8.0, y, filter_names[i]);
        cb.text_size(7.0, 0.0);
        cb.set_status(enabled[i]);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut cb);
    }

    buf
}

// ============================================================================
// Image1 — Image Affine Transformations with Bilinear Filtering
// ============================================================================

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
// Image Filters — iterative rotation showing filter quality degradation
// Matches C++ image_filters.cpp
// ============================================================================

/// Render image filters comparison demo.
///
/// params[0] = filter_idx (0-16: nn, bilinear, bicubic, spline16, spline36,
///             hanning, hamming, hermite, kaiser, quadric, catrom, gaussian,
///             bessel, mitchell, sinc, lanczos, blackman)
/// params[1] = step_degrees (1.0-10.0, default 5.0)
/// params[2] = normalize (0 or 1, default 1)
/// params[3] = radius (2.0-8.0, for sinc/lanczos/blackman)
/// params[4] = num_steps (how many rotations to apply iteratively)
pub fn image_filters_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let filter_idx = params.get(0).copied().unwrap_or(1.0) as usize;
    let step_deg = params.get(1).copied().unwrap_or(5.0).clamp(1.0, 10.0);
    let normalize = params.get(2).copied().unwrap_or(1.0) > 0.5;
    let radius = params.get(3).copied().unwrap_or(4.0).clamp(2.0, 8.0);
    let num_steps = params.get(4).copied().unwrap_or(0.0).max(0.0) as usize;
    let kpix_sec = params.get(5).copied().unwrap_or(0.0);

    // Load spheres image
    let (img_w, img_h, original) = load_spheres_image();
    let img_stride = (img_w * 4) as i32;

    let iw = img_w as f64;
    let ih = img_h as f64;

    // Working buffers: src and dst (like C++ img[1] and img[0])
    let mut src_data = original;
    let mut dst_data = vec![255u8; (img_w * img_h * 4) as usize];

    // Apply initial transform_image(0.0) — C++ does this at startup
    // Then apply num_steps of transform_image(step_deg)
    let total_transforms = 1 + num_steps;

    for step in 0..total_transforms {
        let angle_deg = if step == 0 { 0.0 } else { step_deg };
        let angle_rad = angle_deg * std::f64::consts::PI / 180.0;

        // Clear destination to white
        for chunk in dst_data.chunks_exact_mut(4) {
            chunk[0] = 255;
            chunk[1] = 255;
            chunk[2] = 255;
            chunk[3] = 255;
        }

        let mut src_ra = RowAccessor::new();
        unsafe { src_ra.attach(src_data.as_mut_ptr(), img_w, img_h, img_stride) };

        let mut dst_ra = RowAccessor::new();
        unsafe { dst_ra.attach(dst_data.as_mut_ptr(), img_w, img_h, img_stride) };

        // Transform matrices — matching C++ transform_image()
        let mut src_mtx = TransAffine::new();
        src_mtx.multiply(&TransAffine::new_translation(-iw / 2.0, -ih / 2.0));
        src_mtx.multiply(&TransAffine::new_rotation(angle_rad));
        src_mtx.multiply(&TransAffine::new_translation(iw / 2.0, ih / 2.0));

        let mut img_mtx = src_mtx;
        img_mtx.invert();

        // Clipping ellipse — transformed by src_mtx, matching C++
        let mut r = iw;
        if ih < r { r = ih; }
        r *= 0.5;
        r -= 4.0;
        let mut ell = Ellipse::new(iw / 2.0, ih / 2.0, r, r, 200, false);
        let mut tr = ConvTransform::new(&mut ell, src_mtx);

        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();
        let mut alloc = SpanAllocator::<Rgba8>::new();

        ras.add_path(&mut tr, 0);

        let mut interpolator = SpanInterpolatorLinear::new(img_mtx);

        match filter_idx {
            0 => {
                use agg_rust::image_accessors::ImageAccessorClip;
                let mut accessor = ImageAccessorClip::<4>::new(&src_ra, &[0, 0, 0, 0]);
                let mut sg = SpanImageFilterRgbaNn::new(&mut accessor, &mut interpolator);
                let pf = PixfmtRgba32::new(&mut dst_ra);
                let mut rb = RendererBase::new(pf);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
            }
            1 => {
                let mut sg = SpanImageFilterRgbaBilinearClip::new(
                    &src_ra,
                    Rgba8::new(0, 0, 0, 0),
                    &mut interpolator,
                );
                let pf = PixfmtRgba32::new(&mut dst_ra);
                let mut rb = RendererBase::new(pf);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
            }
            5 | 6 | 7 => {
                use agg_rust::image_accessors::ImageAccessorClip;
                use agg_rust::span_image_filter_rgba::SpanImageFilterRgba2x2;
                let mut lut = ImageFilterLut::new();
                match filter_idx {
                    5 => lut.calculate(&ImageFilterHanning, normalize),
                    6 => lut.calculate(&ImageFilterHamming, normalize),
                    7 => lut.calculate(&ImageFilterHermite, normalize),
                    _ => unreachable!(),
                }
                let mut accessor = ImageAccessorClip::<4>::new(&src_ra, &[0, 0, 0, 0]);
                let mut sg = SpanImageFilterRgba2x2::new(&mut accessor, &mut interpolator, &lut);
                let pf = PixfmtRgba32::new(&mut dst_ra);
                let mut rb = RendererBase::new(pf);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
            }
            _ => {
                use agg_rust::image_accessors::ImageAccessorClip;
                let mut lut = ImageFilterLut::new();
                match filter_idx {
                    2 => lut.calculate(&ImageFilterBicubic, normalize),
                    3 => lut.calculate(&ImageFilterSpline16, normalize),
                    4 => lut.calculate(&ImageFilterSpline36, normalize),
                    8 => lut.calculate(&ImageFilterKaiser::new(6.33), normalize),
                    9 => lut.calculate(&ImageFilterQuadric, normalize),
                    10 => lut.calculate(&ImageFilterCatrom, normalize),
                    11 => lut.calculate(&ImageFilterGaussian, normalize),
                    12 => lut.calculate(&ImageFilterBessel, normalize),
                    13 => lut.calculate(&ImageFilterMitchell::new(1.0 / 3.0, 1.0 / 3.0), normalize),
                    14 => lut.calculate(&ImageFilterSinc::new(radius), normalize),
                    15 => lut.calculate(&ImageFilterLanczos::new(radius), normalize),
                    16 => lut.calculate(&ImageFilterBlackman::new(radius), normalize),
                    _ => lut.calculate(&ImageFilterBicubic, normalize),
                }
                let mut accessor = ImageAccessorClip::<4>::new(&src_ra, &[0, 0, 0, 0]);
                let mut sg = SpanImageFilterRgbaGen::new(&mut accessor, &mut interpolator, &lut);
                let pf = PixfmtRgba32::new(&mut dst_ra);
                let mut rb = RendererBase::new(pf);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
            }
        }

        // Swap: dst becomes new src for next step
        std::mem::swap(&mut src_data, &mut dst_data);
    }

    // Set up output buffer
    let stride = (width * 4) as i32;
    let mut buf = vec![255u8; (width * height * 4) as usize];
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    // Copy image to output at (110, 35) — matching C++ on_draw rb.copy_from
    for y in 0..img_h.min(height.saturating_sub(35)) {
        for x in 0..img_w.min(width.saturating_sub(110)) {
            let si = ((y * img_w + x) * 4) as usize;
            let di = (((y + 35) * width + (x + 110)) * 4) as usize;
            if si + 3 < src_data.len() && di + 3 < buf.len() {
                buf[di] = src_data[si];
                buf[di + 1] = src_data[si + 1];
                buf[di + 2] = src_data[si + 2];
                buf[di + 3] = src_data[si + 3];
            }
        }
    }

    // Render controls — matching C++ on_draw exactly
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // NSteps text
    {
        let label = format!("NSteps={}", num_steps);
        let mut txt = GsvText::new();
        txt.size(10.0, 0.0);
        txt.start_point(10.0, 295.0);
        txt.text(&label);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(1.5);
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Kpix/sec text — matching C++ position at (10, 310)
    if kpix_sec > 0.0 {
        let label = format!("{:.2} Kpix/sec", kpix_sec);
        let mut txt = GsvText::new();
        txt.size(10.0, 0.0);
        txt.start_point(10.0, 310.0);
        txt.text(&label);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(1.5);
        ras.reset();
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
    }

    // Radius slider (only for sinc/lanczos/blackman) — rendered before step slider
    // matching C++ render order
    if filter_idx >= 14 {
        let mut s_radius = SliderCtrl::new(115.0, 20.0, 400.0, 26.0);
        s_radius.label("Filter Radius=%.3f");
        s_radius.range(2.0, 8.0);
        s_radius.set_value(radius);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);
    }

    // Step slider
    let mut s_step = SliderCtrl::new(115.0, 5.0, 400.0, 11.0);
    s_step.label("Step=%3.2f");
    s_step.range(1.0, 10.0);
    s_step.set_value(step_deg);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_step);

    // Filter selection radio box
    let filter_names = [
        "simple (NN)", "bilinear", "bicubic", "spline16", "spline36",
        "hanning", "hamming", "hermite", "kaiser", "quadric", "catrom",
        "gaussian", "bessel", "mitchell", "sinc", "lanczos", "blackman",
    ];
    let mut r_filters = RboxCtrl::new(0.0, 0.0, 110.0, 210.0);
    r_filters.border_width(0.0, 0.0);
    r_filters.background_color(Rgba8::new(0, 0, 0, 26)); // rgba(0,0,0,0.1)
    r_filters.text_size(6.0, 0.0);
    r_filters.text_thickness(0.85);
    for name in &filter_names {
        r_filters.add_item(name);
    }
    r_filters.set_cur_item(filter_idx.min(16) as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_filters);

    // Checkboxes — matching C++ order and positions
    let mut c_run = CboxCtrl::new(8.0, 245.0, "RUN Test!");
    c_run.text_size(7.5, 0.0);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_run);

    let mut c_norm = CboxCtrl::new(8.0, 215.0, "Normalize Filter");
    c_norm.text_size(7.5, 0.0);
    c_norm.set_status(normalize);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_norm);

    let mut c_single = CboxCtrl::new(8.0, 230.0, "Single Step");
    c_single.text_size(7.5, 0.0);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_single);

    let mut c_refresh = CboxCtrl::new(8.0, 265.0, "Refresh");
    c_refresh.text_size(7.5, 0.0);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_refresh);

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

    // Exact polygon data from C++ idea.cpp

    // Lightbulb outline — 20 points
    let poly_bulb: &[(f64, f64)] = &[
        (-6.0,-67.0), (-6.0,-71.0), (-7.0,-74.0), (-8.0,-76.0), (-10.0,-79.0),
        (-10.0,-82.0), (-9.0,-84.0), (-6.0,-86.0), (-4.0,-87.0), (-2.0,-86.0),
        (-1.0,-86.0), (1.0,-84.0), (2.0,-82.0), (2.0,-79.0), (0.0,-77.0),
        (-2.0,-73.0), (-2.0,-71.0), (-2.0,-69.0), (-3.0,-67.0), (-4.0,-65.0),
    ];

    // Light beams — 5 points each
    let poly_beam1: &[(f64, f64)] = &[
        (-14.0,-84.0), (-22.0,-85.0), (-23.0,-87.0), (-22.0,-88.0), (-21.0,-88.0),
    ];
    let poly_beam2: &[(f64, f64)] = &[
        (-10.0,-92.0), (-14.0,-96.0), (-14.0,-98.0), (-12.0,-99.0), (-11.0,-97.0),
    ];
    let poly_beam3: &[(f64, f64)] = &[
        (-1.0,-92.0), (-2.0,-98.0), (0.0,-100.0), (2.0,-100.0), (1.0,-98.0),
    ];
    let poly_beam4: &[(f64, f64)] = &[
        (5.0,-89.0), (11.0,-94.0), (13.0,-93.0), (13.0,-92.0), (12.0,-91.0),
    ];

    // Figure parts — complex polygons
    let poly_fig1: &[(f64, f64)] = &[
        (1.0,-48.0), (-3.0,-54.0), (-7.0,-58.0), (-12.0,-58.0), (-17.0,-55.0),
        (-20.0,-52.0), (-21.0,-47.0), (-20.0,-40.0), (-17.0,-33.0), (-11.0,-28.0),
        (-6.0,-26.0), (-2.0,-25.0), (2.0,-26.0), (4.0,-28.0), (5.0,-33.0),
        (5.0,-39.0), (3.0,-44.0), (12.0,-48.0), (12.0,-50.0), (12.0,-51.0),
        (3.0,-46.0),
    ];
    let poly_fig2: &[(f64, f64)] = &[
        (11.0,-27.0), (6.0,-23.0), (4.0,-22.0), (3.0,-19.0), (5.0,-16.0),
        (6.0,-15.0), (11.0,-17.0), (19.0,-23.0), (25.0,-30.0), (32.0,-38.0),
        (32.0,-41.0), (32.0,-50.0), (30.0,-64.0), (32.0,-72.0), (32.0,-75.0),
        (31.0,-77.0), (28.0,-78.0), (26.0,-80.0), (28.0,-87.0), (27.0,-89.0),
        (25.0,-88.0), (24.0,-79.0), (24.0,-76.0), (23.0,-75.0), (20.0,-76.0),
        (17.0,-76.0), (17.0,-74.0), (19.0,-73.0), (22.0,-73.0), (24.0,-71.0),
        (26.0,-69.0), (27.0,-64.0), (28.0,-55.0), (28.0,-47.0), (28.0,-40.0),
        (26.0,-38.0), (20.0,-33.0), (14.0,-30.0),
    ];
    let poly_fig3: &[(f64, f64)] = &[
        (-6.0,-20.0), (-9.0,-21.0), (-15.0,-21.0), (-20.0,-17.0), (-28.0,-8.0),
        (-32.0,-1.0), (-32.0,1.0), (-30.0,6.0), (-26.0,8.0), (-20.0,10.0),
        (-16.0,12.0), (-14.0,14.0), (-15.0,16.0), (-18.0,20.0), (-22.0,20.0),
        (-25.0,19.0), (-27.0,20.0), (-26.0,22.0), (-23.0,23.0), (-18.0,23.0),
        (-14.0,22.0), (-11.0,20.0), (-10.0,17.0), (-9.0,14.0), (-11.0,11.0),
        (-16.0,9.0), (-22.0,8.0), (-26.0,5.0), (-28.0,2.0), (-27.0,-2.0),
        (-23.0,-8.0), (-19.0,-11.0), (-12.0,-14.0), (-6.0,-15.0), (-6.0,-18.0),
    ];
    let poly_fig4: &[(f64, f64)] = &[
        (11.0,-6.0), (8.0,-16.0), (5.0,-21.0), (-1.0,-23.0), (-7.0,-22.0),
        (-10.0,-17.0), (-9.0,-10.0), (-8.0,0.0), (-8.0,10.0), (-10.0,18.0),
        (-11.0,22.0), (-10.0,26.0), (-7.0,28.0), (-3.0,30.0), (0.0,31.0),
        (5.0,31.0), (10.0,27.0), (14.0,18.0), (14.0,11.0), (11.0,2.0),
    ];
    let poly_fig5: &[(f64, f64)] = &[
        (0.0,22.0), (-5.0,21.0), (-8.0,22.0), (-9.0,26.0), (-8.0,49.0),
        (-8.0,54.0), (-10.0,64.0), (-10.0,75.0), (-9.0,81.0), (-10.0,84.0),
        (-16.0,89.0), (-18.0,95.0), (-18.0,97.0), (-13.0,100.0), (-12.0,99.0),
        (-12.0,95.0), (-10.0,90.0), (-8.0,87.0), (-6.0,86.0), (-4.0,83.0),
        (-3.0,82.0), (-5.0,80.0), (-6.0,79.0), (-7.0,74.0), (-6.0,63.0),
        (-3.0,52.0), (0.0,42.0), (1.0,31.0),
    ];
    let poly_fig6: &[(f64, f64)] = &[
        (12.0,31.0), (12.0,24.0), (8.0,21.0), (3.0,21.0), (2.0,24.0),
        (3.0,30.0), (5.0,40.0), (8.0,47.0), (10.0,56.0), (11.0,64.0),
        (11.0,71.0), (10.0,76.0), (8.0,77.0), (8.0,79.0), (10.0,81.0),
        (13.0,82.0), (17.0,82.0), (26.0,84.0), (28.0,87.0), (32.0,86.0),
        (33.0,81.0), (32.0,80.0), (25.0,79.0), (17.0,79.0), (14.0,79.0),
        (13.0,76.0), (14.0,72.0), (14.0,64.0), (13.0,55.0), (12.0,44.0),
        (12.0,34.0),
    ];

    // Group paths by attribute: (polygons, attr_index)
    let groups: [(&[&[(f64, f64)]], usize); 3] = [
        (&[poly_bulb], 0),
        (&[poly_beam1, poly_beam2, poly_beam3, poly_beam4], 1),
        (&[poly_fig1, poly_fig2, poly_fig3, poly_fig4, poly_fig5, poly_fig6], 2),
    ];

    // Transform chain matches C++ idea.cpp exactly:
    // mtx *= trans_affine_rotation(g_angle * pi / 180.0);
    // mtx *= trans_affine_translation(m_dx / 2, m_dy / 2 + 10);
    // mtx *= trans_affine_scaling(width / m_dx, height / m_dy);
    // m_dx/m_dy are initial window size (250x280), so at initial size scaling = (1,1)
    let m_dx = 250.0_f64;
    let m_dy = 280.0_f64;
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_rotation(angle_deg * std::f64::consts::PI / 180.0));
    mtx.multiply(&TransAffine::new_translation(m_dx / 2.0, m_dy / 2.0 + 10.0));
    mtx.multiply(&TransAffine::new_scaling(w / m_dx, h / m_dy));

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
                if roundoff {
                    // In roundoff mode, path already has transformed+rounded coords
                    let mut stroke = ConvStroke::new(&mut path);
                    stroke.set_width(attr.stroke_width);
                    ras.reset();
                    ras.add_path(&mut stroke, 0);
                } else {
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
                }
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

/// Gamma tuner — gradient background + alpha pattern with gamma correction.
/// Matches C++ gamma_tuner.cpp.
pub fn gamma_tuner(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let gamma_val = params.get(0).copied().unwrap_or(2.2);
    let r_val = params.get(1).copied().unwrap_or(1.0);
    let g_val = params.get(2).copied().unwrap_or(1.0);
    let b_val = params.get(3).copied().unwrap_or(1.0);
    let pattern = params.get(4).copied().unwrap_or(2.0) as usize; // 0=horiz, 1=vert, 2=checkered

    let w = width as i32;
    let h = height as i32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    let square_size = 400i32;
    let x_start = 50i32;
    let y_start = 80i32;

    // User color (from sliders, 0..1)
    let color = Rgba8::new(
        (r_val.clamp(0.0, 1.0) * 255.0) as u32,
        (g_val.clamp(0.0, 1.0) * 255.0) as u32,
        (b_val.clamp(0.0, 1.0) * 255.0) as u32,
        255,
    );
    let black = Rgba8::new(0, 0, 0, 255);

    // Background gradient (full height)
    for i in 0..h {
        let mut k = if i >= y_start && i < y_start + square_size {
            (i - y_start) as f64 / (square_size - 1) as f64
        } else if i < y_start {
            0.0
        } else {
            1.0
        };
        k = k.clamp(0.0, 1.0);
        k = 1.0 - (k / 2.0).powf(1.0 / gamma_val);
        let c = color.gradient(&black, 1.0 - k);
        rb.copy_hline(0, i, w - 1, &c);
    }

    // Black square background
    rb.blend_bar(x_start, y_start, x_start + square_size - 1, y_start + square_size - 1, &black, 255);

    // Alpha pattern
    let mut span1 = vec![Rgba8::new(0, 0, 0, 0); square_size as usize];
    let mut span2 = vec![Rgba8::new(0, 0, 0, 0); square_size as usize];

    for i in (0..square_size).step_by(2) {
        // Compute gradient color for this row
        let k_row = i as f64 / (square_size - 1) as f64;
        let k_color = 1.0 - k_row.powf(1.0 / gamma_val);
        let c = color.gradient(&black, 1.0 - k_color);

        // Set up spans based on pattern
        for j in 0..square_size as usize {
            let alpha_fwd = (j as u32 * 255 / square_size as u32) as u8;
            let alpha_rev = 255 - alpha_fwd;

            match pattern {
                0 => {
                    // Horizontal: span1 = increasing alpha, span2 = decreasing
                    span1[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_fwd };
                    span2[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_rev };
                }
                1 => {
                    // Vertical: odd columns = increasing, even = decreasing
                    let a = if (j & 1) != 0 { alpha_fwd } else { alpha_rev };
                    span1[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a };
                    span2[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a };
                }
                _ => {
                    // Checkered: alternate odd/even columns between spans
                    if (j & 1) != 0 {
                        span1[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_fwd };
                        span2[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_rev };
                    } else {
                        span2[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_fwd };
                        span1[j] = Rgba8 { r: c.r, g: c.g, b: c.b, a: alpha_rev };
                    }
                }
            }
        }

        rb.blend_color_hspan(
            x_start, i + y_start, square_size,
            &span1, &[], 255,
        );
        if i + 1 < square_size {
            rb.blend_color_hspan(
                x_start, i + 1 + y_start, square_size,
                &span2, &[], 255,
            );
        }
    }

    // 5 vertical reference strips
    for i in 0..square_size {
        let k = i as f64 / (square_size - 1) as f64;
        let k_color = 1.0 - (k / 2.0).powf(1.0 / gamma_val);
        let c = color.gradient(&black, 1.0 - k_color);
        for j in 0..5 {
            let xc = square_size * (j + 1) / 6;
            rb.copy_hline(x_start + xc - 10, i + y_start, x_start + xc + 10, &c);
        }
    }

    // Render controls
    let mut m_gamma = SliderCtrl::new(5.0, 5.0, 345.0, 11.0);
    m_gamma.label("Gamma=%.2f");
    m_gamma.range(0.5, 4.0);
    m_gamma.set_value(gamma_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_gamma);

    let mut m_r = SliderCtrl::new(5.0, 20.0, 345.0, 26.0);
    m_r.label("R=%.2f");
    m_r.range(0.0, 1.0);
    m_r.set_value(r_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_r);

    let mut m_g = SliderCtrl::new(5.0, 35.0, 345.0, 41.0);
    m_g.label("G=%.2f");
    m_g.range(0.0, 1.0);
    m_g.set_value(g_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_g);

    let mut m_b = SliderCtrl::new(5.0, 50.0, 345.0, 56.0);
    m_b.label("B=%.2f");
    m_b.range(0.0, 1.0);
    m_b.set_value(b_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_b);

    let mut m_pattern = RboxCtrl::new(355.0, 1.0, 495.0, 60.0);
    m_pattern.text_size(8.0, 0.0);
    m_pattern.add_item("Horizontal");
    m_pattern.add_item("Vertical");
    m_pattern.add_item("Checkered");
    m_pattern.set_cur_item(pattern as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_pattern);

    buf
}

/// Image filters 2 — 4x4 test image filtered through 17 types.
/// Matches C++ image_filters2.cpp.
pub fn image_filters2(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let filter_idx = params.get(0).copied().unwrap_or(1.0) as usize;
    let gamma_val = params.get(1).copied().unwrap_or(1.0);
    let radius = params.get(2).copied().unwrap_or(4.0);
    let normalize = params.get(3).copied().unwrap_or(1.0) > 0.5;

    let _w = width as i32;
    let _h = height as i32;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut sa = SpanAllocator::new();

    // 4x4 test image in RGBA order
    #[rustfmt::skip]
    let g_image: [u8; 64] = [
        0,255,0,255,    0,0,255,255,    255,255,255,255,  255,0,0,255,
        255,0,0,255,    0,0,0,255,      255,255,255,255,  255,255,255,255,
        255,255,255,255, 255,255,255,255, 0,0,255,255,     255,0,0,255,
        0,0,255,255,    255,255,255,255,  0,0,0,255,       0,255,0,255,
    ];

    // Create image rendering buffer
    let mut img_ra = RowAccessor::new();
    let mut img_data = g_image.to_vec();
    unsafe { img_ra.attach(img_data.as_mut_ptr(), 4, 4, 16) };

    // Map 4x4 image to 300x300 area at (200,40)
    // Scale 4→300, translate to (200,40)
    let mut img_mtx = TransAffine::new();
    img_mtx.multiply(&TransAffine::new_scaling(300.0 / 4.0, 300.0 / 4.0));
    img_mtx.multiply(&TransAffine::new_translation(200.0, 40.0));
    img_mtx.invert();
    let mut interp = SpanInterpolatorLinear::new(img_mtx);

    // Rasterize quadrilateral (200,40)-(500,40)-(500,340)-(200,340)
    ras.reset();
    ras.move_to_d(200.0, 40.0);
    ras.line_to_d(500.0, 40.0);
    ras.line_to_d(500.0, 340.0);
    ras.line_to_d(200.0, 340.0);

    let mut source = ImageAccessorClone::<4>::new(&img_ra);

    if filter_idx == 0 {
        // NN (Simple)
        let mut sg = SpanImageFilterRgbaNn::new(&mut source, &mut interp);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);
    } else {
        // Create filter LUT
        let mut filter = ImageFilterLut::new();
        match filter_idx {
            1 => filter.calculate(&ImageFilterBilinear, normalize),
            2 => filter.calculate(&ImageFilterBicubic, normalize),
            3 => filter.calculate(&ImageFilterSpline16, normalize),
            4 => filter.calculate(&ImageFilterSpline36, normalize),
            5 => filter.calculate(&ImageFilterHanning, normalize),
            6 => filter.calculate(&ImageFilterHamming, normalize),
            7 => filter.calculate(&ImageFilterHermite, normalize),
            8 => filter.calculate(&ImageFilterKaiser::new(6.33), normalize),
            9 => filter.calculate(&ImageFilterQuadric, normalize),
            10 => filter.calculate(&ImageFilterCatrom, normalize),
            11 => filter.calculate(&ImageFilterGaussian, normalize),
            12 => filter.calculate(&ImageFilterBessel, normalize),
            13 => filter.calculate(&ImageFilterMitchell::new(1.0/3.0, 1.0/3.0), normalize),
            14 => filter.calculate(&ImageFilterSinc::new(radius), normalize),
            15 => filter.calculate(&ImageFilterLanczos::new(radius), normalize),
            _ => filter.calculate(&ImageFilterBlackman::new(radius), normalize),
        }

        let mut sg = SpanImageFilterRgbaGen::new(&mut source, &mut interp, &filter);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut sg);

        // Draw filter graph (bottom-left area)
        let x_start = 5.0_f64;
        let x_end = 195.0_f64;
        let y_start = 235.0_f64;
        let y_end = height as f64 - 5.0;
        let x_center = (x_start + x_end) / 2.0;

        // Vertical grid lines
        for i in 0..=16 {
            let x = x_start + (x_end - x_start) * i as f64 / 16.0;
            let mut p = PathStorage::new();
            p.move_to(x + 0.5, y_start);
            p.line_to(x + 0.5, y_end);
            let mut stroke = ConvStroke::new(&mut p);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            let alpha = if i == 8 { 255u32 } else { 100u32 };
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, alpha));
        }

        // Horizontal reference line at 1/6 height
        let ys = y_start + (y_end - y_start) / 6.0;
        {
            let mut p = PathStorage::new();
            p.move_to(x_start, ys);
            p.line_to(x_end, ys);
            let mut stroke = ConvStroke::new(&mut p);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));
        }

        // Filter kernel response curve
        let fradius = filter.radius();
        let n = (fradius * 256.0 * 2.0) as usize;
        let dx = (x_end - x_start) * fradius / 8.0;
        let dy = y_end - ys;
        let weights = filter.weight_array();
        let xs = x_center - (filter.diameter() as f64 * (x_end - x_start) / 32.0);
        let nn = filter.diameter() as usize * 256;

        if nn > 0 && n > 0 {
            let mut p = PathStorage::new();
            p.move_to(
                xs + 0.5,
                ys + dy * weights[0] as f64 / IMAGE_FILTER_SCALE as f64,
            );
            for i in 1..nn {
                p.line_to(
                    xs + dx * i as f64 / n as f64 + 0.5,
                    ys + dy * weights[i.min(weights.len() - 1)] as f64 / IMAGE_FILTER_SCALE as f64,
                );
            }
            let mut stroke = ConvStroke::new(&mut p);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(100, 0, 0, 255));
        }
    }

    // Render controls
    let mut m_gamma = SliderCtrl::new(115.0, 5.0, width as f64 - 5.0, 11.0);
    m_gamma.label("Gamma=%.3f");
    m_gamma.range(0.5, 3.0);
    m_gamma.set_value(gamma_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_gamma);

    if filter_idx >= 14 {
        let mut m_radius = SliderCtrl::new(115.0, 20.0, width as f64 - 5.0, 26.0);
        m_radius.label("Filter Radius=%.3f");
        m_radius.range(2.0, 8.0);
        m_radius.set_value(radius);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_radius);
    }

    let filter_names = [
        "simple (NN)", "bilinear", "bicubic", "spline16", "spline36",
        "hanning", "hamming", "hermite", "kaiser", "quadric",
        "catrom", "gaussian", "bessel", "mitchell", "sinc",
        "lanczos", "blackman",
    ];
    let mut m_filters = RboxCtrl::new(0.0, 0.0, 110.0, 210.0);
    m_filters.border_width(0.0, 0.0);
    m_filters.text_size(6.0, 0.0);
    m_filters.text_thickness(0.85);
    for name in &filter_names {
        m_filters.add_item(name);
    }
    m_filters.set_cur_item(filter_idx as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_filters);

    let mut m_normalize = CboxCtrl::new(8.0, 215.0, "Normalize Filter");
    m_normalize.text_size(7.5, 0.0);
    m_normalize.set_status(normalize);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_normalize);

    buf
}

/// Conv dash marker — dashed stroke with cap styles.
/// Simplified from C++ conv_dash_marker.cpp (arrowheads and smooth poly skipped).
pub fn conv_dash_marker_demo(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let x0 = params.get(0).copied().unwrap_or(157.0);
    let y0 = params.get(1).copied().unwrap_or(60.0);
    let x1 = params.get(2).copied().unwrap_or(469.0);
    let y1 = params.get(3).copied().unwrap_or(170.0);
    let x2 = params.get(4).copied().unwrap_or(243.0);
    let y2 = params.get(5).copied().unwrap_or(310.0);
    let cap_type = params.get(6).copied().unwrap_or(0.0) as usize;
    let stroke_width = params.get(7).copied().unwrap_or(3.0);
    let close_poly = params.get(8).copied().unwrap_or(0.0) > 0.5;
    let even_odd = params.get(9).copied().unwrap_or(0.0) > 0.5;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();

    // Build path: triangle + midpoint triangle
    let cx = (x0 + x1 + x2) / 3.0;
    let cy = (y0 + y1 + y2) / 3.0;

    let mut path = PathStorage::new();
    // Triangle 1: v0 -> v1 -> center -> v2 (-> close if enabled)
    path.move_to(x0, y0);
    path.line_to(x1, y1);
    path.line_to(cx, cy);
    path.line_to(x2, y2);
    if close_poly {
        path.close_polygon(0);
    }

    // Midpoint triangle
    path.move_to((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    path.line_to((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    path.line_to((x2 + x0) / 2.0, (y2 + y0) / 2.0);
    if close_poly {
        path.close_polygon(0);
    }

    // Set fill rule
    if even_odd {
        ras.filling_rule(agg_rust::basics::FillingRule::EvenOdd);
    }

    // 1. Fill the solid triangles
    ras.reset();
    ras.add_path(&mut path, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(178, 127, 25, 127)); // rgba(0.7, 0.5, 0.1, 0.5)

    // 2. Outline stroke
    let mut outline = ConvStroke::new(&mut path);
    outline.set_width(1.0);
    ras.reset();
    ras.add_path(&mut outline, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(0, 153, 0, 204)); // rgba(0.0, 0.6, 0.0, 0.8)

    // 3. Dashed stroked path
    let path_inner = outline.source_mut();
    let mut curve = ConvCurve::new(path_inner);
    let mut dash = ConvDash::new(&mut curve);
    dash.add_dash(20.0, 5.0);
    dash.add_dash(5.0, 5.0);
    dash.add_dash(5.0, 5.0);
    dash.dash_start(10.0);

    let mut stroke = ConvStroke::new(&mut dash);
    stroke.set_width(stroke_width);
    let cap = match cap_type {
        1 => LineCap::Square,
        2 => LineCap::Round,
        _ => LineCap::Butt,
    };
    stroke.set_line_cap(cap);

    ras.reset();
    ras.add_path(&mut stroke, 0);
    render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
        &Rgba8::new(0, 0, 0, 255));

    // Render controls
    let mut m_cap = RboxCtrl::new(10.0, 10.0, 130.0, 80.0);
    m_cap.add_item("Butt Cap");
    m_cap.add_item("Square Cap");
    m_cap.add_item("Round Cap");
    m_cap.set_cur_item(cap_type as i32);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_cap);

    let mut m_width = SliderCtrl::new(140.0, 14.0, 290.0, 22.0);
    m_width.label("Width=%1.2f");
    m_width.range(0.0, 10.0);
    m_width.set_value(stroke_width);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_width);

    let mut m_close = CboxCtrl::new(140.0, 34.0, "Close Polygons");
    m_close.set_status(close_poly);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_close);

    let mut m_even_odd = CboxCtrl::new(300.0, 34.0, "Even-Odd Fill");
    m_even_odd.set_status(even_odd);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_even_odd);

    buf
}

/// AA test — radial dashes, ellipses, gradient lines, gradient triangles.
/// Matches C++ aa_test.cpp.
pub fn aa_test(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let gamma_val = params.get(0).copied().unwrap_or(1.6);

    let w = width as f64;
    let h = height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;

    let mut buf = Vec::new();
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(0, 0, 0, 255)); // Black background

    let mut ras = RasterizerScanlineAa::new();
    let mut sl = ScanlineU8::new();
    let mut sa = SpanAllocator::new();

    let r = cx.min(cy);

    // 180 radial dashed lines
    for i in (1..=180).rev() {
        let n = 2.0 * std::f64::consts::PI * i as f64 / 180.0;
        let x1 = cx + r * n.sin();
        let y1 = cy + r * n.cos();
        let dash_len = if i < 90 { i as f64 } else { 0.0 };

        let mut p = PathStorage::new();
        p.move_to(x1, y1);
        p.line_to(cx, cy);

        if dash_len > 0.0 {
            let mut dash = ConvDash::new(&mut p);
            dash.add_dash(dash_len, dash_len);
            let mut stroke = ConvStroke::new(&mut dash);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
        } else {
            let mut stroke = ConvStroke::new(&mut p);
            stroke.set_width(1.0);
            ras.reset();
            ras.add_path(&mut stroke, 0);
        }
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb,
            &Rgba8::new(255, 255, 255, 51)); // rgba(1,1,1,0.2)
    }

    // 20 ellipses — Group 1: Integral point sizes
    for i in 1..=20 {
        let mut ell = Ellipse::new(
            20.0 + (i * (i + 1)) as f64 + 0.5,
            20.5,
            i as f64 / 2.0,
            i as f64 / 2.0,
            (8 + i) as u32,
            false,
        );
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 255));
    }

    // Group 2: Fractional point sizes
    for i in 1..=20 {
        let mut ell = Ellipse::new(
            18.0 + i as f64 * 4.0 + 0.5,
            33.5,
            i as f64 / 20.0,
            i as f64 / 20.0,
            8,
            false,
        );
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 255));
    }

    // Group 3: Fractional positioning
    for i in 1..=20 {
        let mut ell = Ellipse::new(
            18.0 + i as f64 * 4.0 + (i - 1) as f64 / 10.0 + 0.5,
            27.0 + (i - 1) as f64 / 10.0 + 0.5,
            0.5,
            0.5,
            8,
            false,
        );
        ras.reset();
        ras.add_path(&mut ell, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(255, 255, 255, 255));
    }

    // Helper: draw a gradient line from (x1,y1) to (x2,y2) with given width and colors
    macro_rules! draw_gradient_line {
        ($ras:expr, $sl:expr, $rb:expr, $sa:expr,
         $x1:expr, $y1:expr, $x2:expr, $y2:expr, $lw:expr, $c1:expr, $c2:expr) => {{
            let dx = $x2 - $x1;
            let dy = $y2 - $y1;
            let d = (dx * dx + dy * dy).sqrt();
            if d > 0.0001 {
                let mut lut = GradientLut::new_default();
                lut.add_color(0.0, $c1);
                lut.add_color(1.0, $c2);
                lut.build_lut();

                let mut mtx = TransAffine::new();
                mtx.multiply(&TransAffine::new_rotation(dy.atan2(dx)));
                mtx.multiply(&TransAffine::new_translation($x1, $y1));
                mtx.invert();

                let grad_func = GradientX;
                let interp = SpanInterpolatorLinear::new(mtx);
                let mut grad = SpanGradient::new(interp, grad_func, &lut, 0.0, d);

                let mut p = PathStorage::new();
                p.move_to($x1, $y1);
                p.line_to($x2, $y2);
                let mut stroke = ConvStroke::new(&mut p);
                stroke.set_width($lw);
                $ras.reset();
                $ras.add_path(&mut stroke, 0);
                render_scanlines_aa($ras, $sl, $rb, $sa, &mut grad);
            }
        }};
    }

    // Gradient lines — Set 1: Integral line widths
    for i in 1..=20 {
        let x1 = 20.0 + (i * (i + 1)) as f64;
        let y1 = 40.5;
        let x2 = 20.0 + (i * (i + 1)) as f64 + ((i - 1) * 4) as f64;
        let y2 = 100.5;
        let c1 = Rgba8::new(255, 255, 255, 255);
        let c2 = Rgba8::new(
            ((i % 2) as f64 * 255.0) as u32,
            ((i % 3) as f64 * 0.5 * 255.0) as u32,
            ((i % 5) as f64 * 0.25 * 255.0) as u32,
            255,
        );
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, i as f64, c1, c2);
    }

    // Set 2: Fractional line lengths horizontal
    for i in 1..=20 {
        let x1 = 17.5 + i as f64 * 4.0;
        let y1 = 107.0;
        let x2 = 17.5 + i as f64 * 4.0 + i as f64 / 6.66666667;
        let y2 = 107.0;
        let c1 = Rgba8::new(255, 0, 0, 255);
        let c2 = Rgba8::new(0, 0, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, 1.0, c1, c2);
    }

    // Set 3: Fractional line lengths vertical
    for i in 1..=20 {
        let x1 = 18.0 + i as f64 * 4.0;
        let y1 = 112.5;
        let x2 = 18.0 + i as f64 * 4.0;
        let y2 = 112.5 + i as f64 / 6.66666667;
        let c1 = Rgba8::new(255, 0, 0, 255);
        let c2 = Rgba8::new(0, 0, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, 1.0, c1, c2);
    }

    // Set 4: Fractional line positioning
    for i in 1..=20 {
        let x1 = 21.5;
        let y1 = 120.0 + (i - 1) as f64 * 3.1;
        let x2 = 52.5;
        let y2 = y1;
        let c1 = Rgba8::new(255, 0, 0, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, 1.0, c1, c2);
    }

    // Set 5: Fractional width 2..0 green
    for i in 1..=20 {
        let x1 = 52.5;
        let y1 = 118.0 + i as f64 * 3.0;
        let x2 = 83.5;
        let y2 = y1;
        let lw = 2.0 - (i - 1) as f64 / 10.0;
        let c1 = Rgba8::new(0, 255, 0, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, lw, c1, c2);
    }

    // Set 6: Stippled fractional width blue
    for i in 1..=20 {
        let x1 = 83.5;
        let y1 = 119.0 + i as f64 * 3.0;
        let x2 = 114.5;
        let y2 = y1;
        let lw = 2.0 - (i - 1) as f64 / 10.0;
        let c1 = Rgba8::new(0, 0, 255, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        // Dashed version
        let mut p = PathStorage::new();
        p.move_to(x1, y1);
        p.line_to(x2, y2);
        let mut dash = ConvDash::new(&mut p);
        dash.add_dash(3.0, 3.0);
        let mut stroke = ConvStroke::new(&mut dash);
        stroke.set_width(lw);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let d = (dx * dx + dy * dy).sqrt();
        if d > 0.0001 {
            let mut lut = GradientLut::new_default();
            lut.add_color(0.0, c1);
            lut.add_color(1.0, c2);
            lut.build_lut();
            let mut mtx = TransAffine::new();
            mtx.multiply(&TransAffine::new_rotation(dy.atan2(dx)));
            mtx.multiply(&TransAffine::new_translation(x1, y1));
            mtx.invert();
            let interp = SpanInterpolatorLinear::new(mtx);
            let mut grad = SpanGradient::new(interp, GradientX, &lut, 0.0, d);
            ras.reset();
            ras.add_path(&mut stroke, 0);
            render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut grad);
        }
    }

    // Set 7: Integral line width, horizontal aligned (i<=10)
    for i in 1..=10 {
        let x1 = 125.5;
        let y1 = 119.5 + (i + 2) as f64 * (i as f64 / 2.0);
        let x2 = 135.5;
        let y2 = y1;
        let c1 = Rgba8::new(255, 255, 255, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, i as f64, c1, c2);
    }

    // Set 8: Fractional width 0..2, 1px H
    for i in 1..=20 {
        let x1 = 17.5 + i as f64 * 4.0;
        let y1 = 192.0;
        let x2 = 18.5 + i as f64 * 4.0;
        let y2 = 192.0;
        let lw = i as f64 / 10.0;
        let c1 = Rgba8::new(255, 255, 255, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, lw, c1, c2);
    }

    // Set 9: Fractional positioning 1px
    for i in 1..=20 {
        let x1 = 17.5 + i as f64 * 4.0 + (i - 1) as f64 / 10.0;
        let y1 = 186.0;
        let x2 = 18.5 + i as f64 * 4.0 + (i - 1) as f64 / 10.0;
        let y2 = 186.0;
        let c1 = Rgba8::new(255, 255, 255, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        draw_gradient_line!(&mut ras, &mut sl, &mut rb, &mut sa, x1, y1, x2, y2, 1.0, c1, c2);
    }

    // 13 gradient triangles (Gouraud)
    for i in 1..=13 {
        let x1 = w - 150.0;
        let y1 = h - 20.0 - i as f64 * (i as f64 + 1.5);
        let x2 = w - 20.0;
        let y2 = h - 20.0 - i as f64 * (i as f64 + 1.0);
        let x3 = w - 20.0;
        let y3 = h - 20.0 - i as f64 * (i as f64 + 2.0);

        let c1 = Rgba8::new(255, 255, 255, 255);
        let c2 = Rgba8::new(
            ((i % 2) as f64 * 255.0) as u32,
            ((i % 3) as f64 * 0.5 * 255.0) as u32,
            ((i % 5) as f64 * 0.25 * 255.0) as u32,
            255,
        );

        let mut gouraud = SpanGouraudRgba::new_with_triangle(
            c1, c2, c2,
            x1, y1, x2, y2, x3, y3,
            0.0,
        );
        gouraud.prepare();
        ras.reset();
        ras.move_to_d(x1, y1);
        ras.line_to_d(x2, y2);
        ras.line_to_d(x3, y3);
        render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut sa, &mut gouraud);
    }

    // Render gamma slider
    let mut m_gamma = SliderCtrl::new(5.0, 5.0, 340.0, 12.0);
    m_gamma.label("Gamma=%.3f");
    m_gamma.range(0.1, 3.0);
    m_gamma.set_value(gamma_val);
    render_ctrl(&mut ras, &mut sl, &mut rb, &mut m_gamma);

    buf
}

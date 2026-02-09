// Copyright 2025. Native image_filters demo for pixel comparison.

use agg_rust::color::Rgba8;
use agg_rust::conv_stroke::ConvStroke;
use agg_rust::ctrl::{render_ctrl, CboxCtrl, RboxCtrl, SliderCtrl};
use agg_rust::ellipse::Ellipse;
use agg_rust::gsv_text::GsvText;
use agg_rust::image_filters::*;
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::rasterizer_scanline_aa::RasterizerScanlineAa;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_scanline::{render_scanlines_aa, render_scanlines_aa_solid};
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::scanline_u::ScanlineU8;
use agg_rust::span_allocator::SpanAllocator;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;
use agg_rust::conv_transform::ConvTransform;

/// Load spheres.bmp and return (width, height, rgba_data).
fn load_spheres_bmp() -> (u32, u32, Vec<u8>) {
    let d = include_bytes!("../../../../demo/wasm/src/spheres.bmp");
    let off = u32::from_le_bytes([d[10], d[11], d[12], d[13]]) as usize;
    let w = u32::from_le_bytes([d[18], d[19], d[20], d[21]]);
    let h = u32::from_le_bytes([d[22], d[23], d[24], d[25]]);
    let bpp = u16::from_le_bytes([d[28], d[29]]) as usize;
    let bytes_pp = bpp / 8;
    let row_size = ((w as usize * bytes_pp + 3) / 4) * 4;
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        let src_y = h as usize - 1 - y;
        let src_off = off + src_y * row_size;
        for x in 0..w as usize {
            let si = src_off + x * bytes_pp;
            let di = (y * w as usize + x) * 4;
            if bytes_pp >= 3 {
                rgba[di] = d[si + 2];     // R from BGR
                rgba[di + 1] = d[si + 1]; // G
                rgba[di + 2] = d[si];     // B
                rgba[di + 3] = if bytes_pp >= 4 { d[si + 3] } else { 255 };
            }
        }
    }
    (w, h, rgba)
}

/// Render image_filters demo.
///
/// params[0] = filter_idx (0-16, default 1)
/// params[1] = step_deg (1.0-10.0, default 5.0)
/// params[2] = normalize (0/1, default 1)
/// params[3] = radius (2.0-8.0, default 4.0)
/// params[4] = num_steps (default 0)
/// params[5] = skip_controls (0/1, default 0)
pub fn render(width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    let filter_idx = params.get(0).copied().unwrap_or(1.0) as usize;
    let step_deg = params.get(1).copied().unwrap_or(5.0).clamp(1.0, 10.0);
    let normalize = params.get(2).copied().unwrap_or(1.0) > 0.5;
    let radius = params.get(3).copied().unwrap_or(4.0).clamp(2.0, 8.0);
    let num_steps = params.get(4).copied().unwrap_or(0.0).max(0.0) as usize;
    let skip_controls = params.get(5).copied().unwrap_or(0.0) > 0.5;

    let (img_w, img_h, original) = load_spheres_bmp();
    let img_stride = (img_w * 4) as i32;

    let iw = img_w as f64;
    let ih = img_h as f64;

    // Working buffers
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

        // Transform matrix
        let mut src_mtx = TransAffine::new();
        src_mtx.multiply(&TransAffine::new_translation(-iw / 2.0, -ih / 2.0));
        src_mtx.multiply(&TransAffine::new_rotation(angle_rad));
        src_mtx.multiply(&TransAffine::new_translation(iw / 2.0, ih / 2.0));

        let mut img_mtx = src_mtx;
        img_mtx.invert();

        // Clipping ellipse
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
                // Nearest neighbor
                use agg_rust::image_accessors::ImageAccessorClip;
                use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaNn;
                let mut accessor = ImageAccessorClip::<4>::new(&src_ra, &[0, 0, 0, 0]);
                let mut sg = SpanImageFilterRgbaNn::new(&mut accessor, &mut interpolator);
                let pf = PixfmtRgba32::new(&mut dst_ra);
                let mut rb = RendererBase::new(pf);
                render_scanlines_aa(&mut ras, &mut sl, &mut rb, &mut alloc, &mut sg);
            }
            1 => {
                // Bilinear clip
                use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaBilinearClip;
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
                // 2x2 filters
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
                // General filter
                use agg_rust::image_accessors::ImageAccessorClip;
                use agg_rust::span_image_filter_rgba::SpanImageFilterRgbaGen;
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

        // Swap: dst becomes new src
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

    if !skip_controls {
        // Re-attach for control rendering
        unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut ras = RasterizerScanlineAa::new();
        let mut sl = ScanlineU8::new();

        // NSteps text
        let label = format!("NSteps={}", num_steps);
        let mut txt = GsvText::new();
        txt.size(10.0, 0.0);
        txt.start_point(10.0, 295.0);
        txt.text(&label);
        let mut stroke = ConvStroke::new(txt);
        stroke.set_width(1.5);
        ras.add_path(&mut stroke, 0);
        render_scanlines_aa_solid(&mut ras, &mut sl, &mut rb, &Rgba8::new(0, 0, 0, 255));

        // Filter radio box
        let filter_names = [
            "simple (NN)", "bilinear", "bicubic", "spline16", "spline36",
            "hanning", "hamming", "hermite", "kaiser", "quadric", "catrom",
            "gaussian", "bessel", "mitchell", "sinc", "lanczos", "blackman",
        ];
        let mut r_filters = RboxCtrl::new(0.0, 0.0, 110.0, 210.0);
        r_filters.border_width(0.0, 0.0);
        r_filters.text_size(6.0, 0.0);
        r_filters.text_thickness(0.85);
        r_filters.background_color(Rgba8::new(0, 0, 0, 26)); // rgba(0,0,0,0.1)
        for name in &filter_names {
            r_filters.add_item(name);
        }
        r_filters.set_cur_item(filter_idx.min(16) as i32);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut r_filters);

        // Step slider
        let mut s_step = SliderCtrl::new(115.0, 5.0, 400.0, 11.0);
        s_step.label("Step=%3.2f");
        s_step.range(1.0, 10.0);
        s_step.set_value(step_deg);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_step);

        // Radius slider (only for sinc/lanczos/blackman)
        if filter_idx >= 14 {
            let mut s_radius = SliderCtrl::new(115.0, 20.0, 400.0, 26.0);
            s_radius.label("Filter Radius=%.3f");
            s_radius.range(2.0, 8.0);
            s_radius.set_value(radius);
            render_ctrl(&mut ras, &mut sl, &mut rb, &mut s_radius);
        }

        // Normalize checkbox
        let mut c_norm = CboxCtrl::new(8.0, 215.0, "Normalize Filter");
        c_norm.text_size(7.5, 0.0);
        c_norm.set_status(normalize);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_norm);

        // Checkboxes matching C++
        let mut c_single = CboxCtrl::new(8.0, 230.0, "Single Step");
        c_single.text_size(7.5, 0.0);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_single);

        let mut c_run = CboxCtrl::new(8.0, 245.0, "RUN Test!");
        c_run.text_size(7.5, 0.0);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_run);

        let mut c_refresh = CboxCtrl::new(8.0, 265.0, "Refresh");
        c_refresh.text_size(7.5, 0.0);
        render_ctrl(&mut ras, &mut sl, &mut rb, &mut c_refresh);
    }

    buf
}

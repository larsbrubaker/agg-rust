// image_filters headless reproduction. Mirrors the pixel-compare Rust scene
// (tools/pixel-compare/src/render/image_filters.rs): transform the spheres image
// (rotation, clipped to an ellipse) through a selectable reconstruction filter,
// blit it at (110,35), then draw the control panel. Default: bilinear, 0 steps.
#include <cmath>
#include <cstdio>
#include <string>
#include <vector>

#include "agg_basics.h"
#include "agg_rendering_buffer.h"
#include "agg_rasterizer_scanline_aa.h"
#include "agg_ellipse.h"
#include "agg_trans_affine.h"
#include "agg_conv_transform.h"
#include "agg_conv_stroke.h"
#include "agg_scanline_u.h"
#include "agg_image_accessors.h"
#include "agg_renderer_scanline.h"
#include "agg_span_allocator.h"
#include "agg_span_interpolator_linear.h"
#include "agg_span_image_filter_rgba.h"
#include "agg_image_filters.h"
#include "agg_gsv_text.h"
#include "agg_pixfmt_rgba.h"
#include "ctrl/agg_slider_ctrl.h"
#include "ctrl/agg_rbox_ctrl.h"
#include "ctrl/agg_cbox_ctrl.h"

#include "common.h"

void render_image_filters(unsigned w, unsigned h,
                          const std::vector<double>& params, const char* out) {
    typedef agg::pixfmt_rgba32 pixfmt;
    typedef agg::renderer_base<pixfmt> renderer_base;

    int filter_idx = params.size() > 0 ? (int)params[0] : 1;
    double step_deg = params.size() > 1 ? params[1] : 5.0;
    if (step_deg < 1.0) step_deg = 1.0; if (step_deg > 10.0) step_deg = 10.0;
    bool normalize = params.size() > 2 ? params[2] > 0.5 : true;
    double radius = params.size() > 3 ? params[3] : 4.0;
    if (radius < 2.0) radius = 2.0; if (radius > 8.0) radius = 8.0;
    int num_steps = params.size() > 4 ? (int)params[4] : 0;
    if (num_steps < 0) num_steps = 0;
    bool skip_controls = params.size() > 5 ? params[5] > 0.5 : false;

    headless::image_rgba img = headless::load_spheres();
    unsigned iw = img.width, ih = img.height;
    std::vector<unsigned char> src_data, dst_data;
    {
        agg::rendering_buffer tmp;
        headless::pack_image(img, "rgba", src_data, tmp);
    }
    dst_data.assign(static_cast<size_t>(iw) * ih * 4, 255);

    int total = 1 + num_steps;
    for (int step = 0; step < total; ++step) {
        double angle_deg = (step == 0) ? 0.0 : step_deg;
        double angle_rad = angle_deg * agg::pi / 180.0;

        for (size_t i = 0; i < dst_data.size(); i += 4) {
            dst_data[i] = 255; dst_data[i + 1] = 255; dst_data[i + 2] = 255; dst_data[i + 3] = 255;
        }

        agg::rendering_buffer src_rbuf(src_data.data(), iw, ih, (int)(iw * 4));
        agg::rendering_buffer dst_rbuf(dst_data.data(), iw, ih, (int)(iw * 4));

        agg::trans_affine src_mtx;
        src_mtx *= agg::trans_affine_translation(-(double)iw / 2.0, -(double)ih / 2.0);
        src_mtx *= agg::trans_affine_rotation(angle_rad);
        src_mtx *= agg::trans_affine_translation((double)iw / 2.0, (double)ih / 2.0);
        agg::trans_affine img_mtx = src_mtx;
        img_mtx.invert();

        double r = iw; if (ih < r) r = ih; r *= 0.5; r -= 4.0;
        agg::ellipse ell(iw / 2.0, ih / 2.0, r, r, 200);
        agg::conv_transform<agg::ellipse> tr(ell, src_mtx);

        agg::rasterizer_scanline_aa<> ras;
        agg::scanline_u8 sl;
        agg::span_allocator<agg::rgba8> alloc;
        ras.add_path(tr);

        typedef agg::span_interpolator_linear<> interp_t;
        interp_t interp(img_mtx);
        pixfmt src_pixf(src_rbuf);
        pixfmt dst_pixf(dst_rbuf);
        renderer_base rb(dst_pixf);
        typedef agg::image_accessor_clip<pixfmt> accessor_t;
        accessor_t accessor(src_pixf, agg::rgba(0, 0, 0, 0));

        if (filter_idx == 0) {
            typedef agg::span_image_filter_rgba_nn<accessor_t, interp_t> sg_t;
            sg_t sg(accessor, interp);
            agg::render_scanlines_aa(ras, sl, rb, alloc, sg);
        } else if (filter_idx == 1) {
            typedef agg::span_image_filter_rgba_bilinear_clip<pixfmt, interp_t> sg_t;
            sg_t sg(src_pixf, agg::rgba(0, 0, 0, 0), interp);
            agg::render_scanlines_aa(ras, sl, rb, alloc, sg);
        } else if (filter_idx == 5 || filter_idx == 6 || filter_idx == 7) {
            agg::image_filter_lut lut;
            if (filter_idx == 5) lut.calculate(agg::image_filter_hanning(), normalize);
            else if (filter_idx == 6) lut.calculate(agg::image_filter_hamming(), normalize);
            else lut.calculate(agg::image_filter_hermite(), normalize);
            typedef agg::span_image_filter_rgba_2x2<accessor_t, interp_t> sg_t;
            sg_t sg(accessor, interp, lut);
            agg::render_scanlines_aa(ras, sl, rb, alloc, sg);
        } else {
            agg::image_filter_lut lut;
            switch (filter_idx) {
            case 2: lut.calculate(agg::image_filter_bicubic(), normalize); break;
            case 3: lut.calculate(agg::image_filter_spline16(), normalize); break;
            case 4: lut.calculate(agg::image_filter_spline36(), normalize); break;
            case 8: lut.calculate(agg::image_filter_kaiser(), normalize); break;
            case 9: lut.calculate(agg::image_filter_quadric(), normalize); break;
            case 10: lut.calculate(agg::image_filter_catrom(), normalize); break;
            case 11: lut.calculate(agg::image_filter_gaussian(), normalize); break;
            case 12: lut.calculate(agg::image_filter_bessel(), normalize); break;
            case 13: lut.calculate(agg::image_filter_mitchell(), normalize); break;
            case 14: lut.calculate(agg::image_filter_sinc(radius), normalize); break;
            case 15: lut.calculate(agg::image_filter_lanczos(radius), normalize); break;
            case 16: lut.calculate(agg::image_filter_blackman(radius), normalize); break;
            default: lut.calculate(agg::image_filter_bicubic(), normalize); break;
            }
            typedef agg::span_image_filter_rgba<accessor_t, interp_t> sg_t;
            sg_t sg(accessor, interp, lut);
            agg::render_scanlines_aa(ras, sl, rb, alloc, sg);
        }

        std::swap(src_data, dst_data);
    }

    // Output buffer, image blitted at (110, 35).
    headless::canvas cv(w, h, 4);
    pixfmt pixf(cv.rbuf);
    renderer_base rb(pixf);
    rb.clear(agg::rgba8(255, 255, 255, 255));
    for (unsigned y = 0; y < ih && y + 35 < h; ++y) {
        for (unsigned x = 0; x < iw && x + 110 < w; ++x) {
            size_t si = (static_cast<size_t>(y) * iw + x) * 4;
            size_t di = ((static_cast<size_t>(y + 35) * w) + (x + 110)) * 4;
            cv.data[di] = src_data[si];
            cv.data[di + 1] = src_data[si + 1];
            cv.data[di + 2] = src_data[si + 2];
            cv.data[di + 3] = src_data[si + 3];
        }
    }

    if (!skip_controls) {
        agg::rasterizer_scanline_aa<> ras;
        agg::scanline_u8 sl;

        char label[64];
        sprintf(label, "NSteps=%d", num_steps);
        agg::gsv_text txt;
        txt.size(10.0, 0.0);
        txt.start_point(10.0, 295.0);
        txt.text(label);
        agg::conv_stroke<agg::gsv_text> stroke(txt);
        stroke.width(1.5);
        ras.add_path(stroke);
        agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 0, 0, 255));

        const char* filter_names[] = {
            "simple (NN)", "bilinear", "bicubic", "spline16", "spline36",
            "hanning", "hamming", "hermite", "kaiser", "quadric", "catrom",
            "gaussian", "bessel", "mitchell", "sinc", "lanczos", "blackman"};
        agg::rbox_ctrl<agg::rgba8> r_filters(0.0, 0.0, 110.0, 210.0, false);
        r_filters.border_width(0.0, 0.0);
        r_filters.text_size(6.0, 0.0);
        r_filters.text_thickness(0.85);
        r_filters.background_color(agg::rgba8(0, 0, 0, 26));
        for (const char* n : filter_names) r_filters.add_item(n);
        r_filters.cur_item(filter_idx < 16 ? filter_idx : 16);
        agg::render_ctrl(ras, sl, rb, r_filters);

        agg::slider_ctrl<agg::rgba8> s_step(115.0, 5.0, 400.0, 11.0, false);
        s_step.label("Step=%3.2f"); s_step.range(1.0, 10.0); s_step.value(step_deg);
        agg::render_ctrl(ras, sl, rb, s_step);

        if (filter_idx >= 14) {
            agg::slider_ctrl<agg::rgba8> s_radius(115.0, 20.0, 400.0, 26.0, false);
            s_radius.label("Filter Radius=%.3f"); s_radius.range(2.0, 8.0); s_radius.value(radius);
            agg::render_ctrl(ras, sl, rb, s_radius);
        }

        agg::cbox_ctrl<agg::rgba8> c_norm(8.0, 215.0, "Normalize Filter", false);
        c_norm.text_size(7.5, 0.0); c_norm.status(normalize);
        agg::render_ctrl(ras, sl, rb, c_norm);

        agg::cbox_ctrl<agg::rgba8> c_single(8.0, 230.0, "Single Step", false);
        c_single.text_size(7.5, 0.0);
        agg::render_ctrl(ras, sl, rb, c_single);

        agg::cbox_ctrl<agg::rgba8> c_run(8.0, 245.0, "RUN Test!", false);
        c_run.text_size(7.5, 0.0);
        agg::render_ctrl(ras, sl, rb, c_run);

        agg::cbox_ctrl<agg::rgba8> c_refresh(8.0, 265.0, "Refresh", false);
        c_refresh.text_size(7.5, 0.0);
        agg::render_ctrl(ras, sl, rb, c_refresh);
    }

    headless::write_raw(out, pixf, w, h);
}

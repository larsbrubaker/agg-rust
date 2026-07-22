// image_perspective.cpp headless reproduction. Renders the same scene the Rust
// pixel-compare demo defines (default: Perspective transform, quad inset by 100,
// no benchmark-timer text) using authentic AGG C++ image span filters, so verify
// validates the Rust image pipeline against the real library.
#include <vector>

#include "agg_basics.h"
#include "agg_rendering_buffer.h"
#include "agg_rasterizer_scanline_aa.h"
#include "agg_scanline_u.h"
#include "agg_renderer_scanline.h"
#include "agg_path_storage.h"
#include "agg_conv_stroke.h"
#include "agg_ellipse.h"
#include "agg_trans_affine.h"
#include "agg_trans_bilinear.h"
#include "agg_trans_perspective.h"
#include "agg_span_allocator.h"
#include "agg_span_interpolator_linear.h"
#include "agg_span_interpolator_trans.h"
#include "agg_pixfmt_rgba.h"
#include "agg_image_accessors.h"
#include "agg_span_image_filter_rgba.h"
#include "ctrl/agg_rbox_ctrl.h"

#include "common.h"

int render_image_perspective(unsigned w, unsigned h,
                             const std::vector<double>& params, const char* out) {
    typedef agg::pixfmt_bgra32 pixfmt;
    typedef agg::renderer_base<pixfmt> renderer_base;

    double W = w, H = h;
    double q[8] = {
        params.size() > 0 ? params[0] : 100.0,
        params.size() > 1 ? params[1] : 100.0,
        params.size() > 2 ? params[2] : W - 100.0,
        params.size() > 3 ? params[3] : 100.0,
        params.size() > 4 ? params[4] : W - 100.0,
        params.size() > 5 ? params[5] : H - 100.0,
        params.size() > 6 ? params[6] : 100.0,
        params.size() > 7 ? params[7] : H - 100.0,
    };
    int trans_type = params.size() > 8 ? (int)params[8] : 2;

    headless::image_rgba img = headless::load_spheres();
    if (!img.ok) {
        fprintf(stderr, "image_perspective: failed to load spheres image\n");
        return 1;
    }
    std::vector<unsigned char> img_bytes;
    agg::rendering_buffer img_rbuf;
    headless::pack_image(img, "bgra", img_bytes, img_rbuf);
    double g_x1 = 0, g_y1 = 0, g_x2 = img.width, g_y2 = img.height;

    if (trans_type == 0) {
        q[6] = q[0] + (q[4] - q[2]);
        q[7] = q[1] + (q[5] - q[3]);
    }

    headless::canvas cv(w, h, 4);
    pixfmt pixf(cv.rbuf);
    renderer_base rb(pixf);
    rb.clear(agg::rgba(1, 1, 1));

    agg::rasterizer_scanline_aa<> ras;
    agg::scanline_u8 sl;
    agg::span_allocator<agg::rgba8> sa;

    ras.clip_box(0, 0, W, H);
    ras.reset();
    ras.move_to_d(q[0], q[1]);
    ras.line_to_d(q[2], q[3]);
    ras.line_to_d(q[4], q[5]);
    ras.line_to_d(q[6], q[7]);

    pixfmt pixf_img(img_rbuf);
    typedef agg::image_accessor_clone<pixfmt> img_accessor_type;
    img_accessor_type ia(pixf_img);

    agg::image_filter_bilinear filter_kernel;
    agg::image_filter_lut filter(filter_kernel, false);

    if (trans_type == 0) {
        agg::trans_affine tr(q, g_x1, g_y1, g_x2, g_y2);
        typedef agg::span_interpolator_linear<agg::trans_affine> interp_t;
        interp_t interp(tr);
        typedef agg::span_image_filter_rgba_nn<img_accessor_type, interp_t> sg_t;
        sg_t sg(ia, interp);
        agg::render_scanlines_aa(ras, sl, rb, sa, sg);
    } else if (trans_type == 1) {
        agg::trans_bilinear tr(q, g_x1, g_y1, g_x2, g_y2);
        if (tr.is_valid()) {
            typedef agg::span_interpolator_linear<agg::trans_bilinear> interp_t;
            interp_t interp(tr);
            typedef agg::span_image_filter_rgba_2x2<img_accessor_type, interp_t> sg_t;
            sg_t sg(ia, interp, filter);
            agg::render_scanlines_aa(ras, sl, rb, sa, sg);
        }
    } else {
        agg::trans_perspective tr(q, g_x1, g_y1, g_x2, g_y2);
        if (tr.is_valid()) {
            typedef agg::span_interpolator_trans<agg::trans_perspective> interp_t;
            interp_t interp(tr);
            typedef agg::span_image_filter_rgba_2x2<img_accessor_type, interp_t> sg_t;
            sg_t sg(ia, interp, filter);
            agg::render_scanlines_aa(ras, sl, rb, sa, sg);
        }
    }

    // Quad tool overlay: closed 1px stroke + corner circles (drawn on top).
    agg::path_storage quad_path;
    quad_path.move_to(q[0], q[1]);
    quad_path.line_to(q[2], q[3]);
    quad_path.line_to(q[4], q[5]);
    quad_path.line_to(q[6], q[7]);
    quad_path.close_polygon();
    agg::conv_stroke<agg::path_storage> quad_stroke(quad_path);
    quad_stroke.width(1.0);
    ras.reset();
    ras.add_path(quad_stroke);
    agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 76, 127, 153));
    for (int i = 0; i < 4; ++i) {
        agg::ellipse vtx(q[i * 2], q[i * 2 + 1], 5.0, 5.0, 32);
        ras.reset();
        ras.add_path(vtx);
        agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 76, 127, 153));
    }

    agg::rbox_ctrl<agg::rgba8> m_trans(420.0, 5.0, 590.0, 65.0, false);
    m_trans.add_item("Affine Parallelogram");
    m_trans.add_item("Bilinear");
    m_trans.add_item("Perspective");
    m_trans.cur_item(trans_type);
    agg::render_ctrl(ras, sl, rb, m_trans);

    return headless::write_raw(out, pixf, w, h) ? 0 : 1;
}

// perspective.cpp headless reproduction. Default state renders the lion under a
// rect->quad Bilinear transform (m_trans_type default item 0), plus the quad
// tool and the rbox control, exactly as the original on_draw().
#include <vector>

#include "agg_basics.h"
#include "agg_rendering_buffer.h"
#include "agg_rasterizer_scanline_aa.h"
#include "agg_scanline_p.h"
#include "agg_renderer_scanline.h"
#include "agg_path_storage.h"
#include "agg_conv_transform.h"
#include "agg_conv_stroke.h"
#include "agg_bounding_rect.h"
#include "agg_ellipse.h"
#include "agg_trans_bilinear.h"
#include "agg_trans_perspective.h"
#include "agg_pixfmt_rgb.h"
#include "ctrl/agg_rbox_ctrl.h"
#include "interactive_polygon.h"

#include "common.h"

// From examples/parse_lion.cpp
unsigned parse_lion(agg::path_storage& ps, agg::srgba8* colors, unsigned* path_idx);

int render_perspective(unsigned w, unsigned h,
                       const std::vector<double>& params, const char* out) {
    typedef agg::pixfmt_bgr24 pixfmt;
    typedef agg::renderer_base<pixfmt> renderer_base;
    typedef agg::renderer_scanline_aa_solid<renderer_base> renderer_solid;

    agg::path_storage path;
    agg::srgba8 colors[100];
    unsigned path_idx[100];
    unsigned npaths = parse_lion(path, colors, path_idx);

    double x1, y1, x2, y2;
    agg::pod_array_adaptor<unsigned> pia(path_idx, 100);
    agg::bounding_rect(path, pia, 0, npaths, &x1, &y1, &x2, &y2);
    // parse_lion() in perspective.cpp mirrors the lion in both axes.
    path.flip_x(x1, x2);
    path.flip_y(y1, y2);

    // Default quad = bounding rect corners centered in the window. Uses the same
    // centering as the committed reference / Rust port: the rect is placed so its
    // center coincides with the window center (offset excludes the bbox origin).
    double ox = (w - (x2 - x1)) / 2.0 - x1;
    double oy = (h - (y2 - y1)) / 2.0 - y1;
    double qx[4] = {x1 + ox, x2 + ox, x2 + ox, x1 + ox};
    double qy[4] = {y1 + oy, y1 + oy, y2 + oy, y2 + oy};

    // Optional param overrides: 0..7 = quad corners, 8 = trans_type.
    for (int i = 0; i < 4; ++i) {
        if (params.size() > (size_t)(i * 2))     qx[i] = params[i * 2];
        if (params.size() > (size_t)(i * 2 + 1)) qy[i] = params[i * 2 + 1];
    }
    int trans_type = params.size() > 8 ? (int)params[8] : 0;
    double quad_poly[8] = {qx[0], qy[0], qx[1], qy[1], qx[2], qy[2], qx[3], qy[3]};

    headless::canvas cv(w, h, 3);
    pixfmt pixf(cv.rbuf);
    renderer_base rb(pixf);
    renderer_solid r(rb);
    rb.clear(agg::rgba(1, 1, 1));

    agg::rasterizer_scanline_aa<> ras;
    agg::scanline_p8 sl;
    ras.clip_box(0, 0, w, h);

    if (trans_type == 0) {
        agg::trans_bilinear tr(x1, y1, x2, y2, quad_poly);
        if (tr.is_valid()) {
            agg::conv_transform<agg::path_storage, agg::trans_bilinear> trans(path, tr);
            agg::render_all_paths(ras, sl, r, trans, colors, path_idx, npaths);

            agg::ellipse ell((x1 + x2) * 0.5, (y1 + y2) * 0.5,
                             (x2 - x1) * 0.5, (y2 - y1) * 0.5, 200);
            agg::conv_stroke<agg::ellipse> ell_stroke(ell);
            ell_stroke.width(3.0);
            agg::conv_transform<agg::ellipse, agg::trans_bilinear> trans_ell(ell, tr);
            agg::conv_transform<agg::conv_stroke<agg::ellipse>, agg::trans_bilinear>
                trans_ell_stroke(ell_stroke, tr);

            ras.add_path(trans_ell);
            r.color(agg::rgba(0.5, 0.3, 0.0, 0.3));
            agg::render_scanlines(ras, sl, r);

            ras.add_path(trans_ell_stroke);
            r.color(agg::rgba(0.0, 0.3, 0.2, 1.0));
            agg::render_scanlines(ras, sl, r);
        }
    } else {
        agg::trans_perspective tr(x1, y1, x2, y2, quad_poly);
        if (tr.is_valid()) {
            agg::conv_transform<agg::path_storage, agg::trans_perspective> trans(path, tr);
            agg::render_all_paths(ras, sl, r, trans, colors, path_idx, npaths);

            agg::ellipse ell((x1 + x2) * 0.5, (y1 + y2) * 0.5,
                             (x2 - x1) * 0.5, (y2 - y1) * 0.5, 200);
            agg::conv_stroke<agg::ellipse> ell_stroke(ell);
            ell_stroke.width(3.0);
            agg::conv_transform<agg::ellipse, agg::trans_perspective> trans_ell(ell, tr);
            agg::conv_transform<agg::conv_stroke<agg::ellipse>, agg::trans_perspective>
                trans_ell_stroke(ell_stroke, tr);

            ras.add_path(trans_ell);
            r.color(agg::rgba(0.5, 0.3, 0.0, 0.3));
            agg::render_scanlines(ras, sl, r);

            ras.add_path(trans_ell_stroke);
            r.color(agg::rgba(0.0, 0.3, 0.2, 1.0));
            agg::render_scanlines(ras, sl, r);
        }
    }

    // Quad tool: a closed 1px stroke plus a filled circle at each corner, each
    // rendered as a separate pass. This matches the committed reference (and the
    // Rust port), which does not use interactive_polygon's combined open path.
    double q0x = qx[0], q0y = qy[0];
    double q1x = qx[1], q1y = qy[1];
    double q2x = qx[2], q2y = qy[2];
    double q3x = qx[3], q3y = qy[3];
    {
        agg::path_storage quad_path;
        quad_path.move_to(q0x, q0y);
        quad_path.line_to(q1x, q1y);
        quad_path.line_to(q2x, q2y);
        quad_path.line_to(q3x, q3y);
        quad_path.close_polygon();
        agg::conv_stroke<agg::path_storage> stroke(quad_path);
        stroke.width(1.0);
        ras.reset();
        ras.add_path(stroke);
        agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 77, 128, 153));
    }
    {
        const double cxs[4] = {q0x, q1x, q2x, q3x};
        const double cys[4] = {q0y, q1y, q2y, q3y};
        for (int i = 0; i < 4; ++i) {
            agg::ellipse ell(cxs[i], cys[i], 5.0, 5.0, 32);
            ras.reset();
            ras.add_path(ell);
            agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 77, 128, 153));
        }
    }

    // rbox control (flip_y = true in the demo, so ctrl flip = !flip_y = false).

    agg::rbox_ctrl<agg::rgba8> trans_type_ctrl(420, 5.0, 420 + 130.0, 55.0, false);
    trans_type_ctrl.add_item("Bilinear");
    trans_type_ctrl.add_item("Perspective");
    trans_type_ctrl.cur_item(trans_type);
    agg::render_ctrl(ras, sl, rb, trans_type_ctrl);

    return headless::write_raw(out, pixf, w, h) ? 0 : 1;
}

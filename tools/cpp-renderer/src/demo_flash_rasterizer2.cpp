// flash_rasterizer2.cpp headless reproduction. Mirrors the Rust pixel-compare
// scene: fill each style with a plain scanline rasterizer (left-fill paths plus
// inverted right-fill paths), then stroke outlines and draw help text.
#include <algorithm>
#include <cmath>
#include <vector>

#include "agg_basics.h"
#include "agg_rendering_buffer.h"
#include "agg_rasterizer_scanline_aa.h"
#include "agg_scanline_u.h"
#include "agg_renderer_scanline.h"
#include "agg_conv_curve.h"
#include "agg_conv_transform.h"
#include "agg_conv_stroke.h"
#include "agg_gsv_text.h"
#include "agg_pixfmt_rgba.h"

#include "common.h"
#include "flash_shape.h"

static void flash_palette2(agg::rgba8* colors) {
    agg::msvc_rand rng;
    for (int i = 0; i < 100; ++i) {
        colors[i] = agg::rgba8(rng.next() & 0xFF, rng.next() & 0xFF, rng.next() & 0xFF, 230);
        colors[i].premultiply();
    }
}

void render_flash_rasterizer2(unsigned w, unsigned h,
                              const std::vector<double>& params, const char* out) {
    typedef agg::pixfmt_bgra32 pixfmt;
    typedef agg::renderer_base<pixfmt> renderer_base;
    typedef agg::renderer_scanline_aa_solid<renderer_base> renderer_solid;

    int shape_index = 0;
    double user_scale = 1.0;
    if (params.size() >= 7) shape_index = (int)params[0];

    agg::compound_shape shape;
    if (!shape.open(headless::asset_path("shapes.txt").c_str())) {
        headless::canvas cv(w, h, 4);
        pixfmt pixf(cv.rbuf);
        headless::write_raw(out, pixf, w, h);
        return;
    }
    for (int i = 0; i <= shape_index; ++i) {
        if (!shape.read_next()) break;
    }

    agg::rgba8 colors[100];
    flash_palette2(colors);

    headless::canvas cv(w, h, 4);
    pixfmt pixf(cv.rbuf);
    renderer_base rb(pixf);
    rb.clear(agg::rgba8(255, 255, 242, 255));

    agg::trans_affine mtx = shape.compute_viewport(w, h);

    int min_style = 0x7fffffff, max_style = 0;
    for (unsigned i = 0; i < shape.paths(); ++i) {
        const agg::path_style& st = shape.style(i);
        if (st.left_fill >= 0) { min_style = std::min(min_style, st.left_fill); max_style = std::max(max_style, st.left_fill); }
        if (st.right_fill >= 0) { min_style = std::min(min_style, st.right_fill); max_style = std::max(max_style, st.right_fill); }
    }
    if (min_style > max_style) { min_style = 0; max_style = 0; }

    agg::rasterizer_scanline_aa<> ras;
    agg::scanline_u8 sl;
    renderer_solid ren(rb);

    ras.auto_close(false);
    agg::conv_curve<agg::path_storage> fill_curve(shape.shape_path());
    fill_curve.approximation_scale(std::max(1.0, user_scale));
    agg::conv_transform<agg::conv_curve<agg::path_storage> > trans_shape(fill_curve, mtx);

    for (int s = min_style; s <= max_style; ++s) {
        ras.reset();
        for (unsigned i = 0; i < shape.paths(); ++i) {
            const agg::path_style& st = shape.style(i);
            if (st.left_fill != st.right_fill) {
                if (st.left_fill == s) {
                    ras.add_path(trans_shape, st.path_id);
                }
                if (st.right_fill == s) {
                    agg::path_storage tmp;
                    tmp.concat_path(trans_shape, st.path_id);
                    tmp.invert_polygon(0);
                    ras.add_path(tmp, 0);
                }
            }
        }
        agg::rgba8 color = (s >= 0 && s < 100) ? colors[s] : agg::rgba8(0, 0, 0, 255);
        agg::render_scanlines_aa_solid(ras, sl, rb, color);
    }
    ras.auto_close(true);

    // Strokes.
    agg::conv_curve<agg::path_storage> stroke_curve(shape.shape_path());
    stroke_curve.approximation_scale(std::max(1.0, user_scale));
    agg::conv_transform<agg::conv_curve<agg::path_storage> > trans_stroke(stroke_curve, mtx);
    agg::conv_stroke<agg::conv_transform<agg::conv_curve<agg::path_storage> > > stroke(trans_stroke);
    stroke.width(std::sqrt(user_scale));
    stroke.line_join(agg::round_join);
    stroke.line_cap(agg::round_cap);
    for (unsigned i = 0; i < shape.paths(); ++i) {
        const agg::path_style& st = shape.style(i);
        if (st.line >= 0) {
            ras.reset();
            ras.add_path(stroke, st.path_id);
            agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 0, 0, 128));
        }
    }

    // Help text.
    agg::gsv_text t;
    t.size(8.0);
    t.flip(true);
    t.start_point(10.0, 20.0);
    t.text("Space: Next Shape\n\n+/- : ZoomIn/ZoomOut (with respect to the mouse pointer)");
    agg::conv_stroke<agg::gsv_text> ts(t);
    ts.width(1.6);
    ts.line_cap(agg::round_cap);
    ras.reset();
    ras.add_path(ts);
    agg::render_scanlines_aa_solid(ras, sl, rb, agg::rgba8(0, 0, 0, 255));

    headless::write_raw(out, pixf, w, h);
}

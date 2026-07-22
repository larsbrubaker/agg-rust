// flash_rasterizer.cpp headless reproduction. Mirrors the Rust pixel-compare
// scene: read shape 0 from art/shapes.txt, fill it with the compound rasterizer
// (solid per-style palette), stroke the outlines, and draw the help text. The
// non-deterministic benchmark timing prefix is omitted so output is reproducible.
#include <cmath>
#include <vector>

#include "agg_basics.h"
#include "agg_rendering_buffer.h"
#include "agg_rasterizer_scanline_aa.h"
#include "agg_rasterizer_compound_aa.h"
#include "agg_scanline_u.h"
#include "agg_scanline_bin.h"
#include "agg_renderer_scanline.h"
#include "agg_conv_curve.h"
#include "agg_conv_transform.h"
#include "agg_conv_stroke.h"
#include "agg_span_allocator.h"
#include "agg_gsv_text.h"
#include "agg_pixfmt_rgba.h"

#include "common.h"
#include "flash_shape.h"

static void flash_palette(agg::rgba8* colors) {
    agg::msvc_rand rng;
    for (int i = 0; i < 100; ++i) {
        colors[i] = agg::rgba8(rng.next() & 0xFF, rng.next() & 0xFF, rng.next() & 0xFF, 230);
        colors[i].premultiply();
    }
}

int render_flash_rasterizer(unsigned w, unsigned h,
                            const std::vector<double>& params, const char* out) {
    typedef agg::pixfmt_bgra32 pixfmt;
    typedef agg::renderer_base<pixfmt> renderer_base;
    typedef agg::renderer_scanline_aa_solid<renderer_base> renderer_solid;

    int shape_index = params.size() > 0 ? (int)params[0] : 0;
    double user_scale = params.size() > 0 ? std::max(0.001, std::fabs(params[0])) : 1.0;
    // Extended-state params use params[0] as a shape index, not scale.
    if (params.size() >= 7) { shape_index = (int)params[0]; user_scale = 1.0; }
    else shape_index = 0;

    agg::compound_shape shape;
    if (!shape.open(headless::asset_path("shapes.txt").c_str())) {
        fprintf(stderr, "flash_rasterizer: failed to open shapes.txt\n");
        return 1;
    }
    for (int i = 0; i <= shape_index; ++i) {
        if (!shape.read_next()) break;
    }

    agg::rgba8 colors[100];
    flash_palette(colors);

    headless::canvas cv(w, h, 4);
    pixfmt pixf(cv.rbuf);
    renderer_base rb(pixf);
    rb.clear(agg::rgba8(255, 255, 242, 255)); // rgba(1.0, 1.0, 0.95)

    agg::trans_affine mtx = shape.compute_viewport(w, h);

    // Fill via the compound rasterizer (solid per-style colors).
    agg::rasterizer_compound_aa<agg::rasterizer_sl_clip_dbl> rasc;
    agg::scanline_u8 sl;
    agg::scanline_bin sl_bin;
    agg::span_allocator<agg::rgba8> alloc;
    agg::solid_styles style_handler(colors);

    rasc.clip_box(0, 0, w, h);
    rasc.layer_order(agg::layer_direct);
    rasc.reset();

    agg::conv_curve<agg::path_storage> fill_curve(shape.shape_path());
    fill_curve.approximation_scale(std::max(1.0, user_scale));
    agg::conv_transform<agg::conv_curve<agg::path_storage> > trans_shape(fill_curve, mtx);

    for (unsigned i = 0; i < shape.paths(); ++i) {
        const agg::path_style& st = shape.style(i);
        if (st.left_fill >= 0 || st.right_fill >= 0) {
            rasc.styles(st.left_fill, st.right_fill);
            rasc.add_path(trans_shape, st.path_id);
        }
    }
    agg::render_scanlines_compound(rasc, sl, sl_bin, rb, alloc, style_handler);

    // Strokes.
    agg::rasterizer_scanline_aa<agg::rasterizer_sl_clip_dbl> ras;
    renderer_solid ren(rb);
    ras.clip_box(0, 0, w, h);
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
            ren.color(agg::rgba8(0, 0, 0, 128));
            agg::render_scanlines(ras, sl, ren);
        }
    }

    // Help text (timer prefix omitted for deterministic output).
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
    ren.color(agg::rgba8(0, 0, 0, 255));
    agg::render_scanlines(ras, sl, ren);

    return headless::write_raw(out, pixf, w, h) ? 0 : 1;
}

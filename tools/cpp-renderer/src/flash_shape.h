// Extracted compound_shape reader + solid style handler from the AGG
// flash_rasterizer examples, used by the headless flash demos.
#ifndef HEADLESS_FLASH_SHAPE_H
#define HEADLESS_FLASH_SHAPE_H

#include <cstdio>
#include <cstring>
#include <cstdlib>

#include "agg_basics.h"
#include "agg_path_storage.h"
#include "agg_conv_curve.h"
#include "agg_conv_transform.h"
#include "agg_bounding_rect.h"
#include "agg_trans_viewport.h"
#include "agg_math.h"
#include "agg_array.h"
#include "agg_color_rgba.h"

namespace agg {

struct path_style {
    unsigned path_id;
    int left_fill;
    int right_fill;
    int line;
};

class compound_shape {
public:
    ~compound_shape() { if (m_fd) fclose(m_fd); }
    compound_shape()
        : m_path(), m_affine(), m_curve(m_path), m_trans(m_curve, m_affine), m_styles(), m_fd(0) {}

    bool open(const char* fname) { m_fd = fopen(fname, "r"); return m_fd != 0; }

    bool read_next() {
        m_path.remove_all();
        m_styles.remove_all();
        const char space[] = " \t\n\r";
        double ax, ay, cx, cy;
        if (m_fd) {
            char buf[1024];
            char* ts;
            for (;;) {
                if (fgets(buf, 1022, m_fd) == 0) return false;
                if (buf[0] == '=') break;
            }
            while (fgets(buf, 1022, m_fd)) {
                if (buf[0] == '!') break;
                if (buf[0] == 'P') {
                    path_style style;
                    style.path_id = m_path.start_new_path();
                    ts = strtok(buf, space);
                    ts = strtok(0, space); style.left_fill = atoi(ts);
                    ts = strtok(0, space); style.right_fill = atoi(ts);
                    ts = strtok(0, space); style.line = atoi(ts);
                    ts = strtok(0, space); ax = atof(ts);
                    ts = strtok(0, space); ay = atof(ts);
                    m_path.move_to(ax, ay);
                    m_styles.add(style);
                }
                if (buf[0] == 'C') {
                    ts = strtok(buf, space);
                    ts = strtok(0, space); cx = atof(ts);
                    ts = strtok(0, space); cy = atof(ts);
                    ts = strtok(0, space); ax = atof(ts);
                    ts = strtok(0, space); ay = atof(ts);
                    m_path.curve3(cx, cy, ax, ay);
                }
                if (buf[0] == 'L') {
                    ts = strtok(buf, space);
                    ts = strtok(0, space); ax = atof(ts);
                    ts = strtok(0, space); ay = atof(ts);
                    m_path.line_to(ax, ay);
                }
            }
            return true;
        }
        return false;
    }

    unsigned operator[](unsigned i) const { return m_styles[i].path_id; }
    unsigned paths() const { return m_styles.size(); }
    const path_style& style(unsigned i) const { return m_styles[i]; }
    void rewind(unsigned path_id) { m_trans.rewind(path_id); }
    unsigned vertex(double* x, double* y) { return m_trans.vertex(x, y); }
    double scale() const { return m_affine.scale(); }

    void scale(double w, double h) {
        m_affine.reset();
        double x1, y1, x2, y2;
        bounding_rect(m_path, *this, 0, m_styles.size(), &x1, &y1, &x2, &y2);
        if (x1 < x2 && y1 < y2) {
            trans_viewport vp;
            vp.preserve_aspect_ratio(0.5, 0.5, aspect_ratio_meet);
            vp.world_viewport(x1, y1, x2, y2);
            vp.device_viewport(0, 0, w, h);
            m_affine = vp.to_affine();
        }
        m_curve.approximation_scale(m_affine.scale());
    }

    void approximation_scale(double s) { m_curve.approximation_scale(m_affine.scale() * s); }
    const trans_affine& affine() const { return m_affine; }

    // Raw (untransformed, world-space) path with curve3 control points, matching
    // the Rust port's shape.path. Used to apply curve+viewport externally.
    path_storage& shape_path() { return m_path; }

    // The viewport transform (bbox -> device) the Rust port applies externally.
    trans_affine compute_viewport(double w, double h) {
        trans_affine a;
        double x1, y1, x2, y2;
        bounding_rect(m_path, *this, 0, m_styles.size(), &x1, &y1, &x2, &y2);
        if (x1 < x2 && y1 < y2) {
            trans_viewport vp;
            vp.preserve_aspect_ratio(0.5, 0.5, aspect_ratio_meet);
            vp.world_viewport(x1, y1, x2, y2);
            vp.device_viewport(0, 0, w, h);
            a = vp.to_affine();
        }
        return a;
    }

private:
    path_storage m_path;
    trans_affine m_affine;
    conv_curve<path_storage> m_curve;
    conv_transform<conv_curve<path_storage> > m_trans;
    pod_bvector<path_style> m_styles;
    FILE* m_fd;
};

// Solid-only compound style handler (mirrors test_styles with is_solid()==true).
class solid_styles {
public:
    solid_styles(const rgba8* solid_colors) : m_solid_colors(solid_colors) {}
    bool is_solid(unsigned) const { return true; }
    const rgba8& color(unsigned style) const { return m_solid_colors[style]; }
    void generate_span(rgba8*, int, int, unsigned, unsigned) {}
private:
    const rgba8* m_solid_colors;
};

// Replicates the MSVC/C89 rand() sequence so palette bytes match the Rust port
// (which reproduces the same generator in flash_palette).
struct msvc_rand {
    unsigned holdrand = 1;
    int next() { holdrand = holdrand * 214013u + 2531011u; return (holdrand >> 16) & 0x7fff; }
};

} // namespace agg

#endif

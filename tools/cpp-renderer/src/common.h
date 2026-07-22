// Headless harness shared helpers for the AGG C++ reference renderer.
//
// The renderer reproduces the original AGG 2.6 example demos without the
// interactive platform_support GUI framework. Every demo renders into a
// top-down, positive-stride buffer and is written out as a raw RGBA file with
// an 8-byte [width:u32-le][height:u32-le] header, matching the format read by
// the Rust `pixel-compare` tool (see tools/pixel-compare/src/lib.rs load_raw).
#ifndef AGG_HEADLESS_COMMON_H
#define AGG_HEADLESS_COMMON_H

#include <cstdio>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

#include "agg_rendering_buffer.h"
#include "agg_color_rgba.h"

// Absolute path to the AGG example assets, provided at compile time so the demo
// data files (shapes.txt, spheres, compositing.ppm) can be located at runtime.
#ifndef AGG_ASSET_DIR
#define AGG_ASSET_DIR "."
#endif

namespace headless {

// A simple owned rendering surface: raw bytes plus an AGG rendering_buffer that
// views them top-down (row 0 == top of image, positive stride).
struct canvas {
    unsigned width;
    unsigned height;
    unsigned bpp; // bytes per pixel
    std::vector<unsigned char> data;
    agg::rendering_buffer rbuf;

    canvas(unsigned w, unsigned h, unsigned bytes_per_pixel)
        : width(w), height(h), bpp(bytes_per_pixel),
          data(static_cast<size_t>(w) * h * bytes_per_pixel, 0) {
        rbuf.attach(data.data(), w, h, static_cast<int>(w * bytes_per_pixel));
    }
};

// Write a rendered pixel format out as raw RGBA (top-down) with the header the
// pixel-compare tool expects. Uses pixf.pixel() so the logical color is emitted
// regardless of the internal component order (bgr24, bgra32, ...).
template <class Pixfmt>
inline void write_raw(const char* path, Pixfmt& pixf, unsigned w, unsigned h) {
    FILE* f = fopen(path, "wb");
    if (!f) {
        fprintf(stderr, "Failed to open output '%s'\n", path);
        return;
    }
    uint32_t W = w, H = h;
    fwrite(&W, 4, 1, f);
    fwrite(&H, 4, 1, f);
    std::vector<unsigned char> row(static_cast<size_t>(w) * 4);
    for (unsigned y = 0; y < h; ++y) {
        for (unsigned x = 0; x < w; ++x) {
            typename Pixfmt::color_type c = pixf.pixel(static_cast<int>(x), static_cast<int>(y));
            row[x * 4 + 0] = c.r;
            row[x * 4 + 1] = c.g;
            row[x * 4 + 2] = c.b;
            row[x * 4 + 3] = c.a;
        }
        fwrite(row.data(), 1, row.size(), f);
    }
    fclose(f);
}

// Loaded image: RGBA bytes, top-down. Callers repack into whatever component
// order the demo's pixfmt expects.
struct image_rgba {
    unsigned width = 0;
    unsigned height = 0;
    std::vector<unsigned char> rgba; // top-down RGBA
    bool ok = false;
};

// Load a 24/32-bit Windows BMP into top-down RGBA (row 0 == top).
inline image_rgba load_bmp_rgba(const std::string& path) {
    image_rgba out;
    FILE* f = fopen(path.c_str(), "rb");
    if (!f) return out;
    std::vector<unsigned char> d;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    d.resize(sz);
    if (fread(d.data(), 1, sz, f) != (size_t)sz) { fclose(f); return out; }
    fclose(f);
    if (d.size() < 54 || d[0] != 'B' || d[1] != 'M') return out;
    uint32_t off = d[10] | (d[11] << 8) | (d[12] << 16) | (d[13] << 24);
    int32_t w = d[18] | (d[19] << 8) | (d[20] << 16) | (d[21] << 24);
    int32_t h = d[22] | (d[23] << 8) | (d[24] << 16) | (d[25] << 24);
    uint16_t bpp = d[28] | (d[29] << 8);
    bool top_down = h < 0;
    unsigned width = (w < 0) ? -w : w;
    unsigned height = (h < 0) ? -h : h;
    unsigned bytes_pp = bpp / 8;
    unsigned row_stride = ((width * bytes_pp + 3) / 4) * 4;
    out.width = width;
    out.height = height;
    out.rgba.assign(static_cast<size_t>(width) * height * 4, 255);
    for (unsigned y = 0; y < height; ++y) {
        unsigned src_y = top_down ? y : (height - 1 - y);
        size_t ro = off + static_cast<size_t>(src_y) * row_stride;
        for (unsigned x = 0; x < width; ++x) {
            size_t si = ro + static_cast<size_t>(x) * bytes_pp;
            size_t di = (static_cast<size_t>(y) * width + x) * 4;
            if (si + bytes_pp > d.size()) continue;
            out.rgba[di + 0] = d[si + 2];
            out.rgba[di + 1] = d[si + 1];
            out.rgba[di + 2] = d[si + 0];
            out.rgba[di + 3] = (bytes_pp == 4) ? d[si + 3] : 255;
        }
    }
    out.ok = true;
    return out;
}

// Load a binary PPM (P6) into top-down RGBA.
inline image_rgba load_ppm_rgba(const std::string& path) {
    image_rgba out;
    FILE* f = fopen(path.c_str(), "rb");
    if (!f) return out;
    char magic[3] = {0};
    if (fscanf(f, "%2s", magic) != 1 || strcmp(magic, "P6") != 0) { fclose(f); return out; }
    // Read width, height, maxval, skipping comments/whitespace.
    auto read_uint = [&](unsigned& v) -> bool {
        int c;
        // skip whitespace and comments
        for (;;) {
            c = fgetc(f);
            if (c == '#') { while (c != '\n' && c != EOF) c = fgetc(f); }
            else if (c == ' ' || c == '\t' || c == '\n' || c == '\r') continue;
            else break;
        }
        if (c == EOF) return false;
        v = 0;
        while (c >= '0' && c <= '9') { v = v * 10 + (c - '0'); c = fgetc(f); }
        return true;
    };
    unsigned width = 0, height = 0, maxv = 0;
    if (!read_uint(width) || !read_uint(height) || !read_uint(maxv)) { fclose(f); return out; }
    std::vector<unsigned char> rgb(static_cast<size_t>(width) * height * 3);
    if (fread(rgb.data(), 1, rgb.size(), f) != rgb.size()) { fclose(f); return out; }
    fclose(f);
    out.width = width;
    out.height = height;
    out.rgba.assign(static_cast<size_t>(width) * height * 4, 255);
    for (size_t i = 0; i < static_cast<size_t>(width) * height; ++i) {
        out.rgba[i * 4 + 0] = rgb[i * 3 + 0];
        out.rgba[i * 4 + 1] = rgb[i * 3 + 1];
        out.rgba[i * 4 + 2] = rgb[i * 3 + 2];
        out.rgba[i * 4 + 3] = 255;
    }
    out.ok = true;
    return out;
}

inline std::string asset_path(const char* name) {
    return std::string(AGG_ASSET_DIR) + "/" + name;
}

// Load the spheres image (the same one the Rust side embeds from
// demo/wasm/src/spheres.bmp) as top-down RGBA.
inline image_rgba load_spheres() {
#ifdef AGG_SPHERES_BMP
    image_rgba img = load_bmp_rgba(AGG_SPHERES_BMP);
    if (img.ok) return img;
#endif
    return load_ppm_rgba(asset_path("spheres.ppm"));
}

// Pack a top-down RGBA image into a freshly allocated buffer in the given
// byte order and wrap it in a rendering_buffer. `order` is one of "bgra",
// "rgba", "bgr", "rgb". Returns the owned byte vector via out_bytes.
inline void pack_image(const image_rgba& img, const char* order,
                       std::vector<unsigned char>& out_bytes,
                       agg::rendering_buffer& out_rbuf) {
    bool has_alpha = (order[3] != 0);
    unsigned bpp = has_alpha ? 4 : 3;
    out_bytes.assign(static_cast<size_t>(img.width) * img.height * bpp, 0);
    // index of r,g,b,a within the destination pixel
    int ri = 0, gi = 1, bi = 2, ai = 3;
    if (strcmp(order, "bgra") == 0) { ri = 2; gi = 1; bi = 0; ai = 3; }
    else if (strcmp(order, "rgba") == 0) { ri = 0; gi = 1; bi = 2; ai = 3; }
    else if (strcmp(order, "bgr") == 0) { ri = 2; gi = 1; bi = 0; }
    else { ri = 0; gi = 1; bi = 2; }
    for (size_t i = 0; i < static_cast<size_t>(img.width) * img.height; ++i) {
        unsigned char r = img.rgba[i * 4 + 0];
        unsigned char g = img.rgba[i * 4 + 1];
        unsigned char b = img.rgba[i * 4 + 2];
        unsigned char a = img.rgba[i * 4 + 3];
        out_bytes[i * bpp + ri] = r;
        out_bytes[i * bpp + gi] = g;
        out_bytes[i * bpp + bi] = b;
        if (has_alpha) out_bytes[i * bpp + ai] = a;
    }
    out_rbuf.attach(out_bytes.data(), img.width, img.height,
                    static_cast<int>(img.width * bpp));
}

} // namespace headless

#endif

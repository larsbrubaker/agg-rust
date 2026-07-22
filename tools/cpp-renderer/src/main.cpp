// agg-render: headless AGG 2.6 C++ reference renderer.
//
// Usage: agg-render <demo_name> <width> <height> <output.raw> [params...]
//
// Renders the named AGG example demo (in its default interactive state, or with
// optional numeric params mirroring the Rust pixel-compare demos) into a raw
// RGBA file that pixel-compare can byte-compare against the Rust output.
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

struct demo_entry {
    const char* name;
    void (*fn)(unsigned, unsigned, const std::vector<double>&, const char*);
};

// Per-demo render functions (defined in their own translation units).
#define DEMO(n) void render_##n(unsigned, unsigned, const std::vector<double>&, const char*);
DEMO(simple_line)
DEMO(perspective)
DEMO(conv_dash_marker)
DEMO(lion_outline)
DEMO(rasterizers2)
DEMO(image_perspective)
DEMO(image_transforms)
DEMO(image_filters)
DEMO(compositing2)
DEMO(flash_rasterizer)
DEMO(flash_rasterizer2)
#undef DEMO

static const demo_entry g_demos[] = {
    {"simple_line", render_simple_line},
    {"perspective", render_perspective},
    {"conv_dash_marker", render_conv_dash_marker},
    {"lion_outline", render_lion_outline},
    {"rasterizers2", render_rasterizers2},
    {"image_perspective", render_image_perspective},
    {"image_transforms", render_image_transforms},
    {"image_filters", render_image_filters},
    {"compositing2", render_compositing2},
    {"flash_rasterizer", render_flash_rasterizer},
    {"flash_rasterizer2", render_flash_rasterizer2},
};

int main(int argc, char** argv) {
    if (argc < 5) {
        fprintf(stderr, "Usage: %s <demo> <width> <height> <output.raw> [params...]\n", argv[0]);
        fprintf(stderr, "Demos:");
        for (const auto& d : g_demos) fprintf(stderr, " %s", d.name);
        fprintf(stderr, "\n");
        return 2;
    }
    const char* demo = argv[1];
    unsigned width = (unsigned)strtoul(argv[2], nullptr, 10);
    unsigned height = (unsigned)strtoul(argv[3], nullptr, 10);
    const char* out = argv[4];
    std::vector<double> params;
    for (int i = 5; i < argc; ++i) params.push_back(strtod(argv[i], nullptr));

    for (const auto& d : g_demos) {
        if (strcmp(demo, d.name) == 0) {
            d.fn(width, height, params, out);
            return 0;
        }
    }
    fprintf(stderr, "Unknown demo: %s\n", demo);
    return 1;
}

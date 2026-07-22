// agg-render: headless AGG 2.6 C++ reference renderer.
//
// Usage: agg-render <demo_name> <width> <height> <output.raw> [params...]
//
// Renders the named AGG example demo (in its default interactive state, or with
// optional numeric params mirroring the Rust pixel-compare demos) into a raw
// RGBA file that pixel-compare can byte-compare against the Rust output.
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

struct demo_entry {
    const char* name;
    // Returns 0 on success, nonzero on failure (asset load or I/O error).
    int (*fn)(unsigned, unsigned, const std::vector<double>&, const char*);
};

// Per-demo render functions (defined in their own translation units).
#define DEMO(n) int render_##n(unsigned, unsigned, const std::vector<double>&, const char*);
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

static const demo_entry* find_demo(const char* name) {
    for (const auto& d : g_demos) {
        if (strcmp(name, d.name) == 0) return &d;
    }
    return nullptr;
}

// In-process benchmark mode: time just the render call (no file output), so the
// numbers are not dominated by process startup or disk I/O.
// Usage: agg-render bench <demo> <width> <height> [params...] [--iters N]
static int run_bench(int argc, char** argv) {
    if (argc < 5) {
        fprintf(stderr, "Usage: %s bench <demo> <width> <height> [params...] [--iters N]\n", argv[0]);
        return 2;
    }
    const char* demo = argv[2];
    unsigned width = (unsigned)strtoul(argv[3], nullptr, 10);
    unsigned height = (unsigned)strtoul(argv[4], nullptr, 10);
    std::vector<double> params;
    int iters = 10;
    for (int i = 5; i < argc; ++i) {
        if (strcmp(argv[i], "--iters") == 0 && i + 1 < argc) {
            iters = (int)strtol(argv[i + 1], nullptr, 10);
            ++i;
        } else {
            params.push_back(strtod(argv[i], nullptr));
        }
    }
    if (iters < 1) {
        fprintf(stderr, "--iters must be at least 1\n");
        return 2;
    }

    const demo_entry* d = find_demo(demo);
    if (!d) {
        fprintf(stderr, "Unknown demo: %s\n", demo);
        return 1;
    }

    // Passing a null output path renders into the demo's in-memory buffer and
    // skips the file write (see headless::write_raw). `sink` keeps the optimizer
    // from discarding the render calls.
    volatile int sink = 0;

    // 2 untimed warmup iterations.
    for (int w = 0; w < 2; ++w) sink += d->fn(width, height, params, nullptr);

    std::vector<double> times;
    times.reserve((size_t)iters);
    for (int it = 0; it < iters; ++it) {
        auto start = std::chrono::steady_clock::now();
        int rc = d->fn(width, height, params, nullptr);
        auto end = std::chrono::steady_clock::now();
        sink += rc;
        double ms = std::chrono::duration<double, std::milli>(end - start).count();
        times.push_back(ms);
        printf("iter %3d: %.2f ms\n", it + 1, ms);
    }
    (void)sink;

    std::vector<double> sorted = times;
    std::sort(sorted.begin(), sorted.end());
    double best = sorted.front();
    size_t n = sorted.size();
    double median = (n % 2 == 1) ? sorted[n / 2] : (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0;
    double sum = 0.0;
    for (double t : times) sum += t;
    double mean = sum / (double)n;
    printf("best= %.2f ms  median= %.2f ms  mean= %.2f ms\n", best, median, mean);
    return 0;
}

int main(int argc, char** argv) {
    if (argc >= 2 && strcmp(argv[1], "bench") == 0) {
        return run_bench(argc, argv);
    }
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
            return d.fn(width, height, params, out);
        }
    }
    fprintf(stderr, "Unknown demo: %s\n", demo);
    return 1;
}

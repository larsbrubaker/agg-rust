// Copyright 2025. Pixel-perfect comparison CLI for AGG Rust vs C++ demos.
//
// Usage:
//   pixel-compare render <demo> <width> <height> [params...] -o <output.bmp>
//   pixel-compare compare <file_a> <file_b> [-d <diff.bmp>] [-s <sidebyside.bmp>]
//   pixel-compare verify <demo> <width> <height> --cpp <cpp_renderer_exe> [params...]
//   pixel-compare list

use pixel_compare::{
    compare_buffers, generate_diff_image, generate_sidebyside, load_image, save_image,
};
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "render" => cmd_render(&args[2..]),
        "compare" => cmd_compare(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        "bench" => cmd_bench(&args[2..]),
        "bench-compare" => cmd_bench_compare(&args[2..]),
        "list" => cmd_list(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!("pixel-compare — Pixel-perfect comparison tool for AGG demos");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  render <demo> <width> <height> [params...] -o <output.bmp|raw>");
    eprintln!("      Render a Rust demo to an image file.");
    eprintln!();
    eprintln!("  compare <file_a> <file_b> [-d <diff.bmp>] [-s <sidebyside.bmp>]");
    eprintln!("      Compare two image files pixel-by-pixel.");
    eprintln!();
    eprintln!("  verify <demo> <width> <height> --cpp <cpp_exe> [params...]");
    eprintln!("      Render both Rust and C++, then compare.");
    eprintln!();
    eprintln!("  bench <demo> <width> <height> [params...] [--iters N]");
    eprintln!("      Time just the render call (default N=10, plus 2 warmups).");
    eprintln!();
    eprintln!("  bench-compare --cpp <agg_render_exe> [--iters N] [--passes N] [--out <file.md>] [--date <YYYY-MM-DD>]");
    eprintln!("      Benchmark every shared demo in-process (Rust) and via the C++");
    eprintln!("      renderer subprocess, then emit a markdown comparison table.");
    eprintln!("      --passes N (default 1) runs N full interleaved passes and pools");
    eprintln!("      all per-iter samples per demo/side before computing best/median.");
    eprintln!();
    eprintln!("  list");
    eprintln!("      List available demo names.");
}

fn cmd_list() {
    println!("Available demos:");
    for name in pixel_compare::render::available_demos() {
        println!("  {}", name);
    }
}

fn cmd_render(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Usage: pixel-compare render <demo> <width> <height> [params...] -o <output>");
        process::exit(1);
    }

    let demo = &args[0];
    let width: u32 = args[1].parse().expect("Invalid width");
    let height: u32 = args[2].parse().expect("Invalid height");

    // Parse remaining args: params and -o flag
    let mut params = Vec::new();
    let mut output_path: Option<String> = None;
    let mut i = 3;
    while i < args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            output_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            params.push(args[i].parse::<f64>().expect("Invalid param (must be f64)"));
            i += 1;
        }
    }

    let output = output_path.unwrap_or_else(|| format!("{}_{}x{}.bmp", demo, width, height));

    println!("Rendering '{}' at {}x{} with params {:?}...", demo, width, height, params);

    let buf = pixel_compare::render::render_demo(demo, width, height, &params)
        .unwrap_or_else(|| {
            eprintln!("Unknown demo: '{}'. Use 'list' to see available demos.", demo);
            process::exit(1);
        });

    save_image(Path::new(&output), &buf).expect("Failed to save image");
    println!("Saved: {}", output);
}

fn cmd_bench(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Usage: pixel-compare bench <demo> <width> <height> [params...] [--iters N]");
        process::exit(1);
    }

    let demo = &args[0];
    let width: u32 = args[1].parse().expect("Invalid width");
    let height: u32 = args[2].parse().expect("Invalid height");

    // Parse remaining args: params and --iters flag.
    let mut params = Vec::new();
    let mut iters: usize = 10;
    let mut i = 3;
    while i < args.len() {
        if args[i] == "--iters" && i + 1 < args.len() {
            iters = args[i + 1].parse().expect("Invalid --iters (must be a positive integer)");
            i += 2;
        } else {
            params.push(args[i].parse::<f64>().expect("Invalid param (must be f64)"));
            i += 1;
        }
    }
    if iters == 0 {
        eprintln!("--iters must be at least 1");
        process::exit(1);
    }

    let render = || {
        pixel_compare::render::render_demo(demo, width, height, &params).unwrap_or_else(|| {
            eprintln!("Unknown demo: '{}'. Use 'list' to see available demos.", demo);
            process::exit(1);
        })
    };

    // Reuse the shared timing loop so `bench` and `bench-compare` measure the
    // render call identically (2 untimed warmups + `iters` timed iterations).
    let times = time_render_loop(render, WARMUP_ITERS, iters, true);

    let mut sorted = times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let best = sorted[0];
    let n = sorted.len();
    let median = median_ms(&times);
    let mean = times.iter().sum::<f64>() / n as f64;
    println!("best= {:.2} ms  median= {:.2} ms  mean= {:.2} ms", best, median, mean);
}

/// Number of untimed warmup iterations run before the timed loop. Kept in sync
/// with the C++ renderer's `run_bench` (which also does 2 warmups) so both sides
/// measure steady-state render times.
const WARMUP_ITERS: usize = 2;

/// Run `render` for `WARMUP_ITERS` untimed warmups, then `iters` timed
/// iterations, returning the per-iteration times in milliseconds.
///
/// This is the single source of truth for the in-process timing loop shared by
/// the `bench` and `bench-compare` subcommands: the render closure is called,
/// its result is `black_box`ed so the optimizer cannot elide the work, and the
/// wall-clock elapsed time is recorded. When `verbose` is set, each timed
/// iteration prints an `iter %3d: %.2f ms` line matching the C++ output format.
fn time_render_loop<T, F: FnMut() -> T>(
    mut render: F,
    warmups: usize,
    iters: usize,
    verbose: bool,
) -> Vec<f64> {
    for _ in 0..warmups {
        let buf = render();
        std::hint::black_box(&buf);
    }

    let mut times = Vec::with_capacity(iters);
    for it in 0..iters {
        let start = std::time::Instant::now();
        let buf = render();
        let elapsed = start.elapsed();
        // Keep the optimizer from eliding the render work.
        std::hint::black_box(&buf);
        let ms = elapsed.as_secs_f64() * 1000.0;
        times.push(ms);
        if verbose {
            println!("iter {:>3}: {:.2} ms", it + 1, ms);
        }
    }
    times
}

/// Best (minimum) of a slice of per-iteration times, computed identically to the
/// C++ renderer's `run_bench` `best=` value. The project's benchmark methodology
/// compares best-of first (the fastest run is the least contaminated by OS
/// scheduling jitter) and medians second. Panics on an empty slice — callers
/// always pass at least one sample.
fn best_ms(times: &[f64]) -> f64 {
    assert!(!times.is_empty(), "best_ms requires at least one sample");
    times.iter().copied().fold(f64::INFINITY, f64::min)
}

/// Median of a slice of per-iteration times, computed identically to the C++
/// renderer's `run_bench`: sort ascending, take the middle element (odd count)
/// or the mean of the two middle elements (even count). Panics on an empty
/// slice — callers always pass at least one sample.
fn median_ms(times: &[f64]) -> f64 {
    assert!(!times.is_empty(), "median_ms requires at least one sample");
    let mut sorted = times.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// Pool the per-iteration samples from every pass into a single sample set and
/// return `(best, median)` over the pooled samples.
///
/// Multi-pass pooling is the benchmark suite's noise-robustness strategy on a
/// non-quiet machine: the best is the min over *all* pooled samples, so upward
/// scheduling jitter in any one pass cannot survive it, and the median stabilizes
/// as the pooled count grows (N passes x M iters => N*M samples). The min and
/// median of the concatenation are, by definition, the min and median of the
/// pooled set — so pooling is exactly a flatten followed by `best_ms`/`median_ms`.
/// Panics only if there are no samples at all (every pass was empty), which the
/// caller prevents by never invoking this on an absent side.
fn pooled_best_median(pass_samples: &[Vec<f64>]) -> (f64, f64) {
    let pooled: Vec<f64> = pass_samples.iter().flatten().copied().collect();
    (best_ms(&pooled), median_ms(&pooled))
}

/// A demo included in the Rust-vs-C++ benchmark suite, rendered at the canonical
/// size and params used by both registries.
struct BenchCase {
    name: &'static str,
    width: u32,
    height: u32,
    params: &'static [f64],
    /// True when a committed test pins the Rust output byte-for-byte against the
    /// C++ reference at this size (see the `byte_verified` note on `BENCH_CASES`).
    /// Every benchmarked demo is currently byte-verified.
    byte_verified: bool,
}

/// Demos supported by BOTH the Rust (`render_demo`) and C++ (`agg-render`)
/// registries, at the sizes used by the committed reference tests
/// (e.g. compositing2 600x400, lion_outline 512x512, flash_rasterizer 655x520).
///
/// The Rust-only demos (`compositing`, `truetype_test`) are intentionally
/// excluded so every row benchmarks the same scene on both sides. Every demo
/// below has `byte_verified: true`: its Rust output is pinned **byte-for-byte**
/// against the C++ reference by a committed test:
///   - `simple_line`, `lion_outline`, `rasterizers2`, `conv_dash_marker`,
///     `perspective`, `image_perspective`, `image_transforms`, `image_filters`
///     — via `tools/pixel-compare/tests/*_reference.rs`
///   - `compositing2`, `flash_rasterizer`, `flash_rasterizer2` — via the
///     reference checks in `src/render/mod.rs`
///
/// Adding a demo later is a one-line change here.
const BENCH_CASES: &[BenchCase] = &[
    BenchCase { name: "simple_line", width: 512, height: 512, params: &[], byte_verified: true },
    BenchCase { name: "lion_outline", width: 512, height: 512, params: &[], byte_verified: true },
    BenchCase { name: "rasterizers2", width: 500, height: 450, params: &[], byte_verified: true },
    BenchCase { name: "conv_dash_marker", width: 500, height: 330, params: &[], byte_verified: true },
    BenchCase { name: "perspective", width: 600, height: 600, params: &[], byte_verified: true },
    BenchCase { name: "image_perspective", width: 600, height: 600, params: &[], byte_verified: true },
    BenchCase { name: "image_transforms", width: 430, height: 340, params: &[], byte_verified: true },
    BenchCase { name: "image_filters", width: 430, height: 340, params: &[], byte_verified: true },
    BenchCase { name: "compositing2", width: 600, height: 400, params: &[], byte_verified: true },
    BenchCase { name: "flash_rasterizer", width: 655, height: 520, params: &[], byte_verified: true },
    BenchCase { name: "flash_rasterizer2", width: 655, height: 520, params: &[], byte_verified: true },
];

/// Result of benchmarking one demo on both sides. The `cpp_*` fields are `None`
/// when the C++ subprocess failed or produced unparseable output for that demo.
/// `best` and `median` are computed from the same per-iteration samples on each
/// side, so the two ratios are directly comparable.
struct BenchOutcome {
    name: &'static str,
    width: u32,
    height: u32,
    byte_verified: bool,
    rust_best: f64,
    rust_median: f64,
    cpp_best: Option<f64>,
    cpp_median: Option<f64>,
    error: Option<String>,
}

fn cmd_bench_compare(args: &[String]) {
    let mut cpp_exe: Option<String> = None;
    let mut iters: usize = 10;
    let mut passes: usize = 1;
    let mut out_path: Option<String> = None;
    let mut date: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--cpp" if i + 1 < args.len() => {
                cpp_exe = Some(args[i + 1].clone());
                i += 2;
            }
            "--iters" if i + 1 < args.len() => {
                iters = args[i + 1]
                    .parse()
                    .expect("Invalid --iters (must be a positive integer)");
                i += 2;
            }
            "--passes" if i + 1 < args.len() => {
                passes = args[i + 1]
                    .parse()
                    .expect("Invalid --passes (must be a positive integer)");
                i += 2;
            }
            "--out" if i + 1 < args.len() => {
                out_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--date" if i + 1 < args.len() => {
                date = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("Unknown option: {other}");
                process::exit(1);
            }
        }
    }

    if iters == 0 {
        eprintln!("--iters must be at least 1");
        process::exit(1);
    }
    if passes == 0 {
        eprintln!("--passes must be at least 1");
        process::exit(1);
    }

    let cpp_exe = cpp_exe.unwrap_or_else(|| {
        eprintln!("--cpp <path_to_agg_render_exe> is required");
        process::exit(1);
    });

    eprintln!(
        "Benchmarking {} demos ({} passes x {} timed iters + {} warmups per pass)...",
        BENCH_CASES.len(),
        passes,
        iters,
        WARMUP_ITERS
    );

    // Per-demo pooled samples: each inner Vec<f64> is one pass's timed iterations.
    // Every pass appends one entry per side, so `pooled_best_median` sees all
    // passes together. The Rust-present flag and any C++ error are recorded once.
    let n = BENCH_CASES.len();
    let mut rust_passes: Vec<Vec<Vec<f64>>> = vec![Vec::new(); n];
    let mut cpp_passes: Vec<Vec<Vec<f64>>> = vec![Vec::new(); n];
    let mut cpp_error: Vec<Option<String>> = vec![None; n];
    // Presence via the registry's name list rather than a throwaway render, so the
    // Rust side gets exactly `WARMUP_ITERS` warmups per pass — the same count as
    // the C++ side (a probe render would give Rust an extra warmup and skew things).
    let available = pixel_compare::render::available_demos();
    let rust_present: Vec<bool> = BENCH_CASES.iter().map(|c| available.contains(&c.name)).collect();

    // Interleave Rust and C++ per demo within each pass, exactly as the single-pass
    // path did. Running the full sweep `passes` times (rather than `passes` back-to-
    // back reps of one demo) spreads each demo's samples across the whole run, so a
    // transient load spike contaminates one pass of a demo, not all of its samples.
    for pass in 0..passes {
        if passes > 1 {
            eprintln!("Pass {}/{}", pass + 1, passes);
        }
        for (idx, case) in BENCH_CASES.iter().enumerate() {
            eprintln!("  {} {}x{}", case.name, case.width, case.height);

            if !rust_present[idx] {
                if pass == 0 {
                    eprintln!("    Rust renderer missing '{}', skipping", case.name);
                }
                continue;
            }

            let rust_times = time_render_loop(
                || {
                    pixel_compare::render::render_demo(
                        case.name, case.width, case.height, case.params,
                    )
                    .expect("demo verified present in registry above")
                },
                WARMUP_ITERS,
                iters,
                false,
            );
            rust_passes[idx].push(rust_times);

            // C++ side: run the `agg-render bench` subprocess and parse its per-iter
            // lines. On any failure, record the first error and simply pool fewer
            // C++ samples for this demo (the row still reports whatever passes ran).
            match run_cpp_bench(&cpp_exe, case, iters) {
                Ok(cpp_times) => cpp_passes[idx].push(cpp_times),
                Err(e) => {
                    eprintln!("    C++ bench failed for '{}': {e}", case.name);
                    if cpp_error[idx].is_none() {
                        cpp_error[idx] = Some(e);
                    }
                }
            }
        }
    }

    // Collapse the pooled per-pass samples into one outcome per demo.
    let mut outcomes = Vec::with_capacity(n);
    for (idx, case) in BENCH_CASES.iter().enumerate() {
        if !rust_present[idx] {
            outcomes.push(BenchOutcome {
                name: case.name,
                width: case.width,
                height: case.height,
                byte_verified: case.byte_verified,
                rust_best: f64::NAN,
                rust_median: f64::NAN,
                cpp_best: None,
                cpp_median: None,
                error: Some("Rust renderer missing".to_string()),
            });
            continue;
        }

        let (rust_best, rust_median) = pooled_best_median(&rust_passes[idx]);
        let (cpp_best, cpp_median, error) = if cpp_passes[idx].is_empty() {
            (
                None,
                None,
                Some(cpp_error[idx].clone().unwrap_or_else(|| "C++ bench failed".to_string())),
            )
        } else {
            let (b, m) = pooled_best_median(&cpp_passes[idx]);
            (Some(b), Some(m), None)
        };

        outcomes.push(BenchOutcome {
            name: case.name,
            width: case.width,
            height: case.height,
            byte_verified: case.byte_verified,
            rust_best,
            rust_median,
            cpp_best,
            cpp_median,
            error,
        });
    }

    let doc = render_benchmark_doc(&outcomes, iters, passes, &cpp_exe, date.as_deref());

    print!("{doc}");

    if let Some(path) = out_path {
        std::fs::write(&path, &doc).expect("Failed to write benchmark markdown");
        eprintln!("Wrote {path}");
    }
}

/// Run `agg-render bench <demo> <w> <h> [params...] --iters N` as a subprocess
/// and parse the per-iteration `iter %3d: %.2f ms` lines from its stdout into a
/// vector of millisecond samples. Returns an error string (never panics) when
/// the process cannot be spawned, exits non-zero, or emits no parseable samples.
fn run_cpp_bench(cpp_exe: &str, case: &BenchCase, iters: usize) -> Result<Vec<f64>, String> {
    let mut cmd_args = vec![
        "bench".to_string(),
        case.name.to_string(),
        case.width.to_string(),
        case.height.to_string(),
    ];
    for p in case.params {
        cmd_args.push(p.to_string());
    }
    cmd_args.push("--iters".to_string());
    cmd_args.push(iters.to_string());

    let output = std::process::Command::new(cpp_exe)
        .args(&cmd_args)
        .output()
        .map_err(|e| format!("failed to spawn '{cpp_exe}': {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exited with {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let samples = parse_cpp_bench_output(&stdout);
    if samples.is_empty() {
        return Err("no 'iter N: X ms' lines found in C++ output".to_string());
    }
    // Silent sample loss would skew the median unnoticed. A short read means the
    // C++ output format drifted or lines were dropped — warn but still use what
    // we parsed (the median of the surviving samples is better than nothing).
    if samples.len() != iters {
        eprintln!(
            "    warning: '{}' parsed {} of {} expected C++ iter samples",
            case.name,
            samples.len(),
            iters
        );
    }
    Ok(samples)
}

/// Extract the millisecond samples from the C++ renderer's `bench` stdout.
///
/// The C++ side prints one `iter %3d: %.2f ms` line per timed iteration (plus a
/// `best=/median=/mean=` summary line, which is ignored here). We parse only the
/// per-iter lines so both sides compute the median from the same raw samples.
fn parse_cpp_bench_output(stdout: &str) -> Vec<f64> {
    let mut samples = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("iter") else {
            continue;
        };
        // rest looks like "   3: 12.34 ms"
        let Some(colon) = rest.find(':') else {
            continue;
        };
        let after = rest[colon + 1..].trim();
        let Some(value) = after.split_whitespace().next() else {
            continue;
        };
        if let Ok(ms) = value.parse::<f64>() {
            samples.push(ms);
        }
    }
    samples
}

/// Build the full `BENCHMARKS.md` document: metadata header, methodology,
/// results table, and regeneration commands. Structured so re-running
/// `bench-compare` regenerates the entire file deterministically (aside from the
/// measured times and the auto-detected date/machine/compiler fields).
fn render_benchmark_doc(
    outcomes: &[BenchOutcome],
    iters: usize,
    passes: usize,
    cpp_exe: &str,
    date: Option<&str>,
) -> String {
    let date = date
        .map(|d| d.to_string())
        .unwrap_or_else(detect_date);
    let cpu = detect_cpu();
    let os = detect_os();
    let rustc = detect_rustc();
    let msvc = detect_msvc(Path::new(cpp_exe));

    let mut s = String::new();
    s.push_str("# AGG.rs Benchmarks — Rust vs C++\n\n");
    s.push_str(&format!("- **Generated:** {date}\n"));
    s.push_str(&format!("- **Machine:** {cpu}\n"));
    s.push_str(&format!("- **OS:** {os}\n"));
    s.push_str(&format!("- **Rust compiler:** {rustc}\n"));
    s.push_str(&format!("- **C++ compiler:** {msvc}\n"));
    s.push_str(&format!(
        "- **Iterations:** {passes} passes x {iters} timed + {WARMUP_ITERS} warmups per pass\n\n"
    ));

    s.push_str("## Methodology\n\n");
    s.push_str(
        "Each demo is rendered by both the Rust port (in-process, via \
`pixel-compare`) and the original AGG 2.6 C++ library (the `agg-render` \
subprocess). Timings cover **the render call only** — no process startup, asset \
loading, or file I/O is included. Each side runs the same number of untimed \
warmup iterations followed by the same number of timed iterations. From those \
per-iteration samples both the **best (minimum)** and the **median** are \
reported, computed identically on both sides. **Compare best-of first** — the \
fastest run is the least contaminated by OS scheduling jitter, so it is the \
primary signal; the median is a secondary sanity check that resists outliers. \
Single runs are noise, and differences below roughly ±2 ms are at or under the \
measurement floor on this machine. Both sides render at identical sizes with \
identical parameters.\n\n",
    );
    s.push_str(&format!(
        "This table is generated with **{passes} passes x {iters} iterations, \
{WARMUP_ITERS} warmups per pass**; each pass sweeps every demo (interleaving the \
Rust and C++ measurement of one demo before moving to the next), and the \
**best and median are computed over all pooled samples** from every pass. \
Pooling across passes is what makes the numbers robust on a machine that is not \
perfectly idle: a transient load spike can only inflate the samples in one \
pass, so it cannot survive the pooled minimum, and the pooled median stabilizes \
as the sample count grows.\n\n",
    ));
    s.push_str(
        "Critically, both renderers draw **the same scene** at the same size. \
Every demo in the table below is byte-identical: a committed pixel-compare \
reference test pins the Rust output byte-for-byte against the C++ output, so \
each speed difference reflects the implementation rather than a difference in \
what is drawn.\n\n",
    );

    s.push_str("## Results\n\n");
    s.push_str(&render_results_table(outcomes));
    s.push_str(
        "\nThe **Byte-identical** column records, per row, that the demo's Rust \
output is pinned byte-for-byte against the C++ reference by a committed test — \
the invariant that makes each timing an apples-to-apples comparison.\n\n",
    );

    s.push_str("## Regenerating\n\n");
    s.push_str("```bash\n");
    s.push_str("# 1. Build the Rust benchmark tool (release):\n");
    s.push_str("cargo build --release -p pixel-compare\n\n");
    s.push_str("# 2. Build the C++ reference renderer (release):\n");
    s.push_str("cmake -S tools/cpp-renderer -B tools/cpp-renderer/build -A x64\n");
    s.push_str("cmake --build tools/cpp-renderer/build --config Release\n\n");
    s.push_str("# 3. Run the full suite and regenerate this file:\n");
    s.push_str(&format!(
        "target\\release\\pixel-compare bench-compare \\\n  --cpp tools\\cpp-renderer\\build\\Release\\agg-render.exe \\\n  --passes {passes} --iters {iters} --date {date} --out docs\\BENCHMARKS.md\n"
    ));
    s.push_str("```\n");

    s
}

/// Render just the results table (also inlined into the README).
///
/// Best-of columns come first (per the project's benchmark methodology: the
/// fastest run is the least contaminated by OS jitter, so best-of is the primary
/// comparison and medians are the secondary sanity check). Both `best` and
/// `median` are computed from the same per-iteration samples on each side, so the
/// two ratios are directly comparable.
fn render_results_table(outcomes: &[BenchOutcome]) -> String {
    let mut s = String::new();
    s.push_str("| Demo | Size | Byte-identical | C++ best (ms) | Rust best (ms) | Best Rust / C++ | C++ median (ms) | Rust median (ms) | Median Rust / C++ |\n");
    s.push_str("|------|------|----------------|---------------|----------------|-----------------|-----------------|------------------|-------------------|\n");
    for o in outcomes {
        let size = format!("{}x{}", o.width, o.height);
        let verified = if o.byte_verified { "yes" } else { "—" };
        match (o.cpp_best, o.cpp_median, &o.error) {
            (Some(cpp_best), Some(cpp_median), _) => {
                // Guard against a zero C++ time: the ratio would be inf/NaN.
                let best_ratio = if cpp_best > 0.0 {
                    format!("{:.2}x", o.rust_best / cpp_best)
                } else {
                    "n/a".to_string()
                };
                let median_ratio = if cpp_median > 0.0 {
                    format!("{:.2}x", o.rust_median / cpp_median)
                } else {
                    "n/a".to_string()
                };
                s.push_str(&format!(
                    "| {} | {} | {} | {:.2} | {:.2} | {} | {:.2} | {:.2} | {} |\n",
                    o.name,
                    size,
                    verified,
                    cpp_best,
                    o.rust_best,
                    best_ratio,
                    cpp_median,
                    o.rust_median,
                    median_ratio
                ));
            }
            (_, _, err) => {
                let rust_best = if o.rust_best.is_nan() {
                    "FAILED".to_string()
                } else {
                    format!("{:.2}", o.rust_best)
                };
                let rust_median = if o.rust_median.is_nan() {
                    "FAILED".to_string()
                } else {
                    format!("{:.2}", o.rust_median)
                };
                let note = err.as_deref().unwrap_or("failed");
                s.push_str(&format!(
                    "| {} | {} | {} | FAILED ({}) | {} | - | FAILED ({}) | {} | - |\n",
                    o.name, size, verified, note, rust_best, note, rust_median
                ));
            }
        }
    }
    s
}

/// Auto-detect today's date as `YYYY-MM-DD` via PowerShell. Falls back to an
/// explicit placeholder if the command fails (caller can override with --date).
fn detect_date() -> String {
    run_capture(
        "powershell",
        &["-NoProfile", "-Command", "Get-Date -Format yyyy-MM-dd"],
    )
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "unknown".to_string())
}

/// Auto-detect the CPU model name. Tries `wmic cpu get name`, then falls back to
/// the `PROCESSOR_IDENTIFIER` environment variable.
fn detect_cpu() -> String {
    if let Some(out) = run_capture("wmic", &["cpu", "get", "name"]) {
        // Output: header line "Name", then the value, then blanks.
        for line in out.lines() {
            let line = line.trim();
            if !line.is_empty() && line != "Name" {
                return line.to_string();
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown CPU".to_string())
}

/// Auto-detect the OS version string via `cmd /c ver`.
fn detect_os() -> String {
    run_capture("cmd", &["/c", "ver"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown OS".to_string())
}

/// Auto-detect the Rust compiler version via `rustc --version`.
fn detect_rustc() -> String {
    run_capture("rustc", &["--version"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown rustc".to_string())
}

/// Auto-detect the MSVC compiler version by reading `CMAKE_CXX_COMPILER_VERSION`
/// out of the CMake-generated `CMakeCXXCompiler.cmake` in the C++ build tree
/// (found by ascending from the `agg-render` exe). Returns a descriptive
/// fallback when the build tree cannot be located.
fn detect_msvc(cpp_exe: &Path) -> String {
    // The exe normally lives at <build>/Release/agg-render.exe, so search from
    // its grandparent (the build dir) down into CMakeFiles/*/.
    let mut search_root = cpp_exe.parent().map(Path::to_path_buf);
    for _ in 0..4 {
        let Some(dir) = search_root.clone() else {
            break;
        };
        if let Some(found) = find_file_named(&dir, "CMakeCXXCompiler.cmake", 3) {
            if let Ok(contents) = std::fs::read_to_string(&found) {
                for line in contents.lines() {
                    if let Some(rest) = line.trim().strip_prefix("set(CMAKE_CXX_COMPILER_VERSION") {
                        let ver = rest.trim().trim_end_matches(')').trim().trim_matches('"');
                        if !ver.is_empty() {
                            return format!("MSVC {ver}");
                        }
                    }
                }
            }
        }
        search_root = dir.parent().map(Path::to_path_buf);
    }
    "MSVC (version undetected)".to_string()
}

/// Depth-limited search for a file with the given name under `dir`.
fn find_file_named(dir: &Path, name: &str, max_depth: usize) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    if max_depth == 0 {
        return None;
    }
    for sub in subdirs {
        if let Some(found) = find_file_named(&sub, name, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

/// Run a command and capture its stdout as a String, returning None if the
/// command cannot be spawned or exits non-zero.
fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn cmd_compare(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: pixel-compare compare <file_a> <file_b> [-d <diff>] [-s <sidebyside>]");
        process::exit(1);
    }

    let path_a = &args[0];
    let path_b = &args[1];
    let mut diff_path: Option<String> = None;
    let mut sbs_path: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-d" if i + 1 < args.len() => {
                diff_path = Some(args[i + 1].clone());
                i += 2;
            }
            "-s" if i + 1 < args.len() => {
                sbs_path = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                i += 1;
            }
        }
    }

    let a = load_image(Path::new(path_a)).expect("Failed to load file A");
    let b = load_image(Path::new(path_b)).expect("Failed to load file B");

    let result = compare_buffers(&a, &b);
    println!("{}", result);

    if let Some(dp) = diff_path {
        let diff = generate_diff_image(&a, &b);
        save_image(Path::new(&dp), &diff).expect("Failed to save diff image");
        println!("Diff saved: {}", dp);
    }

    if let Some(sp) = sbs_path {
        let sbs = generate_sidebyside(&a, &b);
        save_image(Path::new(&sp), &sbs).expect("Failed to save side-by-side image");
        println!("Side-by-side saved: {}", sp);
    }

    if !result.identical {
        process::exit(1);
    }
}

/// Suffix appended to verify output filenames so runs with different
/// params never overwrite each other. Empty params keep the legacy
/// name (no suffix) for backward compatibility with existing files.
fn params_suffix(params: &[f64]) -> String {
    if params.is_empty() {
        return String::new();
    }

    // Deterministic FNV-1a hash over each param's raw bit pattern.
    // std's DefaultHasher is randomly seeded per process, so it cannot be
    // used here — filenames must be stable across runs and platforms.
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for p in params {
        let bits = p.to_bits();
        for byte in bits.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }

    format!("_p{:08x}", hash as u32)
}

fn cmd_verify(args: &[String]) {
    if args.len() < 3 {
        eprintln!(
            "Usage: pixel-compare verify <demo> <width> <height> --cpp <cpp_exe> [params...]"
        );
        process::exit(1);
    }

    let demo = &args[0];
    let width: u32 = args[1].parse().expect("Invalid width");
    let height: u32 = args[2].parse().expect("Invalid height");

    let mut cpp_exe: Option<String> = None;
    let mut params = Vec::new();
    let mut diff_path: Option<String> = None;

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--cpp" if i + 1 < args.len() => {
                cpp_exe = Some(args[i + 1].clone());
                i += 2;
            }
            "-d" if i + 1 < args.len() => {
                diff_path = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                params.push(args[i].parse::<f64>().expect("Invalid param"));
                i += 1;
            }
        }
    }

    let cpp_exe = cpp_exe.unwrap_or_else(|| {
        eprintln!("--cpp <path_to_cpp_renderer> is required");
        process::exit(1);
    });

    let suffix = params_suffix(&params);

    // Scratch/inspection files go under target/pixel-compare/ rather than the
    // CWD: the repo root holds committed golden .raw references (embedded via
    // include_bytes! in the regression tests), and writing here would silently
    // overwrite them.
    let out_dir = PathBuf::from("target").join("pixel-compare");
    std::fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    // 1. Render Rust version
    println!("Rendering Rust '{}'...", demo);
    let rust_buf = pixel_compare::render::render_demo(demo, width, height, &params)
        .unwrap_or_else(|| {
            eprintln!("Unknown demo: '{}'", demo);
            process::exit(1);
        });

    // Save Rust output for inspection
    let rust_path = out_dir.join(format!("{}_rust_{}x{}{}.bmp", demo, width, height, suffix));
    save_image(&rust_path, &rust_buf).expect("Failed to save Rust output");
    println!("  Saved Rust output: {}", rust_path.display());

    // 2. Run C++ renderer
    let cpp_raw_path = out_dir.join(format!("{}_cpp_{}x{}{}.raw", demo, width, height, suffix));
    println!("Rendering C++ '{}'...", demo);

    let mut cmd_args = vec![
        demo.clone(),
        width.to_string(),
        height.to_string(),
        cpp_raw_path.to_string_lossy().into_owned(),
    ];
    for p in &params {
        cmd_args.push(p.to_string());
    }

    let status = std::process::Command::new(&cpp_exe)
        .args(&cmd_args)
        .status()
        .expect("Failed to run C++ renderer");

    if !status.success() {
        eprintln!("C++ renderer failed with exit code {:?}", status.code());
        process::exit(1);
    }

    let cpp_buf = load_image(&cpp_raw_path).expect("Failed to load C++ output");

    // The headless C++ renderer writes top-down buffers (positive stride),
    // matching Rust's layout, so no row flip is needed here.

    // Save C++ output as BMP for inspection
    let cpp_bmp_path = out_dir.join(format!("{}_cpp_{}x{}{}.bmp", demo, width, height, suffix));
    save_image(&cpp_bmp_path, &cpp_buf).expect("Failed to save C++ BMP");
    println!("  Saved C++ output: {}", cpp_bmp_path.display());

    // 3. Compare
    let result = compare_buffers(&rust_buf, &cpp_buf);
    println!("\n{}", result);

    if let Some(dp) = diff_path {
        let diff = generate_diff_image(&rust_buf, &cpp_buf);
        save_image(Path::new(&dp), &diff).expect("Failed to save diff");
        println!("Diff saved: {}", dp);
    }

    if !result.identical {
        // Always save side-by-side on failure
        let sbs = generate_sidebyside(&rust_buf, &cpp_buf);
        let sbs_path = out_dir.join(format!("{}_sidebyside_{}x{}{}.bmp", demo, width, height, suffix));
        save_image(&sbs_path, &sbs).expect("Failed to save side-by-side");
        println!("Side-by-side saved: {}", sbs_path.display());

        // Print histogram of differences
        println!("\nDifference histogram:");
        for (diff_val, &count) in result.diff_histogram.iter().enumerate() {
            if count > 0 {
                println!("  diff={}: {} channels", diff_val, count);
            }
        }

        process::exit(1);
    }

    println!("\nPIXEL-PERFECT MATCH!");
}

#[cfg(test)]
mod tests {
    use super::{
        best_ms, median_ms, parse_cpp_bench_output, params_suffix, pooled_best_median,
        time_render_loop, BENCH_CASES,
    };

    #[test]
    fn pooled_best_is_min_over_all_passes() {
        // Best must be the global minimum across every pass, so a fast sample in a
        // later pass wins even if earlier passes were slow (contaminated).
        let passes = vec![vec![5.0, 4.0], vec![9.0, 3.0], vec![7.0, 6.0]];
        let (best, _median) = pooled_best_median(&passes);
        assert_eq!(best, 3.0);
    }

    #[test]
    fn pooled_median_is_over_concatenated_samples() {
        // Median is taken over all pooled samples, not a median-of-per-pass-medians.
        // Pooled = [1,2,3,4,5,6] (6 samples) => median = (3+4)/2 = 3.5.
        let passes = vec![vec![3.0, 1.0], vec![5.0, 2.0], vec![4.0, 6.0]];
        let (_best, median) = pooled_best_median(&passes);
        assert_eq!(median, 3.5);
    }

    #[test]
    fn pooled_median_odd_total_is_middle_element() {
        // Pooled = [1,2,3,4,5] (5 samples) => median = middle = 3.0.
        let passes = vec![vec![3.0, 1.0, 5.0], vec![2.0, 4.0]];
        let (_best, median) = pooled_best_median(&passes);
        assert_eq!(median, 3.0);
    }

    #[test]
    fn single_pass_pooling_matches_plain_best_median() {
        // With one pass, pooling must be identical to the pre-existing single-pass
        // best/median so the default (--passes 1) is a no-op vs the old behavior.
        let samples = vec![4.0, 1.0, 3.0, 2.0];
        let (best, median) = pooled_best_median(&[samples.clone()]);
        assert_eq!(best, best_ms(&samples));
        assert_eq!(median, median_ms(&samples));
    }

    #[test]
    fn median_odd_count_is_middle_element() {
        assert_eq!(median_ms(&[3.0, 1.0, 2.0]), 2.0);
    }

    #[test]
    fn median_even_count_averages_two_middle() {
        assert_eq!(median_ms(&[4.0, 1.0, 3.0, 2.0]), 2.5);
    }

    #[test]
    fn best_is_the_minimum_sample() {
        assert_eq!(best_ms(&[3.0, 1.0, 2.0]), 1.0);
        assert_eq!(best_ms(&[5.0]), 5.0);
    }

    /// Every benchmarked demo is pinned byte-for-byte against the C++ reference;
    /// a regressed flag would silently reintroduce the "unverified" hedging.
    #[test]
    fn all_bench_cases_are_byte_verified() {
        for c in BENCH_CASES {
            assert!(c.byte_verified, "BENCH_CASES demo '{}' is not byte_verified", c.name);
        }
    }

    #[test]
    fn parse_cpp_bench_output_extracts_iter_samples_only() {
        let stdout = "iter   1: 3.87 ms\niter   2: 3.63 ms\nbest= 3.63 ms  median= 3.75 ms  mean= 3.75 ms\n";
        assert_eq!(parse_cpp_bench_output(stdout), vec![3.87, 3.63]);
    }

    #[test]
    fn parse_cpp_bench_output_ignores_noise_lines() {
        let stdout = "Unknown demo: foo\nsome other text\n";
        assert!(parse_cpp_bench_output(stdout).is_empty());
    }

    #[test]
    fn time_render_loop_returns_one_sample_per_timed_iter() {
        let times = time_render_loop(|| 1u8, 2, 5, false);
        assert_eq!(times.len(), 5);
    }

    #[test]
    fn bench_cases_have_unique_names_and_sane_sizes() {
        let mut seen = std::collections::HashSet::new();
        for c in BENCH_CASES {
            assert!(seen.insert(c.name), "duplicate demo in BENCH_CASES: {}", c.name);
            assert!(c.width > 0 && c.height > 0, "{} has zero size", c.name);
        }
    }

    /// Every benchmarked demo must actually render in-process; a typo in
    /// `BENCH_CASES` would otherwise silently mark the row failed at runtime.
    #[test]
    fn bench_cases_render_in_rust_registry() {
        for c in BENCH_CASES {
            let out = pixel_compare::render::render_demo(c.name, c.width, c.height, c.params);
            assert!(out.is_some(), "BENCH_CASES demo '{}' not in Rust registry", c.name);
        }
    }

    #[test]
    fn empty_params_yield_empty_suffix() {
        assert_eq!(params_suffix(&[]), "");
    }

    #[test]
    fn non_empty_params_have_fixed_length_prefixed_suffix() {
        let suffix = params_suffix(&[0.5]);
        assert!(suffix.starts_with("_p"), "suffix was {suffix}");
        // "_p" + 8 hex chars (lower 32 bits of the FNV-1a hash).
        assert_eq!(suffix.len(), 10, "suffix was {suffix}");
    }

    #[test]
    fn different_params_yield_different_suffixes() {
        assert_ne!(params_suffix(&[0.5]), params_suffix(&[0.25]));
    }

    #[test]
    fn same_params_yield_identical_suffix() {
        assert_eq!(params_suffix(&[1.0, 2.0, 3.0]), params_suffix(&[1.0, 2.0, 3.0]));
    }

    #[test]
    fn known_value_pins_hash_algorithm() {
        // Pins the exact output for [0.5] so an accidental change to the
        // hashing algorithm (which would silently invalidate on-disk
        // filenames) fails this test loudly.
        assert_eq!(params_suffix(&[0.5]), "_p29e886a8");
    }
}

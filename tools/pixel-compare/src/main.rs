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
use std::path::Path;
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

    // 1. Render Rust version
    println!("Rendering Rust '{}'...", demo);
    let rust_buf = pixel_compare::render::render_demo(demo, width, height, &params)
        .unwrap_or_else(|| {
            eprintln!("Unknown demo: '{}'", demo);
            process::exit(1);
        });

    // Save Rust output for inspection
    let rust_path = format!("{}_rust_{}x{}.bmp", demo, width, height);
    save_image(Path::new(&rust_path), &rust_buf).expect("Failed to save Rust output");
    println!("  Saved Rust output: {}", rust_path);

    // 2. Run C++ renderer
    let cpp_raw_path = format!("{}_cpp_{}x{}.raw", demo, width, height);
    println!("Rendering C++ '{}'...", demo);

    let mut cmd_args = vec![
        demo.clone(),
        width.to_string(),
        height.to_string(),
        cpp_raw_path.clone(),
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

    let mut cpp_buf = load_image(Path::new(&cpp_raw_path)).expect("Failed to load C++ output");

    // The C++ renderer outputs with flip_y=true (negative stride), so the
    // buffer rows are in reverse order. Flip to match Rust's top-down layout.
    cpp_buf.flip_vertical();

    // Save C++ output as BMP for inspection
    let cpp_bmp_path = format!("{}_cpp_{}x{}.bmp", demo, width, height);
    save_image(Path::new(&cpp_bmp_path), &cpp_buf).expect("Failed to save C++ BMP");
    println!("  Saved C++ output: {}", cpp_bmp_path);

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
        let sbs_path = format!("{}_sidebyside_{}x{}.bmp", demo, width, height);
        save_image(Path::new(&sbs_path), &sbs).expect("Failed to save side-by-side");
        println!("Side-by-side saved: {}", sbs_path);

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

// Regression test: the native Rust `lion_outline` demo must render byte-for-byte
// identical to the C++ AGG reference at 512x512 with default parameters.
//
// The reference raw (`lion_outline_cpp_512x512.raw`, workspace root) was produced
// by the headless C++ AGG renderer that faithfully mirrors
// `cpp-references/agg-src/examples/lion_outline.cpp` (pixfmt_bgr24, default width
// slider = 1.0, "Use Scanline Rasterizer" unchecked). Any divergence indicates a
// port defect in the demo or the underlying library.

use std::path::PathBuf;

use pixel_compare::{compare_buffers, load_raw, render::render_demo};

fn reference_path() -> PathBuf {
    // Integration tests run with CARGO_MANIFEST_DIR = tools/pixel-compare.
    // The committed reference lives at the workspace root.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("..");
    p.push("lion_outline_cpp_512x512.raw");
    p
}

#[test]
fn lion_outline_matches_cpp_reference_512x512() {
    let reference = load_raw(&reference_path()).expect("failed to load C++ reference raw");
    assert_eq!(reference.width, 512);
    assert_eq!(reference.height, 512);

    let rust = render_demo("lion_outline", 512, 512, &[])
        .expect("lion_outline demo should be registered");

    let result = compare_buffers(&reference, &rust);
    assert!(
        result.identical,
        "lion_outline diverges from C++ reference: {}",
        result
    );
}

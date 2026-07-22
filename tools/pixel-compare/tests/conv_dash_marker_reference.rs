// Regression test: the native Rust `conv_dash_marker` demo must render
// byte-for-byte identical to the committed C++ reference raw.
//
// The reference `conv_dash_marker_cpp_500x330.raw` at the repository root is the
// C++ AGG ground-truth render (500x330, default parameters). This test renders
// the Rust demo through the same pixel-compare path used by the CLI and asserts
// byte identity, matching the project's pixel-perfect porting requirement.

use std::path::PathBuf;

use pixel_compare::render::render_demo;
use pixel_compare::{compare_buffers, load_raw};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/tools/pixel-compare
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn conv_dash_marker_matches_cpp_reference() {
    let reference_path = repo_root().join("conv_dash_marker_cpp_500x330.raw");
    let expected = load_raw(&reference_path)
        .unwrap_or_else(|e| panic!("failed to load reference {reference_path:?}: {e}"));

    let actual = render_demo("conv_dash_marker", 500, 330, &[])
        .expect("conv_dash_marker demo should be registered");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "conv_dash_marker render does not match C++ reference: {result}"
    );
}

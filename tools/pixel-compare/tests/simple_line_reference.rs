// Byte-for-byte regression test for the `simple_line` demo.
//
// The reference buffer in `tests/refs/simple_line_cpp_512x512.raw` was produced
// by the C++ AGG reference renderer (`agg-render simple_line 512 512 <out.raw>`,
// no params). `simple_line` is a synthetic outline-AA scene (closed polygons
// through a rotate-PI + translate transform) and takes no parameters, so both
// the Rust and C++ sides render their fixed canonical state; the empty params
// slice below matches the C++ invocation exactly.
//
// This guards the `renderer_outline_aa` / `rasterizer_outline_aa` path in the
// RGBA32 pixel format against regressions.

use std::path::Path;

use pixel_compare::{compare_buffers, load_raw};

#[test]
fn simple_line_matches_cpp_reference_512x512() {
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("refs")
        .join("simple_line_cpp_512x512.raw");

    let expected = load_raw(&ref_path).expect("failed to load C++ reference raw");
    assert_eq!(expected.width, 512);
    assert_eq!(expected.height, 512);

    // Render the real production demo code path (no params — fixed scene).
    let actual = pixel_compare::render::render_demo("simple_line", 512, 512, &[])
        .expect("simple_line renderer missing");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "simple_line output diverged from the C++ reference: {result}"
    );
}

// Byte-for-byte regression test for the `perspective` demo.
//
// The reference buffer in `tests/refs/perspective_cpp_600x600.raw` was produced
// by the C++ AGG reference renderer (`agg-render perspective 600 600 <out.raw>`,
// no params). With no params the demo renders its canonical default state: the
// lion under a rect->quad Bilinear transform (trans_type = 0, quad = bounding
// rect centered in the window), plus the ellipse overlay, quad tool, and rbox
// control. The Rust `perspective_demo` uses the identical defaults for an empty
// params slice, so both sides render the same scene.

use std::path::Path;

use pixel_compare::{compare_buffers, load_raw};

#[test]
fn perspective_matches_cpp_reference_600x600() {
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("refs")
        .join("perspective_cpp_600x600.raw");

    let expected = load_raw(&ref_path).expect("failed to load C++ reference raw");
    assert_eq!(expected.width, 600);
    assert_eq!(expected.height, 600);

    // Render the real production demo code path (default parameters).
    let actual = pixel_compare::render::render_demo("perspective", 600, 600, &[])
        .expect("perspective renderer missing");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "perspective output diverged from the C++ reference: {result}"
    );
}

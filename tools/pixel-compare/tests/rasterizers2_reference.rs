// Byte-for-byte regression test for the `rasterizers2` demo.
//
// The reference buffer in `tests/refs/rasterizers2_cpp_500x450.raw` was
// produced by the C++ AGG reference renderer (default parameters, 500x450).
// The Rust `rasterizers2` renderer must match it exactly.
//
// This guards against regressions in the premultiplied pixel format
// (`PixfmtRgba32Pre`), the sRGB-decoded + premultiplied image pattern source,
// and the full-precision `ImagePatternSource::pixel_rgba` scaling path — all of
// which are required for the image-pattern spiral to render identically to C++.

use std::path::Path;

use pixel_compare::{compare_buffers, load_raw};

#[test]
fn rasterizers2_matches_cpp_reference_500x450() {
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("refs")
        .join("rasterizers2_cpp_500x450.raw");

    let expected = load_raw(&ref_path).expect("failed to load C++ reference raw");
    assert_eq!(expected.width, 500);
    assert_eq!(expected.height, 450);

    // Render the real production demo code path (default parameters).
    let actual = pixel_compare::render::render_demo("rasterizers2", 500, 450, &[])
        .expect("rasterizers2 renderer missing");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "rasterizers2 output diverged from the C++ reference: {result}"
    );
}

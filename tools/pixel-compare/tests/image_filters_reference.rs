// Byte-for-byte regression test for the `image_filters` demo.
//
// The reference buffer in `tests/refs/image_filters_cpp_430x340.raw` was produced
// by the C++ AGG reference renderer
// (`agg-render image_filters 430 340 <out.raw>`, no params). With no params the
// demo renders its canonical default state. The Rust `image_filters` renderer
// (pixel-compare's native render path) uses the identical defaults for an empty
// params slice.

use std::path::Path;

use pixel_compare::{compare_buffers, load_raw};

#[test]
fn image_filters_matches_cpp_reference_430x340() {
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("refs")
        .join("image_filters_cpp_430x340.raw");

    let expected = load_raw(&ref_path).expect("failed to load C++ reference raw");
    assert_eq!(expected.width, 430);
    assert_eq!(expected.height, 340);

    // Render the real production demo code path (default parameters).
    let actual = pixel_compare::render::render_demo("image_filters", 430, 340, &[])
        .expect("image_filters renderer missing");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "image_filters output diverged from the C++ reference: {result}"
    );
}

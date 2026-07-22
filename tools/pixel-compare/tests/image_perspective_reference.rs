// Byte-for-byte regression test for the `image_perspective` demo.
//
// The reference buffer in `tests/refs/image_perspective_cpp_600x600.raw` was
// produced by the C++ AGG reference renderer
// (`agg-render image_perspective 600 600 <out.raw>`, no params). With no params
// the demo renders its canonical default state: the spheres image warped through
// a Perspective transform (trans_type = 2) into a quad inset 100px from each
// edge, using a bilinear (2x2) image filter, plus the quad tool overlay and rbox
// control. The Rust `image_perspective_demo` uses the identical defaults for an
// empty params slice.
//
// This exercises the RGBA image-span filter pipeline
// (`span_image_filter_rgba_2x2` + `span_interpolator_trans` / `trans_perspective`).

use std::path::Path;

use pixel_compare::{compare_buffers, load_raw};

#[test]
fn image_perspective_matches_cpp_reference_600x600() {
    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("refs")
        .join("image_perspective_cpp_600x600.raw");

    let expected = load_raw(&ref_path).expect("failed to load C++ reference raw");
    assert_eq!(expected.width, 600);
    assert_eq!(expected.height, 600);

    // Render the real production demo code path (default parameters).
    let actual = pixel_compare::render::render_demo("image_perspective", 600, 600, &[])
        .expect("image_perspective renderer missing");

    let result = compare_buffers(&expected, &actual);
    assert!(
        result.identical,
        "image_perspective output diverged from the C++ reference: {result}"
    );
}

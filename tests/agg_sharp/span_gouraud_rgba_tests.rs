//! agg-sharp's `SpanGouraudRgbaTests` (`Tests/Agg.Tests/Agg/SpanGouraudRgbaTests.cs`): a row's
//! colours depend only on that row, not on rows generated before it.

use agg_rust::color::Rgba8;
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::span_gouraud_rgba::SpanGouraudRgba;

#[test]
fn row_below_the_middle_vertex_does_not_depend_on_the_row_before_it() {
    const ROW: i32 = 5;
    const LENGTH: usize = 20;
    let black = Rgba8::new(0, 0, 0, 0);

    let mut direct = vec![black; LENGTH];
    new_triangle().generate(&mut direct, 0, ROW, LENGTH as u32);

    let mut after_others = vec![black; LENGTH];
    let mut swept = new_triangle();
    let mut scratch = vec![black; LENGTH];
    for y in 0..ROW {
        swept.generate(&mut scratch, 0, y, LENGTH as u32);
    }
    swept.generate(&mut after_others, 0, ROW, LENGTH as u32);

    assert_eq!(direct, after_others);

    // The row starts on the red-to-green edge: red fading, green rising, no black from a zeroed edge.
    assert!(direct[1].r as i32 > 100);
}

// Red at the bottom corner, green at the middle vertex (y 10, the split row), blue at the top.
fn new_triangle() -> SpanGouraudRgba {
    let mut triangle = SpanGouraudRgba::new_with_triangle(
        Rgba8::new(255, 0, 0, 255),
        Rgba8::new(0, 255, 0, 255),
        Rgba8::new(0, 0, 255, 255),
        0.0,
        0.0,
        20.0,
        10.0,
        5.0,
        20.0,
        0.0,
    );
    triangle.prepare();
    triangle
}

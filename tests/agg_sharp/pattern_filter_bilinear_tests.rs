//! agg-sharp's `PatternFilterBilinearTests` (`Tests/Agg.Tests/Agg/PatternFilterBilinearTests.cs`):
//! `pattern_filter_bilinear_rgba` rounds its downshift (agg-rust's documented fix in
//! `src/pattern_filters_rgba.rs`; C++ truncates, darkening a pattern by half a level on average).

use agg_rust::color::Rgba8;
use agg_rust::line_aa_basics::LINE_SUBPIXEL_SCALE;
use agg_rust::pattern_filters_rgba::{PatternFilter, PatternFilterBilinearRgba};

/// Halfway between a 0 and a 1 is 0.5, which rounds to 1 (C++ truncates it to 0).
#[test]
fn halfway_sample_rounds_instead_of_truncating() {
    // Each row: a 0 pixel then a 1 pixel, in every channel.
    let row = vec![Rgba8::new(0, 0, 0, 0), Rgba8::new(1, 1, 1, 1)];
    let source = vec![row.clone(), row];

    let mut result = Rgba8::new(0, 0, 0, 0);
    PatternFilterBilinearRgba::pixel_high_res(&source, &mut result, LINE_SUBPIXEL_SCALE / 2, 0);

    assert_eq!(result.r, 1);
    assert_eq!(result.a, 1);
}

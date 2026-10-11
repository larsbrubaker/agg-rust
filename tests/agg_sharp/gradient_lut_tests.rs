//! agg-sharp's `GradientLutTests` (`Tests/Agg.Tests/Agg/GradientLutTests.cs`): a two-stop ramp
//! reaches its end color (agg-rust's documented `seg_len` fix of C++'s `end - start + 1`), and
//! stops are sorted with flat ends.

use agg_rust::color::Rgba8;
use agg_rust::gradient_lut::{ColorFunction, GradientLut};

/// C# `[Arguments(true)]` / `[Arguments(false)]`: both interpolators.
#[test]
fn two_stop_ramp_reaches_its_end_color() {
    for fast_interpolator in [true, false] {
        let mut lut = GradientLut::new(256);
        lut.set_use_fast_interpolator(fast_interpolator);
        lut.add_color(0.0, Rgba8::new(0, 0, 0, 255));
        lut.add_color(1.0, Rgba8::new(255, 255, 255, 255));
        lut.build_lut();

        assert_eq!(lut.size(), 256);
        assert_eq!(lut.get(0), Rgba8::new(0, 0, 0, 255));
        assert_eq!(lut.get(255), Rgba8::new(255, 255, 255, 255));
    }
}

/// The segment is 51 entries so its 200-step blue ramp divides evenly and the DDA lands exactly.
#[test]
fn stops_are_sorted_and_the_ends_are_flat() {
    let mut lut = GradientLut::new(100);
    lut.add_color(0.71, Rgba8::new(0, 0, 200, 255));
    lut.add_color(0.2, Rgba8::new(200, 0, 0, 255));
    lut.build_lut();

    assert_eq!(lut.get(0), Rgba8::new(200, 0, 0, 255));
    assert_eq!(lut.get(20), Rgba8::new(200, 0, 0, 255));
    assert_eq!(lut.get(70), Rgba8::new(0, 0, 200, 255));
    assert_eq!(lut.get(99), Rgba8::new(0, 0, 200, 255));
}

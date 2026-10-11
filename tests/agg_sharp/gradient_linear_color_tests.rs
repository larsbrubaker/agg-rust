//! agg-sharp's `GradientLinearColorTests` (`Tests/Agg.Tests/Agg/GradientLinearColorTests.cs`):
//! `gradient_linear_color<rgba8>` steps as C++ computes them, `c1.gradient(c2, v * (1 / (size - 1)))`.

use agg_rust::color::Rgba8;
use agg_rust::gradient_lut::{ColorFunction, GradientLinearColor};

/// The expected colors are C++'s rgba8 arithmetic done by hand for raster_text's red to (0, 128, 0).
#[test]
fn steps_match_cpp_rgba8_gradient() {
    let colors =
        GradientLinearColor::new(Rgba8::new(255, 0, 0, 255), Rgba8::new(0, 128, 0, 255), 256);

    assert_eq!(colors.get(0), Rgba8::new(255, 0, 0, 255));
    assert_eq!(colors.get(100), Rgba8::new(155, 50, 0, 255));
    assert_eq!(colors.get(173), Rgba8::new(82, 87, 0, 255));
    assert_eq!(colors.get(200), Rgba8::new(55, 100, 0, 255));
    assert_eq!(colors.get(255), Rgba8::new(0, 128, 0, 255));
}

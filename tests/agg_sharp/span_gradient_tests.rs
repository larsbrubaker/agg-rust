//! agg-sharp's `SpanGradientTests` (`Tests/Agg.Tests/Agg/SpanGradientTests.cs`): C++
//! `gradient_radial`/`gradient_circle` are `fast_sqrt(x*x + y*y)` and `gradient_sqrt_xy` is
//! `fast_sqrt(|x|*|y|)`, where `fast_sqrt` takes an unsigned, so a sum of 2^31 or more is still a
//! valid input. The expected values are C++'s table approximation.

use agg_rust::span_gradient::{GradientFunction, GradientRadial, GradientSqrtXY};

/// C# `[Arguments(x, y, radial, sqrtXy)]`. agg-rust has one `GradientRadial` for C++'s
/// `gradient_radial` and `gradient_circle` (the same function).
#[test]
fn gradient_sqrt_functions_match_cpp_fast_sqrt() {
    for (x, y, radial, sqrt_xy) in [
        (50000, 0, 49998, 0),
        (40000, 30000, 49998, 34635),
        (-46341, 1, 46341, 215),
    ] {
        assert_eq!(GradientRadial.calculate(x, y, 0), radial);
        assert_eq!(GradientSqrtXY.calculate(x, y, 0), sqrt_xy);
    }
}

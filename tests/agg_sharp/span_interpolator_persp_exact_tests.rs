//! agg-sharp's `SpanInterpolatorPerspExactTests`
//! (`Tests/Agg.Tests/Agg/SpanInterpolatorPerspExactTests.cs`): C++ `span_interpolator_persp_exact`
//! (pattern_resample.cpp's "Perspective Resample Exact").

use agg_rust::basics::iround;
use agg_rust::span_interpolator_linear::SpanInterpolator;
use agg_rust::span_interpolator_persp::{SpanInterpolatorPerspExact, SpanInterpolatorPerspLerp};
use agg_rust::trans_perspective::TransPerspective;

const QUAD: [f64; 8] = [100.0, 100.0, 500.0, 130.0, 470.0, 520.0, 90.0, 430.0];

/// Every pixel of the span is on the perspective (in 1/256 pixels, give or take the incremental
/// iterator's last bit), not on a line between the span's ends.
#[test]
fn every_pixel_is_on_the_perspective() {
    let mut interpolator =
        SpanInterpolatorPerspExact::new_quad_to_rect(&QUAD, -150.0, -150.0, 150.0, 150.0);
    let mut perspective = TransPerspective::new();
    perspective.quad_to_rect(&QUAD, -150.0, -150.0, 150.0, 150.0);
    assert!(interpolator.is_valid());

    let x = 103.5;
    let y = 250.5;
    interpolator.begin(x, y, 40);
    for i in 0..40 {
        let mut tx = x + i as f64;
        let mut ty = y;
        perspective.transform(&mut tx, &mut ty);
        let (mut cx, mut cy) = (0, 0);
        interpolator.coordinates(&mut cx, &mut cy);
        assert!((cx - iround(tx * 256.0)).abs() <= 1);
        assert!((cy - iround(ty * 256.0)).abs() <= 1);
        interpolator.next();
    }
}

/// Its local scale is the lerp interpolator's: both measure the inverse transform a subpixel away.
#[test]
fn local_scale_matches_the_lerp_interpolator() {
    let mut exact =
        SpanInterpolatorPerspExact::new_quad_to_rect(&QUAD, -150.0, -150.0, 150.0, 150.0);
    let mut lerp = SpanInterpolatorPerspLerp::new_quad_to_rect(&QUAD, -150.0, -150.0, 150.0, 150.0);
    exact.begin(103.5, 250.5, 16);
    lerp.begin(103.5, 250.5, 16);
    for _ in 0..16 {
        let (mut ex, mut ey, mut lx, mut ly) = (0, 0, 0, 0);
        exact.local_scale(&mut ex, &mut ey);
        lerp.local_scale(&mut lx, &mut ly);
        assert_eq!(ex, lx);
        assert_eq!(ey, ly);
        exact.next();
        lerp.next();
    }
}

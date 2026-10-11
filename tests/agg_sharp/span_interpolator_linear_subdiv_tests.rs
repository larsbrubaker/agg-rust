//! agg-sharp's `SpanInterpolatorLinearSubdivTests`
//! (`Tests/Agg.Tests/Agg/SpanInterpolatorLinearSubdivTests.cs`): C++
//! `span_interpolator_linear_subdiv` (pattern_perspective.cpp's interpolator) over a perspective.

use agg_rust::basics::iround;
use agg_rust::dda_line::Dda2LineInterpolator;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinearSubdiv;
use agg_rust::trans_perspective::TransPerspective;

const QUAD: [f64; 8] = [100.0, 100.0, 500.0, 130.0, 470.0, 520.0, 90.0, 430.0];

fn perspective() -> TransPerspective {
    let mut p = TransPerspective::new();
    p.quad_to_rect(&QUAD, -150.0, -150.0, 150.0, 150.0);
    p
}

/// It lands exactly on the transformed point (in 1/256 pixels) at the span's start and at every
/// subdivision boundary (16 pixels by default), however the dda steps between them rounded.
#[test]
fn hits_the_exact_transform_at_every_subdivision_boundary() {
    let perspective = perspective();
    let mut interpolator = SpanInterpolatorLinearSubdiv::new_default(perspective);

    let x = 103.5;
    let y = 250.5;
    let len = 40;
    interpolator.begin(x, y, len);
    for i in 0..len {
        if i % 16 == 0 {
            let mut tx = x + i as f64;
            let mut ty = y;
            perspective.transform(&mut tx, &mut ty);
            let (mut cx, mut cy) = (0, 0);
            interpolator.coordinates(&mut cx, &mut cy);
            assert_eq!(cx, iround(tx * 256.0));
            assert_eq!(cy, iround(ty * 256.0));
        }
        interpolator.next();
    }
}

/// Between boundaries it steps C++'s forward-adjusted dda2 line: here pixel 5 of the first 16.
#[test]
fn steps_a_dda_line_between_boundaries() {
    let perspective = perspective();
    let mut interpolator = SpanInterpolatorLinearSubdiv::new_default(perspective);
    interpolator.begin(103.5, 250.5, 40);

    let (mut x1, mut y1, mut x2, mut y2) = (103.5, 250.5, 103.5 + 16.0, 250.5);
    perspective.transform(&mut x1, &mut y1);
    perspective.transform(&mut x2, &mut y2);
    let mut expected_x =
        Dda2LineInterpolator::new_forward(iround(x1 * 256.0), iround(x2 * 256.0), 16);
    for _ in 0..5 {
        interpolator.next();
        expected_x.inc();
    }

    let (mut cx, mut cy) = (0, 0);
    interpolator.coordinates(&mut cx, &mut cy);
    assert_eq!(cx, expected_x.y());
}

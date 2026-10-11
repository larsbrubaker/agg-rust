//! agg-sharp's `TransWarpMagnifierTests` (`Tests/Agg.Tests/Agg/TransWarpMagnifierTests.cs`):
//! `trans_warp_magnifier` against C++ AGG, set up as lion_lens.cpp's default lens (center
//! 200,150, magnification 3, radius 70 / 3), compared exactly; plus the zero-radius lens, where
//! C++ returns NaN at the centre and agg-rust leaves every point alone (documented divergence in
//! `src/trans_warp_magnifier.rs`).

use agg_rust::span_interpolator_linear::Transformer;
use agg_rust::trans_warp_magnifier::TransWarpMagnifier;

// Inside the lens, on its rim's far side, far outside, the center itself, and just outside the rim.
// The traced values are C#'s 17-digit round-trip text, kept verbatim.
#[allow(clippy::excessive_precision)]
const TRACED: [(f64, f64, f64, f64, f64, f64); 5] = [
    (210.0, 155.0, 230.0, 165.0, 210.0, 155.0),
    (
        250.0,
        150.0,
        296.66666666666663,
        150.0,
        249.99999999999994,
        150.0,
    ),
    (
        100.0,
        40.0,
        68.608602946838744,
        5.4694632415226181,
        99.999999999999972,
        39.999999999999972,
    ),
    (200.0, 150.0, 200.0, 150.0, 200.0, 150.0),
    (
        263.5,
        171.25,
        307.75443338171846,
        186.05955447813412,
        263.5,
        171.25,
    ),
];

#[test]
fn transform_and_inverse_match_cpp_agg() {
    let mut lens = TransWarpMagnifier::new();
    lens.magnification(3.0);
    lens.set_radius(70.0 / 3.0);
    lens.center(200.0, 150.0);

    for &(px, py, tx, ty, ix, iy) in &TRACED {
        let (mut x, mut y) = (px, py);
        lens.transform(&mut x, &mut y);
        assert_eq!(x, tx);
        assert_eq!(y, ty);

        lens.inverse_transform(&mut x, &mut y);
        assert_eq!(x, ix);
        assert_eq!(y, iy);
    }
}

#[test]
fn zero_radius_leaves_every_point_alone_including_the_center() {
    let mut lens = TransWarpMagnifier::new();
    lens.magnification(3.0);
    lens.set_radius(0.0);
    lens.center(200.1, 150.3);

    for &(px, py) in &[(200.1, 150.3), (210.7, 155.2), (-3.3, 999.9)] {
        let (mut x, mut y) = (px, py);
        lens.transform(&mut x, &mut y);
        assert_eq!(x, px);
        assert_eq!(y, py);

        let (mut x, mut y) = (px, py);
        lens.inverse_transform(&mut x, &mut y);
        assert_eq!(x, px);
        assert_eq!(y, py);
    }
}

#[test]
fn defaults_leave_the_plane_alone() {
    let lens = TransWarpMagnifier::new();
    let (mut x, mut y) = (12.5, -3.0);
    lens.transform(&mut x, &mut y);

    assert_eq!(x, 12.5);
    assert_eq!(y, -3.0);
}

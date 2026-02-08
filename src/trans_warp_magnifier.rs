//! Warp magnifier transformation.
//!
//! Port of `agg_trans_warp_magnifier.h` + `agg_trans_warp_magnifier.cpp`.
//! Creates a magnified circular zone (magnifying glass effect).

use crate::span_interpolator_linear::Transformer;

/// Warp magnifier transformation — magnifies a circular zone.
///
/// Inside the radius, coordinates are scaled by the magnification factor.
/// Outside, coordinates transition smoothly to unmagnified space.
pub struct TransWarpMagnifier {
    pub xc: f64,
    pub yc: f64,
    pub magn: f64,
    pub radius: f64,
}

impl TransWarpMagnifier {
    pub fn new() -> Self {
        Self {
            xc: 0.0,
            yc: 0.0,
            magn: 1.0,
            radius: 1.0,
        }
    }

    pub fn center(&mut self, x: f64, y: f64) {
        self.xc = x;
        self.yc = y;
    }

    pub fn magnification(&mut self, m: f64) {
        self.magn = m;
    }

    pub fn set_radius(&mut self, r: f64) {
        self.radius = r;
    }

    pub fn inverse_transform(&self, x: &mut f64, y: &mut f64) {
        let dx = *x - self.xc;
        let dy = *y - self.yc;
        let r = (dx * dx + dy * dy).sqrt();

        if r < self.radius * self.magn {
            *x = self.xc + dx / self.magn;
            *y = self.yc + dy / self.magn;
        } else {
            let rnew = r - self.radius * (self.magn - 1.0);
            *x = self.xc + rnew * dx / r;
            *y = self.yc + rnew * dy / r;
        }
    }
}

impl Transformer for TransWarpMagnifier {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        let dx = *x - self.xc;
        let dy = *y - self.yc;
        let r = (dx * dx + dy * dy).sqrt();
        if r < self.radius {
            *x = self.xc + dx * self.magn;
            *y = self.yc + dy * self.magn;
            return;
        }
        let m = (r + self.radius * (self.magn - 1.0)) / r;
        *x = self.xc + dx * m;
        *y = self.yc + dy * m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_magnification() {
        let t = TransWarpMagnifier::new();
        let (mut x, mut y) = (5.0, 5.0);
        t.transform(&mut x, &mut y);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_center_magnification() {
        let mut t = TransWarpMagnifier::new();
        t.center(100.0, 100.0);
        t.magnification(2.0);
        t.set_radius(50.0);

        // Point at center stays at center
        let (mut x, mut y) = (100.0, 100.0);
        t.transform(&mut x, &mut y);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 100.0).abs() < 1e-10);

        // Point inside radius is magnified
        let (mut x, mut y) = (120.0, 100.0);
        t.transform(&mut x, &mut y);
        assert!((x - 140.0).abs() < 1e-10); // 100 + 20*2 = 140
        assert!((y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_inverse_roundtrip() {
        let mut t = TransWarpMagnifier::new();
        t.center(50.0, 50.0);
        t.magnification(3.0);
        t.set_radius(30.0);

        for &(ox, oy) in &[(50.0, 50.0), (60.0, 50.0), (90.0, 90.0), (20.0, 30.0)] {
            let (mut x, mut y) = (ox, oy);
            t.transform(&mut x, &mut y);
            t.inverse_transform(&mut x, &mut y);
            assert!((x - ox).abs() < 1e-8, "x: {x} != {ox}");
            assert!((y - oy).abs() < 1e-8, "y: {y} != {oy}");
        }
    }
}

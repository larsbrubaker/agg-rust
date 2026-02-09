//! Polar coordinate transformation.
//!
//! Port of the `trans_polar` class from `examples/trans_polar.cpp`.
//! Converts rectangular coordinates to polar (spiral) coordinates.

use crate::span_interpolator_linear::Transformer;

/// Polar coordinate transformation.
///
/// Maps rectangular coordinates to polar space, optionally with spiral effect.
#[derive(Clone, Copy)]
pub struct TransPolar {
    pub base_angle: f64,
    pub base_scale: f64,
    pub base_x: f64,
    pub base_y: f64,
    pub translation_x: f64,
    pub translation_y: f64,
    pub spiral: f64,
}

impl TransPolar {
    pub fn new() -> Self {
        Self {
            base_angle: 1.0,
            base_scale: 1.0,
            base_x: 0.0,
            base_y: 0.0,
            translation_x: 0.0,
            translation_y: 0.0,
            spiral: 0.0,
        }
    }
}

impl Transformer for TransPolar {
    fn transform(&self, x: &mut f64, y: &mut f64) {
        let x1 = (*x + self.base_x) * self.base_angle;
        let y1 = (*y + self.base_y) * self.base_scale + (*x * self.spiral);
        *x = x1.cos() * y1 + self.translation_x;
        *y = x1.sin() * y1 + self.translation_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_transform() {
        let t = TransPolar::new();
        let (mut x, mut y) = (0.0, 0.0);
        t.transform(&mut x, &mut y);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_nonzero_transform() {
        let mut t = TransPolar::new();
        t.base_angle = std::f64::consts::PI / 180.0;
        t.base_scale = 1.0;
        t.translation_x = 200.0;
        t.translation_y = 200.0;

        let (mut x, mut y) = (90.0, 100.0);
        t.transform(&mut x, &mut y);
        // At 90 degrees: cos(pi/2) ≈ 0, sin(pi/2) ≈ 1
        // x = cos(90 * pi/180) * 100 + 200 ≈ 200
        // y = sin(90 * pi/180) * 100 + 200 ≈ 300
        assert!((x - 200.0).abs() < 1.0);
        assert!((y - 300.0).abs() < 1.0);
    }
}

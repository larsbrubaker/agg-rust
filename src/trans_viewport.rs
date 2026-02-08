//! Viewport transformation.
//!
//! Port of `agg_trans_viewport.h` — simple orthogonal transformation from
//! world coordinates to device (screen) coordinates with aspect ratio control.

use crate::trans_affine::TransAffine;

// ============================================================================
// AspectRatio
// ============================================================================

/// Aspect ratio handling mode for viewport transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatio {
    /// Stretch to fill — no aspect ratio preservation.
    Stretch,
    /// Meet — fit entirely within device viewport (letterbox/pillarbox).
    Meet,
    /// Slice — fill device viewport entirely (may crop).
    Slice,
}

// ============================================================================
// TransViewport
// ============================================================================

/// Viewport transformer.
///
/// Maps world coordinates to device coordinates with optional aspect ratio
/// preservation. Can produce a `TransAffine` matrix for use with other
/// components.
///
/// Port of C++ `trans_viewport`.
pub struct TransViewport {
    world_x1: f64,
    world_y1: f64,
    world_x2: f64,
    world_y2: f64,
    device_x1: f64,
    device_y1: f64,
    device_x2: f64,
    device_y2: f64,
    aspect: AspectRatio,
    is_valid: bool,
    align_x: f64,
    align_y: f64,
    // Computed values
    wx1: f64,
    wy1: f64,
    wx2: f64,
    wy2: f64,
    dx1: f64,
    dy1: f64,
    kx: f64,
    ky: f64,
}

impl Default for TransViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl TransViewport {
    pub fn new() -> Self {
        Self {
            world_x1: 0.0,
            world_y1: 0.0,
            world_x2: 1.0,
            world_y2: 1.0,
            device_x1: 0.0,
            device_y1: 0.0,
            device_x2: 1.0,
            device_y2: 1.0,
            aspect: AspectRatio::Stretch,
            is_valid: true,
            align_x: 0.5,
            align_y: 0.5,
            wx1: 0.0,
            wy1: 0.0,
            wx2: 1.0,
            wy2: 1.0,
            dx1: 0.0,
            dy1: 0.0,
            kx: 1.0,
            ky: 1.0,
        }
    }

    /// Set aspect ratio preservation mode and alignment.
    pub fn preserve_aspect_ratio(&mut self, align_x: f64, align_y: f64, aspect: AspectRatio) {
        self.align_x = align_x;
        self.align_y = align_y;
        self.aspect = aspect;
        self.update();
    }

    /// Set the device (screen) viewport rectangle.
    pub fn set_device_viewport(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.device_x1 = x1;
        self.device_y1 = y1;
        self.device_x2 = x2;
        self.device_y2 = y2;
        self.update();
    }

    /// Set the world (logical) viewport rectangle.
    pub fn set_world_viewport(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.world_x1 = x1;
        self.world_y1 = y1;
        self.world_x2 = x2;
        self.world_y2 = y2;
        self.update();
    }

    /// Get the device viewport rectangle.
    pub fn device_viewport(&self) -> (f64, f64, f64, f64) {
        (
            self.device_x1,
            self.device_y1,
            self.device_x2,
            self.device_y2,
        )
    }

    /// Get the world viewport rectangle.
    pub fn world_viewport(&self) -> (f64, f64, f64, f64) {
        (self.world_x1, self.world_y1, self.world_x2, self.world_y2)
    }

    /// Get the actual (computed) world viewport after aspect ratio adjustment.
    pub fn world_viewport_actual(&self) -> (f64, f64, f64, f64) {
        (self.wx1, self.wy1, self.wx2, self.wy2)
    }

    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    pub fn align_x(&self) -> f64 {
        self.align_x
    }

    pub fn align_y(&self) -> f64 {
        self.align_y
    }

    pub fn aspect_ratio(&self) -> AspectRatio {
        self.aspect
    }

    /// Transform world coordinates to device coordinates.
    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        *x = (*x - self.wx1) * self.kx + self.dx1;
        *y = (*y - self.wy1) * self.ky + self.dy1;
    }

    /// Transform only the scale component (no translation).
    pub fn transform_scale_only(&self, x: &mut f64, y: &mut f64) {
        *x *= self.kx;
        *y *= self.ky;
    }

    /// Transform device coordinates back to world coordinates.
    pub fn inverse_transform(&self, x: &mut f64, y: &mut f64) {
        *x = (*x - self.dx1) / self.kx + self.wx1;
        *y = (*y - self.dy1) / self.ky + self.wy1;
    }

    /// Inverse transform only the scale component.
    pub fn inverse_transform_scale_only(&self, x: &mut f64, y: &mut f64) {
        *x /= self.kx;
        *y /= self.ky;
    }

    pub fn device_dx(&self) -> f64 {
        self.dx1 - self.wx1 * self.kx
    }

    pub fn device_dy(&self) -> f64 {
        self.dy1 - self.wy1 * self.ky
    }

    pub fn scale_x(&self) -> f64 {
        self.kx
    }

    pub fn scale_y(&self) -> f64 {
        self.ky
    }

    pub fn scale(&self) -> f64 {
        (self.kx + self.ky) * 0.5
    }

    /// Convert to an equivalent `TransAffine` matrix.
    pub fn to_affine(&self) -> TransAffine {
        let mut mtx = TransAffine::new_translation(-self.wx1, -self.wy1);
        mtx.multiply(&TransAffine::new_scaling(self.kx, self.ky));
        mtx.multiply(&TransAffine::new_translation(self.dx1, self.dy1));
        mtx
    }

    /// Convert to an affine matrix with only the scale component.
    pub fn to_affine_scale_only(&self) -> TransAffine {
        TransAffine::new_scaling(self.kx, self.ky)
    }

    fn update(&mut self) {
        const EPSILON: f64 = 1e-30;
        if (self.world_x1 - self.world_x2).abs() < EPSILON
            || (self.world_y1 - self.world_y2).abs() < EPSILON
            || (self.device_x1 - self.device_x2).abs() < EPSILON
            || (self.device_y1 - self.device_y2).abs() < EPSILON
        {
            self.wx1 = self.world_x1;
            self.wy1 = self.world_y1;
            self.wx2 = self.world_x1 + 1.0;
            self.wy2 = self.world_y2 + 1.0;
            self.dx1 = self.device_x1;
            self.dy1 = self.device_y1;
            self.kx = 1.0;
            self.ky = 1.0;
            self.is_valid = false;
            return;
        }

        let mut world_x1 = self.world_x1;
        let mut world_y1 = self.world_y1;
        let mut world_x2 = self.world_x2;
        let mut world_y2 = self.world_y2;
        let device_x1 = self.device_x1;
        let device_y1 = self.device_y1;
        let device_x2 = self.device_x2;
        let device_y2 = self.device_y2;

        if self.aspect != AspectRatio::Stretch {
            self.kx = (device_x2 - device_x1) / (world_x2 - world_x1);
            self.ky = (device_y2 - device_y1) / (world_y2 - world_y1);

            if (self.aspect == AspectRatio::Meet) == (self.kx < self.ky) {
                let d = (world_y2 - world_y1) * self.ky / self.kx;
                world_y1 += (world_y2 - world_y1 - d) * self.align_y;
                world_y2 = world_y1 + d;
            } else {
                let d = (world_x2 - world_x1) * self.kx / self.ky;
                world_x1 += (world_x2 - world_x1 - d) * self.align_x;
                world_x2 = world_x1 + d;
            }
        }

        self.wx1 = world_x1;
        self.wy1 = world_y1;
        self.wx2 = world_x2;
        self.wy2 = world_y2;
        self.dx1 = device_x1;
        self.dy1 = device_y1;
        self.kx = (device_x2 - device_x1) / (world_x2 - world_x1);
        self.ky = (device_y2 - device_y1) / (world_y2 - world_y1);
        self.is_valid = true;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_identity() {
        let vp = TransViewport::new();
        assert!(vp.is_valid());
        assert_eq!(vp.scale_x(), 1.0);
        assert_eq!(vp.scale_y(), 1.0);
    }

    #[test]
    fn test_stretch_scaling() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 400.0);
        assert_eq!(vp.scale_x(), 2.0);
        assert_eq!(vp.scale_y(), 4.0);
    }

    #[test]
    fn test_transform() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 200.0);

        let mut x = 50.0;
        let mut y = 50.0;
        vp.transform(&mut x, &mut y);
        assert_eq!(x, 100.0);
        assert_eq!(y, 100.0);
    }

    #[test]
    fn test_inverse_transform() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 200.0);

        let mut x = 100.0;
        let mut y = 100.0;
        vp.inverse_transform(&mut x, &mut y);
        assert_eq!(x, 50.0);
        assert_eq!(y, 50.0);
    }

    #[test]
    fn test_meet_aspect_ratio() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 400.0);
        vp.preserve_aspect_ratio(0.5, 0.5, AspectRatio::Meet);
        // Meet: use smaller scale factor (kx=2.0 < ky=4.0), so kx wins
        assert_eq!(vp.scale_x(), 2.0);
        assert_eq!(vp.scale_y(), 2.0);
    }

    #[test]
    fn test_slice_aspect_ratio() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 400.0);
        vp.preserve_aspect_ratio(0.5, 0.5, AspectRatio::Slice);
        // Slice: use larger scale factor (ky=4.0 > kx=2.0), so ky wins
        assert_eq!(vp.scale_x(), 4.0);
        assert_eq!(vp.scale_y(), 4.0);
    }

    #[test]
    fn test_to_affine() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 200.0);

        let mtx = vp.to_affine();
        let mut x = 50.0;
        let mut y = 50.0;
        mtx.transform(&mut x, &mut y);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_zero_size() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(50.0, 50.0, 50.0, 50.0); // zero-size world
        assert!(!vp.is_valid());
    }

    #[test]
    fn test_device_dx_dy() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(10.0, 20.0, 110.0, 120.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 200.0);
        // kx = 200/100 = 2, dx1 = 0, wx1 = 10
        // device_dx = dx1 - wx1 * kx = 0 - 10 * 2 = -20
        assert_eq!(vp.device_dx(), -20.0);
    }

    #[test]
    fn test_scale() {
        let mut vp = TransViewport::new();
        vp.set_world_viewport(0.0, 0.0, 100.0, 100.0);
        vp.set_device_viewport(0.0, 0.0, 200.0, 400.0);
        assert_eq!(vp.scale(), 3.0); // (2 + 4) / 2
    }
}

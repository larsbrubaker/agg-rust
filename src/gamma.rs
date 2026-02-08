//! Gamma correction functions and lookup tables.
//!
//! Port of `agg_gamma_functions.h` and `agg_gamma_lut.h` — various gamma
//! correction strategies used throughout AGG for anti-aliasing quality.

use crate::basics::uround;

// ============================================================================
// Gamma function trait
// ============================================================================

/// Trait for gamma correction functions.
/// Port of C++ gamma function objects (operator() overload).
pub trait GammaFunction {
    fn call(&self, x: f64) -> f64;
}

// ============================================================================
// Gamma none (identity)
// ============================================================================

/// No gamma correction — returns input unchanged.
/// Port of C++ `gamma_none`.
#[derive(Debug, Clone, Copy, Default)]
pub struct GammaNone;

impl GammaFunction for GammaNone {
    #[inline]
    fn call(&self, x: f64) -> f64 {
        x
    }
}

// ============================================================================
// Gamma power
// ============================================================================

/// Power-law gamma correction: `x^gamma`.
/// Port of C++ `gamma_power`.
#[derive(Debug, Clone, Copy)]
pub struct GammaPower {
    gamma: f64,
}

impl GammaPower {
    pub fn new(gamma: f64) -> Self {
        Self { gamma }
    }

    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    pub fn set_gamma(&mut self, g: f64) {
        self.gamma = g;
    }
}

impl Default for GammaPower {
    fn default() -> Self {
        Self { gamma: 1.0 }
    }
}

impl GammaFunction for GammaPower {
    #[inline]
    fn call(&self, x: f64) -> f64 {
        x.powf(self.gamma)
    }
}

// ============================================================================
// Gamma threshold
// ============================================================================

/// Threshold gamma: returns 0 if x < threshold, 1 otherwise.
/// Port of C++ `gamma_threshold`.
#[derive(Debug, Clone, Copy)]
pub struct GammaThreshold {
    threshold: f64,
}

impl GammaThreshold {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn set_threshold(&mut self, t: f64) {
        self.threshold = t;
    }
}

impl Default for GammaThreshold {
    fn default() -> Self {
        Self { threshold: 0.5 }
    }
}

impl GammaFunction for GammaThreshold {
    #[inline]
    fn call(&self, x: f64) -> f64 {
        if x < self.threshold {
            0.0
        } else {
            1.0
        }
    }
}

// ============================================================================
// Gamma linear
// ============================================================================

/// Linear ramp gamma: 0 below `start`, 1 above `end`, linear between.
/// Port of C++ `gamma_linear`.
#[derive(Debug, Clone, Copy)]
pub struct GammaLinear {
    start: f64,
    end: f64,
}

impl GammaLinear {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> f64 {
        self.start
    }

    pub fn end(&self) -> f64 {
        self.end
    }

    pub fn set_start(&mut self, s: f64) {
        self.start = s;
    }

    pub fn set_end(&mut self, e: f64) {
        self.end = e;
    }

    pub fn set(&mut self, s: f64, e: f64) {
        self.start = s;
        self.end = e;
    }
}

impl Default for GammaLinear {
    fn default() -> Self {
        Self {
            start: 0.0,
            end: 1.0,
        }
    }
}

impl GammaFunction for GammaLinear {
    #[inline]
    fn call(&self, x: f64) -> f64 {
        if x < self.start {
            0.0
        } else if x > self.end {
            1.0
        } else {
            (x - self.start) / (self.end - self.start)
        }
    }
}

// ============================================================================
// Gamma multiply
// ============================================================================

/// Multiplicative gamma: `min(x * multiplier, 1.0)`.
/// Port of C++ `gamma_multiply`.
#[derive(Debug, Clone, Copy)]
pub struct GammaMultiply {
    mul: f64,
}

impl GammaMultiply {
    pub fn new(mul: f64) -> Self {
        Self { mul }
    }

    pub fn value(&self) -> f64 {
        self.mul
    }

    pub fn set_value(&mut self, v: f64) {
        self.mul = v;
    }
}

impl Default for GammaMultiply {
    fn default() -> Self {
        Self { mul: 1.0 }
    }
}

impl GammaFunction for GammaMultiply {
    #[inline]
    fn call(&self, x: f64) -> f64 {
        let y = x * self.mul;
        if y > 1.0 {
            1.0
        } else {
            y
        }
    }
}

// ============================================================================
// sRGB conversion functions
// ============================================================================

/// Convert sRGB value (0..1) to linear.
#[inline]
pub fn srgb_to_linear(x: f64) -> f64 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear value (0..1) to sRGB.
#[inline]
pub fn linear_to_srgb(x: f64) -> f64 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

// ============================================================================
// Gamma LUT (Lookup Table)
// ============================================================================

/// Gamma correction using pre-computed lookup tables.
/// Port of C++ `gamma_lut<LoResT, HiResT, GammaShift, HiResShift>`.
///
/// Default parameters match C++ defaults: u8 for both low/high resolution,
/// shift=8 for both, giving 256-entry tables.
pub struct GammaLut {
    gamma: f64,
    #[allow(dead_code)]
    gamma_shift: u32,
    gamma_size: usize,
    gamma_mask: f64,
    #[allow(dead_code)]
    hi_res_shift: u32,
    hi_res_size: usize,
    hi_res_mask: f64,
    dir_gamma: Vec<u8>,
    inv_gamma: Vec<u8>,
}

impl GammaLut {
    /// Create a gamma LUT with identity gamma (1.0).
    /// Uses the default 8-bit/8-bit configuration.
    pub fn new() -> Self {
        Self::with_shifts(8, 8)
    }

    /// Create a gamma LUT with the specified gamma value.
    pub fn new_with_gamma(g: f64) -> Self {
        let mut lut = Self::new();
        lut.set_gamma(g);
        lut
    }

    /// Create a gamma LUT with custom shift parameters.
    pub fn with_shifts(gamma_shift: u32, hi_res_shift: u32) -> Self {
        let gamma_size = 1usize << gamma_shift;
        let hi_res_size = 1usize << hi_res_shift;

        let mut dir_gamma = vec![0u8; gamma_size];
        let mut inv_gamma = vec![0u8; hi_res_size];

        // Identity gamma: direct mapping
        for (i, entry) in dir_gamma.iter_mut().enumerate() {
            *entry = (i << (hi_res_shift - gamma_shift)) as u8;
        }
        for (i, entry) in inv_gamma.iter_mut().enumerate() {
            *entry = (i >> (hi_res_shift - gamma_shift)) as u8;
        }

        Self {
            gamma: 1.0,
            gamma_shift,
            gamma_size,
            gamma_mask: (gamma_size - 1) as f64,
            hi_res_shift,
            hi_res_size,
            hi_res_mask: (hi_res_size - 1) as f64,
            dir_gamma,
            inv_gamma,
        }
    }

    /// Set the gamma value and rebuild lookup tables.
    pub fn set_gamma(&mut self, g: f64) {
        self.gamma = g;

        for i in 0..self.gamma_size {
            self.dir_gamma[i] =
                uround((i as f64 / self.gamma_mask).powf(self.gamma) * self.hi_res_mask) as u8;
        }

        let inv_g = 1.0 / g;
        for i in 0..self.hi_res_size {
            self.inv_gamma[i] =
                uround((i as f64 / self.hi_res_mask).powf(inv_g) * self.gamma_mask) as u8;
        }
    }

    /// Get the current gamma value.
    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    /// Forward (direct) gamma correction: low-res → high-res.
    #[inline]
    pub fn dir(&self, v: u8) -> u8 {
        self.dir_gamma[v as usize]
    }

    /// Inverse gamma correction: high-res → low-res.
    #[inline]
    pub fn inv(&self, v: u8) -> u8 {
        self.inv_gamma[v as usize]
    }
}

impl Default for GammaLut {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Gamma Spline — interactive gamma curve via bicubic spline
// ============================================================================

/// Spline-based gamma correction curve.
///
/// Port of C++ `gamma_spline` from `ctrl/agg_gamma_spline.h`.
/// Takes 4 control parameters `(kx1, ky1, kx2, ky2)` each in `[0.001..1.999]`
/// and generates a smooth gamma curve through 4 interpolation points:
///   - `(0, 0)`
///   - `(kx1 * 0.25, ky1 * 0.25)`
///   - `(1 - kx2 * 0.25, 1 - ky2 * 0.25)`
///   - `(1, 1)`
///
/// Produces a 256-entry lookup table (`gamma()`) and supports evaluation (`y()`).
/// Also implements a vertex source interface for rendering the curve.
pub struct GammaSpline {
    gamma: [u8; 256],
    x: [f64; 4],
    y_pts: [f64; 4],
    spline: crate::bspline::Bspline,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    cur_x: f64,
}

impl GammaSpline {
    pub fn new() -> Self {
        let mut gs = Self {
            gamma: [0; 256],
            x: [0.0; 4],
            y_pts: [0.0; 4],
            spline: crate::bspline::Bspline::new(),
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            cur_x: 0.0,
        };
        gs.set_values(1.0, 1.0, 1.0, 1.0);
        gs
    }

    /// Set the 4 control parameters and rebuild the spline and gamma table.
    ///
    /// Each parameter should be in `[0.001..1.999]`; values are clamped.
    pub fn set_values(&mut self, kx1: f64, ky1: f64, kx2: f64, ky2: f64) {
        let kx1 = kx1.clamp(0.001, 1.999);
        let ky1 = ky1.clamp(0.001, 1.999);
        let kx2 = kx2.clamp(0.001, 1.999);
        let ky2 = ky2.clamp(0.001, 1.999);

        self.x[0] = 0.0;
        self.y_pts[0] = 0.0;
        self.x[1] = kx1 * 0.25;
        self.y_pts[1] = ky1 * 0.25;
        self.x[2] = 1.0 - kx2 * 0.25;
        self.y_pts[2] = 1.0 - ky2 * 0.25;
        self.x[3] = 1.0;
        self.y_pts[3] = 1.0;

        self.spline.init(&self.x, &self.y_pts);

        for i in 0..256 {
            self.gamma[i] = (self.y(i as f64 / 255.0) * 255.0) as u8;
        }
    }

    /// Get the 4 control parameters back from the stored spline points.
    pub fn get_values(&self) -> (f64, f64, f64, f64) {
        (
            self.x[1] * 4.0,
            self.y_pts[1] * 4.0,
            (1.0 - self.x[2]) * 4.0,
            (1.0 - self.y_pts[2]) * 4.0,
        )
    }

    /// Get the 256-entry gamma lookup table.
    pub fn gamma(&self) -> &[u8; 256] {
        &self.gamma
    }

    /// Evaluate the spline curve at `x` (0..1) → result clamped to `[0..1]`.
    pub fn y(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let val = self.spline.get(x);
        val.clamp(0.0, 1.0)
    }

    /// Set the bounding box for vertex source rendering.
    pub fn set_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;
    }

    /// Rewind vertex source iteration.
    pub fn rewind(&mut self, _idx: u32) {
        self.cur_x = 0.0;
    }

    /// Get the next vertex of the gamma curve path.
    pub fn vertex(&mut self, vx: &mut f64, vy: &mut f64) -> u32 {
        use crate::basics::{PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};

        if self.cur_x == 0.0 {
            *vx = self.x1;
            *vy = self.y1;
            self.cur_x += 1.0 / (self.x2 - self.x1);
            return PATH_CMD_MOVE_TO;
        }

        if self.cur_x > 1.0 {
            return PATH_CMD_STOP;
        }

        *vx = self.x1 + self.cur_x * (self.x2 - self.x1);
        *vy = self.y1 + self.y(self.cur_x) * (self.y2 - self.y1);

        self.cur_x += 1.0 / (self.x2 - self.x1);
        PATH_CMD_LINE_TO
    }
}

impl Default for GammaSpline {
    fn default() -> Self {
        Self::new()
    }
}

impl GammaFunction for GammaSpline {
    fn call(&self, x: f64) -> f64 {
        self.y(x)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_gamma_none() {
        let g = GammaNone;
        assert!((g.call(0.0) - 0.0).abs() < EPSILON);
        assert!((g.call(0.5) - 0.5).abs() < EPSILON);
        assert!((g.call(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_gamma_power_identity() {
        let g = GammaPower::new(1.0);
        assert!((g.call(0.5) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_gamma_power_square() {
        let g = GammaPower::new(2.0);
        assert!((g.call(0.5) - 0.25).abs() < EPSILON);
        assert!((g.call(0.0) - 0.0).abs() < EPSILON);
        assert!((g.call(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_gamma_threshold() {
        let g = GammaThreshold::new(0.5);
        assert_eq!(g.call(0.3), 0.0);
        assert_eq!(g.call(0.5), 1.0);
        assert_eq!(g.call(0.7), 1.0);
    }

    #[test]
    fn test_gamma_linear() {
        let g = GammaLinear::new(0.2, 0.8);
        assert_eq!(g.call(0.1), 0.0);
        assert_eq!(g.call(0.9), 1.0);
        assert!((g.call(0.5) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_gamma_multiply() {
        let g = GammaMultiply::new(2.0);
        assert!((g.call(0.3) - 0.6).abs() < EPSILON);
        assert_eq!(g.call(0.7), 1.0); // Clamped
        assert_eq!(g.call(1.0), 1.0);
    }

    #[test]
    fn test_srgb_roundtrip() {
        for i in 0..=10 {
            let x = i as f64 / 10.0;
            let linear = srgb_to_linear(x);
            let back = linear_to_srgb(linear);
            assert!(
                (x - back).abs() < 1e-6,
                "sRGB roundtrip failed for x={}: got {}",
                x,
                back
            );
        }
    }

    #[test]
    fn test_srgb_endpoints() {
        assert!((srgb_to_linear(0.0)).abs() < EPSILON);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < EPSILON);
        assert!((linear_to_srgb(0.0)).abs() < EPSILON);
        assert!((linear_to_srgb(1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_gamma_lut_identity() {
        let lut = GammaLut::new();
        assert_eq!(lut.gamma(), 1.0);
        // Identity: dir and inv should be identity mappings
        assert_eq!(lut.dir(0), 0);
        assert_eq!(lut.dir(128), 128);
        assert_eq!(lut.dir(255), 255);
        assert_eq!(lut.inv(0), 0);
        assert_eq!(lut.inv(128), 128);
        assert_eq!(lut.inv(255), 255);
    }

    #[test]
    fn test_gamma_lut_roundtrip() {
        let lut = GammaLut::new_with_gamma(2.2);
        // Forward then inverse should be approximately identity
        for v in [0u8, 64, 128, 192, 255] {
            let forward = lut.dir(v);
            let back = lut.inv(forward);
            assert!(
                (v as i32 - back as i32).unsigned_abs() <= 1,
                "Roundtrip failed for v={}: dir={}, inv={}",
                v,
                forward,
                back
            );
        }
    }

    #[test]
    fn test_gamma_lut_gamma_2() {
        let lut = GammaLut::new_with_gamma(2.0);
        // At gamma=2.0, dir(128) should be pow(128/255, 2.0) * 255 ≈ 64
        let d = lut.dir(128);
        assert!(
            (d as i32 - 64).unsigned_abs() <= 1,
            "dir(128) at gamma 2.0 should be ~64, got {}",
            d
        );
    }

    // ====================================================================
    // GammaSpline tests
    // ====================================================================

    #[test]
    fn test_gamma_spline_default() {
        let gs = GammaSpline::new();
        // Default values(1,1,1,1): straight line
        let (kx1, ky1, kx2, ky2) = gs.get_values();
        assert!((kx1 - 1.0).abs() < 0.01);
        assert!((ky1 - 1.0).abs() < 0.01);
        assert!((kx2 - 1.0).abs() < 0.01);
        assert!((ky2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_gamma_spline_identity_curve() {
        let gs = GammaSpline::new();
        // With default values, the curve should approximate identity
        assert!((gs.y(0.0)).abs() < 0.01);
        assert!((gs.y(1.0) - 1.0).abs() < 0.01);
        assert!((gs.y(0.5) - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_gamma_spline_gamma_table() {
        let gs = GammaSpline::new();
        let gamma = gs.gamma();
        // First and last entries
        assert_eq!(gamma[0], 0);
        assert_eq!(gamma[255], 255);
        // Middle should be close to 128
        assert!((gamma[128] as i32 - 128).unsigned_abs() <= 5);
    }

    #[test]
    fn test_gamma_spline_roundtrip_values() {
        let mut gs = GammaSpline::new();
        gs.set_values(0.5, 1.5, 0.8, 1.2);
        let (kx1, ky1, kx2, ky2) = gs.get_values();
        assert!((kx1 - 0.5).abs() < 0.001);
        assert!((ky1 - 1.5).abs() < 0.001);
        assert!((kx2 - 0.8).abs() < 0.001);
        assert!((ky2 - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_gamma_spline_vertex_source() {
        let mut gs = GammaSpline::new();
        gs.set_box(0.0, 0.0, 100.0, 100.0);
        gs.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = gs.vertex(&mut x, &mut y);
        assert_eq!(cmd, crate::basics::PATH_CMD_MOVE_TO);
        assert!((x - 0.0).abs() < 0.01);

        // Should produce line_to vertices until > 1.0
        let mut count = 0;
        loop {
            let cmd = gs.vertex(&mut x, &mut y);
            if cmd == crate::basics::PATH_CMD_STOP {
                break;
            }
            assert_eq!(cmd, crate::basics::PATH_CMD_LINE_TO);
            count += 1;
        }
        assert!(count >= 99 && count <= 101); // ~100 pixels wide box
    }

    #[test]
    fn test_gamma_spline_clamping() {
        let mut gs = GammaSpline::new();
        gs.set_values(0.0, 3.0, -1.0, 2.5);
        let (kx1, ky1, kx2, ky2) = gs.get_values();
        // Should be clamped to [0.001, 1.999]
        assert!((kx1 - 0.001).abs() < 0.001);
        assert!((ky1 - 1.999).abs() < 0.001);
        assert!((kx2 - 0.001).abs() < 0.001);
        assert!((ky2 - 1.999).abs() < 0.001);
    }

    #[test]
    fn test_gamma_spline_as_gamma_function() {
        let gs = GammaSpline::new();
        // GammaSpline implements GammaFunction
        assert!((gs.call(0.0)).abs() < 0.01);
        assert!((gs.call(1.0) - 1.0).abs() < 0.01);
    }
}

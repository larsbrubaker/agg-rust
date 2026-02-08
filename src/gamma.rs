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
}

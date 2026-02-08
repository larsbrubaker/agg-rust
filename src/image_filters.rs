//! Image transformation filters and lookup table.
//!
//! Port of `agg_image_filters.h` and `agg_image_filters.cpp` — filter shape
//! functions and a weight lookup table used by image resampling span generators.

use crate::basics::{iround, uceil};
use crate::math::besj;
use std::f64::consts::PI;

// ============================================================================
// Constants
// ============================================================================

pub const IMAGE_FILTER_SHIFT: u32 = 14;
pub const IMAGE_FILTER_SCALE: i32 = 1 << IMAGE_FILTER_SHIFT; // 16384
pub const IMAGE_FILTER_MASK: i32 = IMAGE_FILTER_SCALE - 1;

pub const IMAGE_SUBPIXEL_SHIFT: u32 = 8;
pub const IMAGE_SUBPIXEL_SCALE: u32 = 1 << IMAGE_SUBPIXEL_SHIFT; // 256
pub const IMAGE_SUBPIXEL_MASK: u32 = IMAGE_SUBPIXEL_SCALE - 1;

// ============================================================================
// ImageFilterFunction trait
// ============================================================================

/// Trait for image filter shape functions.
///
/// Port of C++ template duck-typing for filter functions with
/// `radius()` and `calc_weight(x)` methods.
pub trait ImageFilterFunction {
    /// The radius of the filter kernel.
    fn radius(&self) -> f64;
    /// Calculate the filter weight at distance `x` from center.
    fn calc_weight(&self, x: f64) -> f64;
}

// ============================================================================
// ImageFilterLut — weight lookup table
// ============================================================================

/// Image filter weight lookup table.
///
/// Stores precomputed filter weights at subpixel resolution for fast
/// image resampling. Weights are stored as 14-bit fixed-point integers.
///
/// Port of C++ `image_filter_lut`.
pub struct ImageFilterLut {
    radius: f64,
    diameter: u32,
    start: i32,
    weight_array: Vec<i16>,
}

impl ImageFilterLut {
    /// Create an empty (uninitialized) filter LUT.
    pub fn new() -> Self {
        Self {
            radius: 0.0,
            diameter: 0,
            start: 0,
            weight_array: Vec::new(),
        }
    }

    /// Create and populate a filter LUT from a filter function.
    pub fn new_with_filter<F: ImageFilterFunction>(filter: &F, normalization: bool) -> Self {
        let mut lut = Self::new();
        lut.calculate(filter, normalization);
        lut
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }

    pub fn diameter(&self) -> u32 {
        self.diameter
    }

    pub fn start(&self) -> i32 {
        self.start
    }

    pub fn weight_array(&self) -> &[i16] {
        &self.weight_array
    }

    /// Populate the LUT from a filter function.
    ///
    /// Port of C++ `image_filter_lut::calculate`.
    pub fn calculate<F: ImageFilterFunction>(&mut self, filter: &F, normalization: bool) {
        let r = filter.radius();
        self.realloc_lut(r);
        let pivot = (self.diameter << (IMAGE_SUBPIXEL_SHIFT - 1)) as usize;
        for i in 0..pivot {
            let x = i as f64 / IMAGE_SUBPIXEL_SCALE as f64;
            let y = filter.calc_weight(x);
            let w = iround(y * IMAGE_FILTER_SCALE as f64) as i16;
            self.weight_array[pivot + i] = w;
            self.weight_array[pivot - i] = w;
        }
        let end = ((self.diameter as usize) << IMAGE_SUBPIXEL_SHIFT) - 1;
        self.weight_array[0] = self.weight_array[end];
        if normalization {
            self.normalize();
        }
    }

    /// Normalize weights so that for every subpixel offset, the filter
    /// weights sum to exactly `IMAGE_FILTER_SCALE`.
    ///
    /// Port of C++ `image_filter_lut::normalize`.
    #[allow(clippy::needless_range_loop)]
    pub fn normalize(&mut self) {
        let mut flip: i32 = 1;
        let subpixel_scale = IMAGE_SUBPIXEL_SCALE as usize;
        let diameter = self.diameter as usize;

        for i in 0..subpixel_scale {
            loop {
                let mut sum: i32 = 0;
                for j in 0..diameter {
                    sum += self.weight_array[j * subpixel_scale + i] as i32;
                }

                if sum == IMAGE_FILTER_SCALE {
                    break;
                }

                let k = IMAGE_FILTER_SCALE as f64 / sum as f64;
                sum = 0;
                for j in 0..diameter {
                    let idx = j * subpixel_scale + i;
                    let v = iround(self.weight_array[idx] as f64 * k) as i16;
                    self.weight_array[idx] = v;
                    sum += v as i32;
                }

                sum -= IMAGE_FILTER_SCALE;
                let inc: i32 = if sum > 0 { -1 } else { 1 };

                let mut j = 0;
                while j < diameter && sum != 0 {
                    flip ^= 1;
                    let idx = if flip != 0 {
                        diameter / 2 + j / 2
                    } else {
                        diameter / 2 - j / 2
                    };
                    let arr_idx = idx * subpixel_scale + i;
                    let v = self.weight_array[arr_idx] as i32;
                    if v < IMAGE_FILTER_SCALE {
                        self.weight_array[arr_idx] += inc as i16;
                        sum += inc;
                    }
                    j += 1;
                }
            }
        }

        let pivot = diameter << (IMAGE_SUBPIXEL_SHIFT as usize - 1);
        for i in 0..pivot {
            self.weight_array[pivot + i] = self.weight_array[pivot - i];
        }
        let end = (diameter << IMAGE_SUBPIXEL_SHIFT as usize) - 1;
        self.weight_array[0] = self.weight_array[end];
    }

    fn realloc_lut(&mut self, radius: f64) {
        self.radius = radius;
        self.diameter = uceil(radius) * 2;
        self.start = -((self.diameter / 2 - 1) as i32);
        let size = (self.diameter as usize) << IMAGE_SUBPIXEL_SHIFT;
        if size > self.weight_array.len() {
            self.weight_array.resize(size, 0);
        }
    }
}

impl Default for ImageFilterLut {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Filter shape implementations
// ============================================================================

/// Bilinear filter — radius 1.0, linear interpolation.
pub struct ImageFilterBilinear;
impl ImageFilterFunction for ImageFilterBilinear {
    fn radius(&self) -> f64 {
        1.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        1.0 - x
    }
}

/// Hanning window filter — radius 1.0.
pub struct ImageFilterHanning;
impl ImageFilterFunction for ImageFilterHanning {
    fn radius(&self) -> f64 {
        1.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        0.5 + 0.5 * (PI * x).cos()
    }
}

/// Hamming window filter — radius 1.0.
pub struct ImageFilterHamming;
impl ImageFilterFunction for ImageFilterHamming {
    fn radius(&self) -> f64 {
        1.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        0.54 + 0.46 * (PI * x).cos()
    }
}

/// Hermite filter — radius 1.0, cubic Hermite interpolation.
pub struct ImageFilterHermite;
impl ImageFilterFunction for ImageFilterHermite {
    fn radius(&self) -> f64 {
        1.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        (2.0 * x - 3.0) * x * x + 1.0
    }
}

/// Quadric filter — radius 1.5, piecewise quadratic.
pub struct ImageFilterQuadric;
impl ImageFilterFunction for ImageFilterQuadric {
    fn radius(&self) -> f64 {
        1.5
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x < 0.5 {
            return 0.75 - x * x;
        }
        if x < 1.5 {
            let t = x - 1.5;
            return 0.5 * t * t;
        }
        0.0
    }
}

/// Bicubic filter — radius 2.0, cubic B-spline.
pub struct ImageFilterBicubic;
impl ImageFilterBicubic {
    fn pow3(x: f64) -> f64 {
        if x <= 0.0 {
            0.0
        } else {
            x * x * x
        }
    }
}
impl ImageFilterFunction for ImageFilterBicubic {
    fn radius(&self) -> f64 {
        2.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        (1.0 / 6.0)
            * (Self::pow3(x + 2.0) - 4.0 * Self::pow3(x + 1.0) + 6.0 * Self::pow3(x)
                - 4.0 * Self::pow3(x - 1.0))
    }
}

/// Kaiser filter — radius 1.0, parameterized by `b` (default 6.33).
///
/// Uses modified Bessel function of the first kind (order 0).
pub struct ImageFilterKaiser {
    a: f64,
    i0a: f64,
    epsilon: f64,
}
impl ImageFilterKaiser {
    pub fn new(b: f64) -> Self {
        let epsilon = 1e-12;
        let i0a = 1.0 / Self::bessel_i0(b, epsilon);
        Self { a: b, i0a, epsilon }
    }

    fn bessel_i0(x: f64, epsilon: f64) -> f64 {
        let mut sum = 1.0;
        let y = x * x / 4.0;
        let mut t = y;
        let mut i = 2;
        while t > epsilon {
            sum += t;
            t *= y / (i * i) as f64;
            i += 1;
        }
        sum
    }
}
impl Default for ImageFilterKaiser {
    fn default() -> Self {
        Self::new(6.33)
    }
}
impl ImageFilterFunction for ImageFilterKaiser {
    fn radius(&self) -> f64 {
        1.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        Self::bessel_i0(self.a * (1.0 - x * x).sqrt(), self.epsilon) * self.i0a
    }
}

/// Catmull-Rom spline filter — radius 2.0.
pub struct ImageFilterCatrom;
impl ImageFilterFunction for ImageFilterCatrom {
    fn radius(&self) -> f64 {
        2.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x < 1.0 {
            return 0.5 * (2.0 + x * x * (-5.0 + x * 3.0));
        }
        if x < 2.0 {
            return 0.5 * (4.0 + x * (-8.0 + x * (5.0 - x)));
        }
        0.0
    }
}

/// Mitchell-Netravali filter — radius 2.0, parameterized by `b` and `c`.
///
/// Default: b = 1/3, c = 1/3 (recommended for general image scaling).
pub struct ImageFilterMitchell {
    p0: f64,
    p2: f64,
    p3: f64,
    q0: f64,
    q1: f64,
    q2: f64,
    q3: f64,
}
impl ImageFilterMitchell {
    pub fn new(b: f64, c: f64) -> Self {
        Self {
            p0: (6.0 - 2.0 * b) / 6.0,
            p2: (-18.0 + 12.0 * b + 6.0 * c) / 6.0,
            p3: (12.0 - 9.0 * b - 6.0 * c) / 6.0,
            q0: (8.0 * b + 24.0 * c) / 6.0,
            q1: (-12.0 * b - 48.0 * c) / 6.0,
            q2: (6.0 * b + 30.0 * c) / 6.0,
            q3: (-b - 6.0 * c) / 6.0,
        }
    }
}
impl Default for ImageFilterMitchell {
    fn default() -> Self {
        Self::new(1.0 / 3.0, 1.0 / 3.0)
    }
}
impl ImageFilterFunction for ImageFilterMitchell {
    fn radius(&self) -> f64 {
        2.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x < 1.0 {
            return self.p0 + x * x * (self.p2 + x * self.p3);
        }
        if x < 2.0 {
            return self.q0 + x * (self.q1 + x * (self.q2 + x * self.q3));
        }
        0.0
    }
}

/// Spline16 filter — radius 2.0.
pub struct ImageFilterSpline16;
impl ImageFilterFunction for ImageFilterSpline16 {
    fn radius(&self) -> f64 {
        2.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x < 1.0 {
            return ((x - 9.0 / 5.0) * x - 1.0 / 5.0) * x + 1.0;
        }
        ((-1.0 / 3.0 * (x - 1.0) + 4.0 / 5.0) * (x - 1.0) - 7.0 / 15.0) * (x - 1.0)
    }
}

/// Spline36 filter — radius 3.0.
pub struct ImageFilterSpline36;
impl ImageFilterFunction for ImageFilterSpline36 {
    fn radius(&self) -> f64 {
        3.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x < 1.0 {
            return ((13.0 / 11.0 * x - 453.0 / 209.0) * x - 3.0 / 209.0) * x + 1.0;
        }
        if x < 2.0 {
            return ((-6.0 / 11.0 * (x - 1.0) + 270.0 / 209.0) * (x - 1.0) - 156.0 / 209.0)
                * (x - 1.0);
        }
        ((1.0 / 11.0 * (x - 2.0) - 45.0 / 209.0) * (x - 2.0) + 26.0 / 209.0) * (x - 2.0)
    }
}

/// Gaussian filter — radius 2.0.
pub struct ImageFilterGaussian;
impl ImageFilterFunction for ImageFilterGaussian {
    fn radius(&self) -> f64 {
        2.0
    }
    fn calc_weight(&self, x: f64) -> f64 {
        (-2.0 * x * x).exp() * (2.0 / PI).sqrt()
    }
}

/// Bessel filter — radius 3.2383.
pub struct ImageFilterBessel;
impl ImageFilterFunction for ImageFilterBessel {
    fn radius(&self) -> f64 {
        3.2383
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x == 0.0 {
            PI / 4.0
        } else {
            besj(PI * x, 1) / (2.0 * x)
        }
    }
}

// ============================================================================
// Variable-radius filters
// ============================================================================

/// Sinc filter — variable radius (minimum 2.0).
pub struct ImageFilterSinc {
    radius: f64,
}
impl ImageFilterSinc {
    pub fn new(r: f64) -> Self {
        Self {
            radius: if r < 2.0 { 2.0 } else { r },
        }
    }
}
impl ImageFilterFunction for ImageFilterSinc {
    fn radius(&self) -> f64 {
        self.radius
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x == 0.0 {
            return 1.0;
        }
        let x = x * PI;
        x.sin() / x
    }
}

/// Lanczos filter — variable radius (minimum 2.0).
pub struct ImageFilterLanczos {
    radius: f64,
}
impl ImageFilterLanczos {
    pub fn new(r: f64) -> Self {
        Self {
            radius: if r < 2.0 { 2.0 } else { r },
        }
    }
}
impl ImageFilterFunction for ImageFilterLanczos {
    fn radius(&self) -> f64 {
        self.radius
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x == 0.0 {
            return 1.0;
        }
        if x > self.radius {
            return 0.0;
        }
        let x = x * PI;
        let xr = x / self.radius;
        (x.sin() / x) * (xr.sin() / xr)
    }
}

/// Blackman window filter — variable radius (minimum 2.0).
pub struct ImageFilterBlackman {
    radius: f64,
}
impl ImageFilterBlackman {
    pub fn new(r: f64) -> Self {
        Self {
            radius: if r < 2.0 { 2.0 } else { r },
        }
    }
}
impl ImageFilterFunction for ImageFilterBlackman {
    fn radius(&self) -> f64 {
        self.radius
    }
    fn calc_weight(&self, x: f64) -> f64 {
        if x == 0.0 {
            return 1.0;
        }
        if x > self.radius {
            return 0.0;
        }
        let x = x * PI;
        let xr = x / self.radius;
        (x.sin() / x) * (0.42 + 0.5 * xr.cos() + 0.08 * (2.0 * xr).cos())
    }
}

// ============================================================================
// Fixed-radius convenience wrappers
// ============================================================================

/// Sinc filter with radius 3.0.
pub struct ImageFilterSinc36(ImageFilterSinc);
impl ImageFilterSinc36 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(3.0))
    }
}
impl Default for ImageFilterSinc36 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc36 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Sinc filter with radius 4.0.
pub struct ImageFilterSinc64(ImageFilterSinc);
impl ImageFilterSinc64 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(4.0))
    }
}
impl Default for ImageFilterSinc64 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc64 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Sinc filter with radius 5.0.
pub struct ImageFilterSinc100(ImageFilterSinc);
impl ImageFilterSinc100 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(5.0))
    }
}
impl Default for ImageFilterSinc100 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc100 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Sinc filter with radius 6.0.
pub struct ImageFilterSinc144(ImageFilterSinc);
impl ImageFilterSinc144 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(6.0))
    }
}
impl Default for ImageFilterSinc144 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc144 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Sinc filter with radius 7.0.
pub struct ImageFilterSinc196(ImageFilterSinc);
impl ImageFilterSinc196 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(7.0))
    }
}
impl Default for ImageFilterSinc196 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc196 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Sinc filter with radius 8.0.
pub struct ImageFilterSinc256(ImageFilterSinc);
impl ImageFilterSinc256 {
    pub fn new() -> Self {
        Self(ImageFilterSinc::new(8.0))
    }
}
impl Default for ImageFilterSinc256 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterSinc256 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 3.0.
pub struct ImageFilterLanczos36(ImageFilterLanczos);
impl ImageFilterLanczos36 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(3.0))
    }
}
impl Default for ImageFilterLanczos36 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos36 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 4.0.
pub struct ImageFilterLanczos64(ImageFilterLanczos);
impl ImageFilterLanczos64 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(4.0))
    }
}
impl Default for ImageFilterLanczos64 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos64 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 5.0.
pub struct ImageFilterLanczos100(ImageFilterLanczos);
impl ImageFilterLanczos100 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(5.0))
    }
}
impl Default for ImageFilterLanczos100 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos100 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 6.0.
pub struct ImageFilterLanczos144(ImageFilterLanczos);
impl ImageFilterLanczos144 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(6.0))
    }
}
impl Default for ImageFilterLanczos144 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos144 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 7.0.
pub struct ImageFilterLanczos196(ImageFilterLanczos);
impl ImageFilterLanczos196 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(7.0))
    }
}
impl Default for ImageFilterLanczos196 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos196 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Lanczos filter with radius 8.0.
pub struct ImageFilterLanczos256(ImageFilterLanczos);
impl ImageFilterLanczos256 {
    pub fn new() -> Self {
        Self(ImageFilterLanczos::new(8.0))
    }
}
impl Default for ImageFilterLanczos256 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterLanczos256 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 3.0.
pub struct ImageFilterBlackman36(ImageFilterBlackman);
impl ImageFilterBlackman36 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(3.0))
    }
}
impl Default for ImageFilterBlackman36 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman36 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 4.0.
pub struct ImageFilterBlackman64(ImageFilterBlackman);
impl ImageFilterBlackman64 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(4.0))
    }
}
impl Default for ImageFilterBlackman64 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman64 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 5.0.
pub struct ImageFilterBlackman100(ImageFilterBlackman);
impl ImageFilterBlackman100 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(5.0))
    }
}
impl Default for ImageFilterBlackman100 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman100 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 6.0.
pub struct ImageFilterBlackman144(ImageFilterBlackman);
impl ImageFilterBlackman144 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(6.0))
    }
}
impl Default for ImageFilterBlackman144 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman144 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 7.0.
pub struct ImageFilterBlackman196(ImageFilterBlackman);
impl ImageFilterBlackman196 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(7.0))
    }
}
impl Default for ImageFilterBlackman196 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman196 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

/// Blackman filter with radius 8.0.
pub struct ImageFilterBlackman256(ImageFilterBlackman);
impl ImageFilterBlackman256 {
    pub fn new() -> Self {
        Self(ImageFilterBlackman::new(8.0))
    }
}
impl Default for ImageFilterBlackman256 {
    fn default() -> Self {
        Self::new()
    }
}
impl ImageFilterFunction for ImageFilterBlackman256 {
    fn radius(&self) -> f64 {
        self.0.radius()
    }
    fn calc_weight(&self, x: f64) -> f64 {
        self.0.calc_weight(x)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bilinear_radius_and_weight() {
        let f = ImageFilterBilinear;
        assert_eq!(f.radius(), 1.0);
        assert_eq!(f.calc_weight(0.0), 1.0);
        assert_eq!(f.calc_weight(0.5), 0.5);
        assert_eq!(f.calc_weight(1.0), 0.0);
    }

    #[test]
    fn test_hermite_at_zero() {
        let f = ImageFilterHermite;
        assert_eq!(f.calc_weight(0.0), 1.0);
    }

    #[test]
    fn test_bicubic_at_zero() {
        let f = ImageFilterBicubic;
        let w = f.calc_weight(0.0);
        // (1/6)*(pow3(2) - 4*pow3(1) + 6*pow3(0) - 4*pow3(-1)) = (1/6)*(8-4+0-0) = 2/3
        let expected = 2.0 / 3.0;
        assert!(
            (w - expected).abs() < 1e-10,
            "bicubic at 0 should be ~{expected}, got {w}"
        );
    }

    #[test]
    fn test_gaussian_at_zero() {
        let f = ImageFilterGaussian;
        let expected = (2.0 / PI).sqrt();
        assert!((f.calc_weight(0.0) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_kaiser_at_zero() {
        let f = ImageFilterKaiser::default();
        let w = f.calc_weight(0.0);
        assert!(
            (w - 1.0).abs() < 1e-10,
            "kaiser at 0 should be ~1.0, got {w}"
        );
    }

    #[test]
    fn test_mitchell_at_zero() {
        let f = ImageFilterMitchell::default();
        let w = f.calc_weight(0.0);
        let expected = f.p0; // p0 = (6 - 2/3) / 6 = 8/9
        assert!((w - expected).abs() < 1e-10);
    }

    #[test]
    fn test_sinc_at_zero() {
        let f = ImageFilterSinc::new(3.0);
        assert_eq!(f.calc_weight(0.0), 1.0);
        assert_eq!(f.radius(), 3.0);
    }

    #[test]
    fn test_sinc_minimum_radius() {
        let f = ImageFilterSinc::new(1.0);
        assert_eq!(f.radius(), 2.0); // minimum is 2.0
    }

    #[test]
    fn test_lanczos_at_zero() {
        let f = ImageFilterLanczos::new(3.0);
        assert_eq!(f.calc_weight(0.0), 1.0);
    }

    #[test]
    fn test_lanczos_beyond_radius() {
        let f = ImageFilterLanczos::new(3.0);
        assert_eq!(f.calc_weight(4.0), 0.0);
    }

    #[test]
    fn test_blackman_at_zero() {
        let f = ImageFilterBlackman::new(3.0);
        assert_eq!(f.calc_weight(0.0), 1.0);
    }

    #[test]
    fn test_lut_bilinear() {
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        assert_eq!(lut.radius(), 1.0);
        assert_eq!(lut.diameter(), 2);
        assert_eq!(lut.start(), 0); // -(2/2 - 1) = 0
                                    // Weight array size = diameter * 256 = 512
        assert_eq!(lut.weight_array().len(), 512);
    }

    #[test]
    fn test_lut_weight_symmetry() {
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, false);
        let weights = lut.weight_array();
        let pivot = (lut.diameter() as usize) << (IMAGE_SUBPIXEL_SHIFT as usize - 1);
        // Weights should be symmetric around pivot
        for i in 1..pivot {
            assert_eq!(
                weights[pivot + i],
                weights[pivot - i],
                "asymmetry at offset {i}"
            );
        }
    }

    #[test]
    fn test_lut_normalization() {
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let weights = lut.weight_array();
        let diameter = lut.diameter() as usize;
        let subpixel_scale = IMAGE_SUBPIXEL_SCALE as usize;
        // For each subpixel offset, weights across diameter should sum to IMAGE_FILTER_SCALE
        for i in 0..subpixel_scale {
            let sum: i32 = (0..diameter)
                .map(|j| weights[j * subpixel_scale + i] as i32)
                .sum();
            assert_eq!(sum, IMAGE_FILTER_SCALE, "sum at offset {i} = {sum}");
        }
    }

    #[test]
    fn test_lut_bicubic() {
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBicubic, true);
        assert_eq!(lut.diameter(), 4);
        assert_eq!(lut.start(), -1); // -(4/2 - 1) = -1
    }

    #[test]
    fn test_convenience_wrappers() {
        // Just verify the convenience types create correct radii
        assert_eq!(ImageFilterSinc36::new().radius(), 3.0);
        assert_eq!(ImageFilterSinc64::new().radius(), 4.0);
        assert_eq!(ImageFilterLanczos36::new().radius(), 3.0);
        assert_eq!(ImageFilterLanczos256::new().radius(), 8.0);
        assert_eq!(ImageFilterBlackman36::new().radius(), 3.0);
        assert_eq!(ImageFilterBlackman256::new().radius(), 8.0);
    }

    #[test]
    fn test_bessel_filter() {
        let f = ImageFilterBessel;
        assert_eq!(f.radius(), 3.2383);
        let w0 = f.calc_weight(0.0);
        assert!((w0 - PI / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_quadric_regions() {
        let f = ImageFilterQuadric;
        // x < 0.5: 0.75 - x²
        assert_eq!(f.calc_weight(0.0), 0.75);
        // x = 0.5: boundary
        assert!((f.calc_weight(0.5) - 0.5).abs() < 1e-10);
        // x >= 1.5: 0
        assert_eq!(f.calc_weight(1.5), 0.0);
    }

    #[test]
    fn test_catrom_at_zero() {
        let f = ImageFilterCatrom;
        assert_eq!(f.calc_weight(0.0), 1.0);
    }

    #[test]
    fn test_hanning_at_zero() {
        let f = ImageFilterHanning;
        assert_eq!(f.calc_weight(0.0), 1.0);
    }

    #[test]
    fn test_spline16_at_zero() {
        let f = ImageFilterSpline16;
        assert_eq!(f.calc_weight(0.0), 1.0);
    }
}

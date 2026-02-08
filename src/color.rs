//! Color types and operations.
//!
//! Port of `agg_color_rgba.h`, `agg_color_rgba.cpp`, and `agg_color_gray.h`.
//!
//! Provides RGBA and grayscale color types at different precisions:
//! - `Rgba` — f64 components (linear working space)
//! - `Rgba8` — u8 components (8-bit per channel)
//! - `Rgba16` — u16 components (16-bit per channel)
//! - `Gray8` — u8 grayscale + alpha
//! - `Gray16` — u16 grayscale + alpha
//!
//! Note: sRGB colorspace variants (`srgba8`, `sgray8`) from the C++ code are
//! not included in Phase 1. They will be added when needed (they require sRGB
//! lookup table infrastructure).

use crate::basics::{uround, CoverType, COVER_MASK};

// ============================================================================
// Component orders (for pixel format layer)
// ============================================================================

/// RGB component order: R=0, G=1, B=2
pub struct OrderRgb;
impl OrderRgb {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
    pub const N: usize = 3;
}

/// BGR component order: B=0, G=1, R=2
pub struct OrderBgr;
impl OrderBgr {
    pub const B: usize = 0;
    pub const G: usize = 1;
    pub const R: usize = 2;
    pub const N: usize = 3;
}

/// RGBA component order: R=0, G=1, B=2, A=3
pub struct OrderRgba;
impl OrderRgba {
    pub const R: usize = 0;
    pub const G: usize = 1;
    pub const B: usize = 2;
    pub const A: usize = 3;
    pub const N: usize = 4;
}

/// ARGB component order: A=0, R=1, G=2, B=3
pub struct OrderArgb;
impl OrderArgb {
    pub const A: usize = 0;
    pub const R: usize = 1;
    pub const G: usize = 2;
    pub const B: usize = 3;
    pub const N: usize = 4;
}

/// ABGR component order: A=0, B=1, G=2, R=3
pub struct OrderAbgr;
impl OrderAbgr {
    pub const A: usize = 0;
    pub const B: usize = 1;
    pub const G: usize = 2;
    pub const R: usize = 3;
    pub const N: usize = 4;
}

/// BGRA component order: B=0, G=1, R=2, A=3
pub struct OrderBgra;
impl OrderBgra {
    pub const B: usize = 0;
    pub const G: usize = 1;
    pub const R: usize = 2;
    pub const A: usize = 3;
    pub const N: usize = 4;
}

// ============================================================================
// Rgba (f64 precision color)
// ============================================================================

/// RGBA color with f64 components in range [0, 1].
/// Port of C++ `rgba`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Rgba {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn new_rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_opacity(c: &Rgba, a: f64) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a,
        }
    }

    pub fn clear(&mut self) -> &mut Self {
        self.r = 0.0;
        self.g = 0.0;
        self.b = 0.0;
        self.a = 0.0;
        self
    }

    pub fn transparent(&mut self) -> &mut Self {
        self.a = 0.0;
        self
    }

    pub fn set_opacity(&mut self, a: f64) -> &mut Self {
        if a < 0.0 {
            self.a = 0.0;
        } else if a > 1.0 {
            self.a = 1.0;
        } else {
            self.a = a;
        }
        self
    }

    pub fn opacity(&self) -> f64 {
        self.a
    }

    pub fn premultiply(&mut self) -> &mut Self {
        self.r *= self.a;
        self.g *= self.a;
        self.b *= self.a;
        self
    }

    pub fn premultiply_with_alpha(&mut self, a: f64) -> &mut Self {
        if self.a <= 0.0 || a <= 0.0 {
            self.r = 0.0;
            self.g = 0.0;
            self.b = 0.0;
            self.a = 0.0;
        } else {
            let scale = a / self.a;
            self.r *= scale;
            self.g *= scale;
            self.b *= scale;
            self.a = scale;
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if self.a == 0.0 {
            self.r = 0.0;
            self.g = 0.0;
            self.b = 0.0;
        } else {
            let inv_a = 1.0 / self.a;
            self.r *= inv_a;
            self.g *= inv_a;
            self.b *= inv_a;
        }
        self
    }

    /// Interpolate between `self` and `c` by parameter `k`.
    pub fn gradient(&self, c: &Rgba, k: f64) -> Rgba {
        Rgba {
            r: self.r + (c.r - self.r) * k,
            g: self.g + (c.g - self.g) * k,
            b: self.b + (c.b - self.b) * k,
            a: self.a + (c.a - self.a) * k,
        }
    }

    pub fn no_color() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
    }

    /// Create a color from a visible light wavelength (380–780 nm).
    pub fn from_wavelength(wl: f64, gamma: f64) -> Self {
        let mut t = Rgba::new(0.0, 0.0, 0.0, 1.0);

        if (380.0..=440.0).contains(&wl) {
            t.r = -(wl - 440.0) / (440.0 - 380.0);
            t.b = 1.0;
        } else if (440.0..=490.0).contains(&wl) {
            t.g = (wl - 440.0) / (490.0 - 440.0);
            t.b = 1.0;
        } else if (490.0..=510.0).contains(&wl) {
            t.g = 1.0;
            t.b = -(wl - 510.0) / (510.0 - 490.0);
        } else if (510.0..=580.0).contains(&wl) {
            t.r = (wl - 510.0) / (580.0 - 510.0);
            t.g = 1.0;
        } else if (580.0..=645.0).contains(&wl) {
            t.r = 1.0;
            t.g = -(wl - 645.0) / (645.0 - 580.0);
        } else if (645.0..=780.0).contains(&wl) {
            t.r = 1.0;
        }

        let s = if wl > 700.0 {
            0.3 + 0.7 * (780.0 - wl) / (780.0 - 700.0)
        } else if wl < 420.0 {
            0.3 + 0.7 * (wl - 380.0) / (420.0 - 380.0)
        } else {
            1.0
        };

        t.r = (t.r * s).powf(gamma);
        t.g = (t.g * s).powf(gamma);
        t.b = (t.b * s).powf(gamma);
        t
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Self::no_color()
    }
}

impl core::ops::Add for Rgba {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl core::ops::AddAssign for Rgba {
    fn add_assign(&mut self, rhs: Self) {
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
        self.a += rhs.a;
    }
}

impl core::ops::Mul<f64> for Rgba {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        Self {
            r: self.r * k,
            g: self.g * k,
            b: self.b * k,
            a: self.a * k,
        }
    }
}

impl core::ops::MulAssign<f64> for Rgba {
    fn mul_assign(&mut self, k: f64) {
        self.r *= k;
        self.g *= k;
        self.b *= k;
        self.a *= k;
    }
}

/// Create a pre-multiplied Rgba color.
pub fn rgba_pre(r: f64, g: f64, b: f64, a: f64) -> Rgba {
    let mut c = Rgba::new(r, g, b, a);
    c.premultiply();
    c
}

// ============================================================================
// Rgba8 (8-bit per channel)
// ============================================================================

/// RGBA color with u8 components.
/// Port of C++ `rgba8T<linear>` (linear colorspace variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const BASE_SHIFT: u32 = 8;
    pub const BASE_SCALE: u32 = 1 << Self::BASE_SHIFT;
    pub const BASE_MASK: u32 = Self::BASE_SCALE - 1;
    pub const BASE_MSB: u32 = 1 << (Self::BASE_SHIFT - 1);

    pub fn new(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: a as u8,
        }
    }

    pub fn new_opaque(r: u32, g: u32, b: u32) -> Self {
        Self::new(r, g, b, Self::BASE_MASK)
    }

    pub fn with_opacity(c: &Rgba8, a: u32) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: a as u8,
        }
    }

    /// Convert from `Rgba` (f64) to `Rgba8` (u8).
    pub fn from_rgba(c: &Rgba) -> Self {
        Self {
            r: uround(c.r * Self::BASE_MASK as f64) as u8,
            g: uround(c.g * Self::BASE_MASK as f64) as u8,
            b: uround(c.b * Self::BASE_MASK as f64) as u8,
            a: uround(c.a * Self::BASE_MASK as f64) as u8,
        }
    }

    /// Convert to `Rgba` (f64).
    pub fn to_rgba(&self) -> Rgba {
        Rgba {
            r: self.r as f64 / 255.0,
            g: self.g as f64 / 255.0,
            b: self.b as f64 / 255.0,
            a: self.a as f64 / 255.0,
        }
    }

    pub fn to_double(a: u8) -> f64 {
        a as f64 / Self::BASE_MASK as f64
    }

    pub fn from_double(a: f64) -> u8 {
        uround(a * Self::BASE_MASK as f64) as u8
    }

    pub fn empty_value() -> u8 {
        0
    }

    pub fn full_value() -> u8 {
        Self::BASE_MASK as u8
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    pub fn is_opaque(&self) -> bool {
        self.a == Self::BASE_MASK as u8
    }

    pub fn invert(x: u8) -> u8 {
        Self::BASE_MASK as u8 - x
    }

    /// Fixed-point multiply, exact over u8.
    /// `(a * b + 128) >> 8`, with rounding correction.
    #[inline]
    pub fn multiply(a: u8, b: u8) -> u8 {
        let t: u32 = a as u32 * b as u32 + Self::BASE_MSB;
        (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT) as u8
    }

    /// Fixed-point demultiply.
    #[inline]
    pub fn demultiply_value(a: u8, b: u8) -> u8 {
        if (a as u32) * (b as u32) == 0 {
            0
        } else if a >= b {
            Self::BASE_MASK as u8
        } else {
            (a as u32 * Self::BASE_MASK + (b as u32 >> 1)) as u8 / b
        }
    }

    /// Multiply a color component by a cover.
    #[inline]
    pub fn mult_cover(a: u8, b: CoverType) -> u8 {
        Self::multiply(a, b)
    }

    /// Scale a cover by a value.
    #[inline]
    pub fn scale_cover(a: CoverType, b: u8) -> CoverType {
        Self::multiply(b, a)
    }

    /// Interpolate p to q by a, assuming q is premultiplied by a.
    #[inline]
    pub fn prelerp(p: u8, q: u8, a: u8) -> u8 {
        p.wrapping_add(q).wrapping_sub(Self::multiply(p, a))
    }

    /// Interpolate p to q by a.
    #[inline]
    pub fn lerp(p: u8, q: u8, a: u8) -> u8 {
        let t = (q as i32 - p as i32) * a as i32 + Self::BASE_MSB as i32 - (p > q) as i32;
        (p as i32 + (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT)) as u8
    }

    pub fn clear(&mut self) -> &mut Self {
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 0;
        self
    }

    pub fn transparent(&mut self) -> &mut Self {
        self.a = 0;
        self
    }

    pub fn set_opacity(&mut self, a: f64) -> &mut Self {
        if a < 0.0 {
            self.a = 0;
        } else if a > 1.0 {
            self.a = 1;
        } else {
            self.a = uround(a * Self::BASE_MASK as f64) as u8;
        }
        self
    }

    pub fn opacity(&self) -> f64 {
        self.a as f64 / Self::BASE_MASK as f64
    }

    pub fn premultiply(&mut self) -> &mut Self {
        if self.a != Self::BASE_MASK as u8 {
            if self.a == 0 {
                self.r = 0;
                self.g = 0;
                self.b = 0;
            } else {
                self.r = Self::multiply(self.r, self.a);
                self.g = Self::multiply(self.g, self.a);
                self.b = Self::multiply(self.b, self.a);
            }
        }
        self
    }

    pub fn premultiply_with_alpha(&mut self, a_: u32) -> &mut Self {
        if self.a as u32 != Self::BASE_MASK || a_ < Self::BASE_MASK {
            if self.a == 0 || a_ == 0 {
                self.r = 0;
                self.g = 0;
                self.b = 0;
                self.a = 0;
            } else {
                let r_ = (self.r as u32 * a_) / self.a as u32;
                let g_ = (self.g as u32 * a_) / self.a as u32;
                let b_ = (self.b as u32 * a_) / self.a as u32;
                self.r = if r_ > a_ { a_ as u8 } else { r_ as u8 };
                self.g = if g_ > a_ { a_ as u8 } else { g_ as u8 };
                self.b = if b_ > a_ { a_ as u8 } else { b_ as u8 };
                self.a = a_ as u8;
            }
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.r = 0;
                self.g = 0;
                self.b = 0;
            } else {
                let r_ = (self.r as u32 * Self::BASE_MASK) / self.a as u32;
                let g_ = (self.g as u32 * Self::BASE_MASK) / self.a as u32;
                let b_ = (self.b as u32 * Self::BASE_MASK) / self.a as u32;
                self.r = r_.min(Self::BASE_MASK) as u8;
                self.g = g_.min(Self::BASE_MASK) as u8;
                self.b = b_.min(Self::BASE_MASK) as u8;
            }
        }
        self
    }

    /// Interpolate between `self` and `c` by parameter `k` (0.0 to 1.0).
    pub fn gradient(&self, c: &Rgba8, k: f64) -> Rgba8 {
        let ik = uround(k * Self::BASE_MASK as f64) as u8;
        Rgba8 {
            r: Self::lerp(self.r, c.r, ik),
            g: Self::lerp(self.g, c.g, ik),
            b: Self::lerp(self.b, c.b, ik),
            a: Self::lerp(self.a, c.a, ik),
        }
    }

    /// Add color `c` with coverage `cover`.
    pub fn add(&mut self, c: &Rgba8, cover: u32) {
        let cr: u32;
        let cg: u32;
        let cb: u32;
        let ca: u32;
        if cover == COVER_MASK {
            if c.a as u32 == Self::BASE_MASK {
                *self = *c;
                return;
            } else {
                cr = self.r as u32 + c.r as u32;
                cg = self.g as u32 + c.g as u32;
                cb = self.b as u32 + c.b as u32;
                ca = self.a as u32 + c.a as u32;
            }
        } else {
            cr = self.r as u32 + Self::mult_cover(c.r, cover as u8) as u32;
            cg = self.g as u32 + Self::mult_cover(c.g, cover as u8) as u32;
            cb = self.b as u32 + Self::mult_cover(c.b, cover as u8) as u32;
            ca = self.a as u32 + Self::mult_cover(c.a, cover as u8) as u32;
        }
        self.r = cr.min(Self::BASE_MASK) as u8;
        self.g = cg.min(Self::BASE_MASK) as u8;
        self.b = cb.min(Self::BASE_MASK) as u8;
        self.a = ca.min(Self::BASE_MASK) as u8;
    }

    /// Apply forward gamma correction.
    pub fn apply_gamma_dir(&mut self, gamma: &crate::gamma::GammaLut) {
        self.r = gamma.dir(self.r);
        self.g = gamma.dir(self.g);
        self.b = gamma.dir(self.b);
    }

    /// Apply inverse gamma correction.
    pub fn apply_gamma_inv(&mut self, gamma: &crate::gamma::GammaLut) {
        self.r = gamma.inv(self.r);
        self.g = gamma.inv(self.g);
        self.b = gamma.inv(self.b);
    }

    pub fn no_color() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    pub fn from_wavelength(wl: f64, gamma: f64) -> Self {
        Self::from_rgba(&Rgba::from_wavelength(wl, gamma))
    }
}

impl Default for Rgba8 {
    fn default() -> Self {
        Self::no_color()
    }
}

/// Create an Rgba8 from a packed RGB value (0xRRGGBB).
pub fn rgb8_packed(v: u32) -> Rgba8 {
    Rgba8::new((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF, 255)
}

/// Create an Rgba8 from a packed BGR value (0xBBGGRR).
pub fn bgr8_packed(v: u32) -> Rgba8 {
    Rgba8::new(v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF, 255)
}

/// Create an Rgba8 from a packed ARGB value (0xAARRGGBB).
pub fn argb8_packed(v: u32) -> Rgba8 {
    Rgba8::new((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF, v >> 24)
}

// ============================================================================
// Rgba16 (16-bit per channel)
// ============================================================================

/// RGBA color with u16 components.
/// Port of C++ `rgba16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba16 {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

impl Rgba16 {
    pub const BASE_SHIFT: u32 = 16;
    pub const BASE_SCALE: u32 = 1 << Self::BASE_SHIFT;
    pub const BASE_MASK: u32 = Self::BASE_SCALE - 1;
    pub const BASE_MSB: u32 = 1 << (Self::BASE_SHIFT - 1);

    pub fn new(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    pub fn new_opaque(r: u32, g: u32, b: u32) -> Self {
        Self::new(r, g, b, Self::BASE_MASK)
    }

    /// Convert from Rgba (f64).
    pub fn from_rgba(c: &Rgba) -> Self {
        Self {
            r: uround(c.r * Self::BASE_MASK as f64) as u16,
            g: uround(c.g * Self::BASE_MASK as f64) as u16,
            b: uround(c.b * Self::BASE_MASK as f64) as u16,
            a: uround(c.a * Self::BASE_MASK as f64) as u16,
        }
    }

    /// Convert from Rgba8 (u8) by expanding 8-bit to 16-bit.
    pub fn from_rgba8(c: &Rgba8) -> Self {
        Self {
            r: ((c.r as u16) << 8) | c.r as u16,
            g: ((c.g as u16) << 8) | c.g as u16,
            b: ((c.b as u16) << 8) | c.b as u16,
            a: ((c.a as u16) << 8) | c.a as u16,
        }
    }

    pub fn to_rgba(&self) -> Rgba {
        Rgba {
            r: self.r as f64 / 65535.0,
            g: self.g as f64 / 65535.0,
            b: self.b as f64 / 65535.0,
            a: self.a as f64 / 65535.0,
        }
    }

    pub fn to_rgba8(&self) -> Rgba8 {
        Rgba8::new(
            (self.r >> 8) as u32,
            (self.g >> 8) as u32,
            (self.b >> 8) as u32,
            (self.a >> 8) as u32,
        )
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    pub fn is_opaque(&self) -> bool {
        self.a == Self::BASE_MASK as u16
    }

    pub fn invert(x: u16) -> u16 {
        Self::BASE_MASK as u16 - x
    }

    /// Fixed-point multiply, exact over u16.
    #[inline]
    pub fn multiply(a: u16, b: u16) -> u16 {
        let t: u32 = a as u32 * b as u32 + Self::BASE_MSB;
        (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT) as u16
    }

    /// Interpolate p to q by a.
    #[inline]
    pub fn lerp(p: u16, q: u16, a: u16) -> u16 {
        let t = (q as i32 - p as i32) * a as i32 + Self::BASE_MSB as i32 - (p > q) as i32;
        (p as i32 + (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT)) as u16
    }

    /// Multiply a color component by a cover (8-bit).
    #[inline]
    pub fn mult_cover(a: u16, b: CoverType) -> u16 {
        Self::multiply(a, (b as u16) << 8 | b as u16)
    }

    pub fn clear(&mut self) -> &mut Self {
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 0;
        self
    }

    pub fn premultiply(&mut self) -> &mut Self {
        if self.a as u32 != Self::BASE_MASK {
            if self.a == 0 {
                self.r = 0;
                self.g = 0;
                self.b = 0;
            } else {
                self.r = Self::multiply(self.r, self.a);
                self.g = Self::multiply(self.g, self.a);
                self.b = Self::multiply(self.b, self.a);
            }
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.r = 0;
                self.g = 0;
                self.b = 0;
            } else {
                let r_ = (self.r as u32 * Self::BASE_MASK) / self.a as u32;
                let g_ = (self.g as u32 * Self::BASE_MASK) / self.a as u32;
                let b_ = (self.b as u32 * Self::BASE_MASK) / self.a as u32;
                self.r = r_.min(Self::BASE_MASK) as u16;
                self.g = g_.min(Self::BASE_MASK) as u16;
                self.b = b_.min(Self::BASE_MASK) as u16;
            }
        }
        self
    }

    pub fn gradient(&self, c: &Rgba16, k: f64) -> Rgba16 {
        let ik = uround(k * Self::BASE_MASK as f64) as u16;
        Rgba16 {
            r: Self::lerp(self.r, c.r, ik),
            g: Self::lerp(self.g, c.g, ik),
            b: Self::lerp(self.b, c.b, ik),
            a: Self::lerp(self.a, c.a, ik),
        }
    }

    pub fn no_color() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    pub fn from_wavelength(wl: f64, gamma: f64) -> Self {
        Self::from_rgba(&Rgba::from_wavelength(wl, gamma))
    }
}

impl Default for Rgba16 {
    fn default() -> Self {
        Self::no_color()
    }
}

// ============================================================================
// Gray8 (8-bit grayscale)
// ============================================================================

/// Grayscale color with u8 components (value + alpha).
/// Port of C++ `gray8T<linear>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gray8 {
    pub v: u8,
    pub a: u8,
}

impl Gray8 {
    pub const BASE_SHIFT: u32 = 8;
    pub const BASE_SCALE: u32 = 1 << Self::BASE_SHIFT;
    pub const BASE_MASK: u32 = Self::BASE_SCALE - 1;
    pub const BASE_MSB: u32 = 1 << (Self::BASE_SHIFT - 1);

    pub fn new(v: u32, a: u32) -> Self {
        Self {
            v: v as u8,
            a: a as u8,
        }
    }

    pub fn new_opaque(v: u32) -> Self {
        Self::new(v, Self::BASE_MASK)
    }

    /// Calculate luminance from linear RGB (ITU-R BT.709).
    pub fn luminance_from_rgba(c: &Rgba) -> u8 {
        uround((0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) * Self::BASE_MASK as f64) as u8
    }

    /// Calculate luminance from Rgba8 (ITU-R BT.709 with integer coefficients).
    pub fn luminance_from_rgba8(c: &Rgba8) -> u8 {
        ((55u32 * c.r as u32 + 184u32 * c.g as u32 + 18u32 * c.b as u32) >> 8) as u8
    }

    pub fn from_rgba(c: &Rgba) -> Self {
        Self {
            v: Self::luminance_from_rgba(c),
            a: uround(c.a * Self::BASE_MASK as f64) as u8,
        }
    }

    pub fn from_rgba8(c: &Rgba8) -> Self {
        Self {
            v: Self::luminance_from_rgba8(c),
            a: c.a,
        }
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    pub fn is_opaque(&self) -> bool {
        self.a == Self::BASE_MASK as u8
    }

    #[inline]
    pub fn multiply(a: u8, b: u8) -> u8 {
        let t: u32 = a as u32 * b as u32 + Self::BASE_MSB;
        (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT) as u8
    }

    #[inline]
    pub fn lerp(p: u8, q: u8, a: u8) -> u8 {
        let t = (q as i32 - p as i32) * a as i32 + Self::BASE_MSB as i32 - (p > q) as i32;
        (p as i32 + (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT)) as u8
    }

    #[inline]
    pub fn mult_cover(a: u8, b: CoverType) -> u8 {
        Self::multiply(a, b)
    }

    pub fn clear(&mut self) -> &mut Self {
        self.v = 0;
        self.a = 0;
        self
    }

    pub fn premultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.v = 0;
            } else {
                self.v = Self::multiply(self.v, self.a);
            }
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.v = 0;
            } else {
                let v_ = (self.v as u32 * Self::BASE_MASK) / self.a as u32;
                self.v = v_.min(Self::BASE_MASK) as u8;
            }
        }
        self
    }

    pub fn gradient(&self, c: &Gray8, k: f64) -> Gray8 {
        let ik = uround(k * Self::BASE_SCALE as f64) as u8;
        Gray8 {
            v: Self::lerp(self.v, c.v, ik),
            a: Self::lerp(self.a, c.a, ik),
        }
    }

    pub fn no_color() -> Self {
        Self { v: 0, a: 0 }
    }
}

impl Default for Gray8 {
    fn default() -> Self {
        Self::no_color()
    }
}

// ============================================================================
// Gray16 (16-bit grayscale)
// ============================================================================

/// Grayscale color with u16 components (value + alpha).
/// Port of C++ `gray16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gray16 {
    pub v: u16,
    pub a: u16,
}

impl Gray16 {
    pub const BASE_SHIFT: u32 = 16;
    pub const BASE_SCALE: u32 = 1 << Self::BASE_SHIFT;
    pub const BASE_MASK: u32 = Self::BASE_SCALE - 1;
    pub const BASE_MSB: u32 = 1 << (Self::BASE_SHIFT - 1);

    pub fn new(v: u32, a: u32) -> Self {
        Self {
            v: v as u16,
            a: a as u16,
        }
    }

    pub fn new_opaque(v: u32) -> Self {
        Self::new(v, Self::BASE_MASK)
    }

    /// Calculate luminance from Rgba (ITU-R BT.709).
    pub fn luminance_from_rgba(c: &Rgba) -> u16 {
        uround((0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) * Self::BASE_MASK as f64) as u16
    }

    /// Calculate luminance from Rgba16 (ITU-R BT.709 with integer coefficients).
    pub fn luminance_from_rgba16(c: &Rgba16) -> u16 {
        ((13933u32 * c.r as u32 + 46872u32 * c.g as u32 + 4732u32 * c.b as u32) >> 16) as u16
    }

    pub fn from_rgba(c: &Rgba) -> Self {
        Self {
            v: Self::luminance_from_rgba(c),
            a: uround(c.a * Self::BASE_MASK as f64) as u16,
        }
    }

    pub fn from_rgba8(c: &Rgba8) -> Self {
        Self::from_rgba16(&Rgba16::from_rgba8(c))
    }

    pub fn from_rgba16(c: &Rgba16) -> Self {
        Self {
            v: Self::luminance_from_rgba16(c),
            a: c.a,
        }
    }

    pub fn from_gray8(c: &Gray8) -> Self {
        Self {
            v: ((c.v as u16) << 8) | c.v as u16,
            a: ((c.a as u16) << 8) | c.a as u16,
        }
    }

    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    pub fn is_opaque(&self) -> bool {
        self.a == Self::BASE_MASK as u16
    }

    #[inline]
    pub fn multiply(a: u16, b: u16) -> u16 {
        let t: u32 = a as u32 * b as u32 + Self::BASE_MSB;
        (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT) as u16
    }

    #[inline]
    pub fn lerp(p: u16, q: u16, a: u16) -> u16 {
        let t = (q as i32 - p as i32) * a as i32 + Self::BASE_MSB as i32 - (p > q) as i32;
        (p as i32 + (((t >> Self::BASE_SHIFT) + t) >> Self::BASE_SHIFT)) as u16
    }

    pub fn clear(&mut self) -> &mut Self {
        self.v = 0;
        self.a = 0;
        self
    }

    pub fn premultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.v = 0;
            } else {
                self.v = Self::multiply(self.v, self.a);
            }
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if (self.a as u32) < Self::BASE_MASK {
            if self.a == 0 {
                self.v = 0;
            } else {
                let v_ = (self.v as u32 * Self::BASE_MASK) / self.a as u32;
                self.v = v_.min(Self::BASE_MASK) as u16;
            }
        }
        self
    }

    pub fn gradient(&self, c: &Gray16, k: f64) -> Gray16 {
        let ik = uround(k * Self::BASE_SCALE as f64) as u16;
        Gray16 {
            v: Self::lerp(self.v, c.v, ik),
            a: Self::lerp(self.a, c.a, ik),
        }
    }

    pub fn no_color() -> Self {
        Self { v: 0, a: 0 }
    }
}

impl Default for Gray16 {
    fn default() -> Self {
        Self::no_color()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_new() {
        let c = Rgba::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(c.r, 0.5);
        assert_eq!(c.g, 0.6);
        assert_eq!(c.b, 0.7);
        assert_eq!(c.a, 0.8);
    }

    #[test]
    fn test_rgba_premultiply_demultiply() {
        let mut c = Rgba::new(1.0, 0.5, 0.25, 0.5);
        c.premultiply();
        assert!((c.r - 0.5).abs() < 1e-10);
        assert!((c.g - 0.25).abs() < 1e-10);
        assert!((c.b - 0.125).abs() < 1e-10);
        assert!((c.a - 0.5).abs() < 1e-10);

        c.demultiply();
        assert!((c.r - 1.0).abs() < 1e-10);
        assert!((c.g - 0.5).abs() < 1e-10);
        assert!((c.b - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_rgba_gradient() {
        let c1 = Rgba::new(0.0, 0.0, 0.0, 1.0);
        let c2 = Rgba::new(1.0, 1.0, 1.0, 1.0);
        let mid = c1.gradient(&c2, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-10);
        assert!((mid.g - 0.5).abs() < 1e-10);
        assert!((mid.b - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_rgba_from_wavelength() {
        let c = Rgba::from_wavelength(550.0, 1.0);
        // 550nm is green-yellow region
        assert!(c.r > 0.0);
        assert!(c.g > 0.0);
        assert!(c.b == 0.0 || c.b < 0.01);
    }

    #[test]
    fn test_rgba_operators() {
        let c1 = Rgba::new(0.1, 0.2, 0.3, 0.4);
        let c2 = Rgba::new(0.2, 0.3, 0.4, 0.5);
        let sum = c1 + c2;
        assert!((sum.r - 0.3).abs() < 1e-10);
        assert!((sum.g - 0.5).abs() < 1e-10);

        let scaled = c1 * 2.0;
        assert!((scaled.r - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_rgba8_new() {
        let c = Rgba8::new(128, 64, 32, 255);
        assert_eq!(c.r, 128);
        assert_eq!(c.g, 64);
        assert_eq!(c.b, 32);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_rgba8_multiply() {
        assert_eq!(Rgba8::multiply(255, 255), 255);
        assert_eq!(Rgba8::multiply(255, 0), 0);
        assert_eq!(Rgba8::multiply(0, 255), 0);
        assert_eq!(Rgba8::multiply(128, 255), 128);
    }

    #[test]
    fn test_rgba8_lerp() {
        assert_eq!(Rgba8::lerp(0, 255, 128), 128);
        assert_eq!(Rgba8::lerp(0, 255, 0), 0);
        assert_eq!(Rgba8::lerp(0, 255, 255), 255);
        assert_eq!(Rgba8::lerp(100, 200, 128), 150);
    }

    #[test]
    fn test_rgba8_premultiply() {
        let mut c = Rgba8::new(255, 128, 64, 128);
        c.premultiply();
        // With alpha=128 (≈0.502), components should be roughly halved
        assert!(c.r > 120 && c.r < 132);
        assert!(c.g > 60 && c.g < 68);
        assert!(c.b > 28 && c.b < 36);
    }

    #[test]
    fn test_rgba8_demultiply() {
        let mut c = Rgba8::new(64, 32, 16, 128);
        c.demultiply();
        // After demultiplying by alpha=128, values should roughly double
        assert!(c.r > 124 && c.r < 132);
        assert!(c.g > 60 && c.g < 68);
    }

    #[test]
    fn test_rgba8_from_rgba_roundtrip() {
        let orig = Rgba::new(0.5, 0.25, 0.75, 1.0);
        let c8 = Rgba8::from_rgba(&orig);
        let back = c8.to_rgba();
        assert!((orig.r - back.r).abs() < 0.01);
        assert!((orig.g - back.g).abs() < 0.01);
        assert!((orig.b - back.b).abs() < 0.01);
    }

    #[test]
    fn test_rgba8_gradient() {
        let c1 = Rgba8::new(0, 0, 0, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        let mid = c1.gradient(&c2, 0.5);
        assert!(mid.r > 125 && mid.r < 130);
        assert!(mid.g > 125 && mid.g < 130);
    }

    #[test]
    fn test_rgba8_packed() {
        let c = rgb8_packed(0xFF8040);
        assert_eq!(c.r, 0xFF);
        assert_eq!(c.g, 0x80);
        assert_eq!(c.b, 0x40);
        assert_eq!(c.a, 255);

        let c = bgr8_packed(0xFF8040);
        assert_eq!(c.r, 0x40);
        assert_eq!(c.g, 0x80);
        assert_eq!(c.b, 0xFF);

        let c = argb8_packed(0x80FF8040);
        assert_eq!(c.a, 0x80);
        assert_eq!(c.r, 0xFF);
        assert_eq!(c.g, 0x80);
        assert_eq!(c.b, 0x40);
    }

    #[test]
    fn test_rgba16_from_rgba8() {
        let c8 = Rgba8::new(128, 64, 32, 255);
        let c16 = Rgba16::from_rgba8(&c8);
        // 128 expanded to 16-bit: (128 << 8) | 128 = 32896
        assert_eq!(c16.r, (128 << 8) | 128);
        assert_eq!(c16.g, (64 << 8) | 64);
    }

    #[test]
    fn test_rgba16_multiply() {
        assert_eq!(Rgba16::multiply(65535, 65535), 65535);
        assert_eq!(Rgba16::multiply(65535, 0), 0);
    }

    #[test]
    fn test_gray8_luminance() {
        let white = Rgba8::new(255, 255, 255, 255);
        let lum = Gray8::luminance_from_rgba8(&white);
        // White should have luminance ≈ 255
        assert!(lum > 250);

        let black = Rgba8::new(0, 0, 0, 255);
        let lum = Gray8::luminance_from_rgba8(&black);
        assert_eq!(lum, 0);
    }

    #[test]
    fn test_gray8_premultiply() {
        let mut g = Gray8::new(200, 128);
        g.premultiply();
        // 200 * 128/255 ≈ 100
        assert!(g.v > 95 && g.v < 105);
    }

    #[test]
    fn test_gray16_from_gray8() {
        let g8 = Gray8::new(128, 255);
        let g16 = Gray16::from_gray8(&g8);
        assert_eq!(g16.v, (128 << 8) | 128);
        assert_eq!(g16.a, (255 << 8) | 255);
    }

    #[test]
    fn test_component_orders() {
        assert_eq!(OrderRgba::R, 0);
        assert_eq!(OrderRgba::G, 1);
        assert_eq!(OrderRgba::B, 2);
        assert_eq!(OrderRgba::A, 3);
        assert_eq!(OrderBgra::B, 0);
        assert_eq!(OrderBgra::G, 1);
        assert_eq!(OrderBgra::R, 2);
        assert_eq!(OrderBgra::A, 3);
    }
}

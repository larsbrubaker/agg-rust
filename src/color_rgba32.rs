//! Floating-point RGBA color.
//!
//! Port of C++ `rgba32` from `agg_color_rgba.h`: four `f32` channels in
//! 0..1, with the arithmetic (`multiply`, `lerp`, `prelerp`, `mult_cover`)
//! the float pixel formats (`pixfmt_rgba32`) blend with. sRGB-encoded 8-bit
//! colors (`srgba8`) convert through `srgb_lut`.

use crate::basics::{uround, CoverType, COVER_MASK};
use crate::color::{Rgba, Rgba16, Rgba8};
use crate::srgb_lut::{alpha_from_srgb, alpha_to_srgb, rgb_from_srgb, rgb_to_srgb};

/// RGBA color with `f32` components. Port of C++ `rgba32`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rgba32 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba32 {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Opaque color (C++ `rgba32(r, g, b)`, alpha defaulting to 1).
    pub fn new_opaque(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// `c` with alpha `a`.
    pub fn with_opacity(c: &Rgba32, a: f32) -> Self {
        Self::new(c.r, c.g, c.b, a)
    }

    pub fn from_rgba(c: &Rgba) -> Self {
        Self::new(c.r as f32, c.g as f32, c.b as f32, c.a as f32)
    }

    /// Linear 8-bit color, each channel divided by 255.
    pub fn from_rgba8(c: &Rgba8) -> Self {
        Self::new(
            (c.r as f64 / 255.0) as f32,
            (c.g as f64 / 255.0) as f32,
            (c.b as f64 / 255.0) as f32,
            (c.a as f64 / 255.0) as f32,
        )
    }

    /// sRGB-encoded 8-bit color (C++ `rgba32(const srgba8&)`): `c`'s red,
    /// green and blue are sRGB bytes, decoded through the sRGB table; alpha is
    /// linear and only rescaled.
    pub fn from_srgba8(c: &Rgba8) -> Self {
        Self::new(
            rgb_from_srgb(c.r),
            rgb_from_srgb(c.g),
            rgb_from_srgb(c.b),
            alpha_from_srgb(c.a),
        )
    }

    pub fn from_rgba16(c: &Rgba16) -> Self {
        Self::new(
            (c.r as f64 / 65535.0) as f32,
            (c.g as f64 / 65535.0) as f32,
            (c.b as f64 / 65535.0) as f32,
            (c.a as f64 / 65535.0) as f32,
        )
    }

    pub fn to_rgba(&self) -> Rgba {
        Rgba::new(self.r as f64, self.g as f64, self.b as f64, self.a as f64)
    }

    /// Linear 8-bit color: each channel `uround(v * 255)`.
    ///
    /// Divergence from C++ AGG: C++ passes `uround(v * 255.0)` to `rgba8`,
    /// whose `int8u` channels keep only the low byte, so a channel above 1
    /// wraps (1.2 becomes 50) and one below 0 is undefined behaviour. Colour
    /// arithmetic clamps here: channels clamp to 0..=255.
    pub fn to_rgba8(&self) -> Rgba8 {
        let q = |v: f32| uround((v as f64 * 255.0).clamp(0.0, 255.0));
        Rgba8::new(q(self.r), q(self.g), q(self.b), q(self.a))
    }

    /// sRGB-encoded 8-bit color (C++ `operator srgba8`): red, green and blue
    /// through the sRGB inverse table (clamped at both ends), alpha
    /// `int8u(0.5 + a * 255)` clamped.
    pub fn to_srgba8(&self) -> Rgba8 {
        Rgba8::new(
            rgb_to_srgb(self.r) as u32,
            rgb_to_srgb(self.g) as u32,
            rgb_to_srgb(self.b) as u32,
            alpha_to_srgb(self.a) as u32,
        )
    }

    /// 16-bit color: each channel `uround(v * 65535)`.
    ///
    /// Divergence from C++ AGG: C++ `operator rgba16` builds an `rgba8` from
    /// the 16-bit values (a typo), truncating every channel to its low byte
    /// before widening back. This returns the 16-bit values, clamped to
    /// 0..=65535 as colour arithmetic clamps.
    pub fn to_rgba16(&self) -> Rgba16 {
        let q = |v: f32| uround((v as f64 * 65535.0).clamp(0.0, 65535.0));
        Rgba16::new(q(self.r), q(self.g), q(self.b), q(self.a))
    }

    pub fn to_double(a: f32) -> f64 {
        a as f64
    }

    pub fn from_double(a: f64) -> f32 {
        a as f32
    }

    pub fn empty_value() -> f32 {
        0.0
    }

    pub fn full_value() -> f32 {
        1.0
    }

    pub fn is_transparent(&self) -> bool {
        self.a <= 0.0
    }

    pub fn is_opaque(&self) -> bool {
        self.a >= 1.0
    }

    pub fn invert(x: f32) -> f32 {
        1.0 - x
    }

    pub fn multiply(a: f32, b: f32) -> f32 {
        a * b
    }

    pub fn demultiply_value(a: f32, b: f32) -> f32 {
        if b == 0.0 {
            0.0
        } else {
            a / b
        }
    }

    /// `a / 2^n` (C++ `downshift`; `downscale` is the identity for floats).
    pub fn downshift(a: f32, n: u32) -> f32 {
        if n > 0 {
            a / (1u32 << n) as f32
        } else {
            a
        }
    }

    /// `a * b / 255` in `f32` (C++ `mult_cover`: float * int / int).
    pub fn mult_cover(a: f32, b: CoverType) -> f32 {
        a * b as f32 / COVER_MASK as f32
    }

    /// `uround(a * b)` as a cover (C++ `scale_cover`), clamped to the cover
    /// range rather than wrapping.
    pub fn scale_cover(a: CoverType, b: f32) -> CoverType {
        uround((a as f32 * b) as f64).min(COVER_MASK) as CoverType
    }

    /// Interpolates `p` to `q` by `a`, assuming `q` is premultiplied by `a`:
    /// `(1 - a) * p + q`, more accurate than `p + q - p * a`.
    pub fn prelerp(p: f32, q: f32, a: f32) -> f32 {
        (1.0 - a) * p + q
    }

    /// Interpolates `p` to `q` by `a`: `(1 - a) * p + a * q`. The form
    /// `p + a * (q - p)` avoids a multiplication but may be inexact; this one
    /// is exact at both extremes.
    pub fn lerp(p: f32, q: f32, a: f32) -> f32 {
        (1.0 - a) * p + a * q
    }

    pub fn clear(&mut self) -> &mut Self {
        *self = Self::new(0.0, 0.0, 0.0, 0.0);
        self
    }

    pub fn transparent(&mut self) -> &mut Self {
        self.a = 0.0;
        self
    }

    /// Sets alpha, clamped to 0..1.
    pub fn set_opacity(&mut self, a: f64) -> &mut Self {
        self.a = if a < 0.0 {
            0.0
        } else if a > 1.0 {
            1.0
        } else {
            a as f32
        };
        self
    }

    pub fn opacity(&self) -> f64 {
        self.a as f64
    }

    pub fn premultiply(&mut self) -> &mut Self {
        if self.a < 1.0 {
            if self.a <= 0.0 {
                self.r = 0.0;
                self.g = 0.0;
                self.b = 0.0;
            } else {
                self.r *= self.a;
                self.g *= self.a;
                self.b *= self.a;
            }
        }
        self
    }

    pub fn demultiply(&mut self) -> &mut Self {
        if self.a < 1.0 {
            if self.a <= 0.0 {
                self.r = 0.0;
                self.g = 0.0;
                self.b = 0.0;
            } else {
                self.r /= self.a;
                self.g /= self.a;
                self.b /= self.a;
            }
        }
        self
    }

    /// The color `k` of the way from `self` to `c` (computed in `f64`).
    pub fn gradient(&self, c: &Rgba32, k: f64) -> Rgba32 {
        let g = |p: f32, q: f32| (p as f64 + (q - p) as f64 * k) as f32;
        Rgba32::new(
            g(self.r, c.r),
            g(self.g, c.g),
            g(self.b, c.b),
            g(self.a, c.a),
        )
    }

    /// Adds premultiplied `c` at `cover`, then clamps alpha to 1 and each
    /// channel to alpha.
    pub fn add(&mut self, c: &Rgba32, cover: u32) {
        if cover == COVER_MASK {
            if c.is_opaque() {
                *self = *c;
                return;
            }
            self.r += c.r;
            self.g += c.g;
            self.b += c.b;
            self.a += c.a;
        } else {
            let cover = cover as CoverType;
            self.r += Self::mult_cover(c.r, cover);
            self.g += Self::mult_cover(c.g, cover);
            self.b += Self::mult_cover(c.b, cover);
            self.a += Self::mult_cover(c.a, cover);
        }
        if self.a > 1.0 {
            self.a = 1.0;
        }
        if self.r > self.a {
            self.r = self.a;
        }
        if self.g > self.a {
            self.g = self.a;
        }
        if self.b > self.a {
            self.b = self.a;
        }
    }

    /// Applies a direct gamma function to red, green and blue (C++
    /// `apply_gamma_dir` with a float gamma LUT's `dir`).
    pub fn apply_gamma_dir(&mut self, dir: impl Fn(f32) -> f32) {
        self.r = dir(self.r);
        self.g = dir(self.g);
        self.b = dir(self.b);
    }

    /// Applies an inverse gamma function to red, green and blue (C++
    /// `apply_gamma_inv` with a float gamma LUT's `inv`).
    pub fn apply_gamma_inv(&mut self, inv: impl Fn(f32) -> f32) {
        self.r = inv(self.r);
        self.g = inv(self.g);
        self.b = inv(self.b);
    }

    pub fn no_color() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn from_wavelength(wl: f64, gamma: f64) -> Self {
        Self::from_rgba(&Rgba::from_wavelength(wl, gamma))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C++ keeps the low byte of `uround(v * 255)`, so 1.2 would come out as 50 and 300 / 255
    /// as 44; channels clamp instead.
    #[test]
    fn to_rgba8_clamps_out_of_range_channels_instead_of_wrapping() {
        let c = Rgba32::new(1.2, -0.5, 0.5, 300.0 / 255.0).to_rgba8();
        assert_eq!((c.r, c.g, c.b, c.a), (255, 0, 128, 255));
    }

    /// C++ `operator rgba16` builds an `rgba8` (keeping each channel's low byte); this returns the
    /// 16-bit values.
    #[test]
    fn to_rgba16_returns_sixteen_bit_channels() {
        let c = Rgba32::new(1.0, 0.5, 0.0, 2.0).to_rgba16();
        assert_eq!((c.r, c.g, c.b, c.a), (65535, 32768, 0, 65535));
    }

    #[test]
    fn scale_cover_clamps_to_the_cover_range() {
        assert_eq!(Rgba32::scale_cover(255, 0.5), 128);
        assert_eq!(Rgba32::scale_cover(200, 2.0), 255);
    }
}

//! sRGB <-> linear lookup tables for floating-point colors.
//!
//! Port of `sRGB_lut<float>`, `sRGB_lut_base` and `sRGB_conv<float>` from
//! `agg_gamma_lut.h`. The tables are built once from `gamma::srgb_to_linear`
//! (C++ `sRGB_to_linear`); `color_rgba32` uses them to convert `srgba8`
//! colors to and from `Rgba32`.

use std::sync::OnceLock;

use crate::gamma::srgb_to_linear;

/// The direct and inverse sRGB tables for `f32` linear values.
///
/// Port of C++ `sRGB_lut<float>`: `dir[i]` is the linear value of sRGB byte
/// `i`, `inv[i]` the linear value halfway below byte `i`, so [`inv`](Self::inv)'s
/// binary search rounds a linear value to the nearest sRGB byte.
pub struct SrgbLutF32 {
    dir_table: [f32; 256],
    inv_table: [f32; 256],
}

impl SrgbLutF32 {
    pub fn new() -> Self {
        let mut dir_table = [0.0f32; 256];
        let mut inv_table = [0.0f32; 256];
        for i in 1..=255usize {
            // Floating-point RGB is in range [0,1].
            dir_table[i] = srgb_to_linear(i as f64 / 255.0) as f32;
            inv_table[i] = srgb_to_linear((i as f64 - 0.5) / 255.0) as f32;
        }
        Self {
            dir_table,
            inv_table,
        }
    }

    /// The linear value of sRGB byte `v`.
    pub fn dir(&self, v: u8) -> f32 {
        self.dir_table[v as usize]
    }

    /// The sRGB byte nearest linear value `v`: C++'s unrolled binary search
    /// over the inverse table, so values below 0 give 0 and above 1 give 255
    /// (a NaN compares false everywhere and also gives 0).
    pub fn inv(&self, v: f32) -> u8 {
        let t = &self.inv_table;
        let mut x = 0usize;
        if v > t[128] {
            x = 128;
        }
        if v > t[x + 64] {
            x += 64;
        }
        if v > t[x + 32] {
            x += 32;
        }
        if v > t[x + 16] {
            x += 16;
        }
        if v > t[x + 8] {
            x += 8;
        }
        if v > t[x + 4] {
            x += 4;
        }
        if v > t[x + 2] {
            x += 2;
        }
        if v > t[x + 1] {
            x += 1;
        }
        x as u8
    }
}

impl Default for SrgbLutF32 {
    fn default() -> Self {
        Self::new()
    }
}

/// The shared table C++ keeps as `sRGB_conv_base<float>::lut`.
fn lut() -> &'static SrgbLutF32 {
    static LUT: OnceLock<SrgbLutF32> = OnceLock::new();
    LUT.get_or_init(SrgbLutF32::new)
}

/// sRGB byte to linear `f32` color channel (C++ `sRGB_conv<float>::rgb_from_sRGB`).
pub fn rgb_from_srgb(x: u8) -> f32 {
    lut().dir(x)
}

/// Linear `f32` color channel to sRGB byte (C++ `sRGB_conv<float>::rgb_to_sRGB`).
pub fn rgb_to_srgb(x: f32) -> u8 {
    lut().inv(x)
}

/// Alpha byte to `f32` (C++ `sRGB_conv<float>::alpha_from_sRGB`: alpha is not
/// sRGB-encoded, only rescaled, as `x * (1 / 255.0)` in double).
pub fn alpha_from_srgb(x: u8) -> f32 {
    const Y: f64 = 1.0 / 255.0;
    (x as f64 * Y) as f32
}

/// `f32` alpha to byte, clamped to 0..=255 (C++ `sRGB_conv<float>::alpha_to_sRGB`).
pub fn alpha_to_srgb(x: f32) -> u8 {
    if x < 0.0 {
        return 0;
    }
    if x > 1.0 {
        return 255;
    }
    // C++ `int8u(0.5 + x * 255)`: float * int is float, then promoted to double for 0.5 +.
    (0.5 + (x * 255.0) as f64) as u8
}

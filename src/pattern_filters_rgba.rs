//! Pattern filters for RGBA pixel formats.
//!
//! Port of `agg_pattern_filters_rgba.h`.
//! Provides nearest-neighbor and bilinear pattern filter implementations
//! for use with line pattern rendering.

use crate::color::Rgba8;

/// Subpixel shift for line coordinate precision (same as line_aa_basics).
pub const LINE_SUBPIXEL_SHIFT: u32 = 8;
pub const LINE_SUBPIXEL_SCALE: i32 = 1 << LINE_SUBPIXEL_SHIFT;
pub const LINE_SUBPIXEL_MASK: i32 = LINE_SUBPIXEL_SCALE - 1;

/// Trait for pattern pixel access.
pub trait PatternFilter {
    /// Number of extra pixels needed on each side of the pattern.
    fn dilation() -> u32;

    /// Get pixel at integer coordinates.
    fn pixel_low_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8;

    /// Get pixel at subpixel coordinates (shifted by LINE_SUBPIXEL_SHIFT).
    fn pixel_high_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8;
}

/// Nearest-neighbor pattern filter.
///
/// Port of C++ `pattern_filter_nn<rgba8>`.
/// Simple point sampling — no interpolation.
pub struct PatternFilterNn;

impl PatternFilter for PatternFilterNn {
    fn dilation() -> u32 {
        0
    }

    #[inline]
    fn pixel_low_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8 {
        buf[y as usize][x as usize]
    }

    #[inline]
    fn pixel_high_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8 {
        Self::pixel_low_res(
            buf,
            x >> LINE_SUBPIXEL_SHIFT,
            y >> LINE_SUBPIXEL_SHIFT,
        )
    }
}

/// Bilinear pattern filter.
///
/// Port of C++ `pattern_filter_bilinear_rgba<rgba8>`.
/// Interpolates between 4 neighboring pixels for smooth pattern rendering.
pub struct PatternFilterBilinearRgba;

impl PatternFilter for PatternFilterBilinearRgba {
    fn dilation() -> u32 {
        1
    }

    #[inline]
    fn pixel_low_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8 {
        buf[y as usize][x as usize]
    }

    #[inline]
    fn pixel_high_res(buf: &[&[Rgba8]], x: i32, y: i32) -> Rgba8 {
        let x_lr = x >> LINE_SUBPIXEL_SHIFT;
        let y_lr = y >> LINE_SUBPIXEL_SHIFT;

        let x_hr = x & LINE_SUBPIXEL_MASK;
        let y_hr = y & LINE_SUBPIXEL_MASK;

        let row0 = &buf[y_lr as usize];
        let row1 = &buf[y_lr as usize + 1];

        let p00 = &row0[x_lr as usize];
        let p01 = &row0[x_lr as usize + 1];
        let p10 = &row1[x_lr as usize];
        let p11 = &row1[x_lr as usize + 1];

        let weight = LINE_SUBPIXEL_SCALE;

        let r = (p00.r as i32 * (weight - x_hr) * (weight - y_hr)
            + p01.r as i32 * x_hr * (weight - y_hr)
            + p10.r as i32 * (weight - x_hr) * y_hr
            + p11.r as i32 * x_hr * y_hr)
            >> (LINE_SUBPIXEL_SHIFT * 2);

        let g = (p00.g as i32 * (weight - x_hr) * (weight - y_hr)
            + p01.g as i32 * x_hr * (weight - y_hr)
            + p10.g as i32 * (weight - x_hr) * y_hr
            + p11.g as i32 * x_hr * y_hr)
            >> (LINE_SUBPIXEL_SHIFT * 2);

        let b = (p00.b as i32 * (weight - x_hr) * (weight - y_hr)
            + p01.b as i32 * x_hr * (weight - y_hr)
            + p10.b as i32 * (weight - x_hr) * y_hr
            + p11.b as i32 * x_hr * y_hr)
            >> (LINE_SUBPIXEL_SHIFT * 2);

        let a = (p00.a as i32 * (weight - x_hr) * (weight - y_hr)
            + p01.a as i32 * x_hr * (weight - y_hr)
            + p10.a as i32 * (weight - x_hr) * y_hr
            + p11.a as i32 * x_hr * y_hr)
            >> (LINE_SUBPIXEL_SHIFT * 2);

        Rgba8::new(r as u32, g as u32, b as u32, a as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(colors: &[Rgba8]) -> Vec<Rgba8> {
        colors.to_vec()
    }

    #[test]
    fn test_nn_low_res() {
        let row0 = make_row(&[Rgba8::new(255, 0, 0, 255), Rgba8::new(0, 255, 0, 255)]);
        let row1 = make_row(&[Rgba8::new(0, 0, 255, 255), Rgba8::new(255, 255, 0, 255)]);
        let buf: Vec<&[Rgba8]> = vec![&row0, &row1];

        let p = PatternFilterNn::pixel_low_res(&buf, 0, 0);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);

        let p = PatternFilterNn::pixel_low_res(&buf, 1, 0);
        assert_eq!(p.r, 0);
        assert_eq!(p.g, 255);

        let p = PatternFilterNn::pixel_low_res(&buf, 0, 1);
        assert_eq!(p.b, 255);
    }

    #[test]
    fn test_nn_high_res() {
        let row0 = make_row(&[Rgba8::new(255, 0, 0, 255), Rgba8::new(0, 255, 0, 255)]);
        let row1 = make_row(&[Rgba8::new(0, 0, 255, 255), Rgba8::new(255, 255, 0, 255)]);
        let buf: Vec<&[Rgba8]> = vec![&row0, &row1];

        // High-res coord (128, 0) → low-res (0, 0) since 128 >> 8 = 0
        let p = PatternFilterNn::pixel_high_res(&buf, 128, 0);
        assert_eq!(p.r, 255);

        // High-res coord (256, 0) → low-res (1, 0)
        let p = PatternFilterNn::pixel_high_res(&buf, 256, 0);
        assert_eq!(p.r, 0);
        assert_eq!(p.g, 255);
    }

    #[test]
    fn test_bilinear_at_integer_coord() {
        let row0 = make_row(&[
            Rgba8::new(255, 0, 0, 255),
            Rgba8::new(0, 255, 0, 255),
            Rgba8::new(0, 0, 0, 255),
        ]);
        let row1 = make_row(&[
            Rgba8::new(0, 0, 255, 255),
            Rgba8::new(255, 255, 0, 255),
            Rgba8::new(0, 0, 0, 255),
        ]);
        let row2 = make_row(&[
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(0, 0, 0, 255),
        ]);
        let buf: Vec<&[Rgba8]> = vec![&row0, &row1, &row2];

        // At exact integer coord (0,0)*256 → should return (255,0,0,255)
        let p = PatternFilterBilinearRgba::pixel_high_res(&buf, 0, 0);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 0);
        assert_eq!(p.b, 0);
    }

    #[test]
    fn test_bilinear_midpoint() {
        // 2x2 pattern: top-left=white, top-right=black, bottom-left=black, bottom-right=black
        let white = Rgba8::new(255, 255, 255, 255);
        let black = Rgba8::new(0, 0, 0, 255);
        let row0 = make_row(&[white, black]);
        let row1 = make_row(&[black, black]);
        let buf: Vec<&[Rgba8]> = vec![&row0, &row1];

        // At midpoint (128, 128) → should blend all 4 corners
        let p = PatternFilterBilinearRgba::pixel_high_res(&buf, 128, 128);
        // weight=(256-128)*(256-128)*255 / 65536 ≈ 64 for top-left only
        assert!(p.r > 50 && p.r < 80, "r={}", p.r);
    }
}

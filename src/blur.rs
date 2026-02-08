//! Stack blur and recursive blur implementations.
//!
//! Port of `agg_blur.h`.
//! Provides fast approximate Gaussian blur using the stack blur algorithm
//! and an IIR recursive blur alternative.

use crate::color::Rgba8;
use crate::rendering_buffer::RowAccessor;

// ============================================================================
// Lookup tables for stack blur (fast division replacement)
// ============================================================================

/// Multiplication factors for fast division in stack blur.
/// Indexed by radius (0..254). Replaces division with multiply+shift.
#[rustfmt::skip]
const STACK_BLUR8_MUL: [u32; 255] = [
    512,512,456,512,328,456,335,512,405,328,271,456,388,335,292,512,
    454,405,364,328,298,271,496,456,420,388,360,335,312,292,273,512,
    482,454,428,405,383,364,345,328,312,298,284,271,259,496,475,456,
    437,420,404,388,374,360,347,335,323,312,302,292,282,273,265,512,
    497,482,468,454,441,428,417,405,394,383,373,364,354,345,337,328,
    320,312,305,298,291,284,278,271,265,259,507,496,485,475,465,456,
    446,437,428,420,412,404,396,388,381,374,367,360,354,347,341,335,
    329,323,318,312,307,302,297,292,287,282,278,273,269,265,261,512,
    505,497,489,482,475,468,461,454,447,441,435,428,422,417,411,405,
    399,394,389,383,378,373,368,364,359,354,350,345,341,337,332,328,
    324,320,316,312,309,305,301,298,294,291,287,284,281,278,274,271,
    268,265,262,259,257,507,501,496,491,485,480,475,470,465,460,456,
    451,446,442,437,433,428,424,420,416,412,408,404,400,396,392,388,
    385,381,377,374,370,367,363,360,357,354,350,347,344,341,338,335,
    332,329,326,323,320,318,315,312,310,307,304,302,299,297,294,292,
    289,287,285,282,280,278,275,273,271,269,267,265,263,261,259,
];

/// Right-shift amounts for fast division in stack blur.
/// Indexed by radius (0..254).
#[rustfmt::skip]
const STACK_BLUR8_SHR: [u32; 255] = [
     9, 11, 12, 13, 13, 14, 14, 15, 15, 15, 15, 16, 16, 16, 16, 17,
    17, 17, 17, 17, 17, 17, 18, 18, 18, 18, 18, 18, 18, 18, 18, 19,
    19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 19, 20, 20, 20,
    20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 21,
    21, 21, 21, 21, 21, 21, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22,
    22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 22, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23, 23,
    23, 23, 23, 23, 23, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
];

// ============================================================================
// Stack blur for RGBA32
// ============================================================================

/// Apply stack blur to an RGBA32 rendering buffer.
///
/// Port of C++ `stack_blur_rgba32<Img>`.
/// Operates in-place on the buffer with independent horizontal and vertical
/// radii. Radius is clamped to 254.
///
/// Component order: R=0, G=1, B=2, A=3.
pub fn stack_blur_rgba32(rbuf: &mut RowAccessor, mut rx: u32, mut ry: u32) {
    let w = rbuf.width() as usize;
    let h = rbuf.height() as usize;
    if w == 0 || h == 0 {
        return;
    }
    let wm = w - 1;
    let hm = h - 1;

    // Horizontal pass
    if rx > 0 {
        if rx > 254 {
            rx = 254;
        }
        let rx = rx as usize;
        let div = rx * 2 + 1;
        let mul_sum = STACK_BLUR8_MUL[rx] as u64;
        let shr_sum = STACK_BLUR8_SHR[rx];

        let mut stack = vec![[0u8; 4]; div];

        for y in 0..h {
            let row = unsafe {
                let ptr = rbuf.row_ptr(y as i32);
                std::slice::from_raw_parts_mut(ptr, w * 4)
            };

            let mut sum_r: u64 = 0;
            let mut sum_g: u64 = 0;
            let mut sum_b: u64 = 0;
            let mut sum_a: u64 = 0;
            let mut sum_in_r: u64 = 0;
            let mut sum_in_g: u64 = 0;
            let mut sum_in_b: u64 = 0;
            let mut sum_in_a: u64 = 0;
            let mut sum_out_r: u64 = 0;
            let mut sum_out_g: u64 = 0;
            let mut sum_out_b: u64 = 0;
            let mut sum_out_a: u64 = 0;

            // Initialize with first pixel (edge extension)
            let src_r = row[0] as u64;
            let src_g = row[1] as u64;
            let src_b = row[2] as u64;
            let src_a = row[3] as u64;

            for i in 0..=rx {
                stack[i] = [row[0], row[1], row[2], row[3]];
                let w = (i + 1) as u64;
                sum_r += src_r * w;
                sum_g += src_g * w;
                sum_b += src_b * w;
                sum_a += src_a * w;
                sum_out_r += src_r;
                sum_out_g += src_g;
                sum_out_b += src_b;
                sum_out_a += src_a;
            }

            let mut src_off = 0usize; // offset into row for source pixel
            for i in 1..=rx {
                if i <= wm {
                    src_off = i * 4;
                }
                stack[i + rx] = [row[src_off], row[src_off + 1], row[src_off + 2], row[src_off + 3]];
                let w = (rx + 1 - i) as u64;
                sum_r += row[src_off] as u64 * w;
                sum_g += row[src_off + 1] as u64 * w;
                sum_b += row[src_off + 2] as u64 * w;
                sum_a += row[src_off + 3] as u64 * w;
                sum_in_r += row[src_off] as u64;
                sum_in_g += row[src_off + 1] as u64;
                sum_in_b += row[src_off + 2] as u64;
                sum_in_a += row[src_off + 3] as u64;
            }

            let mut stack_ptr = rx;
            let mut xp = rx;
            if xp > wm {
                xp = wm;
            }
            src_off = xp * 4;

            for x in 0..w {
                let dst_off = x * 4;
                row[dst_off] = ((sum_r * mul_sum) >> shr_sum) as u8;
                row[dst_off + 1] = ((sum_g * mul_sum) >> shr_sum) as u8;
                row[dst_off + 2] = ((sum_b * mul_sum) >> shr_sum) as u8;
                row[dst_off + 3] = ((sum_a * mul_sum) >> shr_sum) as u8;

                sum_r -= sum_out_r;
                sum_g -= sum_out_g;
                sum_b -= sum_out_b;
                sum_a -= sum_out_a;

                let mut stack_start = stack_ptr + div - rx;
                if stack_start >= div {
                    stack_start -= div;
                }

                let sp = &stack[stack_start];
                sum_out_r -= sp[0] as u64;
                sum_out_g -= sp[1] as u64;
                sum_out_b -= sp[2] as u64;
                sum_out_a -= sp[3] as u64;

                if xp < wm {
                    src_off += 4;
                    xp += 1;
                }

                stack[stack_start] = [row[src_off], row[src_off + 1], row[src_off + 2], row[src_off + 3]];

                sum_in_r += row[src_off] as u64;
                sum_in_g += row[src_off + 1] as u64;
                sum_in_b += row[src_off + 2] as u64;
                sum_in_a += row[src_off + 3] as u64;
                sum_r += sum_in_r;
                sum_g += sum_in_g;
                sum_b += sum_in_b;
                sum_a += sum_in_a;

                stack_ptr += 1;
                if stack_ptr >= div {
                    stack_ptr = 0;
                }

                let sp = &stack[stack_ptr];
                sum_out_r += sp[0] as u64;
                sum_out_g += sp[1] as u64;
                sum_out_b += sp[2] as u64;
                sum_out_a += sp[3] as u64;
                sum_in_r -= sp[0] as u64;
                sum_in_g -= sp[1] as u64;
                sum_in_b -= sp[2] as u64;
                sum_in_a -= sp[3] as u64;
            }
        }
    }

    // Vertical pass
    if ry > 0 {
        if ry > 254 {
            ry = 254;
        }
        let ry = ry as usize;
        let div = ry * 2 + 1;
        let mul_sum = STACK_BLUR8_MUL[ry] as u64;
        let shr_sum = STACK_BLUR8_SHR[ry];

        let mut stack = vec![[0u8; 4]; div];
        let stride = rbuf.stride() as isize;

        for x in 0..w {
            let base_ptr = unsafe { rbuf.row_ptr(0).add(x * 4) };

            let mut sum_r: u64 = 0;
            let mut sum_g: u64 = 0;
            let mut sum_b: u64 = 0;
            let mut sum_a: u64 = 0;
            let mut sum_in_r: u64 = 0;
            let mut sum_in_g: u64 = 0;
            let mut sum_in_b: u64 = 0;
            let mut sum_in_a: u64 = 0;
            let mut sum_out_r: u64 = 0;
            let mut sum_out_g: u64 = 0;
            let mut sum_out_b: u64 = 0;
            let mut sum_out_a: u64 = 0;

            let src_pix = unsafe { std::slice::from_raw_parts(base_ptr, 4) };
            for i in 0..=ry {
                stack[i] = [src_pix[0], src_pix[1], src_pix[2], src_pix[3]];
                let w = (i + 1) as u64;
                sum_r += src_pix[0] as u64 * w;
                sum_g += src_pix[1] as u64 * w;
                sum_b += src_pix[2] as u64 * w;
                sum_a += src_pix[3] as u64 * w;
                sum_out_r += src_pix[0] as u64;
                sum_out_g += src_pix[1] as u64;
                sum_out_b += src_pix[2] as u64;
                sum_out_a += src_pix[3] as u64;
            }

            let mut src_ptr = base_ptr;
            for i in 1..=ry {
                if i <= hm {
                    src_ptr = unsafe { src_ptr.offset(stride) };
                }
                let p = unsafe { std::slice::from_raw_parts(src_ptr, 4) };
                stack[i + ry] = [p[0], p[1], p[2], p[3]];
                let w = (ry + 1 - i) as u64;
                sum_r += p[0] as u64 * w;
                sum_g += p[1] as u64 * w;
                sum_b += p[2] as u64 * w;
                sum_a += p[3] as u64 * w;
                sum_in_r += p[0] as u64;
                sum_in_g += p[1] as u64;
                sum_in_b += p[2] as u64;
                sum_in_a += p[3] as u64;
            }

            let mut stack_ptr = ry;
            let mut yp = ry;
            if yp > hm {
                yp = hm;
            }
            src_ptr = unsafe { base_ptr.offset(yp as isize * stride) };
            let mut dst_ptr = base_ptr;

            for _y in 0..h {
                let dst = unsafe { std::slice::from_raw_parts_mut(dst_ptr, 4) };
                dst[0] = ((sum_r * mul_sum) >> shr_sum) as u8;
                dst[1] = ((sum_g * mul_sum) >> shr_sum) as u8;
                dst[2] = ((sum_b * mul_sum) >> shr_sum) as u8;
                dst[3] = ((sum_a * mul_sum) >> shr_sum) as u8;
                dst_ptr = unsafe { dst_ptr.offset(stride) };

                sum_r -= sum_out_r;
                sum_g -= sum_out_g;
                sum_b -= sum_out_b;
                sum_a -= sum_out_a;

                let mut stack_start = stack_ptr + div - ry;
                if stack_start >= div {
                    stack_start -= div;
                }

                let sp = &stack[stack_start];
                sum_out_r -= sp[0] as u64;
                sum_out_g -= sp[1] as u64;
                sum_out_b -= sp[2] as u64;
                sum_out_a -= sp[3] as u64;

                if yp < hm {
                    src_ptr = unsafe { src_ptr.offset(stride) };
                    yp += 1;
                }

                let p = unsafe { std::slice::from_raw_parts(src_ptr, 4) };
                stack[stack_start] = [p[0], p[1], p[2], p[3]];

                sum_in_r += p[0] as u64;
                sum_in_g += p[1] as u64;
                sum_in_b += p[2] as u64;
                sum_in_a += p[3] as u64;
                sum_r += sum_in_r;
                sum_g += sum_in_g;
                sum_b += sum_in_b;
                sum_a += sum_in_a;

                stack_ptr += 1;
                if stack_ptr >= div {
                    stack_ptr = 0;
                }

                let sp = &stack[stack_ptr];
                sum_out_r += sp[0] as u64;
                sum_out_g += sp[1] as u64;
                sum_out_b += sp[2] as u64;
                sum_out_a += sp[3] as u64;
                sum_in_r -= sp[0] as u64;
                sum_in_g -= sp[1] as u64;
                sum_in_b -= sp[2] as u64;
                sum_in_a -= sp[3] as u64;
            }
        }
    }
}

// ============================================================================
// Recursive blur (IIR Gaussian approximation)
// ============================================================================

/// Recursive blur calculator for RGBA channels.
///
/// Port of C++ `recursive_blur_calc_rgba<double>`.
#[derive(Clone, Copy, Default)]
struct RecursiveBlurCalcRgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl RecursiveBlurCalcRgba {
    fn from_pix(c: &Rgba8) -> Self {
        Self {
            r: c.r as f64,
            g: c.g as f64,
            b: c.b as f64,
            a: c.a as f64,
        }
    }

    fn calc(
        b_coeff: f64,
        b1: f64,
        b2: f64,
        b3: f64,
        c1: &Self,
        c2: &Self,
        c3: &Self,
        c4: &Self,
    ) -> Self {
        Self {
            r: b_coeff * c1.r + b1 * c2.r + b2 * c3.r + b3 * c4.r,
            g: b_coeff * c1.g + b1 * c2.g + b2 * c3.g + b3 * c4.g,
            b: b_coeff * c1.b + b1 * c2.b + b2 * c3.b + b3 * c4.b,
            a: b_coeff * c1.a + b1 * c2.a + b2 * c3.a + b3 * c4.a,
        }
    }

    fn to_pix(&self) -> Rgba8 {
        Rgba8::new(
            self.r as u32,
            self.g as u32,
            self.b as u32,
            self.a as u32,
        )
    }
}

/// Apply recursive (IIR) Gaussian blur to an RGBA32 rendering buffer.
///
/// Port of C++ `recursive_blur<rgba8, recursive_blur_calc_rgba<>>`.
/// Uses Young-van Vliet recursive Gaussian approximation.
/// Operates in-place on the buffer.
pub fn recursive_blur_rgba32(rbuf: &mut RowAccessor, radius: f64) {
    recursive_blur_rgba32_x(rbuf, radius);
    recursive_blur_rgba32_y(rbuf, radius);
}

/// Horizontal recursive blur pass.
pub fn recursive_blur_rgba32_x(rbuf: &mut RowAccessor, radius: f64) {
    if radius < 0.62 {
        return;
    }
    let w = rbuf.width() as usize;
    let h = rbuf.height() as usize;
    if w < 3 {
        return;
    }

    let s = radius * 0.5;
    let q = if s < 2.5 {
        3.97156 - 4.14554 * (1.0 - 0.26891 * s).sqrt()
    } else {
        0.98711 * s - 0.96330
    };

    let q2 = q * q;
    let q3 = q2 * q;

    let b0 = 1.0 / (1.578250 + 2.444130 * q + 1.428100 * q2 + 0.422205 * q3);
    let mut b1 = 2.44413 * q + 2.85619 * q2 + 1.26661 * q3;
    let mut b2 = -1.42810 * q2 - 1.26661 * q3;
    let mut b3 = 0.422205 * q3;
    let b = 1.0 - (b1 + b2 + b3) * b0;

    b1 *= b0;
    b2 *= b0;
    b3 *= b0;

    let wm = w as i32 - 1;

    let mut sum1 = vec![RecursiveBlurCalcRgba::default(); w];
    let mut sum2 = vec![RecursiveBlurCalcRgba::default(); w];
    let mut buf = vec![Rgba8::new(0, 0, 0, 0); w];

    for y in 0..h {
        // Read pixels from row
        let row = unsafe {
            let ptr = rbuf.row_ptr(y as i32);
            std::slice::from_raw_parts(ptr, w * 4)
        };

        let pix = |x: usize| -> Rgba8 {
            let off = x * 4;
            Rgba8::new(
                row[off] as u32,
                row[off + 1] as u32,
                row[off + 2] as u32,
                row[off + 3] as u32,
            )
        };

        // Forward pass
        let c = RecursiveBlurCalcRgba::from_pix(&pix(0));
        sum1[0] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &c, &c, &c);
        let c = RecursiveBlurCalcRgba::from_pix(&pix(1));
        sum1[1] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &sum1[0], &sum1[0], &sum1[0]);
        let c = RecursiveBlurCalcRgba::from_pix(&pix(2));
        sum1[2] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &sum1[1], &sum1[0], &sum1[0]);

        for x in 3..w {
            let c = RecursiveBlurCalcRgba::from_pix(&pix(x));
            sum1[x] = RecursiveBlurCalcRgba::calc(
                b, b1, b2, b3, &c, &sum1[x - 1], &sum1[x - 2], &sum1[x - 3],
            );
        }

        // Backward pass
        let wmi = wm as usize;
        sum2[wmi] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[wmi], &sum1[wmi], &sum1[wmi], &sum1[wmi],
        );
        sum2[wmi - 1] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[wmi - 1], &sum2[wmi], &sum2[wmi], &sum2[wmi],
        );
        sum2[wmi - 2] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[wmi - 2], &sum2[wmi - 1], &sum2[wmi], &sum2[wmi],
        );
        buf[wmi] = sum2[wmi].to_pix();
        buf[wmi - 1] = sum2[wmi - 1].to_pix();
        buf[wmi - 2] = sum2[wmi - 2].to_pix();

        for x in (0..=(wmi as i32 - 3)).rev() {
            let x = x as usize;
            sum2[x] = RecursiveBlurCalcRgba::calc(
                b, b1, b2, b3, &sum1[x], &sum2[x + 1], &sum2[x + 2], &sum2[x + 3],
            );
            buf[x] = sum2[x].to_pix();
        }

        // Write back to row
        let row = unsafe {
            let ptr = rbuf.row_ptr(y as i32);
            std::slice::from_raw_parts_mut(ptr, w * 4)
        };
        for x in 0..w {
            let off = x * 4;
            row[off] = buf[x].r;
            row[off + 1] = buf[x].g;
            row[off + 2] = buf[x].b;
            row[off + 3] = buf[x].a;
        }
    }
}

/// Vertical recursive blur pass.
pub fn recursive_blur_rgba32_y(rbuf: &mut RowAccessor, radius: f64) {
    if radius < 0.62 {
        return;
    }
    let w = rbuf.width() as usize;
    let h = rbuf.height() as usize;
    if h < 3 {
        return;
    }

    let s = radius * 0.5;
    let q = if s < 2.5 {
        3.97156 - 4.14554 * (1.0 - 0.26891 * s).sqrt()
    } else {
        0.98711 * s - 0.96330
    };

    let q2 = q * q;
    let q3 = q2 * q;

    let b0 = 1.0 / (1.578250 + 2.444130 * q + 1.428100 * q2 + 0.422205 * q3);
    let mut b1 = 2.44413 * q + 2.85619 * q2 + 1.26661 * q3;
    let mut b2 = -1.42810 * q2 - 1.26661 * q3;
    let mut b3 = 0.422205 * q3;
    let b = 1.0 - (b1 + b2 + b3) * b0;

    b1 *= b0;
    b2 *= b0;
    b3 *= b0;

    let hm = h as i32 - 1;
    let stride = rbuf.stride() as isize;

    let mut sum1 = vec![RecursiveBlurCalcRgba::default(); h];
    let mut sum2 = vec![RecursiveBlurCalcRgba::default(); h];
    let mut buf = vec![Rgba8::new(0, 0, 0, 0); h];

    for x in 0..w {
        let base_ptr = unsafe { rbuf.row_ptr(0).add(x * 4) };

        let pix = |yi: usize| -> Rgba8 {
            let p = unsafe { std::slice::from_raw_parts(base_ptr.offset(yi as isize * stride), 4) };
            Rgba8::new(p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32)
        };

        // Forward pass
        let c = RecursiveBlurCalcRgba::from_pix(&pix(0));
        sum1[0] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &c, &c, &c);
        let c = RecursiveBlurCalcRgba::from_pix(&pix(1));
        sum1[1] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &sum1[0], &sum1[0], &sum1[0]);
        let c = RecursiveBlurCalcRgba::from_pix(&pix(2));
        sum1[2] = RecursiveBlurCalcRgba::calc(b, b1, b2, b3, &c, &sum1[1], &sum1[0], &sum1[0]);

        for yi in 3..h {
            let c = RecursiveBlurCalcRgba::from_pix(&pix(yi));
            sum1[yi] = RecursiveBlurCalcRgba::calc(
                b, b1, b2, b3, &c, &sum1[yi - 1], &sum1[yi - 2], &sum1[yi - 3],
            );
        }

        // Backward pass
        let hmi = hm as usize;
        sum2[hmi] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[hmi], &sum1[hmi], &sum1[hmi], &sum1[hmi],
        );
        sum2[hmi - 1] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[hmi - 1], &sum2[hmi], &sum2[hmi], &sum2[hmi],
        );
        sum2[hmi - 2] = RecursiveBlurCalcRgba::calc(
            b, b1, b2, b3, &sum1[hmi - 2], &sum2[hmi - 1], &sum2[hmi], &sum2[hmi],
        );
        buf[hmi] = sum2[hmi].to_pix();
        buf[hmi - 1] = sum2[hmi - 1].to_pix();
        buf[hmi - 2] = sum2[hmi - 2].to_pix();

        for yi in (0..=(hmi as i32 - 3)).rev() {
            let yi = yi as usize;
            sum2[yi] = RecursiveBlurCalcRgba::calc(
                b, b1, b2, b3, &sum1[yi], &sum2[yi + 1], &sum2[yi + 2], &sum2[yi + 3],
            );
            buf[yi] = sum2[yi].to_pix();
        }

        // Write back column
        for yi in 0..h {
            let p = unsafe {
                std::slice::from_raw_parts_mut(base_ptr.offset(yi as isize * stride), 4)
            };
            p[0] = buf[yi].r;
            p[1] = buf[yi].g;
            p[2] = buf[yi].b;
            p[3] = buf[yi].a;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering_buffer::RowAccessor;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * 4) as i32;
        let buf = vec![0u8; (h * w * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    fn set_pixel(ra: &mut RowAccessor, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8) {
        let row = unsafe {
            let ptr = ra.row_ptr(y as i32);
            std::slice::from_raw_parts_mut(ptr, (ra.width() as usize) * 4)
        };
        let off = x * 4;
        row[off] = r;
        row[off + 1] = g;
        row[off + 2] = b;
        row[off + 3] = a;
    }

    fn get_pixel(ra: &RowAccessor, x: usize, y: usize) -> [u8; 4] {
        let row = unsafe {
            let ptr = ra.row_ptr(y as i32);
            std::slice::from_raw_parts(ptr, (ra.width() as usize) * 4)
        };
        let off = x * 4;
        [row[off], row[off + 1], row[off + 2], row[off + 3]]
    }

    #[test]
    fn test_stack_blur_zero_radius() {
        let (_buf, mut ra) = make_buffer(10, 10);
        set_pixel(&mut ra, 5, 5, 255, 0, 0, 255);

        let before = get_pixel(&ra, 5, 5);
        stack_blur_rgba32(&mut ra, 0, 0);
        let after = get_pixel(&ra, 5, 5);
        assert_eq!(before, after);
    }

    #[test]
    fn test_stack_blur_spreads_pixel() {
        let (_buf, mut ra) = make_buffer(20, 20);
        // Set center pixel to white
        set_pixel(&mut ra, 10, 10, 255, 255, 255, 255);

        stack_blur_rgba32(&mut ra, 3, 3);

        // Center should still have some value
        let center = get_pixel(&ra, 10, 10);
        assert!(center[0] > 0, "center should have some red after blur");

        // Neighboring pixel should have received some blur
        let neighbor = get_pixel(&ra, 11, 10);
        assert!(neighbor[0] > 0, "neighbor should have some red after blur");
    }

    #[test]
    fn test_stack_blur_uniform_stays_uniform() {
        let (_buf, mut ra) = make_buffer(10, 10);
        // Fill with uniform gray
        for y in 0..10 {
            for x in 0..10 {
                set_pixel(&mut ra, x, y, 128, 128, 128, 255);
            }
        }

        stack_blur_rgba32(&mut ra, 2, 2);

        // All pixels should remain approximately 128
        for y in 0..10 {
            for x in 0..10 {
                let p = get_pixel(&ra, x, y);
                assert!(
                    (p[0] as i32 - 128).abs() <= 1,
                    "pixel ({x},{y}) r={} should be ~128",
                    p[0]
                );
            }
        }
    }

    #[test]
    fn test_recursive_blur_zero_radius() {
        let (_buf, mut ra) = make_buffer(10, 10);
        set_pixel(&mut ra, 5, 5, 255, 0, 0, 255);

        let before = get_pixel(&ra, 5, 5);
        recursive_blur_rgba32(&mut ra, 0.0); // radius < 0.62, no-op
        let after = get_pixel(&ra, 5, 5);
        assert_eq!(before, after);
    }

    #[test]
    fn test_recursive_blur_spreads_pixel() {
        let (_buf, mut ra) = make_buffer(20, 20);
        set_pixel(&mut ra, 10, 10, 255, 255, 255, 255);

        recursive_blur_rgba32(&mut ra, 3.0);

        let center = get_pixel(&ra, 10, 10);
        assert!(center[0] > 0);

        let neighbor = get_pixel(&ra, 11, 10);
        assert!(neighbor[0] > 0, "neighbor should have some value after blur");
    }

    #[test]
    fn test_recursive_blur_uniform_stays_uniform() {
        let (_buf, mut ra) = make_buffer(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                set_pixel(&mut ra, x, y, 100, 100, 100, 255);
            }
        }

        recursive_blur_rgba32(&mut ra, 2.0);

        for y in 0..10 {
            for x in 0..10 {
                let p = get_pixel(&ra, x, y);
                assert!(
                    (p[0] as i32 - 100).abs() <= 2,
                    "pixel ({x},{y}) r={} should be ~100",
                    p[0]
                );
            }
        }
    }
}

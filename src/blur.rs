//! Stack blur, recursive blur and slight blur.
//!
//! Port of `agg_blur.h`. This file holds the RGBA32 stack blur and the RGBA32 recursive-blur
//! wrappers; its submodules hold the byte-channel view the other blurs run on
//! (`channel_view`), `stack_blur_gray8` (`stack_gray`), the recursive blur and its calculators
//! (`recursive`) and `slight_blur` (`slight`).

mod channel_view;
mod recursive;
mod slight;
mod stack_gray;

pub use channel_view::ChannelView;
pub use recursive::{
    recursive_blur_coefficients, RecursiveBlur, RecursiveBlurCalc, RecursiveBlurCalcGray,
    RecursiveBlurCalcRgb, RecursiveBlurCalcRgba,
};
pub use slight::{apply_slight_blur, apply_slight_blur_all, SlightBlur};
pub use stack_gray::stack_blur_gray8;

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
// Whole-row slice helpers
// ============================================================================

/// Row of RGBA32 pixels at `y` as a byte slice (shared, read-only view).
#[cfg(test)]
#[inline]
fn row<'r>(rbuf: &'r RowAccessor, y: i32) -> &'r [u8] {
    rbuf.row_bytes(y, rbuf.width() as usize * 4)
}

/// Row of RGBA32 pixels at `y` as a mutable byte slice.
#[inline]
fn row_mut<'r>(rbuf: &'r mut RowAccessor, y: i32) -> &'r mut [u8] {
    rbuf.row_bytes_mut(y, rbuf.width() as usize * 4)
}

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
pub fn stack_blur_rgba32(rbuf: &mut RowAccessor, mut rx: u32, ry: u32) {
    let w = rbuf.width() as usize;
    let h = rbuf.height() as usize;
    if w == 0 || h == 0 {
        return;
    }
    let wm = w - 1;

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
            let row = row_mut(rbuf, y as i32);

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

            for (i, slot) in stack.iter_mut().take(rx + 1).enumerate() {
                *slot = [row[0], row[1], row[2], row[3]];
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
                stack[i + rx] = [
                    row[src_off],
                    row[src_off + 1],
                    row[src_off + 2],
                    row[src_off + 3],
                ];
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

                stack[stack_start] = [
                    row[src_off],
                    row[src_off + 1],
                    row[src_off + 2],
                    row[src_off + 3],
                ];

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
        stack_blur_rgba32_vertical(rbuf, ry.min(254) as usize);
    }
}

/// Columns the vertical pass sweeps together. Each column's running sums and stack live in
/// arrays indexed by column, so the pass keeps `VERTICAL_STRIP * ((2 * ry + 1) * 4 + 96)`
/// bytes of state (about 12 KB at radius 10, 136 KB at radius 254) and visits each row as
/// `VERTICAL_STRIP * 4` contiguous bytes. Timing on 1024x1024 barely moved between 16 and
/// 256 columns.
const VERTICAL_STRIP: usize = 64;

/// The vertical pass of [`stack_blur_rgba32`], radius `ry` in `1..=254`, on a buffer with at
/// least one row and one column.
///
/// C++ walks each column top to bottom with a pixel pointer stepped by the stride. Columns
/// are independent (each reads and writes only its own pixels), so this runs the same
/// per-column steps with the columns of a strip interleaved: for each row it advances every
/// column of the strip by one step, reading and writing whole row slices. Within a column
/// the order of reads and writes is C++'s, the output row before the row entering the
/// stack, which matters only on the last row, where both are row `h - 1`; the arithmetic is
/// the same integer arithmetic, so the output is identical. Every access is a checked row
/// slice, checked once per row and strip instead of per pixel, and the rows are touched in
/// memory order rather than one pixel per row.
fn stack_blur_rgba32_vertical(rbuf: &mut RowAccessor, ry: usize) {
    let w = rbuf.width() as usize;
    let h = rbuf.height() as usize;
    let hm = h - 1;
    let div = ry * 2 + 1;
    let mul_sum = STACK_BLUR8_MUL[ry] as u64;
    let shr_sum = STACK_BLUR8_SHR[ry];

    // Checks up front, before allocating, that a `w * 4`-byte row fits in the stride (with
    // `rx == 0` the horizontal pass never ran to check it), and panics like every other
    // checked row access when it does not. The length saturates: on a 32-bit `usize` a width
    // up to `i32::MAX` can make `w * 4` wrap to a small number that would pass the check, so
    // an overflowing product must become `usize::MAX`, which fails it.
    let row_len = w.saturating_mul(4);
    rbuf.row_bytes(0, row_len);

    let strip = w.min(VERTICAL_STRIP);
    // Per column: `[sum, sum_in, sum_out]`, each over R, G, B, A.
    let mut sums = vec![[[0u64; 4]; 3]; strip];
    // The column stacks, row-major: slot `i` of column `x` (in the strip) is
    // `stack[i * strip + x]`.
    let mut stack = vec![[0u8; 4]; div * strip];

    let mut x0 = 0;
    while x0 < w {
        let n = (w - x0).min(strip);
        let sums = &mut sums[..n];
        for s in sums.iter_mut() {
            *s = [[0; 4]; 3];
        }

        // Stack slots 0..=ry: the first row, repeated (edge extension).
        for i in 0..=ry {
            let row = &rbuf.row_bytes(0, row_len).as_chunks::<4>().0[x0..x0 + n];
            let weight = (i + 1) as u64;
            let slots = &mut stack[i * strip..][..n];
            for ((slot, s), p) in slots.iter_mut().zip(sums.iter_mut()).zip(row) {
                *slot = *p;
                for c in 0..4 {
                    s[0][c] += p[c] as u64 * weight;
                    s[2][c] += p[c] as u64;
                }
            }
        }

        // Stack slots ry+1..=2*ry: rows 1..=ry, clamped to the last row.
        for i in 1..=ry {
            let row = &rbuf.row_bytes(i.min(hm) as i32, row_len).as_chunks::<4>().0[x0..x0 + n];
            let weight = (ry + 1 - i) as u64;
            let slots = &mut stack[(i + ry) * strip..][..n];
            for ((slot, s), p) in slots.iter_mut().zip(sums.iter_mut()).zip(row) {
                *slot = *p;
                for c in 0..4 {
                    s[0][c] += p[c] as u64 * weight;
                    s[1][c] += p[c] as u64;
                }
            }
        }

        let mut stack_ptr = ry;
        let mut yp = ry.min(hm);
        for y in 0..h {
            let mut stack_start = stack_ptr + div - ry;
            if stack_start >= div {
                stack_start -= div;
            }

            // Write output row `y`, then take the leaving pixels out of the sums.
            {
                let dst =
                    &mut rbuf.row_bytes_mut(y as i32, row_len).as_chunks_mut::<4>().0[x0..x0 + n];
                let leaving = &stack[stack_start * strip..][..n];
                for ((d, s), o) in dst.iter_mut().zip(sums.iter_mut()).zip(leaving) {
                    for c in 0..4 {
                        d[c] = ((s[0][c] * mul_sum) >> shr_sum) as u8;
                    }
                    for c in 0..4 {
                        s[0][c] -= s[2][c];
                        s[2][c] -= o[c] as u64;
                    }
                }
            }

            if yp < hm {
                yp += 1;
            }
            let mut next_ptr = stack_ptr + 1;
            if next_ptr >= div {
                next_ptr = 0;
            }

            // Push row `yp`'s pixels into slot `stack_start`, then move the pixels of slot
            // `next_ptr` from the incoming to the outgoing sums. The two slots differ:
            // `stack_start - next_ptr == ry (mod div)` and `0 < ry < div`.
            let (entering, next) = two_stack_rows(&mut stack, strip, n, stack_start, next_ptr);
            let src = &rbuf.row_bytes(yp as i32, row_len).as_chunks::<4>().0[x0..x0 + n];
            for (((slot, s), p), q) in entering.iter_mut().zip(sums.iter_mut()).zip(src).zip(next) {
                *slot = *p;
                for c in 0..4 {
                    s[1][c] += p[c] as u64;
                    s[0][c] += s[1][c];
                    s[2][c] += q[c] as u64;
                    s[1][c] -= q[c] as u64;
                }
            }

            stack_ptr = next_ptr;
        }

        x0 += n;
    }
}

/// Slot `a` of the strip's stacks, writable, and slot `b`, readable, `n` columns each; `a`
/// and `b` must differ.
#[inline]
fn two_stack_rows(
    stack: &mut [[u8; 4]],
    strip: usize,
    n: usize,
    a: usize,
    b: usize,
) -> (&mut [[u8; 4]], &[[u8; 4]]) {
    debug_assert_ne!(a, b);
    if a < b {
        let (lo, hi) = stack.split_at_mut(b * strip);
        (&mut lo[a * strip..][..n], &hi[..n])
    } else {
        let (lo, hi) = stack.split_at_mut(a * strip);
        (&mut hi[..n], &lo[b * strip..][..n])
    }
}

// ============================================================================
// Recursive blur (IIR Gaussian approximation) on RGBA32
// ============================================================================

/// Apply recursive (IIR) Gaussian blur to an RGBA32 rendering buffer.
///
/// Port of C++ `recursive_blur<rgba8, recursive_blur_calc_rgba<>>`.
/// Uses Young-van Vliet recursive Gaussian approximation.
/// Operates in-place on the buffer.
pub fn recursive_blur_rgba32(rbuf: &mut RowAccessor, radius: f64) {
    RecursiveBlur::<4>::new().blur(&mut ChannelView::rgba32(rbuf), radius);
}

/// Horizontal recursive blur pass.
pub fn recursive_blur_rgba32_x(rbuf: &mut RowAccessor, radius: f64) {
    RecursiveBlur::<4>::new().blur_x(&mut ChannelView::rgba32(rbuf), radius);
}

/// Vertical recursive blur pass.
pub fn recursive_blur_rgba32_y(rbuf: &mut RowAccessor, radius: f64) {
    RecursiveBlur::<4>::new().blur_y(&mut ChannelView::rgba32(rbuf), radius);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering_buffer::RowAccessor;

    fn make_buffer(buf: &mut Vec<u8>, w: u32, h: u32) -> RowAccessor<'_> {
        let stride = (w * 4) as i32;
        *buf = vec![0u8; (h * w * 4) as usize];
        RowAccessor::new_with_buf(buf, w, h, stride)
    }

    fn set_pixel(ra: &mut RowAccessor, x: usize, y: usize, r: u8, g: u8, b: u8, a: u8) {
        let row = row_mut(ra, y as i32);
        let off = x * 4;
        row[off] = r;
        row[off + 1] = g;
        row[off + 2] = b;
        row[off + 3] = a;
    }

    fn get_pixel(ra: &RowAccessor, x: usize, y: usize) -> [u8; 4] {
        let row = row(ra, y as i32);
        let off = x * 4;
        [row[off], row[off + 1], row[off + 2], row[off + 3]]
    }

    #[test]
    fn test_stack_blur_zero_radius() {
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 10, 10);
        set_pixel(&mut ra, 5, 5, 255, 0, 0, 255);

        let before = get_pixel(&ra, 5, 5);
        stack_blur_rgba32(&mut ra, 0, 0);
        let after = get_pixel(&ra, 5, 5);
        assert_eq!(before, after);
    }

    #[test]
    fn test_stack_blur_spreads_pixel() {
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 20, 20);
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
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 10, 10);
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
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 10, 10);
        set_pixel(&mut ra, 5, 5, 255, 0, 0, 255);

        let before = get_pixel(&ra, 5, 5);
        recursive_blur_rgba32(&mut ra, 0.0); // radius < 0.62, no-op
        let after = get_pixel(&ra, 5, 5);
        assert_eq!(before, after);
    }

    #[test]
    fn test_recursive_blur_spreads_pixel() {
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 20, 20);
        set_pixel(&mut ra, 10, 10, 255, 255, 255, 255);

        recursive_blur_rgba32(&mut ra, 3.0);

        let center = get_pixel(&ra, 10, 10);
        assert!(center[0] > 0);

        let neighbor = get_pixel(&ra, 11, 10);
        assert!(
            neighbor[0] > 0,
            "neighbor should have some value after blur"
        );
    }

    #[test]
    fn test_recursive_blur_uniform_stays_uniform() {
        let mut buf = Vec::new();
        let mut ra = make_buffer(&mut buf, 10, 10);
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

    /// The panic message of `f`, or `None` when it returned normally.
    fn panic_message(f: impl FnOnce()) -> Option<String> {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).err()?;
        Some(if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".into()
        })
    }

    /// With `rx == 0` the horizontal pass, which reads rows through the checked `row_mut`, is
    /// skipped, so the vertical pass must check on its own that a `width * 4`-byte row fits
    /// in the stride. Here the stride is half a row, so a row reaches past the attached
    /// region: the check must reject it before any row is touched, so the region still holds
    /// its fill afterwards.
    #[test]
    fn test_stack_blur_vertical_only_rejects_short_stride() {
        let (w, h) = (4u32, 3u32);
        let stride = (w * 4 / 2) as i32;
        vertical_only_panics_without_writing(
            w,
            h,
            stride,
            "row of 16 bytes does not fit in stride 8",
        );
    }

    /// The 32-bit case of the same check: on a 32-bit `usize` (wasm32), `w * 4` for
    /// `w = 1 << 30` wraps to 0, which a plain multiply would pass off as a row that fits in
    /// any stride. The row length must saturate instead, so the check fails. On a 64-bit host
    /// the product does not wrap, so only a 32-bit target exercises the wrap.
    #[test]
    fn test_stack_blur_vertical_only_rejects_row_length_overflow() {
        #[cfg(target_pointer_width = "64")]
        const EXPECT: &str = "row of 4294967296 bytes does not fit in stride 16";
        #[cfg(target_pointer_width = "32")]
        const EXPECT: &str = "row of 4294967295 bytes does not fit in stride 16";
        vertical_only_panics_without_writing(1 << 30, 1, 16, EXPECT);
    }

    /// Attaches a `w` x `h` accessor with `stride` to a region of exactly `h * stride` bytes,
    /// all `FILL`, runs a vertical-only blur, and asserts it panicked with `expect` (the
    /// row-length check's own message) and left every byte of the region holding `FILL`: the
    /// check rejected the row before anything was written. The accessor borrows only that
    /// region, so it is all the blur could have written.
    fn vertical_only_panics_without_writing(w: u32, h: u32, stride: i32, expect: &str) {
        const FILL: u8 = 0x11;
        let mut mem = vec![FILL; h as usize * stride as usize];
        let mut ra = RowAccessor::new_with_buf(&mut mem, w, h, stride);

        let msg = panic_message(|| stack_blur_rgba32(&mut ra, 0, 1));

        let untouched = mem.iter().all(|&b| b == FILL);
        assert_eq!(
            (msg.as_deref(), untouched),
            (Some(expect), true),
            "(panic message, region untouched)"
        );
    }

    /// A negative stride (bottom-up buffer) must blur exactly like the positive-stride buffer
    /// holding the same rows: the vertical pass walks rows by `stride`, so it must not step
    /// past either end of the buffer in either direction. Exact-size buffers, so any row the
    /// blur would reach outside them fails the slice bounds check.
    #[test]
    fn test_stack_blur_negative_stride_matches_positive() {
        for (w, h, rx, ry) in [
            (5u32, 6u32, 0u32, 2u32),
            (5, 6, 2, 3),
            (3, 4, 1, 7),
            (4, 1, 0, 3),
        ] {
            let row_len = (w * 4) as usize;
            let rows: Vec<Vec<u8>> = (0..h as usize)
                .map(|y| {
                    (0..row_len)
                        .map(|i| ((y * 37 + i * 11) % 251) as u8)
                        .collect()
                })
                .collect();

            // Top-down: row y at offset y * row_len.
            let mut top_down: Vec<u8> = rows.concat();
            // Bottom-up: the same rows stored in reverse, attached with a negative stride.
            let mut bottom_up: Vec<u8> = rows.iter().rev().flatten().copied().collect();

            let mut pos = RowAccessor::new_with_buf(&mut top_down, w, h, row_len as i32);
            let mut neg = RowAccessor::new_with_buf(&mut bottom_up, w, h, -(row_len as i32));
            stack_blur_rgba32(&mut pos, rx, ry);
            stack_blur_rgba32(&mut neg, rx, ry);

            for y in 0..h as i32 {
                assert_eq!(
                    row(&pos, y),
                    row(&neg, y),
                    "{w}x{h} rx={rx} ry={ry} row {y}"
                );
            }
        }
    }

    /// The vertical pass sweeps columns in strips of `VERTICAL_STRIP`, reusing the per-column
    /// sums and stacks from strip to strip; the horizontal pass walks one row at a time. Both
    /// are the same 1-D stack blur along their axis (same tables, same edge clamping, same
    /// order of steps, including the in-place write of the last pixel before it is read), so
    /// a vertical-only blur must equal the transpose of a horizontal-only blur of the
    /// transposed image. The width, 130, spans two full strips and a short third one, so a
    /// strip that started from the previous strip's state, or read the wrong columns, would
    /// break the equality. Both stride signs are used for the vertical-pass buffer.
    #[test]
    fn test_stack_blur_vertical_strips_match_transposed_horizontal() {
        const W: usize = 130;
        for (h, r) in [(5usize, 2u32), (5, 9), (4, 254), (1, 3)] {
            // Non-uniform pixels, so every column and channel differs.
            let px =
                |x: usize, y: usize, c: usize| ((x * 29 + y * 71 + c * 53 + x * y) % 256) as u8;
            let row_len = W * 4;

            // The transposed image: width `h`, height `W`, pixel (y, x) = original (x, y).
            let mut t: Vec<u8> = (0..W)
                .flat_map(|x| (0..h).flat_map(move |y| (0..4).map(move |c| px(x, y, c))))
                .collect();
            let mut ta = RowAccessor::new_with_buf(&mut t, h as u32, W as u32, (h * 4) as i32);
            stack_blur_rgba32(&mut ta, r, 0);
            let expected: Vec<Vec<u8>> = (0..h)
                .map(|y| {
                    (0..W)
                        .flat_map(|x| get_pixel(&ta, y, x))
                        .collect::<Vec<u8>>()
                })
                .collect();

            for negative in [false, true] {
                let rows: Vec<Vec<u8>> = (0..h)
                    .map(|y| (0..row_len).map(|i| px(i / 4, y, i % 4)).collect())
                    .collect();
                let mut mem: Vec<u8> = if negative {
                    rows.iter().rev().flatten().copied().collect()
                } else {
                    rows.concat()
                };
                let stride = if negative {
                    -(row_len as i32)
                } else {
                    row_len as i32
                };
                let mut ra = RowAccessor::new_with_buf(&mut mem, W as u32, h as u32, stride);
                stack_blur_rgba32(&mut ra, 0, r);
                for (y, want) in expected.iter().enumerate() {
                    assert_eq!(
                        row(&ra, y as i32),
                        &want[..],
                        "{W}x{h} ry={r} negative stride={negative} row {y}"
                    );
                }
            }
        }
    }
}

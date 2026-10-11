//! Stack blur of one byte per pixel.
//!
//! Port of `stack_blur_gray8` from `agg_blur.h` (agg-sharp `stack_blur.BlurGray8`), run on a
//! [`ChannelView`]'s first channel, so it blurs a gray image or one channel of a colour image.
//! Shares the multiply-and-shift tables with `stack_blur_rgba32` in `blur`.

use super::channel_view::ChannelView;
use super::{STACK_BLUR8_MUL, STACK_BLUR8_SHR};

/// C++ `stack_blur_gray8`: the stack blur of channel 0 of `img`, `rx` across then `ry` down.
/// Radii are capped at 254, where the multiply-and-shift tables end; 0 skips that direction.
pub fn stack_blur_gray8(img: &mut ChannelView, rx: u32, ry: u32) {
    let w = img.width();
    let h = img.height();
    if w == 0 || h == 0 {
        return;
    }

    if rx > 0 {
        let rx = rx.min(254);
        let mut stack = vec![0u32; (rx * 2 + 1) as usize];
        for y in 0..h {
            blur_line(img, w, rx, &mut stack, |x| (x, y));
        }
    }

    if ry > 0 {
        let ry = ry.min(254);
        let mut stack = vec![0u32; (ry * 2 + 1) as usize];
        for x in 0..w {
            blur_line(img, h, ry, &mut stack, |y| (x, y));
        }
    }
}

/// One pass along a line of `len` pixels; `at(i)` is the i'th pixel's position.
fn blur_line(
    img: &mut ChannelView,
    len: u32,
    radius: u32,
    stack: &mut [u32],
    at: impl Fn(u32) -> (u32, u32),
) {
    let get = |img: &ChannelView, i: u32| {
        let (x, y) = at(i);
        img.get(x, y, 0) as u32
    };
    let div = radius * 2 + 1;
    let mul_sum = STACK_BLUR8_MUL[radius as usize];
    let shr_sum = STACK_BLUR8_SHR[radius as usize];
    let last = len - 1;

    // C++ keeps the sums unsigned: the product needs all 32 bits at large radii.
    let mut sum: u32 = 0;
    let mut sum_in: u32 = 0;
    let mut sum_out: u32 = 0;

    let mut pix = get(img, 0);
    for i in 0..=radius {
        stack[i as usize] = pix;
        sum = sum.wrapping_add(pix * (i + 1));
        sum_out += pix;
    }
    for i in 1..=radius {
        // Past the end the last pixel is repeated.
        pix = get(img, i.min(last));
        stack[(i + radius) as usize] = pix;
        sum = sum.wrapping_add(pix * (radius + 1 - i));
        sum_in += pix;
    }

    let mut stack_ptr = radius;
    let mut read = radius.min(last);
    for i in 0..len {
        let (x, y) = at(i);
        img.set(x, y, 0, (sum.wrapping_mul(mul_sum) >> shr_sum) as u8);

        sum = sum.wrapping_sub(sum_out);

        let mut stack_start = stack_ptr + div - radius;
        if stack_start >= div {
            stack_start -= div;
        }
        sum_out = sum_out.wrapping_sub(stack[stack_start as usize]);

        if read < last {
            read += 1;
            pix = get(img, read);
        }

        stack[stack_start as usize] = pix;

        sum_in = sum_in.wrapping_add(pix);
        sum = sum.wrapping_add(sum_in);

        stack_ptr += 1;
        if stack_ptr >= div {
            stack_ptr = 0;
        }
        let stack_pix = stack[stack_ptr as usize];

        sum_out = sum_out.wrapping_add(stack_pix);
        sum_in = sum_in.wrapping_sub(stack_pix);
    }
}

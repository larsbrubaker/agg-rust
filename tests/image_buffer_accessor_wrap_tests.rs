//! Port of agg-sharp `ImageBufferAccessorWrapTests` (Tests/Agg.Tests/Agg).
//!
//! `ImageAccessorWrap` is C++'s `image_accessor_wrap`: reads past any edge
//! fold back into the image through its wrap modes (`wrap_mode_repeat` /
//! `reflect` and their pow2 variants), so a pattern tiles the plane.

use agg_rust::image_accessors::{
    ImageAccessorWrap, WrapMode, WrapModeReflect, WrapModeReflectAutoPow2, WrapModeReflectPow2,
    WrapModeRepeat, WrapModeRepeatAutoPow2, WrapModeRepeatPow2,
};
use agg_rust::rendering_buffer::RowAccessor;

/// Each mode's call operator at `start`, then its increment stepping on.
fn walk<W: WrapMode>(size: u32, start: i32, count: usize) -> Vec<u32> {
    let mut mode = W::new(size);
    let mut values = vec![mode.func(start)];
    while values.len() < count {
        values.push(mode.inc());
    }
    values
}

#[test]
fn repeat_and_reflect_fold_every_coordinate_into_the_image() {
    assert_eq!(walk::<WrapModeRepeat>(3, -4, 6), [2, 0, 1, 2, 0, 1]);
    assert_eq!(walk::<WrapModeRepeatPow2>(4, -4, 6), [0, 1, 2, 3, 0, 1]);
    assert_eq!(walk::<WrapModeRepeatAutoPow2>(3, -4, 6), [2, 0, 1, 2, 0, 1]);
    assert_eq!(walk::<WrapModeRepeatAutoPow2>(4, -4, 6), [0, 1, 2, 3, 0, 1]);
    assert_eq!(walk::<WrapModeReflect>(3, -4, 8), [2, 2, 1, 0, 0, 1, 2, 2]);
    assert_eq!(
        walk::<WrapModeReflectPow2>(4, -4, 10),
        [3, 2, 1, 0, 0, 1, 2, 3, 3, 2]
    );
    assert_eq!(
        walk::<WrapModeReflectAutoPow2>(3, -4, 8),
        [2, 2, 1, 0, 0, 1, 2, 2]
    );
    assert_eq!(
        walk::<WrapModeReflectAutoPow2>(4, -4, 10),
        [3, 2, 1, 0, 0, 1, 2, 3, 3, 2]
    );
}

#[test]
fn spans_read_the_wrapped_pixels() {
    // A 3x2 RGBA image whose red channel is x and green channel is y.
    let (w, h) = (3u32, 2u32);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let at = ((y * w + x) * 4) as usize;
            pixels[at] = x as u8;
            pixels[at + 1] = y as u8;
        }
    }
    let rbuf = unsafe { RowAccessor::new_with_buf(pixels.as_mut_ptr(), w, h, (w * 4) as i32) };
    let mut accessor = ImageAccessorWrap::<4, WrapModeRepeat, WrapModeReflect>::new(&rbuf);

    let p = accessor.span(-1, 2, 3);
    assert_eq!((p[0], p[1]), (2, 1));
    let p = accessor.next_x();
    assert_eq!((p[0], p[1]), (0, 1));

    // The next row down reflects back to row 0 and starts again at the span's first x.
    let p = accessor.next_y();
    assert_eq!((p[0], p[1]), (2, 0));
}

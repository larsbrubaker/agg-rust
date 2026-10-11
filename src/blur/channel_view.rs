//! A byte-channel view of a rendering buffer, the image the blurs below `blur` run on.
//!
//! C++ AGG's blurs take a pixel format; the ones that read raw channels (`slight_blur`,
//! `stack_blur_gray8`, `recursive_blur` with a gray calculator) only need where each pixel's
//! bytes are. A gray view of one channel of an RGBA buffer is C++'s
//! `pixfmt_alpha_blend_gray<blender_gray8, rendering_buffer, 4, Offset>` (step 4, offset 0..3),
//! which `ChannelView::channel` builds; `rgba32`, `rgb24` and `gray8` cover the whole-pixel
//! formats.

use crate::rendering_buffer::RowAccessor;

/// The pixels of a [`RowAccessor`] as `channels` adjacent bytes, `offset` bytes into each pixel,
/// pixels `step` bytes apart.
pub struct ChannelView<'a> {
    rbuf: &'a mut RowAccessor,
    step: usize,
    offset: usize,
    channels: usize,
}

impl<'a> ChannelView<'a> {
    /// A view of `channels` bytes at `offset` in pixels `step` bytes apart.
    ///
    /// # Panics
    /// When `channels` is 0 or the channels reach past the pixel (`offset + channels > step`).
    pub fn new(rbuf: &'a mut RowAccessor, step: usize, offset: usize, channels: usize) -> Self {
        assert!(
            channels > 0 && offset + channels <= step,
            "channels {offset}..{} must lie inside a {step}-byte pixel",
            offset + channels
        );
        Self {
            rbuf,
            step,
            offset,
            channels,
        }
    }

    /// Every byte of 4-byte RGBA pixels (channel 0 is the buffer's first byte, R in RGBA order).
    pub fn rgba32(rbuf: &'a mut RowAccessor) -> Self {
        Self::new(rbuf, 4, 0, 4)
    }

    /// Every byte of 3-byte RGB pixels.
    pub fn rgb24(rbuf: &'a mut RowAccessor) -> Self {
        Self::new(rbuf, 3, 0, 3)
    }

    /// One-byte gray pixels.
    pub fn gray8(rbuf: &'a mut RowAccessor) -> Self {
        Self::new(rbuf, 1, 0, 1)
    }

    /// One channel, `offset` bytes into pixels `step` bytes apart, seen as gray: C++'s
    /// `pixfmt_alpha_blend_gray<..., Step, Offset>`.
    pub fn channel(rbuf: &'a mut RowAccessor, step: usize, offset: usize) -> Self {
        Self::new(rbuf, step, offset, 1)
    }

    pub fn width(&self) -> u32 {
        self.rbuf.width()
    }

    pub fn height(&self) -> u32 {
        self.rbuf.height()
    }

    /// The number of bytes each pixel is seen as.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Row `y`'s bytes. Callers keep `y` inside the buffer; the slice spans whole pixels.
    fn row(&self, y: u32) -> &[u8] {
        assert!(y < self.rbuf.height(), "row {y} outside the buffer");
        // SAFETY: `y` is inside the attached buffer, whose rows are `width * step` bytes long
        // (the caller's buffer holds `step`-byte pixels), and the view borrows the accessor.
        unsafe {
            std::slice::from_raw_parts(
                self.rbuf.row_ptr(y as i32),
                self.rbuf.width() as usize * self.step,
            )
        }
    }

    fn row_mut(&mut self, y: u32) -> &mut [u8] {
        assert!(y < self.rbuf.height(), "row {y} outside the buffer");
        // SAFETY: as in `row`; the view holds the accessor mutably, so no other slice aliases.
        unsafe {
            std::slice::from_raw_parts_mut(
                self.rbuf.row_ptr(y as i32),
                self.rbuf.width() as usize * self.step,
            )
        }
    }

    /// Channel `c` of the pixel at `x`, `y`.
    #[inline]
    pub fn get(&self, x: u32, y: u32, c: usize) -> u8 {
        debug_assert!(c < self.channels);
        self.row(y)[x as usize * self.step + self.offset + c]
    }

    /// Sets channel `c` of the pixel at `x`, `y`.
    #[inline]
    pub fn set(&mut self, x: u32, y: u32, c: usize, v: u8) {
        debug_assert!(c < self.channels);
        let i = x as usize * self.step + self.offset + c;
        self.row_mut(y)[i] = v;
    }
}

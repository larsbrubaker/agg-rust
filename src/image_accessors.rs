//! Image pixel access with boundary handling modes.
//!
//! Port of `agg_image_accessors.h` — provides pixel access to image buffers
//! with different boundary handling: clip (background color), no-clip,
//! clone (clamp to edge), and wrap (tiling).
//!
//! Also includes 6 wrap mode structs for coordinate transformation.

use crate::rendering_buffer::RowAccessor;

// ============================================================================
// ImageSource trait
// ============================================================================

/// Trait for image pixel sources used by span image filters.
///
/// All image accessor types implement this trait, providing a common
/// interface for the span generators to read pixels.
pub trait ImageSource {
    /// Begin reading a span at (x, y) with `len` pixels. Returns first pixel.
    fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8];

    /// Advance to the next pixel in the x direction.
    fn next_x(&mut self) -> &[u8];

    /// Advance to the next row (y+1), resetting x to the span start.
    fn next_y(&mut self) -> &[u8];

    /// The `len` pixels `(x, y)` to `(x + len - 1, y)` as `N`-byte pixels, borrowed from the
    /// image, when the source can lend them as one run; `None` when it cannot. Each is the
    /// pixel `span(x, y, len)` and the `next_x` calls after it would return, so a `Some`
    /// answers a whole row of a filter kernel at once. A `Some` must hold exactly `len`
    /// pixels; the filters treat a row of any other length as not lent. It reads no state and
    /// changes none: after it, `span`, `next_x` and `next_y` go on as if it had not been
    /// called.
    ///
    /// `SpanImageFilterRgba2x2` and `SpanImageFilterRgbaGen` ask it for each row of a kernel
    /// first and read the kernel through `span`, `next_x` and `next_y` on a `None`, so it is
    /// only a fast path: no state to store per pixel, and no slow path in the loop. The
    /// default answers `None`.
    /// `ImageAccessorClip` and `ImageAccessorClone` answer for pixels inside the image when
    /// `N` is their pixel width and a row of the image fits in its stride (their fast path's
    /// conditions); outside the image their pixel is the background or a clamped edge pixel,
    /// which is not a run of the image, so they answer `None` there.
    #[inline]
    fn pixel_row<const N: usize>(&self, x: i32, y: i32, len: u32) -> Option<&[[u8; N]]>
    where
        Self: Sized,
    {
        let _ = (x, y, len);
        None
    }
}

impl<const PIX_WIDTH: usize> ImageSource for ImageAccessorClip<'_, PIX_WIDTH> {
    #[inline]
    fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.span(x, y, len)
    }
    #[inline]
    fn next_x(&mut self) -> &[u8] {
        self.next_x()
    }
    #[inline]
    fn next_y(&mut self) -> &[u8] {
        self.next_y()
    }
    #[inline]
    fn pixel_row<const N: usize>(&self, x: i32, y: i32, len: u32) -> Option<&[[u8; N]]> {
        self.fast.pixels(x, y, len)
    }
}

impl<const PIX_WIDTH: usize> ImageSource for ImageAccessorNoClip<'_, PIX_WIDTH> {
    fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.span(x, y, len)
    }
    fn next_x(&mut self) -> &[u8] {
        self.next_x()
    }
    fn next_y(&mut self) -> &[u8] {
        self.next_y()
    }
}

impl<const PIX_WIDTH: usize> ImageSource for ImageAccessorClone<'_, PIX_WIDTH> {
    #[inline]
    fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.span(x, y, len)
    }
    #[inline]
    fn next_x(&mut self) -> &[u8] {
        self.next_x()
    }
    #[inline]
    fn next_y(&mut self) -> &[u8] {
        self.next_y()
    }
    #[inline]
    fn pixel_row<const N: usize>(&self, x: i32, y: i32, len: u32) -> Option<&[[u8; N]]> {
        self.fast.pixels(x, y, len)
    }
}

impl<const PIX_WIDTH: usize, WX: WrapMode, WY: WrapMode> ImageSource
    for ImageAccessorWrap<'_, PIX_WIDTH, WX, WY>
{
    fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.span(x, y, len)
    }
    fn next_x(&mut self) -> &[u8] {
        self.next_x()
    }
    fn next_y(&mut self) -> &[u8] {
        self.next_y()
    }
}

// ============================================================================
// WrapMode trait
// ============================================================================

/// Coordinate wrapping mode for tiled image access.
pub trait WrapMode {
    /// Create a wrap mode for the given image dimension.
    fn new(size: u32) -> Self;

    /// Map a coordinate to wrapped output, storing internal state.
    fn func(&mut self, v: i32) -> u32;

    /// Increment the internal state, returning the next wrapped coordinate.
    fn inc(&mut self) -> u32;
}

// ============================================================================
// WrapModeRepeat — modulo wrapping
// ============================================================================

/// Repeat (modulo) wrapping for tiling.
///
/// Port of C++ `wrap_mode_repeat`.
pub struct WrapModeRepeat {
    size: u32,
    add: u32,
    value: u32,
}

impl WrapMode for WrapModeRepeat {
    fn new(size: u32) -> Self {
        Self {
            size,
            add: size.wrapping_mul(0x3FFF_FFFF / size),
            value: 0,
        }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        self.value = (v as u32).wrapping_add(self.add) % self.size;
        self.value
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        if self.value >= self.size {
            self.value = 0;
        }
        self.value
    }
}

// ============================================================================
// WrapModeRepeatPow2 — fast bitwise repeat for power-of-2 sizes
// ============================================================================

/// Power-of-2 repeat wrapping using bitwise AND.
///
/// Port of C++ `wrap_mode_repeat_pow2`.
pub struct WrapModeRepeatPow2 {
    mask: u32,
    value: u32,
}

impl WrapMode for WrapModeRepeatPow2 {
    fn new(size: u32) -> Self {
        let mut mask = 1u32;
        while mask < size {
            mask = (mask << 1) | 1;
        }
        mask >>= 1;
        Self { mask, value: 0 }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        self.value = v as u32 & self.mask;
        self.value
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        if self.value > self.mask {
            self.value = 0;
        }
        self.value
    }
}

// ============================================================================
// WrapModeRepeatAutoPow2 — auto-detect pow2 vs modulo
// ============================================================================

/// Auto-detecting repeat: uses bitwise AND for power-of-2 sizes, modulo otherwise.
///
/// Port of C++ `wrap_mode_repeat_auto_pow2`.
pub struct WrapModeRepeatAutoPow2 {
    size: u32,
    add: u32,
    mask: u32,
    value: u32,
}

impl WrapMode for WrapModeRepeatAutoPow2 {
    fn new(size: u32) -> Self {
        let mask = if size & (size - 1) == 0 { size - 1 } else { 0 };
        Self {
            size,
            add: size.wrapping_mul(0x3FFF_FFFF / size),
            mask,
            value: 0,
        }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        if self.mask != 0 {
            self.value = v as u32 & self.mask;
        } else {
            self.value = (v as u32).wrapping_add(self.add) % self.size;
        }
        self.value
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        if self.value >= self.size {
            self.value = 0;
        }
        self.value
    }
}

// ============================================================================
// WrapModeReflect — mirror reflection
// ============================================================================

/// Reflect (mirror) wrapping for tiling.
///
/// Port of C++ `wrap_mode_reflect`.
pub struct WrapModeReflect {
    size: u32,
    size2: u32,
    add: u32,
    value: u32,
}

impl WrapMode for WrapModeReflect {
    fn new(size: u32) -> Self {
        let size2 = size * 2;
        Self {
            size,
            size2,
            add: size2.wrapping_mul(0x3FFF_FFFF / size2),
            value: 0,
        }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        self.value = (v as u32).wrapping_add(self.add) % self.size2;
        if self.value >= self.size {
            self.size2 - self.value - 1
        } else {
            self.value
        }
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        if self.value >= self.size2 {
            self.value = 0;
        }
        if self.value >= self.size {
            self.size2 - self.value - 1
        } else {
            self.value
        }
    }
}

// ============================================================================
// WrapModeReflectPow2 — fast power-of-2 reflection
// ============================================================================

/// Power-of-2 reflect wrapping using bitwise operations.
///
/// Port of C++ `wrap_mode_reflect_pow2`.
pub struct WrapModeReflectPow2 {
    size: u32,
    mask: u32,
    value: u32,
}

impl WrapMode for WrapModeReflectPow2 {
    fn new(size: u32) -> Self {
        let mut mask = 1u32;
        let mut sz = 1u32;
        while mask < size {
            mask = (mask << 1) | 1;
            sz <<= 1;
        }
        Self {
            size: sz,
            mask,
            value: 0,
        }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        self.value = v as u32 & self.mask;
        if self.value >= self.size {
            self.mask - self.value
        } else {
            self.value
        }
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        self.value &= self.mask;
        if self.value >= self.size {
            self.mask - self.value
        } else {
            self.value
        }
    }
}

// ============================================================================
// WrapModeReflectAutoPow2 — auto-detect pow2 vs general reflection
// ============================================================================

/// Auto-detecting reflect: uses bitwise for power-of-2 sizes, modulo otherwise.
///
/// Port of C++ `wrap_mode_reflect_auto_pow2`.
pub struct WrapModeReflectAutoPow2 {
    size: u32,
    size2: u32,
    add: u32,
    mask: u32,
    value: u32,
}

impl WrapMode for WrapModeReflectAutoPow2 {
    fn new(size: u32) -> Self {
        let size2 = size * 2;
        let mask = if size2 & (size2 - 1) == 0 {
            size2 - 1
        } else {
            0
        };
        Self {
            size,
            size2,
            add: size2.wrapping_mul(0x3FFF_FFFF / size2),
            mask,
            value: 0,
        }
    }

    #[inline]
    fn func(&mut self, v: i32) -> u32 {
        self.value = if self.mask != 0 {
            v as u32 & self.mask
        } else {
            (v as u32).wrapping_add(self.add) % self.size2
        };
        if self.value >= self.size {
            self.size2 - self.value - 1
        } else {
            self.value
        }
    }

    #[inline]
    fn inc(&mut self) -> u32 {
        self.value += 1;
        if self.value >= self.size2 {
            self.value = 0;
        }
        if self.value >= self.size {
            self.size2 - self.value - 1
        } else {
            self.value
        }
    }
}

// ============================================================================
// FastRow — the bounded fast path of Clip and Clone
// ============================================================================

mod fast_row {
    use crate::rendering_buffer::RowAccessor;
    use std::slice;

    /// What [`FastRow::next_x`] found.
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(super) enum Step {
        /// The next pixel of the row, still on the fast path.
        Pixel,
        /// The fast path was on the row's last pixel: it has left the fast path, and the
        /// next pixel (x = width) is past the row.
        RowEnded,
        /// The fast path was not active; nothing changed.
        Inactive,
    }

    /// The methods return the pixel beside a flag, `(hit, pixel)` or `(Step, pixel)` with
    /// `pixel` empty on a miss, rather than as `Option<&[u8]>` or a `Step` carrying the
    /// slice: each branch here sets the flag to a constant, which folds away once inlined,
    /// where an `Option` built from the row would make the accessors' `match` test the
    /// pixel pointer for null, which the compiler cannot prove it is not.
    const MISS: &[u8] = &[];

    /// The fast path `ImageAccessorClip` and `ImageAccessorClone` share (C++ `m_pix_ptr`):
    /// a slice iterator over the pixels of the current row after the current one, borrowed
    /// from the image for `'a`. `next_x` is the iterator's `next`, a compare and a pointer
    /// add, instead of reslicing the row for every pixel, and it ends at the end of the row
    /// (Rust only; C++ has no bound), so stepping past the row ends the fast path instead of
    /// reading past it. The accessors differ only in their slow paths.
    ///
    /// The rows come from the checked `RowAccessor::row_bytes`, so every pixel the fast path
    /// returns is one of a row's first `width * PIX_WIDTH` bytes by construction. `enter`
    /// tests the row with `row_fits` first, so `row_bytes`'s own test folds into it and no
    /// panic path is left in the accessors' hot code.
    ///
    /// It also lends runs of pixels without entering them (`pixels`, behind
    /// `ImageSource::pixel_row`), which the filters ask for once per kernel row of every
    /// output pixel. For that `new` computes once what the image does not change while it is
    /// borrowed (the rows that may be lent, the offset of row 0, the signed stride), so a
    /// run is a compare of `y`, a multiply-add for its offset and one checked `get`.
    pub(super) struct FastRow<'a, const PIX_WIDTH: usize> {
        rbuf: &'a RowAccessor<'a>,
        /// The image's attached bytes (`RowAccessor::buf`), for `pixels`.
        bytes: &'a [u8],
        /// The pixels of the current row after the current one; empty when inactive.
        pixels: slice::Iter<'a, [u8; PIX_WIDTH]>,
        /// Offset in `bytes` of row 0 (`RowAccessor::row0_offset`).
        row0: isize,
        /// The signed stride.
        stride: isize,
        /// The rows `pixels` may lend: the height when a `width * PIX_WIDTH`-byte row fits
        /// in the stride, else 0, so `y < fast_rows` tests the row index and the row length
        /// in one compare.
        fast_rows: u32,
        /// The span's first x, where `next_row` enters the next row.
        x0: i32,
        /// Whether the fast path is on a row. With `pixels` empty it tells a row that ended
        /// (`next_x`'s `RowEnded`) from no row at all (`Inactive`).
        active: bool,
    }

    impl<'a, const PIX_WIDTH: usize> FastRow<'a, PIX_WIDTH> {
        /// Inactive, with the row facts `pixels` needs computed once.
        pub(super) fn new(rbuf: &'a RowAccessor<'a>) -> Self {
            // A zero-width pixel has no bytes to read (and `as_chunks::<0>` panics), so no
            // run is lent; the walk returns the same empty slices.
            let row_fits = PIX_WIDTH != 0
                && (rbuf.width() as usize)
                    .checked_mul(PIX_WIDTH)
                    .is_some_and(|len| len <= rbuf.stride_abs() as usize);
            Self {
                rbuf,
                bytes: rbuf.buf(),
                pixels: [].iter(),
                // Both are below `isize::MAX`: the offset is inside the attached bytes and
                // the stride is an `i32`.
                row0: rbuf.row0_offset() as isize,
                stride: rbuf.stride() as isize,
                fast_rows: if row_fits { rbuf.height() } else { 0 },
                x0: 0,
                active: false,
            }
        }

        /// The image. The accessors read it from here rather than keeping their own copy
        /// of the reference: two copies are two loads in the generators' loops.
        #[inline]
        pub(super) fn rbuf(&self) -> &'a RowAccessor<'a> {
            self.rbuf
        }

        /// The `x` of the last `enter_span`, whether or not it entered the fast path.
        #[inline]
        pub(super) fn x0(&self) -> i32 {
            self.x0
        }

        /// Enters the fast path at pixel `x` of row `y` and returns `(true, pixel)` when the
        /// span `x..x + len` lies in a row it may enter; otherwise leaves the fast path and
        /// returns `(false, MISS)`. C++'s test, `y in [0, height)` and
        /// `x >= 0 && x + len <= width`, without its overflow, plus `x < width` so a zero
        /// `len` cannot start one pixel past the row, and the row fitting in the stride.
        /// Width is at most `i32::MAX` (`attach`), so a negative `x` cast to `u32` fails the
        /// `<` test.
        #[inline]
        pub(super) fn enter_span(&mut self, x: i32, y: i32, len: u32) -> (bool, &'a [u8]) {
            self.x0 = x;
            let w = self.rbuf.width();
            if (x as u32) < w && len <= w - x as u32 {
                self.enter(y)
            } else {
                self.leave();
                (false, MISS)
            }
        }

        /// While active, enters row `y` at the span's first pixel and returns `(true, pixel)`
        /// when `y` is a row it may enter; otherwise leaves the fast path and returns
        /// `(false, MISS)`. Inactive, it stays inactive: after a span that was not on the
        /// fast path, or one that walked off its row, the next row is the slow path's too.
        #[inline]
        pub(super) fn next_row(&mut self, y: i32) -> (bool, &'a [u8]) {
            if self.active {
                self.enter(y)
            } else {
                (false, MISS)
            }
        }

        /// Steps to the next pixel of the row; see [`Step`].
        #[inline]
        pub(super) fn next_x(&mut self) -> (Step, &'a [u8]) {
            match self.pixels.next() {
                Some(pixel) => (Step::Pixel, pixel),
                None if self.active => {
                    self.active = false;
                    (Step::RowEnded, MISS)
                }
                None => (Step::Inactive, MISS),
            }
        }

        /// The pixels `x..x + len` of row `y` as `N`-byte pixels (`ImageSource::pixel_row`),
        /// when `N` is the pixel width and the run lies in a row the fast path may enter
        /// (`enter_span`'s test, with `fast_rows` for the row and its fit in the stride);
        /// otherwise `None`. Reads and changes no state. Width and height are at most
        /// `i32::MAX` (`attach`), so a negative `x` or `y` cast to `u32` fails the `<` tests.
        #[inline]
        pub(super) fn pixels<const N: usize>(
            &self,
            x: i32,
            y: i32,
            len: u32,
        ) -> Option<&'a [[u8; N]]> {
            let w = self.rbuf.width();
            // `N != 0` keeps `as_chunks::<0>` (which panics) out; `fast_rows` is 0 for a
            // zero-width pixel anyway.
            if N != PIX_WIDTH
                || N == 0
                || (y as u32) >= self.fast_rows
                || (x as u32) >= w
                || len > w - x as u32
            {
                return None;
            }
            // `0 <= y < height`, so the row starts inside the attached bytes, a whole stride
            // before their end (see `RowAccessor::row_offset`); `x + len <= width` and a row
            // of `width * N` bytes fits in the stride, so the run ends inside the row.
            // Nothing overflows, and the `get` (which keeps the slice checked) always
            // succeeds.
            let start = (self.row0 + y as isize * self.stride) as usize + x as usize * N;
            let bytes = self.bytes.get(start..start + len as usize * N)?;
            Some(bytes.as_chunks::<N>().0)
        }

        /// Leaves the fast path.
        #[inline]
        fn leave(&mut self) {
            self.pixels = [].iter();
            self.active = false;
        }

        /// Enters row `y` at pixel `x0` and returns `(true, pixel)` with that pixel, or
        /// leaves the fast path and returns `(false, MISS)` when `y` is not a row, a
        /// `width * PIX_WIDTH`-byte row does not fit in the stride (the slow path's checked
        /// row access then panics, as the accessors document), or `x0` is not a pixel of
        /// the row.
        #[inline]
        fn enter(&mut self, y: i32) -> (bool, &'a [u8]) {
            // `as_chunks::<0>` panics. A zero-width pixel has no bytes to read, so the slow
            // path returns the same empty slices.
            if PIX_WIDTH != 0 {
                let x = self.x0 as u32 as usize;
                // Saturating, so a product past `usize::MAX` fails the stride test instead
                // of wrapping to a short row.
                let len = (self.rbuf.width() as usize).saturating_mul(PIX_WIDTH);
                if self.rbuf.row_fits(y, len) {
                    let pixels = self.rbuf.row_bytes(y, len).as_chunks::<PIX_WIDTH>().0;
                    if x < pixels.len() {
                        self.pixels = pixels[x + 1..].iter();
                        self.active = true;
                        return (true, &pixels[x]);
                    }
                }
            }
            self.leave();
            (false, MISS)
        }
    }
}

use fast_row::{FastRow, Step};

// ============================================================================
// ImageAccessorClip — returns background color for out-of-bounds
// ============================================================================

/// Image accessor with clipping: returns a background color for out-of-bounds pixels.
///
/// Port of C++ `image_accessor_clip<PixFmt>`. Every pixel it returns is the pixel at (x, y)
/// when that is inside the image and the background colour otherwise, however `span`,
/// `next_x` and `next_y` are called. An empty image (width or height 0) is all background.
///
/// # Panics
/// On reading a pixel inside the image when a `width * PIX_WIDTH`-byte row does not fit in
/// the stride: "row of {len} bytes does not fit in stride {stride}", from the checked row
/// access, as `ImageAccessorClone` does. A pixel outside the image reads no row, so it is
/// the background even then.
///
/// Deliberate divergence from C++: `image_accessor_clip` trusts its caller. Its fast path
/// checks `x + len <= width` once in `span` and then lets `next_x` advance `m_pix_ptr`
/// without limit, so stepping past `len` reads past the row (past the buffer on the last
/// row) where the pixel is outside the image; `span(width, y, 0)` passes the check yet points
/// past the row; `x + len` overflows for a huge `len`; and nothing checks the row against the
/// stride. The span generators do step past `len`: the resample loops
/// (`SpanImageResampleRgbaAffine`, `SpanImageResampleRgba` and their rgb counterparts) step
/// `next_x` until the filter weight runs out, which can be one pixel more than the `len`
/// they pass to `span`, so at the right edge of the image an exact-stride image made the
/// checked row slice here panic and a padded one returned the padding as a pixel. Here the
/// fast path is bounded by the end of the row (`FastRow`): stepping past it falls back to
/// the slow path, which returns the background, the pixel's meaning there. `span` adds
/// `x < width` to C++'s test and computes it without overflow, and the fast path is only
/// taken when a row fits in the stride. Every pixel the C++ fast path returns
/// in range is returned unchanged. Locked in by `tests/image_accessor_bounds_tests.rs`.
pub struct ImageAccessorClip<'a, const PIX_WIDTH: usize> {
    // Background pixel, laid out as the image's pixels are. Sized by the pixel
    // width as C++'s `int8u m_bk_buf[pix_width]` is, so wide formats (a float
    // RGBA pixel is 16 bytes) get their whole background back.
    bk_buf: [u8; PIX_WIDTH],
    x: i32,
    y: i32,
    /// The bounded fast path, shared with `ImageAccessorClone`.
    fast: FastRow<'a, PIX_WIDTH>,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorClip<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor<'a>, bk_color: &[u8]) -> Self {
        let mut bk_buf = [0u8; PIX_WIDTH];
        let len = bk_color.len().min(PIX_WIDTH);
        bk_buf[..len].copy_from_slice(&bk_color[..len]);
        Self {
            bk_buf,
            x: 0,
            y: 0,
            fast: FastRow::new(rbuf),
        }
    }

    pub fn set_background(&mut self, bk_color: &[u8]) {
        let len = bk_color.len().min(PIX_WIDTH);
        self.bk_buf[..len].copy_from_slice(&bk_color[..len]);
    }

    /// The slow path: the pixel at (x, y), or the background outside the image. Width and
    /// height are at most `i32::MAX` (`attach`), so a negative coordinate cast to `u32`
    /// fails the `<` test.
    fn pixel(&self) -> &[u8] {
        let rbuf = self.fast.rbuf();
        if (self.y as u32) < rbuf.height() && (self.x as u32) < rbuf.width() {
            // Saturating, so a product past `usize::MAX` fails the checked row access's
            // stride test instead of wrapping to a short row.
            let row_len = (rbuf.width() as usize).saturating_mul(PIX_WIDTH);
            let row = rbuf.row_bytes(self.y, row_len);
            let off = self.x as usize * PIX_WIDTH;
            &row[off..off + PIX_WIDTH]
        } else {
            &self.bk_buf
        }
    }

    #[inline]
    pub fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.x = x;
        self.y = y;
        let (hit, pixel) = self.fast.enter_span(x, y, len);
        if hit {
            pixel
        } else {
            self.pixel()
        }
    }

    #[inline]
    pub fn next_x(&mut self) -> &[u8] {
        let (step, pixel) = self.fast.next_x();
        match step {
            Step::Pixel => return pixel,
            Step::RowEnded => {
                // Stepped off the row's last pixel (x = width - 1): continue on the slow
                // path at x = width, outside the image, so the background. The resample
                // generators reach this (see the struct docs); C++ reads past the row here.
                self.x = self.fast.rbuf().width() as i32;
            }
            Step::Inactive => {
                // Saturating: at `i32::MAX` the pixel is outside the image either way, where a
                // wrap would jump to the left edge (C++ overflows here).
                self.x = self.x.saturating_add(1);
            }
        }
        // One call of the slow path, after the match, so `next_x` stays small enough to
        // be inlined into the generators' loops.
        self.pixel()
    }

    #[inline]
    pub fn next_y(&mut self) -> &[u8] {
        // Saturating, as in `next_x`: a wrap at `i32::MAX` would jump to the top row.
        self.y = self.y.saturating_add(1);
        self.x = self.fast.x0();
        let (hit, pixel) = self.fast.next_row(self.y);
        if hit {
            pixel
        } else {
            self.pixel()
        }
    }
}

// ============================================================================
// ImageAccessorNoClip — unchecked access
// ============================================================================

/// Image accessor without clipping: it never reads outside the image's rows, and panics on
/// any access outside a row's `stride_abs()` bytes or outside the rows.
///
/// Port of C++ `image_accessor_no_clip<PixFmt>`, which reads whatever memory its unchecked
/// pointer reaches and is meant for sources the caller knows every read stays inside. Here
/// `span`, `next_x` and `next_y` are safe fns, so every access goes through `row_slice` (the
/// row's full `stride_abs()` bytes) and a checked slice of it: a row outside the image panics
/// ("row {y} out of bounds (height=..)", from `span`, or from `next_y` past the bottom), so
/// does a pixel in `span` that does not fit in the stride bytes ("pixel {x} is outside the
/// {len}-byte row"), and so does a `next_x` step past the end of the stride ("range end
/// index .. out of range for slice of length .."). The bound is the stride, not
/// `width * PIX_WIDTH`: in a padded stride, `span(width, y, _)` or a `next_x` step past the
/// row's last pixel returns the padding bytes as a pixel. The accessor gives them no
/// meaning, but never reads outside the image's memory.
///
/// It does not hold the pixels a generator reads to the `len` passed to `span`, and the
/// resample generators (`SpanImageResampleRgbaAffine`, `SpanImageResampleRgba` and their rgb
/// counterparts) step `next_x` one pixel past it at some scales: with them it panics at the
/// right edge of an exact-stride image. Use it only where every pixel the generator reads
/// is inside the image; `ImageAccessorClip` or `ImageAccessorClone` define every pixel.
pub struct ImageAccessorNoClip<'a, const PIX_WIDTH: usize> {
    rbuf: &'a RowAccessor<'a>,
    /// Byte offset of the span's first pixel, where `next_y` starts the next row.
    x_off: usize,
    y: i32,
    pix_off: usize,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorNoClip<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor<'a>) -> Self {
        Self {
            rbuf,
            x_off: 0,
            y: 0,
            pix_off: 0,
        }
    }

    pub fn span(&mut self, x: i32, y: i32, _len: u32) -> &[u8] {
        self.y = y;
        let row = self.rbuf.row_slice(y as u32);
        // Rust only: C++'s `x * pix_width` wraps, and so would `x as usize * PIX_WIDTH`
        // without overflow checks: a negative `x`, or on a 32-bit target an `x` of `2^30`
        // and up, can wrap to an offset inside the row and read some other pixel instead of
        // panicking. `usize::MAX` stands for both, past any row.
        let off = usize::try_from(x)
            .ok()
            .and_then(|x| x.checked_mul(PIX_WIDTH))
            .unwrap_or(usize::MAX);
        if off >= row.len() || row.len() - off < PIX_WIDTH {
            panic!("pixel {x} is outside the {}-byte row", row.len());
        }
        self.x_off = off;
        self.pix_off = off;
        &row[off..off + PIX_WIDTH]
    }

    pub fn next_x(&mut self) -> &[u8] {
        self.pix_off += PIX_WIDTH;
        let row = self.rbuf.row_slice(self.y as u32);
        &row[self.pix_off..self.pix_off + PIX_WIDTH]
    }

    pub fn next_y(&mut self) -> &[u8] {
        self.y += 1;
        self.pix_off = self.x_off;
        let row = self.rbuf.row_slice(self.y as u32);
        &row[self.pix_off..self.pix_off + PIX_WIDTH]
    }
}

// ============================================================================
// ImageAccessorClone — clamp to edge pixels
// ============================================================================

/// Image accessor with clamping: out-of-bounds coordinates snap to edge pixels.
///
/// Port of C++ `image_accessor_clone<PixFmt>`. Every pixel it returns is the pixel at
/// (clamp(x), clamp(y)), however `span`, `next_x` and `next_y` are called.
///
/// # Panics
/// On any access when the image is empty or its rows do not fit in the stride:
/// - height 0: "row 0 out of bounds (height=0)";
/// - width 0 (height > 0): "range end index {PIX_WIDTH} out of range for slice of length 0";
/// - a stride shorter than a `width * PIX_WIDTH`-byte row: "row of {len} bytes does not fit
///   in stride {stride}".
///
/// Deliberate divergence from C++: `image_accessor_clone` trusts its caller. Its fast path
/// checks `x + len <= width` once in `span` and then lets `next_x` advance `m_pix_ptr`
/// without limit, so stepping past `len` reads past the row (past the buffer on the last
/// row); `span(width, y, 0)` passes the check yet points past the row; `x + len` overflows
/// for a huge `len`; and nothing checks the row against the stride. Here those are safe fns,
/// so the fast path is bounded by the end of the row (`FastRow`): stepping past it falls
/// back to the clamping slow path, which returns the same edge pixel C++'s slow path would.
/// `span` adds `x < width` to C++'s test and computes it without overflow, and the fast path
/// is only taken when a row fits in the stride; otherwise the slow path's
/// checked row access panics. Every pixel the C++ fast path returns in range is returned
/// unchanged. Locked in by `tests/image_accessor_bounds_tests.rs`.
pub struct ImageAccessorClone<'a, const PIX_WIDTH: usize> {
    x: i32,
    y: i32,
    /// The bounded fast path, shared with `ImageAccessorClip`.
    fast: FastRow<'a, PIX_WIDTH>,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorClone<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor<'a>) -> Self {
        Self {
            x: 0,
            y: 0,
            fast: FastRow::new(rbuf),
        }
    }

    /// Bytes of one row of pixels. Saturating, so a product past `usize::MAX` fails the
    /// checked row access's stride test instead of wrapping to a short row.
    #[inline]
    fn row_len(&self) -> usize {
        (self.fast.rbuf().width() as usize).saturating_mul(PIX_WIDTH)
    }

    /// The slow path: the pixel at (clamp(x), clamp(y)). Clamps to the far edge first and
    /// then to 0, so an empty image clamps to 0 and fails a defined check (the row check
    /// for height 0, the pixel slice for width 0) instead of computing `-1 as usize`.
    fn pixel(&self) -> &[u8] {
        let cx = self.x.min(self.fast.rbuf().width() as i32 - 1).max(0);
        let cy = self.y.min(self.fast.rbuf().height() as i32 - 1).max(0);
        let row = self.fast.rbuf().row_bytes(cy, self.row_len());
        let off = cx as usize * PIX_WIDTH;
        &row[off..off + PIX_WIDTH]
    }

    #[inline]
    pub fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.x = x;
        self.y = y;
        let (hit, pixel) = self.fast.enter_span(x, y, len);
        if hit {
            pixel
        } else {
            self.pixel()
        }
    }

    #[inline]
    pub fn next_x(&mut self) -> &[u8] {
        let (step, pixel) = self.fast.next_x();
        match step {
            Step::Pixel => return pixel,
            Step::RowEnded => {
                // Stepped off the row's last pixel (x = width - 1): continue on the slow
                // path at x = width, which clamps to that same edge pixel.
                //
                // In-crate generators do reach this: the resample loops
                // (`SpanImageResampleRgbaAffine`, `SpanImageResampleRgba` and their rgb
                // counterparts) step `next_x` until the filter weight runs out, which with
                // `rx_inv` rounded down can be one pixel more than the `len` they pass to
                // `span` (e.g. bilinear at scale 2.5). There C++ advances `m_pix_ptr` past the
                // row and reads the next row's first pixel, or past the buffer on the last
                // row; here it is the clamped edge pixel. Locked in by
                // `resample_past_the_right_edge_keeps_the_meaning` in
                // `tests/image_accessor_bounds_tests.rs`.
                self.x = self.fast.rbuf().width() as i32;
            }
            Step::Inactive => {
                // Saturating: at `i32::MAX` the clamped pixel is the right edge either way,
                // where a wrap would jump to the left edge (C++ overflows here).
                self.x = self.x.saturating_add(1);
            }
        }
        // One call of the slow path, after the match, so `next_x` stays small enough to
        // be inlined into the generators' loops.
        self.pixel()
    }

    #[inline]
    pub fn next_y(&mut self) -> &[u8] {
        // Saturating, as in `next_x`: a wrap at `i32::MAX` would jump to the top row.
        self.y = self.y.saturating_add(1);
        self.x = self.fast.x0();
        let (hit, pixel) = self.fast.next_row(self.y);
        if hit {
            pixel
        } else {
            self.pixel()
        }
    }
}

// ============================================================================
// ImageAccessorWrap — tiling modes
// ============================================================================

/// Image accessor with tiling: wraps coordinates using WrapMode policies.
///
/// Port of C++ `image_accessor_wrap<PixFmt, WrapX, WrapY>`.
pub struct ImageAccessorWrap<'a, const PIX_WIDTH: usize, WX: WrapMode, WY: WrapMode> {
    rbuf: &'a RowAccessor<'a>,
    x: i32,
    wrap_x: WX,
    wrap_y: WY,
    row_y: u32,
}

impl<'a, const PIX_WIDTH: usize, WX: WrapMode, WY: WrapMode>
    ImageAccessorWrap<'a, PIX_WIDTH, WX, WY>
{
    pub fn new(rbuf: &'a RowAccessor<'a>) -> Self {
        Self {
            rbuf,
            x: 0,
            wrap_x: WX::new(rbuf.width()),
            wrap_y: WY::new(rbuf.height()),
            row_y: 0,
        }
    }

    pub fn span(&mut self, x: i32, y: i32, _len: u32) -> &[u8] {
        self.x = x;
        self.row_y = self.wrap_y.func(y);
        let wx = self.wrap_x.func(x) as usize * PIX_WIDTH;
        let row = self.rbuf.row_slice(self.row_y);
        &row[wx..wx + PIX_WIDTH]
    }

    pub fn next_x(&mut self) -> &[u8] {
        let wx = self.wrap_x.inc() as usize * PIX_WIDTH;
        let row = self.rbuf.row_slice(self.row_y);
        &row[wx..wx + PIX_WIDTH]
    }

    pub fn next_y(&mut self) -> &[u8] {
        self.row_y = self.wrap_y.inc();
        let wx = self.wrap_x.func(self.x) as usize * PIX_WIDTH;
        let row = self.rbuf.row_slice(self.row_y);
        &row[wx..wx + PIX_WIDTH]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rgba_buffer(width: u32, height: u32, data: &mut Vec<u8>) -> RowAccessor<'_> {
        let stride = width as usize * 4;
        data.resize(stride * height as usize, 0);
        RowAccessor::new_with_buf(data, width, height, stride as i32)
    }

    fn set_rgba_pixel(data: &mut [u8], width: u32, x: u32, y: u32, rgba: [u8; 4]) {
        let off = (y * width * 4 + x * 4) as usize;
        data[off..off + 4].copy_from_slice(&rgba);
    }

    // -- WrapMode tests --

    #[test]
    fn test_wrap_repeat() {
        let mut w = WrapModeRepeat::new(4);
        assert_eq!(w.func(0), 0);
        assert_eq!(w.func(3), 3);
        assert_eq!(w.func(4), 0);
        assert_eq!(w.func(5), 1);
        assert_eq!(w.func(-1), 3);
    }

    #[test]
    fn test_wrap_repeat_inc() {
        let mut w = WrapModeRepeat::new(3);
        w.func(0);
        assert_eq!(w.inc(), 1);
        assert_eq!(w.inc(), 2);
        assert_eq!(w.inc(), 0); // wraps
    }

    #[test]
    fn test_wrap_repeat_pow2() {
        let mut w = WrapModeRepeatPow2::new(4); // mask=3
        assert_eq!(w.func(0), 0);
        assert_eq!(w.func(3), 3);
        assert_eq!(w.func(4), 0);
        assert_eq!(w.func(7), 3);
    }

    #[test]
    fn test_wrap_reflect() {
        let mut w = WrapModeReflect::new(4); // size=4, size2=8
        assert_eq!(w.func(0), 0);
        assert_eq!(w.func(3), 3);
        assert_eq!(w.func(4), 3); // reflected
        assert_eq!(w.func(7), 0); // reflected
    }

    #[test]
    fn test_wrap_reflect_inc() {
        let mut w = WrapModeReflect::new(3); // size=3, size2=6
        w.func(0);
        assert_eq!(w.inc(), 1);
        assert_eq!(w.inc(), 2);
        assert_eq!(w.inc(), 2); // reflected: 6-3-1=2
        assert_eq!(w.inc(), 1); // 6-4-1=1
        assert_eq!(w.inc(), 0); // 6-5-1=0
        assert_eq!(w.inc(), 0); // wraps to 0
    }

    #[test]
    fn test_wrap_repeat_auto_pow2() {
        // Power-of-2 size: should use mask
        let mut w = WrapModeRepeatAutoPow2::new(8);
        assert_eq!(w.func(8), 0);
        assert_eq!(w.func(9), 1);

        // Non-power-of-2: should use modulo
        let mut w2 = WrapModeRepeatAutoPow2::new(5);
        assert_eq!(w2.func(5), 0);
        assert_eq!(w2.func(7), 2);
    }

    #[test]
    fn test_wrap_reflect_pow2() {
        let mut w = WrapModeReflectPow2::new(4);
        assert_eq!(w.func(0), 0);
        assert_eq!(w.func(3), 3);
        assert_eq!(w.func(4), 3); // reflected
        assert_eq!(w.func(7), 0); // reflected
    }

    // -- ImageAccessorClip tests --

    #[test]
    fn test_clip_in_bounds() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 1, 1, [10, 20, 30, 40]);

        let mut acc = ImageAccessorClip::<4>::new(&rbuf, &[0, 0, 0, 0]);
        let pix = acc.span(1, 1, 1);
        assert_eq!(&pix[..4], &[10, 20, 30, 40]);
    }

    #[test]
    fn test_clip_out_of_bounds() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);

        let mut acc = ImageAccessorClip::<4>::new(&rbuf, &[99, 88, 77, 66]);
        let pix = acc.span(-1, 0, 1);
        assert_eq!(&pix[..4], &[99, 88, 77, 66]);
    }

    #[test]
    fn test_clip_span_fast_path() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 0, 0, [1, 2, 3, 4]);
        set_rgba_pixel(rbuf.buf_mut(), 4, 1, 0, [5, 6, 7, 8]);

        let mut acc = ImageAccessorClip::<4>::new(&rbuf, &[0, 0, 0, 0]);
        let pix = acc.span(0, 0, 2);
        assert_eq!(&pix[..4], &[1, 2, 3, 4]);
        let pix = acc.next_x();
        assert_eq!(&pix[..4], &[5, 6, 7, 8]);
    }

    // -- ImageAccessorNoClip tests --

    #[test]
    fn test_no_clip_span() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 2, 1, [11, 22, 33, 44]);

        let mut acc = ImageAccessorNoClip::<4>::new(&rbuf);
        let pix = acc.span(2, 1, 1);
        assert_eq!(&pix[..4], &[11, 22, 33, 44]);
    }

    #[test]
    fn test_no_clip_next_x() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 4, 1, 0, [20, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 4, 2, 0, [30, 0, 0, 0]);

        let mut acc = ImageAccessorNoClip::<4>::new(&rbuf);
        acc.span(0, 0, 3);
        let p1 = acc.next_x();
        assert_eq!(p1[0], 20);
        let p2 = acc.next_x();
        assert_eq!(p2[0], 30);
    }

    // -- ImageAccessorClone tests --

    #[test]
    fn test_clone_in_bounds() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 1, 1, [50, 60, 70, 80]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(1, 1, 1);
        assert_eq!(&pix[..4], &[50, 60, 70, 80]);
    }

    #[test]
    fn test_clone_clamps_negative() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 0, 0, [10, 20, 30, 40]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(-5, -3, 1);
        // Should clamp to (0, 0)
        assert_eq!(&pix[..4], &[10, 20, 30, 40]);
    }

    #[test]
    fn test_clone_clamps_overflow() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 4, 3, 3, [99, 88, 77, 66]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(100, 100, 1);
        // Should clamp to (3, 3)
        assert_eq!(&pix[..4], &[99, 88, 77, 66]);
    }

    #[test]
    fn test_clone_fast_path_border_walk() {
        // Exercise the fast path (span/next_x/next_y) at the exact
        // right/bottom border and off the edge, asserting each fetch matches
        // the clamping `pixel()` path. The other clone tests only use len=1
        // and never advance along the fast path, so this guards its row
        // iterator's entry and end.
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(4, 4, &mut data);
        for y in 0..4u32 {
            for x in 0..4u32 {
                set_rgba_pixel(
                    rbuf.buf_mut(),
                    4,
                    x,
                    y,
                    [x as u8, y as u8, (x * 4 + y) as u8, 255],
                );
            }
        }

        // Oracle: fetch the clamped pixel at (x, y) via the production slow
        // path. A huge `len` fails the fast-path bounds check, forcing span()
        // into the clamping `pixel()` branch, so this reuses the real clamping
        // logic rather than duplicating it.
        let clamp_pixel = |x: i32, y: i32| -> [u8; 4] {
            let mut oracle = ImageAccessorClone::<4>::new(&rbuf);
            let p = oracle.span(x, y, 1_000_000);
            [p[0], p[1], p[2], p[3]]
        };

        // Fast-path span flush against the right border: x + len == width
        // (2 + 2 == 4) and y == height - 1 (3). span() takes the fast path.
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let p = acc.span(2, 3, 2);
        assert_eq!(&p[..4], &clamp_pixel(2, 3), "span at right/bottom border");

        // Fast-path next_x advance to the last valid column (3, 3).
        let p = acc.next_x();
        assert_eq!(&p[..4], &clamp_pixel(3, 3), "next_x fast-path advance");

        // next_y steps off the bottom edge (y -> 4): must drop off the fast
        // path and clamp, returning x0 = 2 clamped to (2, 3).
        let p = acc.next_y();
        assert_eq!(&p[..4], &clamp_pixel(2, 4), "next_y off bottom edge clamps");

        // next_x after leaving the fast path advances logical x and clamps
        // (3, 4) -> (3, 3).
        let p = acc.next_x();
        assert_eq!(&p[..4], &clamp_pixel(3, 4), "next_x after clamp fallback");

        // Separate fully in-bounds walk exercising several consecutive
        // fast-path next_x advances across a row.
        let mut acc2 = ImageAccessorClone::<4>::new(&rbuf);
        let p = acc2.span(0, 1, 4);
        assert_eq!(&p[..4], &clamp_pixel(0, 1));
        for x in 1..4 {
            let p = acc2.next_x();
            assert_eq!(&p[..4], &clamp_pixel(x, 1), "in-bounds fast-path next_x");
        }
    }

    // -- ImageAccessorWrap tests --

    #[test]
    fn test_wrap_repeat_access() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(2, 2, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 2, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 2, 1, 0, [20, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 2, 0, 1, [30, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 2, 1, 1, [40, 0, 0, 0]);

        let mut acc = ImageAccessorWrap::<4, WrapModeRepeat, WrapModeRepeat>::new(&rbuf);
        // x=2, y=0 should wrap to x=0, y=0
        let pix = acc.span(2, 0, 1);
        assert_eq!(pix[0], 10);
        // next_x → x=3 wraps to x=1
        let pix = acc.next_x();
        assert_eq!(pix[0], 20);
    }

    #[test]
    fn test_wrap_next_y() {
        let mut data = Vec::new();
        let mut rbuf = make_rgba_buffer(2, 2, &mut data);
        set_rgba_pixel(rbuf.buf_mut(), 2, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(rbuf.buf_mut(), 2, 0, 1, [30, 0, 0, 0]);

        let mut acc = ImageAccessorWrap::<4, WrapModeRepeat, WrapModeRepeat>::new(&rbuf);
        acc.span(0, 0, 1);
        let pix = acc.next_y();
        assert_eq!(pix[0], 30);
    }

    // -- pixel_row: the fast path's lent runs --

    /// The bytes of `height` rows of `stride` (negative: row 0 last in memory), every one
    /// distinct mod 256 within a few rows.
    fn patterned(height: u32, stride: i32) -> Vec<u8> {
        let len = height as usize * stride.unsigned_abs() as usize;
        (0..len).map(|i| (i * 7 + 3) as u8).collect()
    }

    /// The pixels `span(x, y, len)` and `len - 1` `next_x` calls return.
    fn walk<S: ImageSource>(acc: &mut S, x: i32, y: i32, len: u32) -> Vec<Vec<u8>> {
        let mut out = vec![acc.span(x, y, len).to_vec()];
        for _ in 1..len {
            out.push(acc.next_x().to_vec());
        }
        out
    }

    /// Checks `pixel_row` on every run around the image: `Some` exactly for runs inside
    /// the image when a row fits in the stride, holding the image's own pixels, which are
    /// what `span` and `next_x` return there.
    fn check_pixel_row(width: u32, height: u32, stride: i32) {
        let mut data = patterned(height, stride);
        let ra = RowAccessor::new_with_buf(&mut data, width, height, stride);
        let fits = width as usize * 4 <= stride.unsigned_abs() as usize;
        let mut clip = ImageAccessorClip::<4>::new(&ra, &[1, 2, 3, 4]);
        let mut clone = ImageAccessorClone::<4>::new(&ra);
        for y in -2..height as i32 + 2 {
            for x in -2..width as i32 + 2 {
                for len in 0..width + 3 {
                    let inside = fits
                        && (0..height as i32).contains(&y)
                        && (0..width as i32).contains(&x)
                        && x as u32 + len <= width;
                    let lent = [
                        clip.pixel_row::<4>(x, y, len),
                        clone.pixel_row::<4>(x, y, len),
                    ];
                    for row in lent {
                        assert_eq!(row.is_some(), inside, "({x}, {y}) len {len}");
                        let Some(row) = row else { continue };
                        assert_eq!(row.len(), len as usize);
                        let image = &ra.row_slice(y as u32)[x as usize * 4..];
                        assert_eq!(row.as_flattened(), &image[..len as usize * 4]);
                    }
                    if inside && len > 0 {
                        let row = clip.pixel_row::<4>(x, y, len).unwrap();
                        let pixels: Vec<Vec<u8>> = row.iter().map(|p| p.to_vec()).collect();
                        assert_eq!(walk(&mut clip, x, y, len), pixels);
                        assert_eq!(walk(&mut clone, x, y, len), pixels);
                    }
                }
            }
        }
        // Another pixel width is not a run of these pixels.
        assert!(clip.pixel_row::<3>(0, 0, 1).is_none());
        assert!(clone.pixel_row::<8>(0, 0, 1).is_none());
    }

    #[test]
    fn pixel_row_lends_the_image_pixels_inside_and_nothing_else() {
        check_pixel_row(5, 4, 20);
        check_pixel_row(5, 4, 23);
        check_pixel_row(5, 4, -24);
        // A row does not fit in the stride: nothing is lent (the walk would panic).
        check_pixel_row(5, 4, 16);
        check_pixel_row(0, 3, 8);
        check_pixel_row(4, 0, 16);
    }

    #[test]
    fn pixel_row_leaves_the_walk_unchanged() {
        let mut data = patterned(5, 24);
        let ra = RowAccessor::new_with_buf(&mut data, 6, 5, 24);
        let pixel = |x: usize, y: u32| ra.row_slice(y)[x * 4..x * 4 + 4].to_vec();
        let mut clip = ImageAccessorClip::<4>::new(&ra, &[0; 4]);
        let mut clone = ImageAccessorClone::<4>::new(&ra);

        assert_eq!(clip.span(1, 1, 3), pixel(1, 1));
        assert_eq!(clone.span(1, 1, 3), pixel(1, 1));
        assert!(clip.pixel_row::<4>(0, 3, 4).is_some());
        assert!(clone.pixel_row::<4>(0, 3, 4).is_some());
        assert_eq!(clip.next_x(), pixel(2, 1));
        assert_eq!(clone.next_x(), pixel(2, 1));
        assert!(clip.pixel_row::<4>(4, 0, 2).is_some());
        assert!(clone.pixel_row::<4>(4, 0, 2).is_some());
        assert_eq!(clip.next_y(), pixel(1, 2));
        assert_eq!(clone.next_y(), pixel(1, 2));
        assert_eq!(clip.next_x(), pixel(2, 2));
        assert_eq!(clone.next_x(), pixel(2, 2));
    }

    #[test]
    fn pixel_row_lends_nothing_by_default() {
        let mut data = patterned(4, 16);
        let ra = RowAccessor::new_with_buf(&mut data, 4, 4, 16);
        assert!(ImageAccessorNoClip::<4>::new(&ra)
            .pixel_row::<4>(0, 0, 2)
            .is_none());
        assert!(
            ImageAccessorWrap::<4, WrapModeRepeat, WrapModeRepeat>::new(&ra)
                .pixel_row::<4>(0, 0, 2)
                .is_none()
        );
    }
}

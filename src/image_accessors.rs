//! Image pixel access with boundary handling modes.
//!
//! Port of `agg_image_accessors.h` — provides pixel access to image buffers
//! with different boundary handling: clip (background color), no-clip,
//! clone (clamp to edge), and wrap (tiling).
//!
//! Also includes 6 wrap mode structs for coordinate transformation.

use crate::rendering_buffer::RowAccessor;

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
// ImageAccessorClip — returns background color for out-of-bounds
// ============================================================================

/// Image accessor with clipping: returns a background color for out-of-bounds pixels.
///
/// Port of C++ `image_accessor_clip<PixFmt>`.
pub struct ImageAccessorClip<'a, const PIX_WIDTH: usize> {
    rbuf: &'a RowAccessor,
    bk_buf: [u8; 8], // background color buffer (max 8 bytes per pixel)
    x: i32,
    x0: i32,
    y: i32,
    fast_path: bool,
    pix_off: usize,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorClip<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor, bk_color: &[u8]) -> Self {
        let mut bk_buf = [0u8; 8];
        let len = bk_color.len().min(8);
        bk_buf[..len].copy_from_slice(&bk_color[..len]);
        Self {
            rbuf,
            bk_buf,
            x: 0,
            x0: 0,
            y: 0,
            fast_path: false,
            pix_off: 0,
        }
    }

    pub fn set_background(&mut self, bk_color: &[u8]) {
        let len = bk_color.len().min(8);
        self.bk_buf[..len].copy_from_slice(&bk_color[..len]);
    }

    fn pixel(&self) -> &[u8] {
        if self.y >= 0
            && self.y < self.rbuf.height() as i32
            && self.x >= 0
            && self.x < self.rbuf.width() as i32
        {
            let row = self.rbuf.row_slice(self.y as u32);
            let off = self.x as usize * PIX_WIDTH;
            &row[off..off + PIX_WIDTH]
        } else {
            &self.bk_buf[..PIX_WIDTH]
        }
    }

    pub fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.x = x;
        self.x0 = x;
        self.y = y;
        if y >= 0
            && y < self.rbuf.height() as i32
            && x >= 0
            && (x + len as i32) <= self.rbuf.width() as i32
        {
            self.fast_path = true;
            self.pix_off = x as usize * PIX_WIDTH;
            let row = self.rbuf.row_slice(y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.fast_path = false;
            self.pixel()
        }
    }

    pub fn next_x(&mut self) -> &[u8] {
        if self.fast_path {
            self.pix_off += PIX_WIDTH;
            let row = self.rbuf.row_slice(self.y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.x += 1;
            self.pixel()
        }
    }

    pub fn next_y(&mut self) -> &[u8] {
        self.y += 1;
        self.x = self.x0;
        if self.fast_path && self.y >= 0 && self.y < self.rbuf.height() as i32 {
            self.pix_off = self.x as usize * PIX_WIDTH;
            let row = self.rbuf.row_slice(self.y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.fast_path = false;
            self.pixel()
        }
    }
}

// ============================================================================
// ImageAccessorNoClip — unchecked access
// ============================================================================

/// Image accessor without bounds checking — fastest, assumes all coordinates are valid.
///
/// Port of C++ `image_accessor_no_clip<PixFmt>`.
pub struct ImageAccessorNoClip<'a, const PIX_WIDTH: usize> {
    rbuf: &'a RowAccessor,
    x: i32,
    y: i32,
    pix_off: usize,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorNoClip<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor) -> Self {
        Self {
            rbuf,
            x: 0,
            y: 0,
            pix_off: 0,
        }
    }

    pub fn span(&mut self, x: i32, y: i32, _len: u32) -> &[u8] {
        self.x = x;
        self.y = y;
        self.pix_off = x as usize * PIX_WIDTH;
        let row = self.rbuf.row_slice(y as u32);
        &row[self.pix_off..self.pix_off + PIX_WIDTH]
    }

    pub fn next_x(&mut self) -> &[u8] {
        self.pix_off += PIX_WIDTH;
        let row = self.rbuf.row_slice(self.y as u32);
        &row[self.pix_off..self.pix_off + PIX_WIDTH]
    }

    pub fn next_y(&mut self) -> &[u8] {
        self.y += 1;
        self.pix_off = self.x as usize * PIX_WIDTH;
        let row = self.rbuf.row_slice(self.y as u32);
        &row[self.pix_off..self.pix_off + PIX_WIDTH]
    }
}

// ============================================================================
// ImageAccessorClone — clamp to edge pixels
// ============================================================================

/// Image accessor with clamping: out-of-bounds coordinates snap to edge pixels.
///
/// Port of C++ `image_accessor_clone<PixFmt>`.
pub struct ImageAccessorClone<'a, const PIX_WIDTH: usize> {
    rbuf: &'a RowAccessor,
    x: i32,
    x0: i32,
    y: i32,
    fast_path: bool,
    pix_off: usize,
}

impl<'a, const PIX_WIDTH: usize> ImageAccessorClone<'a, PIX_WIDTH> {
    pub fn new(rbuf: &'a RowAccessor) -> Self {
        Self {
            rbuf,
            x: 0,
            x0: 0,
            y: 0,
            fast_path: false,
            pix_off: 0,
        }
    }

    fn pixel(&self) -> &[u8] {
        let cx = self.x.max(0).min(self.rbuf.width() as i32 - 1);
        let cy = self.y.max(0).min(self.rbuf.height() as i32 - 1);
        let row = self.rbuf.row_slice(cy as u32);
        let off = cx as usize * PIX_WIDTH;
        &row[off..off + PIX_WIDTH]
    }

    pub fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
        self.x = x;
        self.x0 = x;
        self.y = y;
        if y >= 0
            && y < self.rbuf.height() as i32
            && x >= 0
            && (x + len as i32) <= self.rbuf.width() as i32
        {
            self.fast_path = true;
            self.pix_off = x as usize * PIX_WIDTH;
            let row = self.rbuf.row_slice(y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.fast_path = false;
            self.pixel()
        }
    }

    pub fn next_x(&mut self) -> &[u8] {
        if self.fast_path {
            self.pix_off += PIX_WIDTH;
            let row = self.rbuf.row_slice(self.y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.x += 1;
            self.pixel()
        }
    }

    pub fn next_y(&mut self) -> &[u8] {
        self.y += 1;
        self.x = self.x0;
        if self.fast_path && self.y >= 0 && self.y < self.rbuf.height() as i32 {
            self.pix_off = self.x as usize * PIX_WIDTH;
            let row = self.rbuf.row_slice(self.y as u32);
            &row[self.pix_off..self.pix_off + PIX_WIDTH]
        } else {
            self.fast_path = false;
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
    rbuf: &'a RowAccessor,
    x: i32,
    wrap_x: WX,
    wrap_y: WY,
    row_y: u32,
}

impl<'a, const PIX_WIDTH: usize, WX: WrapMode, WY: WrapMode>
    ImageAccessorWrap<'a, PIX_WIDTH, WX, WY>
{
    pub fn new(rbuf: &'a RowAccessor) -> Self {
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

    fn make_rgba_buffer(width: u32, height: u32, data: &mut Vec<u8>) -> RowAccessor {
        let stride = width as usize * 4;
        data.resize(stride * height as usize, 0);
        unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), width, height, stride as i32) }
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
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 1, 1, [10, 20, 30, 40]);

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
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 0, 0, [1, 2, 3, 4]);
        set_rgba_pixel(&mut data, 4, 1, 0, [5, 6, 7, 8]);

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
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 2, 1, [11, 22, 33, 44]);

        let mut acc = ImageAccessorNoClip::<4>::new(&rbuf);
        let pix = acc.span(2, 1, 1);
        assert_eq!(&pix[..4], &[11, 22, 33, 44]);
    }

    #[test]
    fn test_no_clip_next_x() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(&mut data, 4, 1, 0, [20, 0, 0, 0]);
        set_rgba_pixel(&mut data, 4, 2, 0, [30, 0, 0, 0]);

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
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 1, 1, [50, 60, 70, 80]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(1, 1, 1);
        assert_eq!(&pix[..4], &[50, 60, 70, 80]);
    }

    #[test]
    fn test_clone_clamps_negative() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 0, 0, [10, 20, 30, 40]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(-5, -3, 1);
        // Should clamp to (0, 0)
        assert_eq!(&pix[..4], &[10, 20, 30, 40]);
    }

    #[test]
    fn test_clone_clamps_overflow() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_rgba_pixel(&mut data, 4, 3, 3, [99, 88, 77, 66]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let pix = acc.span(100, 100, 1);
        // Should clamp to (3, 3)
        assert_eq!(&pix[..4], &[99, 88, 77, 66]);
    }

    // -- ImageAccessorWrap tests --

    #[test]
    fn test_wrap_repeat_access() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(2, 2, &mut data);
        set_rgba_pixel(&mut data, 2, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(&mut data, 2, 1, 0, [20, 0, 0, 0]);
        set_rgba_pixel(&mut data, 2, 0, 1, [30, 0, 0, 0]);
        set_rgba_pixel(&mut data, 2, 1, 1, [40, 0, 0, 0]);

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
        let rbuf = make_rgba_buffer(2, 2, &mut data);
        set_rgba_pixel(&mut data, 2, 0, 0, [10, 0, 0, 0]);
        set_rgba_pixel(&mut data, 2, 0, 1, [30, 0, 0, 0]);

        let mut acc = ImageAccessorWrap::<4, WrapModeRepeat, WrapModeRepeat>::new(&rbuf);
        acc.span(0, 0, 1);
        let pix = acc.next_y();
        assert_eq!(pix[0], 30);
    }
}

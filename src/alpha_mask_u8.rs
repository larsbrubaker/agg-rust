//! Alpha masking with clipping support.
//!
//! Port of `agg_alpha_mask_u8.h` — provides alpha mask functionality where
//! pixel coverage values are modulated by a grayscale mask buffer.

use crate::rendering_buffer::RowAccessor;

// ============================================================================
// MaskFunction trait
// ============================================================================

/// Trait for computing a mask value from pixel data.
///
/// Port of C++ `one_component_mask_u8` / `rgb_to_gray_mask_u8` patterns.
pub trait MaskFunction {
    fn calculate(&self, p: &[u8]) -> u8;
}

/// Single-component mask: returns the first byte.
///
/// Port of C++ `one_component_mask_u8`.
#[derive(Clone, Copy, Default)]
pub struct OneComponentMask;

impl MaskFunction for OneComponentMask {
    #[inline]
    fn calculate(&self, p: &[u8]) -> u8 {
        p[0]
    }
}

/// RGB-to-gray mask: weighted sum of R, G, B channels.
///
/// Port of C++ `rgb_to_gray_mask_u8<R, G, B>`.
/// Uses luminance weights: R*77 + G*150 + B*29 >> 8.
#[derive(Clone, Copy)]
pub struct RgbToGrayMask {
    pub r_offset: usize,
    pub g_offset: usize,
    pub b_offset: usize,
}

impl RgbToGrayMask {
    pub const fn new(r: usize, g: usize, b: usize) -> Self {
        Self {
            r_offset: r,
            g_offset: g,
            b_offset: b,
        }
    }
}

impl MaskFunction for RgbToGrayMask {
    #[inline]
    fn calculate(&self, p: &[u8]) -> u8 {
        ((p[self.r_offset] as u32 * 77
            + p[self.g_offset] as u32 * 150
            + p[self.b_offset] as u32 * 29)
            >> 8) as u8
    }
}

// ============================================================================
// AlphaMask trait
// ============================================================================

/// Alpha mask interface for coverage modulation.
pub trait AlphaMask {
    fn pixel(&self, x: i32, y: i32) -> u8;
    fn combine_pixel(&self, x: i32, y: i32, val: u8) -> u8;
    fn fill_hspan(&self, x: i32, y: i32, dst: &mut [u8]);
    fn combine_hspan(&self, x: i32, y: i32, dst: &mut [u8]);
    fn fill_vspan(&self, x: i32, y: i32, dst: &mut [u8]);
    fn combine_vspan(&self, x: i32, y: i32, dst: &mut [u8]);
}

// ============================================================================
// AlphaMaskU8 — clipped alpha mask
// ============================================================================

const COVER_SHIFT: u32 = 8;
const COVER_FULL: u32 = 255;

/// Alpha mask with bounds-checked access to a rendering buffer.
///
/// `STEP` is the number of bytes per pixel, `OFFSET` is the byte offset
/// within each pixel to the mask component.
///
/// Port of C++ `alpha_mask_u8<Step, Offset, MaskF>`.
pub struct AlphaMaskU8<'a, const STEP: usize, const OFFSET: usize, MF: MaskFunction> {
    rbuf: &'a RowAccessor,
    mask_function: MF,
}

impl<'a, const STEP: usize, const OFFSET: usize, MF: MaskFunction>
    AlphaMaskU8<'a, STEP, OFFSET, MF>
{
    pub fn new(rbuf: &'a RowAccessor, mask_function: MF) -> Self {
        Self {
            rbuf,
            mask_function,
        }
    }

    pub fn mask_function(&self) -> &MF {
        &self.mask_function
    }
}

impl<const STEP: usize, const OFFSET: usize, MF: MaskFunction> AlphaMask
    for AlphaMaskU8<'_, STEP, OFFSET, MF>
{
    fn pixel(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && x < self.rbuf.width() as i32 && y < self.rbuf.height() as i32 {
            let row = self.rbuf.row_slice(y as u32);
            let off = x as usize * STEP + OFFSET;
            self.mask_function.calculate(&row[off..])
        } else {
            0
        }
    }

    fn combine_pixel(&self, x: i32, y: i32, val: u8) -> u8 {
        if x >= 0 && y >= 0 && x < self.rbuf.width() as i32 && y < self.rbuf.height() as i32 {
            let row = self.rbuf.row_slice(y as u32);
            let off = x as usize * STEP + OFFSET;
            ((COVER_FULL + val as u32 * self.mask_function.calculate(&row[off..]) as u32)
                >> COVER_SHIFT) as u8
        } else {
            0
        }
    }

    fn fill_hspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let num_pix = dst.len() as i32;
        let xmax = self.rbuf.width() as i32 - 1;
        let ymax = self.rbuf.height() as i32 - 1;

        let mut count = num_pix;
        let mut covers_off: usize = 0;
        let mut x = x;

        if y < 0 || y > ymax {
            dst.iter_mut().for_each(|d| *d = 0);
            return;
        }

        if x < 0 {
            count += x;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[..(-x) as usize].iter_mut().for_each(|d| *d = 0);
            covers_off = (-x) as usize;
            x = 0;
        }

        if x + count > xmax + 1 {
            let rest = x + count - xmax - 1;
            count -= rest;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[(covers_off + count as usize)..]
                .iter_mut()
                .for_each(|d| *d = 0);
        }

        let row = self.rbuf.row_slice(y as u32);
        let mut mask_off = x as usize * STEP + OFFSET;
        for i in 0..count as usize {
            dst[covers_off + i] = self.mask_function.calculate(&row[mask_off..]);
            mask_off += STEP;
        }
    }

    fn combine_hspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let num_pix = dst.len() as i32;
        let xmax = self.rbuf.width() as i32 - 1;
        let ymax = self.rbuf.height() as i32 - 1;

        let mut count = num_pix;
        let mut covers_off: usize = 0;
        let mut x = x;

        if y < 0 || y > ymax {
            dst.iter_mut().for_each(|d| *d = 0);
            return;
        }

        if x < 0 {
            count += x;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[..(-x) as usize].iter_mut().for_each(|d| *d = 0);
            covers_off = (-x) as usize;
            x = 0;
        }

        if x + count > xmax + 1 {
            let rest = x + count - xmax - 1;
            count -= rest;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[(covers_off + count as usize)..]
                .iter_mut()
                .for_each(|d| *d = 0);
        }

        let row = self.rbuf.row_slice(y as u32);
        let mut mask_off = x as usize * STEP + OFFSET;
        for i in 0..count as usize {
            let idx = covers_off + i;
            dst[idx] = ((COVER_FULL
                + dst[idx] as u32 * self.mask_function.calculate(&row[mask_off..]) as u32)
                >> COVER_SHIFT) as u8;
            mask_off += STEP;
        }
    }

    fn fill_vspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let num_pix = dst.len() as i32;
        let xmax = self.rbuf.width() as i32 - 1;
        let ymax = self.rbuf.height() as i32 - 1;

        let mut count = num_pix;
        let mut covers_off: usize = 0;
        let mut y = y;

        if x < 0 || x > xmax {
            dst.iter_mut().for_each(|d| *d = 0);
            return;
        }

        if y < 0 {
            count += y;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[..(-y) as usize].iter_mut().for_each(|d| *d = 0);
            covers_off = (-y) as usize;
            y = 0;
        }

        if y + count > ymax + 1 {
            let rest = y + count - ymax - 1;
            count -= rest;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[(covers_off + count as usize)..]
                .iter_mut()
                .for_each(|d| *d = 0);
        }

        let col = x as usize * STEP + OFFSET;
        for i in 0..count as usize {
            let row = self.rbuf.row_slice((y + i as i32) as u32);
            dst[covers_off + i] = self.mask_function.calculate(&row[col..]);
        }
    }

    fn combine_vspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let num_pix = dst.len() as i32;
        let xmax = self.rbuf.width() as i32 - 1;
        let ymax = self.rbuf.height() as i32 - 1;

        let mut count = num_pix;
        let mut covers_off: usize = 0;
        let mut y = y;

        if x < 0 || x > xmax {
            dst.iter_mut().for_each(|d| *d = 0);
            return;
        }

        if y < 0 {
            count += y;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[..(-y) as usize].iter_mut().for_each(|d| *d = 0);
            covers_off = (-y) as usize;
            y = 0;
        }

        if y + count > ymax + 1 {
            let rest = y + count - ymax - 1;
            count -= rest;
            if count <= 0 {
                dst.iter_mut().for_each(|d| *d = 0);
                return;
            }
            dst[(covers_off + count as usize)..]
                .iter_mut()
                .for_each(|d| *d = 0);
        }

        let col = x as usize * STEP + OFFSET;
        for i in 0..count as usize {
            let row = self.rbuf.row_slice((y + i as i32) as u32);
            let idx = covers_off + i;
            dst[idx] = ((COVER_FULL
                + dst[idx] as u32 * self.mask_function.calculate(&row[col..]) as u32)
                >> COVER_SHIFT) as u8;
        }
    }
}

// ============================================================================
// AmaskNoClipU8 — unchecked alpha mask
// ============================================================================

/// Alpha mask without bounds checking — faster but caller must ensure in-range.
///
/// Port of C++ `amask_no_clip_u8<Step, Offset, MaskF>`.
pub struct AmaskNoClipU8<'a, const STEP: usize, const OFFSET: usize, MF: MaskFunction> {
    rbuf: &'a RowAccessor,
    mask_function: MF,
}

impl<'a, const STEP: usize, const OFFSET: usize, MF: MaskFunction>
    AmaskNoClipU8<'a, STEP, OFFSET, MF>
{
    pub fn new(rbuf: &'a RowAccessor, mask_function: MF) -> Self {
        Self {
            rbuf,
            mask_function,
        }
    }

    pub fn mask_function(&self) -> &MF {
        &self.mask_function
    }
}

impl<const STEP: usize, const OFFSET: usize, MF: MaskFunction> AlphaMask
    for AmaskNoClipU8<'_, STEP, OFFSET, MF>
{
    fn pixel(&self, x: i32, y: i32) -> u8 {
        let row = self.rbuf.row_slice(y as u32);
        let off = x as usize * STEP + OFFSET;
        self.mask_function.calculate(&row[off..])
    }

    fn combine_pixel(&self, x: i32, y: i32, val: u8) -> u8 {
        let row = self.rbuf.row_slice(y as u32);
        let off = x as usize * STEP + OFFSET;
        ((COVER_FULL + val as u32 * self.mask_function.calculate(&row[off..]) as u32)
            >> COVER_SHIFT) as u8
    }

    fn fill_hspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let row = self.rbuf.row_slice(y as u32);
        let mut mask_off = x as usize * STEP + OFFSET;
        for d in dst.iter_mut() {
            *d = self.mask_function.calculate(&row[mask_off..]);
            mask_off += STEP;
        }
    }

    fn combine_hspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let row = self.rbuf.row_slice(y as u32);
        let mut mask_off = x as usize * STEP + OFFSET;
        for d in dst.iter_mut() {
            *d = ((COVER_FULL + *d as u32 * self.mask_function.calculate(&row[mask_off..]) as u32)
                >> COVER_SHIFT) as u8;
            mask_off += STEP;
        }
    }

    fn fill_vspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let col = x as usize * STEP + OFFSET;
        for (i, d) in dst.iter_mut().enumerate() {
            let row = self.rbuf.row_slice((y + i as i32) as u32);
            *d = self.mask_function.calculate(&row[col..]);
        }
    }

    fn combine_vspan(&self, x: i32, y: i32, dst: &mut [u8]) {
        let col = x as usize * STEP + OFFSET;
        for (i, d) in dst.iter_mut().enumerate() {
            let row = self.rbuf.row_slice((y + i as i32) as u32);
            *d = ((COVER_FULL + *d as u32 * self.mask_function.calculate(&row[col..]) as u32)
                >> COVER_SHIFT) as u8;
        }
    }
}

// ============================================================================
// Type aliases for common configurations
// ============================================================================

/// Gray8 alpha mask (1 byte per pixel, offset 0).
pub type AlphaMaskGray8<'a> = AlphaMaskU8<'a, 1, 0, OneComponentMask>;

/// RGBA32 red channel mask.
pub type AlphaMaskRgba32r<'a> = AlphaMaskU8<'a, 4, 0, OneComponentMask>;
/// RGBA32 green channel mask.
pub type AlphaMaskRgba32g<'a> = AlphaMaskU8<'a, 4, 1, OneComponentMask>;
/// RGBA32 blue channel mask.
pub type AlphaMaskRgba32b<'a> = AlphaMaskU8<'a, 4, 2, OneComponentMask>;
/// RGBA32 alpha channel mask.
pub type AlphaMaskRgba32a<'a> = AlphaMaskU8<'a, 4, 3, OneComponentMask>;

/// RGB24 red channel mask.
pub type AlphaMaskRgb24r<'a> = AlphaMaskU8<'a, 3, 0, OneComponentMask>;
/// RGB24 green channel mask.
pub type AlphaMaskRgb24g<'a> = AlphaMaskU8<'a, 3, 1, OneComponentMask>;
/// RGB24 blue channel mask.
pub type AlphaMaskRgb24b<'a> = AlphaMaskU8<'a, 3, 2, OneComponentMask>;

// No-clip variants
/// Gray8 alpha mask, no clipping.
pub type AmaskNoClipGray8<'a> = AmaskNoClipU8<'a, 1, 0, OneComponentMask>;
/// RGBA32 alpha channel mask, no clipping.
pub type AmaskNoClipRgba32a<'a> = AmaskNoClipU8<'a, 4, 3, OneComponentMask>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gray_buffer(width: u32, height: u32, data: &mut Vec<u8>) -> RowAccessor {
        data.resize((width * height) as usize, 0);
        unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), width, height, width as i32) }
    }

    fn make_rgba_buffer(width: u32, height: u32, data: &mut Vec<u8>) -> RowAccessor {
        let stride = width * 4;
        data.resize((stride * height) as usize, 0);
        unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), width, height, stride as i32) }
    }

    #[test]
    fn test_one_component_mask() {
        let m = OneComponentMask;
        assert_eq!(m.calculate(&[128, 0, 0, 0]), 128);
        assert_eq!(m.calculate(&[255]), 255);
        assert_eq!(m.calculate(&[0, 99]), 0);
    }

    #[test]
    fn test_rgb_to_gray_mask() {
        let m = RgbToGrayMask::new(0, 1, 2);
        // Pure red: 255*77/256 ≈ 76
        let val = m.calculate(&[255, 0, 0]);
        assert_eq!(val, ((255u32 * 77) >> 8) as u8);

        // Pure green: 255*150/256 ≈ 149
        let val = m.calculate(&[0, 255, 0]);
        assert_eq!(val, ((255u32 * 150) >> 8) as u8);

        // Pure blue: 255*29/256 ≈ 28
        let val = m.calculate(&[0, 0, 255]);
        assert_eq!(val, ((255u32 * 29) >> 8) as u8);

        // White: (255*77 + 255*150 + 255*29)/256 = 255*256/256 = 255
        let val = m.calculate(&[255, 255, 255]);
        assert_eq!(val, 255);
    }

    #[test]
    fn test_pixel_in_bounds() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        // Set pixel (2,1) to 200
        data[1 * 4 + 2] = 200;
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);
        assert_eq!(mask.pixel(2, 1), 200);
    }

    #[test]
    fn test_pixel_out_of_bounds() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);
        assert_eq!(mask.pixel(-1, 0), 0);
        assert_eq!(mask.pixel(0, -1), 0);
        assert_eq!(mask.pixel(4, 0), 0);
        assert_eq!(mask.pixel(0, 4), 0);
    }

    #[test]
    fn test_combine_pixel() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        data[0] = 128;
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);
        // combine: (255 + val * mask_val) >> 8 = (255 + 200 * 128) >> 8
        let result = mask.combine_pixel(0, 0, 200);
        assert_eq!(result, ((255 + 200u32 * 128) >> 8) as u8);
    }

    #[test]
    fn test_combine_pixel_out_of_bounds() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);
        assert_eq!(mask.combine_pixel(-1, 0, 200), 0);
    }

    #[test]
    fn test_fill_hspan_in_bounds() {
        let mut data = vec![0u8; 16];
        for i in 0..4 {
            data[i] = (i as u8 + 1) * 50;
        }
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![0u8; 4];
        mask.fill_hspan(0, 0, &mut dst);
        assert_eq!(dst, vec![50, 100, 150, 200]);
    }

    #[test]
    fn test_fill_hspan_y_out_of_range() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![99u8; 4];
        mask.fill_hspan(0, -1, &mut dst);
        assert_eq!(dst, vec![0, 0, 0, 0]);

        let mut dst = vec![99u8; 4];
        mask.fill_hspan(0, 4, &mut dst);
        assert_eq!(dst, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_fill_hspan_left_clip() {
        let mut data = vec![0u8; 16];
        for i in 0..4 {
            data[i] = (i as u8 + 1) * 50;
        }
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![99u8; 3];
        mask.fill_hspan(-1, 0, &mut dst);
        // First element should be 0 (clipped), rest from x=0,1
        assert_eq!(dst[0], 0);
        assert_eq!(dst[1], 50); // data[0]
        assert_eq!(dst[2], 100); // data[1]
    }

    #[test]
    fn test_fill_hspan_right_clip() {
        let mut data = vec![0u8; 16];
        for i in 0..4 {
            data[i] = (i as u8 + 1) * 50;
        }
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![99u8; 3];
        mask.fill_hspan(2, 0, &mut dst);
        // x=2,3 valid, x=4 clipped
        assert_eq!(dst[0], 150); // data[2]
        assert_eq!(dst[1], 200); // data[3]
        assert_eq!(dst[2], 0); // clipped
    }

    #[test]
    fn test_combine_hspan() {
        let mut data = vec![0u8; 16];
        data[0] = 128;
        data[1] = 255;
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![200u8, 100u8];
        mask.combine_hspan(0, 0, &mut dst);
        assert_eq!(dst[0], ((255 + 200u32 * 128) >> 8) as u8);
        assert_eq!(dst[1], ((255 + 100u32 * 255) >> 8) as u8);
    }

    #[test]
    fn test_fill_vspan() {
        let mut data = vec![0u8; 16];
        // Set column 1, rows 0-3
        data[1] = 10;
        data[4 + 1] = 20;
        data[8 + 1] = 30;
        data[12 + 1] = 40;
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![0u8; 4];
        mask.fill_vspan(1, 0, &mut dst);
        assert_eq!(dst, vec![10, 20, 30, 40]);
    }

    #[test]
    fn test_fill_vspan_x_out_of_range() {
        let mut data = Vec::new();
        let rbuf = make_gray_buffer(4, 4, &mut data);
        let mask = AlphaMaskGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![99u8; 4];
        mask.fill_vspan(-1, 0, &mut dst);
        assert_eq!(dst, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_rgba_mask() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        // Set pixel (0,0) RGBA = (100, 150, 200, 255)
        data[0] = 100;
        data[1] = 150;
        data[2] = 200;
        data[3] = 255;

        // Alpha channel mask (step=4, offset=3)
        let mask = AlphaMaskRgba32a::new(&rbuf, OneComponentMask);
        assert_eq!(mask.pixel(0, 0), 255);

        // Red channel mask (step=4, offset=0)
        let rmask = AlphaMaskRgba32r::new(&rbuf, OneComponentMask);
        assert_eq!(rmask.pixel(0, 0), 100);
    }

    #[test]
    fn test_no_clip_pixel() {
        let mut data = vec![0u8; 16];
        data[5] = 42; // row 1, pixel 1
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AmaskNoClipGray8::new(&rbuf, OneComponentMask);
        assert_eq!(mask.pixel(1, 1), 42);
    }

    #[test]
    fn test_no_clip_fill_hspan() {
        let mut data = vec![0u8; 16];
        data[0] = 10;
        data[1] = 20;
        data[2] = 30;
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AmaskNoClipGray8::new(&rbuf, OneComponentMask);

        let mut dst = vec![0u8; 3];
        mask.fill_hspan(0, 0, &mut dst);
        assert_eq!(dst, vec![10, 20, 30]);
    }

    #[test]
    fn test_no_clip_combine_pixel() {
        let mut data = vec![0u8; 16];
        data[0] = 128;
        let rbuf = unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), 4, 4, 4) };
        let mask = AmaskNoClipGray8::new(&rbuf, OneComponentMask);
        let result = mask.combine_pixel(0, 0, 200);
        assert_eq!(result, ((255 + 200u32 * 128) >> 8) as u8);
    }
}

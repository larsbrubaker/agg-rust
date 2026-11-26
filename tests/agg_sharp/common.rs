//! Shared helpers for the agg-sharp test ports: an owned RGBA32 image that hands out a
//! `RowAccessor` over itself, standing in for agg-sharp's `ImageBuffer` (zeroed, so
//! untouched pixels have alpha 0, as in C#).

use agg_rust::rendering_buffer::RowAccessor;

/// An owned RGBA32 pixel buffer and its size.
pub struct Image {
    pub buf: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Image {
    /// A `width` x `height` RGBA32 image, every byte 0.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: vec![0u8; (width * height * 4) as usize],
            width,
            height,
        }
    }

    /// A row accessor over the image's bytes, borrowing them for its lifetime.
    pub fn accessor(&mut self) -> RowAccessor<'_> {
        RowAccessor::new_with_buf(
            &mut self.buf,
            self.width,
            self.height,
            (self.width * 4) as i32,
        )
    }

    /// The pixel at `x`, `y` as `[r, g, b, a]`.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let o = ((y * self.width + x) * 4) as usize;
        [
            self.buf[o],
            self.buf[o + 1],
            self.buf[o + 2],
            self.buf[o + 3],
        ]
    }
}

impl Image {
    /// Every pixel set to `color`.
    pub fn fill(&mut self, color: [u8; 4]) {
        for px in self.buf.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&color);
        }
    }

    /// Sets the pixel at `x`, `y` to `color`.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        let o = ((y * self.width + x) * 4) as usize;
        self.buf[o..o + 4].copy_from_slice(&color);
    }
}

/// An owned one-byte-per-pixel gray image and its size (agg-sharp's
/// `ImageBuffer(w, h, 8, new blender_gray(1))`).
pub struct GrayImage {
    pub buf: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl GrayImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            buf: vec![0u8; (width * height) as usize],
            width,
            height,
        }
    }

    /// A row accessor over the image's bytes, borrowing them for its lifetime.
    pub fn accessor(&mut self) -> RowAccessor<'_> {
        RowAccessor::new_with_buf(&mut self.buf, self.width, self.height, self.width as i32)
    }

    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.buf[(y * self.width + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, v: u8) {
        self.buf[(y * self.width + x) as usize] = v;
    }
}

//! Shared helpers for the agg-sharp test ports: an owned byte buffer that hands
//! out a `RowAccessor` over itself, standing in for agg-sharp's `ImageBuffer`.

use agg_rust::rendering_buffer::RowAccessor;

/// Pixel bytes and the geometry of the rows they hold.
pub struct Image {
    pub bytes: Vec<u8>,
    width: u32,
    height: u32,
    stride: i32,
}

impl Image {
    /// A `width` x `height` image of `bpp` bytes per pixel, every byte `fill`.
    pub fn new(width: u32, height: u32, bpp: u32, fill: u8) -> Self {
        Self {
            bytes: vec![fill; (width * height * bpp) as usize],
            width,
            height,
            stride: (width * bpp) as i32,
        }
    }

    /// A row accessor over the image's bytes, borrowing them for its lifetime.
    pub fn accessor(&mut self) -> RowAccessor<'_> {
        RowAccessor::new_with_buf(&mut self.bytes, self.width, self.height, self.stride)
    }
}

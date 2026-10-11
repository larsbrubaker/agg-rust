//! Shared helpers for the agg-sharp test ports: an owned byte buffer with a
//! `RowAccessor` attached, standing in for agg-sharp's `ImageBuffer`.

use agg_rust::rendering_buffer::RowAccessor;

/// Pixel bytes plus a row accessor over them. The `Vec` is never resized, so
/// the accessor's pointer stays valid for the struct's lifetime.
pub struct Image {
    pub bytes: Vec<u8>,
    pub rows: RowAccessor,
}

impl Image {
    /// A `width` x `height` image of `bpp` bytes per pixel, every byte `fill`.
    pub fn new(width: u32, height: u32, bpp: u32, fill: u8) -> Self {
        let mut bytes = vec![fill; (width * height * bpp) as usize];
        let mut rows = RowAccessor::new();
        // SAFETY: `bytes` holds exactly width * height * bpp bytes and lives
        // (unmoved on the heap) as long as `rows`.
        unsafe {
            rows.attach(bytes.as_mut_ptr(), width, height, (width * bpp) as i32);
        }
        Self { bytes, rows }
    }
}

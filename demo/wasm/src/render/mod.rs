//! Demo render functions.
//!
//! Each function renders a specific demo into an RGBA pixel buffer.
//! The buffer is width * height * 4 bytes (RGBA order).
//!
//! Split into submodules to keep each file under 2000 lines.

mod basic;
mod images;
mod alpha;
mod transforms;
mod compositing;

pub use basic::*;
pub use images::*;
pub use alpha::*;
pub use transforms::*;
pub use compositing::*;

use agg_rust::rendering_buffer::RowAccessor;

/// Create a rendering buffer, pixel format, and renderer base from dimensions.
pub(crate) fn setup_renderer(
    buf: &mut Vec<u8>,
    ra: &mut RowAccessor,
    width: u32,
    height: u32,
) {
    let stride = (width * 4) as i32;
    buf.resize((width * height * 4) as usize, 255);
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
}

// ============================================================================
// Fallback
// ============================================================================

pub fn fallback(width: u32, height: u32) -> Vec<u8> {
    use agg_rust::color::Rgba8;
    use agg_rust::pixfmt_rgba::PixfmtRgba32;
    use agg_rust::renderer_base::RendererBase;

    let mut buf = vec![0u8; (width * height * 4) as usize];
    let mut ra = RowAccessor::new();
    setup_renderer(&mut buf, &mut ra, width, height);
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(40, 40, 80, 255));
    buf
}

// ============================================================================
// Shared image loading
// ============================================================================

static SPHERES_BMP: &[u8] = include_bytes!("../spheres.bmp");

/// Parse the embedded spheres BMP and return (width, height, rgba_data).
pub(super) fn load_spheres_image() -> (u32, u32, Vec<u8>) {
    let d = SPHERES_BMP;
    let off = u32::from_le_bytes([d[10], d[11], d[12], d[13]]) as usize;
    let w = u32::from_le_bytes([d[18], d[19], d[20], d[21]]);
    let h = u32::from_le_bytes([d[22], d[23], d[24], d[25]]);
    let bpp = u16::from_le_bytes([d[28], d[29]]) as usize;
    let bytes_pp = bpp / 8;
    let row_size = ((w as usize * bytes_pp + 3) / 4) * 4;
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        let src_y = h as usize - 1 - y;
        let src_off = off + src_y * row_size;
        for x in 0..w as usize {
            let si = src_off + x * bytes_pp;
            let di = (y * w as usize + x) * 4;
            if bytes_pp >= 3 {
                rgba[di] = d[si + 2];
                rgba[di + 1] = d[si + 1];
                rgba[di + 2] = d[si];
                rgba[di + 3] = if bytes_pp >= 4 { d[si + 3] } else { 255 };
            }
        }
    }
    (w, h, rgba)
}

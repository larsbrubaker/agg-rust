use wasm_bindgen::prelude::*;

/// Render a demo scene into an RGBA pixel buffer.
///
/// Returns a Vec<u8> of RGBA pixel data (width * height * 4 bytes).
/// The TypeScript frontend will copy this into an ImageData for display on a canvas.
#[wasm_bindgen]
pub fn render_demo(_demo_id: u32, _width: u32, _height: u32, _params: &[f64]) -> Vec<u8> {
    // Placeholder - will be implemented as modules are ported
    let size = (_width * _height * 4) as usize;
    let mut buf = vec![255u8; size];

    // Fill with a simple gradient to verify WASM pipeline works
    for y in 0.._height {
        for x in 0.._width {
            let offset = ((y * _width + x) * 4) as usize;
            buf[offset] = (x * 255 / _width) as u8; // R
            buf[offset + 1] = (y * 255 / _height) as u8; // G
            buf[offset + 2] = 128; // B
            buf[offset + 3] = 255; // A
        }
    }

    buf
}

/// Get the library version string.
#[wasm_bindgen]
pub fn version() -> String {
    "agg-rust 0.1.0".to_string()
}

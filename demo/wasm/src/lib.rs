use wasm_bindgen::prelude::*;

mod lion_data;
mod render;

/// Render a named demo into an RGBA pixel buffer.
///
/// Returns a `Vec<u8>` of RGBA pixel data (width * height * 4 bytes).
/// The TypeScript frontend copies this into an ImageData for canvas display.
#[wasm_bindgen]
pub fn render_demo(name: &str, width: u32, height: u32, params: &[f64]) -> Vec<u8> {
    match name {
        "lion" => render::lion(width, height, params),
        "shapes" => render::shapes(width, height, params),
        "gradients" => render::gradients(width, height, params),
        "gouraud" => render::gouraud(width, height, params),
        "strokes" => render::strokes(width, height, params),
        "curves" => render::curves(width, height, params),
        _ => render::fallback(width, height),
    }
}

/// Get the library version string.
#[wasm_bindgen]
pub fn version() -> String {
    "agg-rust 0.1.0".to_string()
}

/// Get list of available demo names.
#[wasm_bindgen]
pub fn demo_names() -> String {
    "lion,shapes,gradients,gouraud,strokes,curves".to_string()
}

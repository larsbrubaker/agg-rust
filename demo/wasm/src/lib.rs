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
        "conv_stroke" => render::conv_stroke(width, height, params),
        "bezier_div" => render::bezier_div(width, height, params),
        "circles" => render::circles(width, height, params),
        "rounded_rect" => render::rounded_rect_demo(width, height, params),
        "aa_demo" => render::aa_demo(width, height, params),
        "gamma_correction" => render::gamma_correction(width, height, params),
        "line_thickness" => render::line_thickness(width, height, params),
        "rasterizers" => render::rasterizers(width, height, params),
        "conv_contour" => render::conv_contour_demo(width, height, params),
        "conv_dash" => render::conv_dash_demo(width, height, params),
        "gsv_text" => render::gsv_text_demo(width, height, params),
        "perspective" => render::perspective_demo(width, height, params),
        "image_fltr_graph" => render::image_fltr_graph(width, height, params),
        "image1" => render::image1(width, height, params),
        "gradient_focal" => render::gradient_focal(width, height, params),
        "idea" => render::idea(width, height, params),
        "graph_test" => render::graph_test(width, height, params),
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
    "lion,shapes,gradients,gouraud,conv_stroke,bezier_div,circles,rounded_rect,aa_demo,gamma_correction,line_thickness,rasterizers,conv_contour,conv_dash,gsv_text,perspective,image_fltr_graph,image1,gradient_focal,idea,graph_test".to_string()
}

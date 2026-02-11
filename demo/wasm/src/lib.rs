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
        "perspective" => render::perspective_demo(width, height, params),
        "image_fltr_graph" => render::image_fltr_graph(width, height, params),
        "image1" => render::image1(width, height, params),
        "image_filters" => render::image_filters_demo(width, height, params),
        "gradient_focal" => render::gradient_focal(width, height, params),
        "idea" => render::idea(width, height, params),
        "graph_test" => render::graph_test(width, height, params),
        "gamma_tuner" => render::gamma_tuner(width, height, params),
        "image_filters2" => render::image_filters2(width, height, params),
        "conv_dash_marker" => render::conv_dash_marker_demo(width, height, params),
        "aa_test" => render::aa_test(width, height, params),
        "bspline" => render::bspline_demo(width, height, params),
        "image_perspective" => render::image_perspective_demo(width, height, params),
        "alpha_mask" => render::alpha_mask_demo(width, height, params),
        "alpha_gradient" => render::alpha_gradient(width, height, params),
        "image_alpha" => render::image_alpha(width, height, params),
        "alpha_mask3" => render::alpha_mask3(width, height, params),
        "image_transforms" => render::image_transforms_demo(width, height, params),
        "mol_view" => render::mol_view(width, height, params),
        "raster_text" => render::raster_text(width, height, params),
        "gamma_ctrl" => render::gamma_ctrl_demo(width, height, params),
        "trans_polar" => render::trans_polar_demo(width, height, params),
        "multi_clip" => render::multi_clip(width, height, params),
        "simple_blur" => render::simple_blur(width, height, params),
        "blur" => render::blur_demo(width, height, params),
        "trans_curve1" => render::trans_curve1(width, height, params),
        "trans_curve2" => render::trans_curve2(width, height, params),
        "lion_lens" => render::lion_lens(width, height, params),
        "distortions" => render::distortions(width, height, params),
        "blend_color" => render::blend_color(width, height, params),
        "component_rendering" => render::component_rendering(width, height, params),
        "polymorphic_renderer" => render::polymorphic_renderer(width, height, params),
        "scanline_boolean" => render::scanline_boolean(width, height, params),
        "scanline_boolean2" => render::scanline_boolean2(width, height, params),
        "pattern_fill" => render::pattern_fill(width, height, params),
        "pattern_perspective" => render::pattern_perspective(width, height, params),
        "pattern_resample" => render::pattern_resample(width, height, params),
        "lion_outline" => render::lion_outline(width, height, params),
        "rasterizers2" => render::rasterizers2(width, height, params),
        "line_patterns" => render::line_patterns(width, height, params),
        "line_patterns_clip" => render::line_patterns_clip(width, height, params),
        "compositing" => render::compositing(width, height, params),
        "compositing2" => render::compositing2(width, height, params),
        "flash_rasterizer" => render::flash_rasterizer(width, height, params),
        "flash_rasterizer2" => render::flash_rasterizer2(width, height, params),
        "rasterizer_compound" => render::rasterizer_compound(width, height, params),
        "gouraud_mesh" => render::gouraud_mesh(width, height, params),
        "truetype_test" => render::truetype_test(width, height, params),
        "image_resample" => render::image_resample_demo(width, height, params),
        "alpha_mask2" => render::alpha_mask2(width, height, params),
        _ => render::fallback(width, height),
    }
}

/// Flash demos: pick nearest editable vertex under cursor.
///
/// `demo_name` must be either "flash_rasterizer" or "flash_rasterizer2".
#[wasm_bindgen]
pub fn flash_pick_vertex(
    demo_name: &str,
    width: u32,
    height: u32,
    params: &[f64],
    x: f64,
    y: f64,
    radius: f64,
) -> i32 {
    match demo_name {
        "flash_rasterizer2" => render::flash_pick_vertex(true, width, height, params, x, y, radius),
        _ => render::flash_pick_vertex(false, width, height, params, x, y, radius),
    }
}

/// Flash demos: convert screen/device coordinates to shape-local coordinates.
#[wasm_bindgen]
pub fn flash_screen_to_shape(
    _demo_name: &str,
    width: u32,
    height: u32,
    params: &[f64],
    x: f64,
    y: f64,
) -> Vec<f64> {
    render::flash_screen_to_shape(width, height, params, x, y)
}

/// Get the library version string.
#[wasm_bindgen]
pub fn version() -> String {
    "agg-rust 0.1.0".to_string()
}

/// Get list of available demo names.
#[wasm_bindgen]
pub fn demo_names() -> String {
    "lion,gradients,gouraud,conv_stroke,bezier_div,circles,rounded_rect,aa_demo,gamma_correction,line_thickness,rasterizers,conv_contour,conv_dash,perspective,image_fltr_graph,image1,image_filters,gradient_focal,idea,graph_test,gamma_tuner,image_filters2,conv_dash_marker,aa_test,bspline,image_perspective,alpha_mask,alpha_gradient,image_alpha,alpha_mask3,image_transforms,mol_view,raster_text,gamma_ctrl,trans_polar,multi_clip,simple_blur,blur,trans_curve1,trans_curve2,lion_lens,distortions,blend_color,component_rendering,polymorphic_renderer,scanline_boolean,scanline_boolean2,pattern_fill,pattern_perspective,pattern_resample,lion_outline,rasterizers2,line_patterns,line_patterns_clip,compositing,compositing2,flash_rasterizer,flash_rasterizer2,rasterizer_compound,gouraud_mesh,image_resample,alpha_mask2,truetype_test".to_string()
}

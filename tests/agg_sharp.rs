//! agg-sharp's test classes (`Tests/Agg.Tests/Agg/*.cs` in MatterCAD's agg-sharp submodule),
//! ported 1:1: one file per C# class, the same test names in snake_case and the same expected
//! values. They run against agg-rust's public API only. agg-sharp (MatterCAD's C# port of AGG)
//! pins behaviour of the same C++ library this crate ports.

#[path = "agg_sharp/common.rs"]
mod common;
#[path = "agg_sharp/support.rs"]
mod support;

#[path = "agg_sharp/alpha_mask_adaptor_tests.rs"]
mod alpha_mask_adaptor_tests;
#[path = "agg_sharp/apply_gamma_inv_tests.rs"]
mod apply_gamma_inv_tests;
#[path = "agg_sharp/blender_comp_op_bgra_tests.rs"]
mod blender_comp_op_bgra_tests;
#[path = "agg_sharp/blender_gamma_bgr_tests.rs"]
mod blender_gamma_bgr_tests;
#[path = "agg_sharp/blender_pre_mult_bgr_tests.rs"]
mod blender_pre_mult_bgr_tests;
#[path = "agg_sharp/blender_pre_mult_bgra_tests.rs"]
mod blender_pre_mult_bgra_tests;
#[path = "agg_sharp/blender_table_tests.rs"]
mod blender_table_tests;
#[path = "agg_sharp/gradient_linear_color_tests.rs"]
mod gradient_linear_color_tests;
#[path = "agg_sharp/gradient_lut_tests.rs"]
mod gradient_lut_tests;
#[path = "agg_sharp/pattern_filter_bilinear_tests.rs"]
mod pattern_filter_bilinear_tests;
#[path = "agg_sharp/pixel_format_bgra_float_tests.rs"]
mod pixel_format_bgra_float_tests;
#[path = "agg_sharp/raster_text_tests.rs"]
mod raster_text_tests;
#[path = "agg_sharp/rasterizer_cells_aa_tests.rs"]
mod rasterizer_cells_aa_tests;
#[path = "agg_sharp/recursive_blur_tests.rs"]
mod recursive_blur_tests;
#[path = "agg_sharp/renderer_base_float_tests.rs"]
mod renderer_base_float_tests;
#[path = "agg_sharp/renderer_markers_tests.rs"]
mod renderer_markers_tests;
#[path = "agg_sharp/renderer_primitives_tests.rs"]
mod renderer_primitives_tests;
#[path = "agg_sharp/scanline_boolean_algebra_tests.rs"]
mod scanline_boolean_algebra_tests;
#[path = "agg_sharp/slight_blur_tests.rs"]
mod slight_blur_tests;
#[path = "agg_sharp/span_gouraud_rgba_tests.rs"]
mod span_gouraud_rgba_tests;
#[path = "agg_sharp/span_gradient_tests.rs"]
mod span_gradient_tests;
#[path = "agg_sharp/span_image_filter_rgb_tests.rs"]
mod span_image_filter_rgb_tests;
#[path = "agg_sharp/span_image_filter_rgba2x2_tests.rs"]
mod span_image_filter_rgba2x2_tests;
#[path = "agg_sharp/span_image_filter_rgba_bilinear_clip_tests.rs"]
mod span_image_filter_rgba_bilinear_clip_tests;
#[path = "agg_sharp/span_image_filter_rgba_bilinear_tests.rs"]
mod span_image_filter_rgba_bilinear_tests;
#[path = "agg_sharp/span_image_filter_rgba_nn_tests.rs"]
mod span_image_filter_rgba_nn_tests;
#[path = "agg_sharp/span_image_filter_rgba_tests.rs"]
mod span_image_filter_rgba_tests;
#[path = "agg_sharp/span_image_resample_rgb_tests.rs"]
mod span_image_resample_rgb_tests;
#[path = "agg_sharp/span_image_resample_rgba_tests.rs"]
mod span_image_resample_rgba_tests;
#[path = "agg_sharp/span_interpolator_linear_subdiv_tests.rs"]
mod span_interpolator_linear_subdiv_tests;
#[path = "agg_sharp/span_interpolator_persp_exact_tests.rs"]
mod span_interpolator_persp_exact_tests;
#[path = "agg_sharp/srgb_lut_float_tests.rs"]
mod srgb_lut_float_tests;
#[path = "agg_sharp/stack_blur_gray8_tests.rs"]
mod stack_blur_gray8_tests;
#[path = "agg_sharp/stack_blur_tests.rs"]
mod stack_blur_tests;
#[path = "agg_sharp/trans_warp_magnifier_tests.rs"]
mod trans_warp_magnifier_tests;

// Not ported, each for want of a C++ AGG counterpart or because it pins agg-sharp-only behaviour:
// BlenderGammaBGRATests (C++ has no RGBA gamma blender) and BlenderSpanPolicyTests (agg-sharp's
// blender set and vspans); SpanImageFilterRgbaTests.StraightAlphaColorIsNotClampedToAlpha (agg-sharp
// drops C++'s premultiplied colour-to-alpha clamp for its straight-alpha images).

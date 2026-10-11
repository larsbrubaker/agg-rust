//! agg-sharp's test classes (`Tests/Agg.Tests/Agg/*.cs` in MatterCAD's agg-sharp submodule),
//! ported 1:1: one file per C# class, the same test names in snake_case and the same expected
//! values. They run against agg-rust's public API only. agg-sharp (MatterCAD's C# port of AGG)
//! pins behaviour of the same C++ library this crate ports.

#[path = "agg_sharp/common.rs"]
mod common;
#[path = "agg_sharp/support.rs"]
mod support;

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
#[path = "agg_sharp/raster_text_tests.rs"]
mod raster_text_tests;
#[path = "agg_sharp/rasterizer_cells_aa_tests.rs"]
mod rasterizer_cells_aa_tests;
#[path = "agg_sharp/recursive_blur_tests.rs"]
mod recursive_blur_tests;
#[path = "agg_sharp/renderer_markers_tests.rs"]
mod renderer_markers_tests;
#[path = "agg_sharp/renderer_primitives_tests.rs"]
mod renderer_primitives_tests;
#[path = "agg_sharp/slight_blur_tests.rs"]
mod slight_blur_tests;
#[path = "agg_sharp/stack_blur_gray8_tests.rs"]
mod stack_blur_gray8_tests;
#[path = "agg_sharp/scanline_boolean_algebra_tests.rs"]
mod scanline_boolean_algebra_tests;
#[path = "agg_sharp/stack_blur_tests.rs"]
mod stack_blur_tests;

// Not ported, each for want of a C++ AGG counterpart or because it pins agg-sharp-only behaviour:
// AlphaMaskAdaptorTests and PixelFormatBGRAFloatTests / SrgbLutFloatTests (no
// pixfmt_amask_adaptor, rgba32 float color or sRGB LUT here yet), BlenderGammaBGRATests (C++
// has no RGBA gamma blender) and BlenderSpanPolicyTests (agg-sharp's blender set and vspans).

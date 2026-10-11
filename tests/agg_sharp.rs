//! agg-sharp's test classes (`Tests/Agg.Tests/Agg/*.cs` in MatterCAD's agg-sharp submodule),
//! ported 1:1: one file per C# class, the same test names in snake_case and the same expected
//! values. They run against agg-rust's public API only.

#[path = "agg_sharp/common.rs"]
mod common;
#[path = "agg_sharp/raster_text_tests.rs"]
mod raster_text_tests;
#[path = "agg_sharp/rasterizer_cells_aa_tests.rs"]
mod rasterizer_cells_aa_tests;
#[path = "agg_sharp/renderer_primitives_tests.rs"]
mod renderer_primitives_tests;

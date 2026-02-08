# AGG Demo Porting Status

Cross-reference of all 75 C++ demos from `cpp-references/agg-web/demo/` against our WASM demo page.

Reference screenshots: `cpp-references/agg-web/demo/<name>/<name>.gif|png|jpg`
Reference source: `cpp-references/agg-web/demo/<name>/<name>.cpp.html`

## Currently Implemented (34 demos)

| Demo | C++ Original | Window | Status |
|------|-------------|--------|--------|
| aa_demo | aa_demo.cpp | 600x400 | Done - fixed colors, vertices, threshold |
| aa_test | aa_test.cpp | 480x350 | Done - radial dashes, ellipses, gradient lines, Gouraud triangles |
| alpha_gradient | alpha_gradient.cpp | 512x400 | Done - gradient with alpha curve control, random ellipse background |
| alpha_mask | alpha_mask.cpp | 512x400 | Done - lion with elliptical alpha mask (manual compositing) |
| alpha_mask3 | alpha_mask3.cpp | 640x520 | Done - alpha mask polygon clipping (AND/SUB), 5 scenarios |
| bezier_div | bezier_div.cpp | 600x600 | Done |
| bspline | bspline.cpp | 600x600 | Done - 6 draggable control points, B-spline curve |
| circles | circles.cpp | 400x400 | Done |
| conv_contour | conv_contour.cpp | 600x600 | Done |
| conv_dash | conv_dash_marker.cpp | 600x600 | Done |
| conv_dash_marker | conv_dash_marker.cpp | 500x330 | Done - dashed strokes with cap styles (arrowheads skipped) |
| conv_stroke | conv_stroke.cpp | 600x600 | Done |
| gamma_correction | gamma_correction.cpp | 500x400 | Done |
| gamma_tuner | gamma_tuner.cpp | 500x500 | Done - gradient background + alpha pattern with gamma |
| gouraud | gouraud.cpp | 400x320 | Done |
| gradient_focal | gradient_focal.cpp | 600x400 | Done |
| gradients | gradients.cpp | 512x400 | Done |
| graph_test | graph_test.cpp | 700x530 | Done |
| gsv_text | (custom) | 600x400 | Done |
| idea | idea.cpp | 250x280 | Done - fixed polygon data, transform chain |
| image_alpha | image_alpha.cpp | 512x400 | Done - brightness-to-alpha mapping, random ellipse background |
| image_filters | image_filters.cpp | 430x340 | Done - iterative rotation, 17 filters |
| image_filters2 | image_filters2.cpp | 500x340 | Done - 4x4 test image, 17 filters, kernel graph |
| image_perspective | image_perspective.cpp | 600x600 | Done - affine/bilinear/perspective image transform |
| image_transforms | image_transforms.cpp | 430x340 | Done - star polygon + image, 7 transform modes |
| image_fltr_graph | image_fltr_graph.cpp | 780x300 | Done - 16 checkboxes, 3 curve types |
| image1 | image1.cpp | 600x500 | Done |
| line_thickness | (custom) | 600x400 | Done |
| lion | lion.cpp | 512x400 | Done |
| mol_view | mol_view.cpp | 400x400 | Done - molecular structure viewer (Caffeine/Aspirin/Benzene) |
| perspective | perspective.cpp | 600x600 | Done |
| rasterizers | rasterizers.cpp | 600x400 | Done |
| rounded_rect | rounded_rect.cpp | 600x400 | Done |
| shapes | (custom) | 600x400 | Done |

## Deferred (30 demos - need missing Rust modules)

### Missing: rasterizer_outline_aa + line_profile_aa (4 demos)
- line_patterns
- line_patterns_clip
- lion_outline
- rasterizers2

### Missing: rasterizer_compound_aa + comp_op (5 demos)
- compositing
- compositing2
- flash_rasterizer
- flash_rasterizer2
- rasterizer_compound

### Missing: stack_blur / recursive_blur (2 demos)
- blur
- simple_blur

### Missing: span_pattern (3 demos)
- pattern_fill
- pattern_perspective
- pattern_resample

### Missing: scanline_boolean_algebra (2 demos)
- scanline_boolean
- scanline_boolean2

### Missing: trans_single_path (2 demos)
- trans_curve1
- trans_curve2

### Missing: renderer_mclip (2 demos)
- alpha_mask2
- multi_clip

### Missing: span_interpolator_persp (1 demo)
- image_resample

### Missing: trans_warp_magnifier (2 demos)
- distortions
- lion_lens

### Missing: trans_polar (1 demo)
- trans_polar

### Missing: various single modules (6 demos)
- blend_color (pixfmt_rgb, multiple pixel formats)
- component_rendering (pixfmt_gray)
- gamma_ctrl (gamma_ctrl widget)
- gouraud_mesh (mesh data structures)
- polymorphic_renderer (pixfmt_rgb)
- raster_text (embedded_raster_fonts)

## Not Applicable for WASM (11)

| Demo | Reason |
|------|--------|
| freetype_test | System font dependency (FreeType) |
| truetype_test | System font dependency (TrueType) |
| GDI_graph_test | Windows GDI specific |
| gdip_curves | Windows GDI+ specific |
| svg_viewer | External SVG parser dependency |
| make_arrows | Build-time utility |
| make_gb_poly | Build-time utility |
| parse_lion | Build-time utility |
| interactive_polygon | Internal utility class |
| gpc_test | Excluded (non-commercial GPC license) |
| ideaDA | Just an HTML page, not a demo |

## Summary

| Category | Count |
|----------|-------|
| Implemented | 34 |
| Can port now | 0 |
| Deferred (missing modules) | 30 |
| Not applicable | 11 |
| **Total C++ demos** | **75** |

## Module Porting Priority (to unblock deferred demos)

| Module | Demos Unblocked | Complexity |
|--------|----------------|------------|
| rasterizer_outline_aa + line_profile_aa | 4 | HIGH |
| rasterizer_compound_aa + comp_op | 5 | VERY HIGH |
| stack_blur + recursive_blur | 2 | MED |
| span_pattern | 3 | MED |
| scanline_boolean_algebra | 2 | HIGH |
| trans_single_path | 2 | MED |
| renderer_mclip | 2 | MED |
| pixfmt_rgb + pixfmt_gray | 3 | MED |
| trans_warp_magnifier | 2 | LOW |
| trans_polar | 1 | LOW |
| embedded_raster_fonts | 1 | LOW |
| gamma_ctrl widget | 1 | LOW |
| span_interpolator_persp | 1 | MED |
| mesh data structures | 1 | HIGH |

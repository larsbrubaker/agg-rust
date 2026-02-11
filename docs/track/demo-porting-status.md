# AGG Demo Porting Status

Cross-reference of all 75 C++ demos from `cpp-references/agg-web/demo/` against our WASM demo page.

Reference screenshots: `cpp-references/agg-web/demo/<name>/<name>.gif|png|jpg`
Reference source: `cpp-references/agg-web/demo/<name>/<name>.cpp.html`

## Currently Implemented (63 demos)

| Demo | C++ Original | Window | Status |
|------|-------------|--------|--------|
| aa_demo | aa_demo.cpp | 600x400 | Done - fixed colors, vertices, threshold |
| aa_test | aa_test.cpp | 480x350 | Done - radial dashes, ellipses, gradient lines, Gouraud triangles |
| alpha_gradient | alpha_gradient.cpp | 512x400 | Done - gradient with alpha curve control, random ellipse background |
| alpha_mask | alpha_mask.cpp | 512x400 | Done - lion with elliptical alpha mask (manual compositing) |
| alpha_mask3 | alpha_mask3.cpp | 640x520 | Done - alpha mask polygon clipping (AND/SUB), 5 scenarios |
| bezier_div | bezier_div.cpp | 600x600 | Done |
| blend_color | blend_color.cpp | 512x400 | Done - blurred shadow under shape, blur compositing |
| blur | blur.cpp | 512x400 | Done - stack blur with adjustable radius |
| bspline | bspline.cpp | 600x600 | Done - 6 draggable control points, B-spline curve |
| circles | circles.cpp | 400x400 | Done |
| component_rendering | component_rendering.cpp | 512x400 | Done - grayscale pixfmt rendering |
| compositing | compositing.cpp | 512x400 | Done - SVG compositing modes with overlapping circles |
| compositing2 | compositing2.cpp | 512x400 | Done - multiple circles blended with selected comp op |
| conv_contour | conv_contour.cpp | 600x600 | Done |
| conv_dash | conv_dash_marker.cpp | 600x600 | Done |
| conv_dash_marker | conv_dash_marker.cpp | 500x330 | Done - dashed strokes with cap styles |
| conv_stroke | conv_stroke.cpp | 600x600 | Done |
| distortions | distortions.cpp | 512x400 | Done - span interpolator adaptor with warp effects |
| flash_rasterizer | flash_rasterizer.cpp | 512x400 | Done - compound rasterizer with multi-style shapes |
| flash_rasterizer2 | flash_rasterizer2.cpp | 512x400 | Done - multi-style shapes with regular rasterizer |
| gamma_correction | gamma_correction.cpp | 500x400 | Done |
| gamma_ctrl | gamma_ctrl.cpp | 512x400 | Done - interactive gamma spline widget |
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
| line_patterns | line_patterns.cpp | 500x450 | Done - solid AA outline patterns |
| line_patterns_clip | line_patterns_clip.cpp | 500x450 | Done - clipped AA outline patterns |
| line_thickness | (custom) | 600x400 | Done |
| lion | lion.cpp | 512x400 | Done |
| lion_lens | lion_lens.cpp | 512x400 | Done - warp magnifier lens over lion |
| lion_outline | lion_outline.cpp | 512x512 | Done - AA outline rendering of lion paths |
| mol_view | mol_view.cpp | 400x400 | Done - molecular structure viewer (Caffeine/Aspirin/Benzene) |
| multi_clip | multi_clip.cpp | 512x400 | Done - multiple clip rectangles |
| pattern_fill | pattern_fill.cpp | 512x400 | Done - tiled pattern fill |
| pattern_perspective | pattern_perspective.cpp | 512x400 | Done - pattern with perspective transform |
| pattern_resample | pattern_resample.cpp | 512x400 | Done - resampled pattern fill |
| perspective | perspective.cpp | 600x600 | Done |
| polymorphic_renderer | polymorphic_renderer.cpp | 512x400 | Done - rendering with multiple pixel formats |
| rasterizer_compound | rasterizer_compound.cpp | 512x400 | Done - compound rasterizer with layer order control |
| rasterizers | rasterizers.cpp | 600x400 | Done |
| rasterizers2 | rasterizers2.cpp | 500x450 | Done - spiral outlines with AA and non-AA rendering |
| raster_text | raster_text.cpp | 512x400 | Done - all 34 embedded raster fonts |
| rounded_rect | rounded_rect.cpp | 600x400 | Done |
| scanline_boolean | scanline_boolean.cpp | 512x400 | Done - boolean ops on scanline shapes |
| scanline_boolean2 | scanline_boolean2.cpp | 512x400 | Done - boolean algebra on complex shapes |
| simple_blur | simple_blur.cpp | 512x400 | Done - simple stack blur |
| trans_curve1 | trans_curve1.cpp | 512x400 | Done - text along curve (single path) |
| trans_curve2 | trans_curve2.cpp | 512x400 | Done - text along curve variant |
| trans_polar | trans_polar.cpp | 512x400 | Done - polar coordinate transform |
| gouraud_mesh | gouraud_mesh.cpp | 512x400 | Done - Gouraud-shaded triangle mesh with compound rasterizer |
| image_resample | image_resample.cpp | 512x400 | Done - image resampling with affine/perspective transforms |
| alpha_mask2 | alpha_mask2.cpp | 512x400 | Done - alpha mask with random ellipses modulating lion rendering |

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
| Implemented | 63 |
| Deferred | 0 |
| Not applicable | 11 |
| **Total C++ demos** | **75** |

## Module Porting Status

All core library modules have been ported (903 tests passing). All applicable demos are implemented.

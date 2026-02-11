# Port All 78 AGG C++ Examples as Interactive WASM Demos

## Context

Phase 7 complete: 61 modules, 731 tests, 6 basic demos working. The user wants ALL examples from `cpp-references/agg-src/examples/` ported exactly as interactive WASM demos with mouse controls matching the C++ originals.

## Scope: 78 C++ examples → 35 portable now, 25 deferred, 12 not applicable, 6 excluded

## Architecture

**Stateless WASM pattern (unchanged):** JS owns all state → passes `params: f64[]` → WASM renders RGBA pixels → JS displays on canvas. Mouse interaction computed in JS, passed as params.

**Key files to modify:**
- `demo/wasm/src/render.rs` — all Rust render functions (currently 6, will grow to ~35)
- `demo/wasm/src/lib.rs` — dispatch match + new modules
- `demo/src/render-canvas.ts` — add checkbox, radio group, button controls
- `demo/src/mouse-helpers.ts` — NEW: reusable mouse drag/rotate helpers
- `demo/src/main.ts` — add all demo module imports
- `demo/index.html` — categorized sidebar navigation
- `demo/src/demos/*.ts` — one TS file per demo

## Phase A: TypeScript Infrastructure

### A1. Add controls to `render-canvas.ts`
```typescript
addCheckbox(sidebar, label, initial, onChange): HTMLInputElement
addRadioGroup(sidebar, label, options, initialIndex, onChange): void
```

### A2. New `demo/src/mouse-helpers.ts`
Two reusable patterns covering all demos:

**Pattern 1 — Vertex dragging** (aa_demo, gouraud, conv_stroke, rounded_rect, bspline, etc.):
```typescript
setupVertexDrag({ canvas, vertices: {x,y}[], threshold: 10, onDrag: () => void }): cleanup
```
On mousedown find nearest vertex within threshold, on mousemove update position, call onDrag.

**Pattern 2 — Rotate/scale from center** (lion, gradients):
```typescript
setupRotateScale({ canvas, onLeft: (cx,cy) => void, onRight: (angle, scale) => void }): cleanup
```
Left-drag translates center, right-drag computes angle+scale via atan2/distance.

### A3. Update sidebar to categorized sections
```
Anti-Aliasing: aa_demo, aa_test, rasterizers, gamma_correction, gamma_tuner
Rendering: lion, perspective, circles
Gradients: gradients, gradient_focal, alpha_gradient
Gouraud: gouraud
Paths & Strokes: conv_stroke, conv_dash_marker, conv_contour, line_thickness
Curves: curves, bezier_div, bspline
Images: image1, image_filters, image_filters2, image_fltr_graph, image_perspective, image_transforms, image_alpha
Shapes: rounded_rect, idea, mol_view
Alpha: alpha_mask, alpha_mask3
Advanced: graph_test
```

## Phase B: Rewrite Existing 6 Demos to Match C++ Originals

### B1. `lion` — add mouse rotate/scale/skew + alpha
- Params: `[angle_rad, scale, skew_x, skew_y, alpha]`
- TS: Pattern 2 (left=rotate/scale, right=skew), alpha slider
- Rust: add skew via `trans_affine_skewing`, multiply alpha into lion colors

### B2. `gouraud` — 3 draggable vertices, 6 sub-triangles
- Params: `[x0,y0, x1,y1, x2,y2, dilation, gamma, alpha]`
- TS: Pattern 1 (3 vertices + drag-all), dilation/gamma/alpha sliders
- Rust: compute centroid, render 6 sub-triangles matching C++ exactly

### B3. `gradients` — 6 gradient types + mouse center/rotate
- Params: `[cx, cy, angle, scale, gradient_type, scale_x, scale_y]`
- TS: Pattern 2 (left=translate, right=rotate/scale), radio group for type
- Rust: GradientReflectAdaptor wrapping each of 6 gradient types, 256-entry color profile

### B4. `conv_stroke` (replaces "strokes") — vertex drag + join/cap controls
- Params: `[x0,y0, x1,y1, x2,y2, join_type, cap_type, width, miter_limit]`
- TS: Pattern 1 (3 vertices), radio groups for join+cap, width+miter sliders

### B5. `curves` → `bezier_div` — 4 draggable control points
- Params: `[x1,y1, x2,y2, x3,y3, x4,y4, width, show_pts, show_outline]`
- TS: Pattern 1 (4 vertices), width slider, show_pts + show_outline checkboxes

### B6. `shapes` — removed (custom demo, not a C++ original)

## Phase C: New Demos (batched by similarity)

### Batch C1: Slider-only demos (no mouse drag)
| Demo | C++ file | Params | Key types |
|------|----------|--------|-----------|
| gamma_correction | gamma_correction.cpp | `[thickness, contrast, gamma]` | gamma_lut, conv_stroke |
| gamma_tuner | gamma_tuner.cpp | `[gamma]` | gamma_lut |
| aa_test | aa_test.cpp | `[technique]` | rasterizer_scanline_aa, span_gradient |
| circles | circles.cpp | `[z_min, z_max, size, seed]` | Ellipse, seeded RNG |
| idea | idea.cpp | `[scale, outline]` | path_storage (embedded data), conv_stroke |

### Batch C2: Vertex-drag demos A
| Demo | C++ file | Params | Vertices |
|------|----------|--------|----------|
| aa_demo | aa_demo.cpp | `[x0,y0,x1,y1,x2,y2, pixel_size]` | 3 |
| conv_contour | conv_contour.cpp | `[close_type, width, auto_detect]` | 0 (radio+slider) |
| conv_dash_marker | conv_dash_marker.cpp | `[x0..y2, cap, width, close, even_odd]` | 3 |
| rasterizers | rasterizers.cpp | `[x0..y2, gamma, alpha]` | 3 |
| rounded_rect | rounded_rect.cpp | `[x0,y0,x1,y1, radius, offset, dark]` | 2 |

### Batch C3: Vertex-drag demos B
| Demo | C++ file | Params | Vertices |
|------|----------|--------|----------|
| alpha_gradient | alpha_gradient.cpp | `[x0..y2, spread]` | 3 |
| alpha_mask | alpha_mask.cpp | `[mask_type, angle, scale]` | 0 |
| alpha_mask3 | alpha_mask3.cpp | `[text_size, mask_type]` | 0 |
| bspline | bspline.cpp | `[n, x0..yn, closed]` | N |
| line_thickness | line_thickness.cpp | `[x0,y0,x1,y1]` | 2 |

### Batch C4: Gradient & advanced rendering
| Demo | C++ file | Params |
|------|----------|--------|
| gradient_focal | gradient_focal.cpp | `[cx,cy, fx,fy, radius]` |
| graph_test | graph_test.cpp | `[seed, n_nodes, n_edges]` |
| mol_view | mol_view.cpp | `[z_min, z_max, size]` |
| image_fltr_graph | image_fltr_graph.cpp | `[filter_type, radius]` |

### Batch C5: Image demos (need procedural test image)
| Demo | C++ file | Params |
|------|----------|--------|
| image1 | image1.cpp | `[angle, scale]` |
| image_filters | image_filters.cpp | `[filter_type, angle, scale]` |
| image_filters2 | image_filters2.cpp | `[filter_kernel, radius, angle]` |
| image_alpha | image_alpha.cpp | `[alpha, angle, scale]` |
| image_transforms | image_transforms.cpp | `[angle, scale, filter_type]` |
| image_perspective | image_perspective.cpp | `[qx0..qy3, transform_type]` |
| perspective | perspective.cpp | `[qx0..qy3, transform_type]` |

## Phase D: Procedural Test Images

New file `demo/wasm/src/test_images.rs`:
1. **Checkerboard** — 8x8 grid, warm/cool alternating colors (256x256)
2. **Gradient sphere** — radial gradient bright→dark (256x256)
3. **Color wheel** — HSL wheel for filter quality testing (256x256)

Generated once, stored as `Vec<u8>` RGBA via `std::sync::OnceLock`.

## Deferred Demos (25 — need missing Rust modules)

| Demo | Blocking module |
|------|----------------|
| blur, simple_blur | stack_blur, recursive_blur |
| lion_outline | renderer_outline_aa, line_profile_aa |
| lion_lens | trans_warp_magnifier |
| distortions | span_image_filter_rgb |
| line_patterns, line_patterns_clip | rasterizer_outline_aa |
| pattern_fill, pattern_perspective, pattern_resample | span_pattern |
| compositing, compositing2 | rasterizer_compound_aa, comp_op |
| flash_rasterizer, flash_rasterizer2 | rasterizer_compound_aa |
| scanline_boolean, scanline_boolean2 | scanline_boolean_algebra |
| trans_curve1, trans_curve2 | trans_single_path, fonts |
| trans_polar | trans_polar |
| raster_text | embedded_raster_fonts |
| multi_clip, alpha_mask2 | renderer_mclip |
| component_rendering | pixfmt_gray |
| polymorphic_renderer | pixfmt_rgb |
| gamma_ctrl | gamma_ctrl widget |
| gradients_contour | span_gradient_contour |
| image_resample | span_interpolator_persp |
| rasterizers2, bezier_div (partial) | rasterizer_outline_aa |
| gouraud_mesh | mesh data structures |
| blend_color | multiple pixel formats |

## Not Applicable (12)
StdAfx, make_arrows, make_gb_poly, parse_lion, interactive_polygon, pure_api, svg_test, agg_svg_*, agg2d_demo, freetype_test, truetype_test, trans_curve*_ft, gpc_test (excluded license)

## Verification

1. `cargo test` — 731 tests still pass
2. `wasm-pack build wasm --target web --release` — WASM compiles
3. `bun run build.ts` — TS bundles
4. Manual browser test: each demo loads, controls work, render time <50ms
5. Each demo visually matches C++ reference screenshots

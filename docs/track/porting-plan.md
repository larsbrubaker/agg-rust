# AGG 2.6 Rust Port - Project Setup & Incremental Porting Plan

## Context

We're porting Anti-Grain Geometry (AGG) 2.6, a legendary C++ 2D vector graphics rendering library by Maxim Shemanarev, to pure Rust. This follows our successful port of Clipper2 to Rust (clipper2-rust), reusing the same project structure, tooling, agents, and quality philosophy. The goal is a pixel-perfect, dependency-ordered port with a full interactive WASM demo website reproducing every original AGG demo.

**License**: AGG 2.6 is based on AGG 2.4 which is dual-licensed under **Modified BSD** and the AGG Public License. AGG 2.5 (GPL) was a license-only change with no code differences from 2.4 - its code never made it into 2.6. Our Rust port will use the **Modified BSD License (3-clause)**, which is fully permissive and compatible with MIT. The GPC (General Polygon Clipper) component has a non-commercial license and will be **excluded entirely** from this port - we'll use AGG's own `scanline_boolean_algebra` instead (or clipper2-rust as a dependency if needed).

**Scope**: 117 header files, 26 .cpp implementation files, ~78 demo programs, producing a complete Rust crate with WASM demos.

---

## Phase 0: Project Setup (THIS SESSION)

### Step 1: Initialize Git Repository

```
cd C:\Development\agg-rust
git init
```

### Step 2: Create Directory Structure

```
agg-rust/
├── .claude/
│   ├── agents/
│   │   ├── rust-expert.md        (adapted from clipper2-rust)
│   │   ├── fix-test-failures.md  (adapted from clipper2-rust)
│   │   ├── code-reviewer.md      (adapted from clipper2-rust)
│   │   └── test-writer.md        (adapted from clipper2-rust)
│   ├── skills/
│   │   ├── checkin/SKILL.md      (adapted from clipper2-rust)
│   │   └── fix-test-failures/SKILL.md (adapted from clipper2-rust)
│   └── settings.local.json      (adapted permissions)
├── .github/
│   └── workflows/
│       └── deploy-demo.yml      (GitHub Pages deployment)
├── cpp-references/              (ALREADY EXISTS - keep as-is)
│   ├── README.md
│   ├── agg-doc/
│   ├── agg-src/
│   └── agg-web/
├── src/
│   └── lib.rs                   (initial module declarations)
├── demo/
│   ├── wasm/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── src/
│   │   ├── main.ts
│   │   ├── wasm.ts
│   │   ├── canvas.ts
│   │   ├── controls.ts
│   │   └── demos/               (one .ts per demo page)
│   ├── styles/
│   │   └── main.css
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── server.ts
│   ├── build.ts
│   ├── build-wasm.ps1
│   └── build-wasm.sh
├── scripts/
│   ├── pre-commit-check.ps1
│   └── pre-commit-check.sh
├── tests/                       (integration tests)
├── benches/                     (criterion benchmarks)
├── examples/                    (runnable examples)
├── Cargo.toml                   (workspace)
├── Cargo.lock
├── CLAUDE.md
├── README.md
├── LICENSE                      (Modified BSD 3-clause)
└── .gitignore
```

### Step 3: Create Root Cargo.toml

```toml
[package]
name = "agg-rust"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
license = "BSD-3-Clause"
description = "Pure Rust port of Anti-Grain Geometry (AGG) 2.6 - high quality 2D vector graphics rendering"
repository = "https://github.com/anthropics/agg-rust"
keywords = ["graphics", "rendering", "anti-aliasing", "2d", "vector"]
categories = ["graphics", "rendering"]
exclude = ["demo/", "docs/", "tests/", ".github/", ".claude/", "benches/", "scripts/", "cpp-references/"]

[workspace]
members = [".", "demo/wasm"]

[dependencies]
# No external dependencies for core library (matching AGG's zero-dependency philosophy)

[dev-dependencies]
criterion = "0.5"
```

### Step 4: Create LICENSE (Modified BSD)

The license from `cpp-references/agg-src/copying` adapted for the Rust port, crediting both the original work and this port.

### Step 5: Adapt .claude/ from clipper2-rust

All 4 agents and 2 skills adapted with:
- "clipper2-rust" → "agg-rust"
- "Clipper2" → "AGG 2.6"
- "polygon clipping" → "2D vector graphics rendering"
- C++ reference URL updated to the AGG source in cpp-references/
- Type mapping table updated for AGG types (rgba8, trans_affine, path_storage, etc.)
- AGG-specific porting patterns (C++ templates → Rust traits/generics)

### Step 6: Create CLAUDE.md

Adapted from clipper2-rust with AGG-specific context:
- Same philosophy (quality through iterations, no stubs, exact behavioral matching)
- AGG-specific testing guidance (pixel-perfect output comparison)
- AGG module organization reference
- Template-to-trait porting guidelines

### Step 7: Create .gitignore

```
/target
*.svg
*.png
*.bmp
node_modules/
demo/dist/
demo/public/pkg/
```

### Step 8: Create .github/workflows/deploy-demo.yml

Same structure as clipper2-rust (Rust + wasm-pack + Bun + GitHub Pages).

### Step 9: Create scripts/

Pre-commit check scripts adapted from clipper2-rust (PowerShell + bash).

### Step 10: Create initial src/lib.rs

Skeleton with module declarations for Phase 1 modules only.

### Step 11: Create GitHub Repository

```bash
gh repo create agg-rust --public --source=. --description "Pure Rust port of Anti-Grain Geometry (AGG) 2.6 - high quality 2D vector graphics rendering"
```

Repository: personal account, BSD-3-Clause license.

### Step 12: Initial Commit & Push

Commit all setup files, push to main.

---

## Porting Strategy: Rust Module Organization

### Key Architectural Decision: Templates → Traits

AGG's C++ uses heavy template metaprogramming. Our Rust approach:

| C++ Pattern | Rust Approach |
|------------|---------------|
| `template<class ColorT>` pixel formats | `trait Color` + generic `PixelFormat<C: Color>` |
| `template<class VertexSource>` converters | `trait VertexSource` + generic converters |
| `template<class Scanline>` renderers | `trait Scanline` + generic renderers |
| `template<class Rasterizer>` render functions | Generic functions with trait bounds |
| Component ordering (rgba, argb, bgra) | `trait ComponentOrder` with const generics or associated types |
| Gamma functions | `trait GammaFunction` |

### Rust Module Map

```
src/
├── lib.rs                    // Public API, module declarations
├── basics.rs                 // agg_basics.h: path commands, point/rect types, constants
├── math.rs                   // agg_math.h: geometric math utilities
├── array.rs                  // agg_array.h: pod_vector, pod_array, vertex_sequence
├── color.rs                  // agg_color_rgba.h + agg_color_gray.h: rgba, rgba8, gray8
├── gamma.rs                  // agg_gamma_functions.h + agg_gamma_lut.h
├── rendering_buffer.rs       // agg_rendering_buffer.h: row_accessor, row_ptr_cache
├── path_storage.rs           // agg_path_storage.h: vertex storage, path commands
├── trans_affine.rs           // agg_trans_affine.h: affine transformation matrix
├── trans_perspective.rs      // agg_trans_perspective.h
├── trans_bilinear.rs         // agg_trans_bilinear.h
├── trans_viewport.rs         // agg_trans_viewport.h
├── curves.rs                 // agg_curves.h: Bezier curve flattening
├── arc.rs                    // agg_arc.h: circular arc generation
├── ellipse.rs                // agg_ellipse.h: ellipse vertex generation
├── rounded_rect.rs           // agg_rounded_rect.h
├── arrowhead.rs              // agg_arrowhead.h
├── bspline.rs                // agg_bspline.h
├── gsv_text.rs               // agg_gsv_text.h: built-in vector font
├── conv_stroke.rs            // agg_conv_stroke.h + agg_vcgen_stroke.h
├── conv_dash.rs              // agg_conv_dash.h + agg_vcgen_dash.h
├── conv_curve.rs             // agg_conv_curve.h
├── conv_contour.rs           // agg_conv_contour.h + agg_vcgen_contour.h
├── conv_transform.rs         // agg_conv_transform.h
├── conv_marker.rs            // agg_conv_marker.h + agg_vcgen_markers_term.h
├── conv_smooth.rs            // agg_conv_smooth_poly1.h + agg_vcgen_smooth_poly1.h
├── conv_bspline.rs           // agg_conv_bspline.h + agg_vcgen_bspline.h
├── conv_clip.rs              // agg_conv_clip_polygon.h + agg_conv_clip_polyline.h
├── conv_adaptor.rs           // agg_conv_adaptor_vcgen.h + agg_conv_adaptor_vpgen.h
├── vpgen.rs                  // agg_vpgen_*.h: viewport generators
├── dda_line.rs               // agg_dda_line.h: DDA line algorithm
├── clip_liang_barsky.rs      // agg_clip_liang_barsky.h
├── rasterizer_cells.rs       // agg_rasterizer_cells_aa.h: cell-based AA
├── rasterizer_scanline.rs    // agg_rasterizer_scanline_aa.h: main rasterizer
├── rasterizer_outline.rs     // agg_rasterizer_outline.h + agg_rasterizer_outline_aa.h
├── rasterizer_compound.rs    // agg_rasterizer_compound_aa.h
├── scanline.rs               // agg_scanline_u.h + agg_scanline_p.h + agg_scanline_bin.h
├── scanline_boolean.rs       // agg_scanline_boolean_algebra.h (replaces GPC!)
├── scanline_storage.rs       // agg_scanline_storage_aa.h + _bin.h
├── pixfmt/
│   ├── mod.rs                // Pixel format traits
│   ├── rgb.rs                // agg_pixfmt_rgb.h
│   ├── rgba.rs               // agg_pixfmt_rgba.h (all 30+ compositing modes)
│   ├── gray.rs               // agg_pixfmt_gray.h
│   ├── rgb_packed.rs         // agg_pixfmt_rgb_packed.h (rgb555, rgb565)
│   ├── amask_adaptor.rs      // agg_pixfmt_amask_adaptor.h
│   └── transposer.rs         // agg_pixfmt_transposer.h
├── renderer_base.rs          // agg_renderer_base.h: clipping base renderer
├── renderer_scanline.rs      // agg_renderer_scanline.h: render_scanlines()
├── renderer_primitives.rs    // agg_renderer_primitives.h: Bresenham lines, rectangles
├── renderer_outline.rs       // agg_renderer_outline_aa.h + agg_renderer_outline_image.h
├── renderer_markers.rs       // agg_renderer_markers.h: marker symbols
├── renderer_mclip.rs         // agg_renderer_mclip.h: multi-clip regions
├── renderer_raster_text.rs   // agg_renderer_raster_text.h
├── span_allocator.rs         // agg_span_allocator.h
├── span_solid.rs             // agg_span_solid.h (trivial)
├── span_gradient.rs          // agg_span_gradient.h + gradient functions
├── span_gouraud.rs           // agg_span_gouraud.h + _rgba.h + _gray.h
├── span_image_filter.rs      // agg_span_image_filter.h + _rgba.h + _rgb.h + _gray.h
├── span_interpolator.rs      // agg_span_interpolator_linear.h + _persp.h + _trans.h
├── span_pattern.rs           // agg_span_pattern_rgba.h + _rgb.h + _gray.h
├── span_converter.rs         // agg_span_converter.h
├── span_subdiv_adaptor.rs    // agg_span_subdiv_adaptor.h
├── image_accessors.rs        // agg_image_accessors.h
├── image_filters.rs          // agg_image_filters.h: filter kernels
├── gradient_lut.rs           // agg_gradient_lut.h
├── alpha_mask.rs             // agg_alpha_mask_u8.h
├── blur.rs                   // agg_blur.h: stack blur + recursive Gaussian
├── bounding_rect.rs          // agg_bounding_rect.h
├── line_aa_basics.rs         // agg_line_aa_basics.h
├── math_stroke.rs            // agg_math_stroke.h
├── embedded_fonts.rs         // agg_embedded_raster_fonts.h (large data)
├── trans_warp_magnifier.rs   // agg_trans_warp_magnifier.h
├── trans_double_path.rs      // agg_trans_double_path.h
├── trans_single_path.rs      // agg_trans_single_path.h
├── simul_eq.rs               // agg_simul_eq.h: simultaneous equation solver
└── pattern_filters.rs        // agg_pattern_filters_rgba.h
```

---

## Porting Phases

### Phase 1: Foundation Types & Math (Est. ~2,500 lines of Rust)

**Goal**: All basic types, math utilities, and containers that everything else depends on.

**Files to port**:
- `agg_basics.h` → `basics.rs` — path commands (move_to, line_to, curve3, curve4, close, stop), point_d, rect_i/rect_d, cover constants, subpixel scale, filling_rule_e, poly_subpixel constants
- `agg_math.h` → `math.rs` — calc_distance, calc_line_point_distance, calc_intersection, cross_product, iround, uround, etc.
- `agg_array.h` → `array.rs` — pod_vector → Vec wrapper with AGG semantics, pod_array
- `agg_vertex_sequence.h` → part of `array.rs` — vertex_dist, vertex_sequence
- `agg_color_rgba.h` + `agg_color_rgba.cpp` → `color.rs` — rgba (f64), rgba8 (u8), rgba16 (u16), color arithmetic, premultiply/demultiply
- `agg_color_gray.h` → `color.rs` — gray8, gray16
- `agg_gamma_functions.h` + `agg_gamma_lut.h` → `gamma.rs` — gamma_none, gamma_power, gamma_threshold, gamma_linear, gamma_multiply, gamma_lut

**Tests**: Unit tests for every type and function, exact match with C++ behavior.

### Phase 2: Memory & Geometry Primitives (Est. ~3,000 lines)

**Goal**: Rendering buffer, path storage, transformations, basic shapes.

**Files to port**:
- `agg_rendering_buffer.h` → `rendering_buffer.rs` — row_accessor, row_ptr_cache (the rendering_buffer typedef)
- `agg_path_storage.h` → `path_storage.rs` — vertex_block_storage, path_storage with all path commands
- `agg_trans_affine.h` + `.cpp` → `trans_affine.rs` — 6-coefficient affine matrix, compose, invert, rotate, scale, translate, skew
- `agg_curves.h` + `.cpp` → `curves.rs` — curve3/curve4 (quadratic/cubic Bezier) with recursive subdivision and incremental methods
- `agg_arc.h` + `.cpp` → `arc.rs`
- `agg_ellipse.h` → `ellipse.rs`
- `agg_rounded_rect.h` + `.cpp` → `rounded_rect.rs`
- `agg_bezier_arc.h` + `.cpp` → part of `arc.rs`
- `agg_bspline.h` + `.cpp` → `bspline.rs`
- `agg_arrowhead.h` + `.cpp` → `arrowhead.rs`
- `agg_dda_line.h` → `dda_line.rs`
- `agg_clip_liang_barsky.h` → `clip_liang_barsky.rs`
- `agg_bounding_rect.h` → `bounding_rect.rs`
- `agg_math_stroke.h` → `math_stroke.rs`
- `agg_simul_eq.h` → `simul_eq.rs`

**Tests**: Geometry output validation, transformation accuracy (compare transformed points with C++ output).

### Phase 3: Scanline Rasterizer (Est. ~4,000 lines) — THE CORE

**Goal**: The heart of AGG - converting vector paths to pixel coverage data.

**Files to port**:
- `agg_rasterizer_cells_aa.h` → `rasterizer_cells.rs` — sorted cell storage with anti-aliased coverage
- `agg_rasterizer_sl_clip.h` → part of `rasterizer_scanline.rs` — scanline clipping helpers
- `agg_rasterizer_scanline_aa.h` + `_nogamma.h` → `rasterizer_scanline.rs` — THE main rasterizer
- `agg_scanline_u.h` → `scanline.rs` — scanline_u8 (unpacked)
- `agg_scanline_p.h` → `scanline.rs` — scanline_p8 (packed/RLE)
- `agg_scanline_bin.h` → `scanline.rs` — scanline_bin (aliased)
- `agg_scanline_boolean_algebra.h` → `scanline_boolean.rs` — boolean ops WITHOUT GPC
- `agg_scanline_storage_aa.h` + `_bin.h` → `scanline_storage.rs`
- `agg_line_aa_basics.h` + `.cpp` → `line_aa_basics.rs`

**Tests**: Rasterize known polygons, compare cell output and coverage values with C++. This is where pixel-perfect fidelity matters most.

### Phase 4: Pixel Formats & Renderers (Est. ~5,000 lines)

**Goal**: All pixel format implementations and the renderer stack.

**Files to port**:
- `agg_pixfmt_base.h` → `pixfmt/mod.rs` — PixelFormat trait, blender traits
- `agg_pixfmt_rgba.h` → `pixfmt/rgba.rs` — All 30+ compositing modes (src_over, dst_over, src_in, dst_in, src_out, dst_out, src_atop, dst_atop, xor, plus, multiply, screen, overlay, darken, lighten, etc.)
- `agg_pixfmt_rgb.h` → `pixfmt/rgb.rs`
- `agg_pixfmt_gray.h` → `pixfmt/gray.rs`
- `agg_pixfmt_rgb_packed.h` → `pixfmt/rgb_packed.rs`
- `agg_pixfmt_amask_adaptor.h` → `pixfmt/amask_adaptor.rs`
- `agg_pixfmt_transposer.h` → `pixfmt/transposer.rs`
- `agg_renderer_base.h` → `renderer_base.rs`
- `agg_renderer_scanline.h` → `renderer_scanline.rs` — render_scanlines() and variants
- `agg_renderer_primitives.h` → `renderer_primitives.rs`
- `agg_renderer_outline_aa.h` → `renderer_outline.rs`
- `agg_renderer_outline_image.h` → part of `renderer_outline.rs`
- `agg_renderer_markers.h` → `renderer_markers.rs`
- `agg_renderer_mclip.h` → `renderer_mclip.rs`
- `agg_alpha_mask_u8.h` → `alpha_mask.rs`
- `agg_line_profile_aa.cpp` → part of `renderer_outline.rs`

**Tests**: Render known shapes to pixel buffers, compare byte-for-byte with C++ output. Test all 30+ compositing modes.

### Phase 5: Converter Pipeline (Est. ~3,500 lines)

**Goal**: All path converters that form the coordinate conversion pipeline.

**Files to port**:
- `agg_conv_adaptor_vcgen.h` + `_vpgen.h` → `conv_adaptor.rs`
- `agg_vcgen_stroke.h` + `.cpp` → `conv_stroke.rs`
- `agg_vcgen_dash.h` + `.cpp` → `conv_dash.rs`
- `agg_vcgen_contour.h` + `.cpp` → `conv_contour.rs`
- `agg_vcgen_smooth_poly1.h` + `.cpp` → `conv_smooth.rs`
- `agg_vcgen_bspline.h` + `.cpp` → `conv_bspline.rs`
- `agg_vcgen_markers_term.h` + `.cpp` → `conv_marker.rs`
- `agg_conv_curve.h` → `conv_curve.rs`
- `agg_conv_transform.h` → `conv_transform.rs`
- `agg_conv_clip_polygon.h` + `_polyline.h` → `conv_clip.rs`
- `agg_vpgen_clip_polygon.h/.cpp` + `_polyline.h/.cpp` + `_segmentator.h/.cpp` → `vpgen.rs`
- `agg_conv_close_polygon.h` + `_unclose_polygon.h` → small utilities in `conv_adaptor.rs`
- `agg_conv_concat.h` + `_segmentator.h` + `_shorten_path.h` → small utilities
- `agg_conv_marker_adaptor.h` → part of `conv_marker.rs`

**Tests**: Stroke paths and compare vertex output with C++. Dash patterns. Contour dilation.

### Phase 6: Span Generators & Image Processing (Est. ~5,000 lines)

**Goal**: Gradients, image transforms, Gouraud shading, patterns, blur.

**Files to port**:
- `agg_span_allocator.h` → `span_allocator.rs`
- `agg_span_solid.h` → `span_solid.rs`
- `agg_span_gradient.h` + `_alpha.h` + `_contour.h` + `_image.h` → `span_gradient.rs`
- `agg_gradient_lut.h` → `gradient_lut.rs`
- `agg_span_interpolator_linear.h` + `_persp.h` + `_trans.h` + `_adaptor.h` → `span_interpolator.rs`
- `agg_span_subdiv_adaptor.h` → `span_subdiv_adaptor.rs`
- `agg_span_image_filter.h` + `_rgba.h` + `_rgb.h` + `_gray.h` → `span_image_filter.rs`
- `agg_image_filters.h` + `.cpp` → `image_filters.rs` (bilinear, bicubic, sinc, blackman, etc.)
- `agg_image_accessors.h` → `image_accessors.rs`
- `agg_span_gouraud.h` + `_rgba.h` + `_gray.h` → `span_gouraud.rs`
- `agg_span_pattern_rgba.h` + `_rgb.h` + `_gray.h` → `span_pattern.rs`
- `agg_span_converter.h` → `span_converter.rs`
- `agg_pattern_filters_rgba.h` → `pattern_filters.rs`
- `agg_blur.h` → `blur.rs` (stack blur + recursive Gaussian filter)

**Tests**: Render gradients/images/Gouraud triangles and compare pixel output.

### Phase 7: Text & Advanced Transforms (Est. ~2,000 lines)

**Goal**: Text rendering, perspective transforms, warp effects.

**Files to port**:
- `agg_gsv_text.h` + `.cpp` → `gsv_text.rs`
- `agg_embedded_raster_fonts.h/.cpp` → `embedded_fonts.rs`
- `agg_glyph_raster_bin.h` → part of `embedded_fonts.rs`
- `agg_font_cache_manager.h` → `font_cache.rs` (without platform-specific font engines)
- `agg_renderer_raster_text.h` → `renderer_raster_text.rs`
- `agg_trans_perspective.h` → `trans_perspective.rs`
- `agg_trans_bilinear.h` → `trans_bilinear.rs`
- `agg_trans_viewport.h` → `trans_viewport.rs`
- `agg_trans_warp_magnifier.h/.cpp` → `trans_warp_magnifier.rs`
- `agg_trans_double_path.h/.cpp` → `trans_double_path.rs`
- `agg_trans_single_path.h/.cpp` → `trans_single_path.rs`
- `agg_rasterizer_outline.h` + `_aa.h` → `rasterizer_outline.rs`
- `agg_rasterizer_compound_aa.h` → `rasterizer_compound.rs`
- `agg_sqrt_tables.cpp` → part of `math.rs` or `line_aa_basics.rs`

**Tests**: Text rendering output, perspective transform point mapping, warp magnifier distortion.

### Phase 8: WASM Demo Website (Est. ~8,000 lines TypeScript + Rust bindings)

**Goal**: Interactive web demos reproducing ALL original AGG demos.

**Demo categories** (matching original antigrain.com):

1. **Anti-Aliasing & Gamma** (6 demos): aa_demo, aa_test, gamma_correction, gamma_ctrl, gamma_tuner, rounded_rect
2. **Core Rendering** (3 demos): lion, lion_outline, idea
3. **Rasterization & Clipping** (6 demos): rasterizers, rasterizers2, scanline_boolean, scanline_boolean2, component_rendering, multi_clip
4. **Gradients & Color** (5 demos): gradients, gradient_focal, alpha_gradient, gouraud, gouraud_mesh
5. **Alpha Masking** (3 demos): alpha_mask, alpha_mask2, alpha_mask3
6. **Image Processing** (8 demos): image1, image_alpha, image_filters, image_filters2, image_fltr_graph, image_transforms, image_perspective, image_resample
7. **Distortions** (4 demos): distortions, lion_lens, trans_polar, perspective
8. **Patterns** (4 demos): pattern_fill, line_patterns, line_patterns_clip, pattern_perspective, pattern_resample
9. **Text** (4 demos): raster_text, freetype_test, trans_curve1, trans_curve2
10. **Compositing** (3 demos): compositing, compositing2, polymorphic_renderer
11. **Shapes & Curves** (6 demos): circles, graph_test, conv_contour, conv_dash_marker, conv_stroke, bezier_div
12. **Specialized** (6 demos): mol_view, simple_blur, blur, flash_rasterizer, flash_rasterizer2, rasterizer_compound

Each demo will have:
- Interactive canvas with draggable control points (matching original AGG interaction)
- Real-time rendering via WASM
- Parameter controls (sliders, dropdowns)
- Code snippet showing how to use the Rust API
- Side-by-side comparison with C++ original (where possible)

---

## Verification Strategy

### Per-Phase Testing
1. **Unit tests** for every function, comparing output with C++ reference
2. **Pixel buffer comparison** tests: render to buffer in both C++ and Rust, compare byte-for-byte
3. **Visual regression tests**: render demo scenes, compare PNG output

### Integration Testing
- Build C++ reference executables that output pixel data for known inputs
- Rust tests read the same inputs, produce output, and compare

### WASM Testing
- `wasm-pack test` for WASM-specific behavior
- Manual visual testing of all demo pages

### Pre-commit Checks
- `cargo test` (all unit + integration tests must pass)
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- File length validation (1000-line limit per file)

### CI/CD
- GitHub Actions runs full test suite on every push
- Demo site auto-deploys on main branch push

---

## Files to Create in This Session

1. `Cargo.toml` (workspace root)
2. `src/lib.rs` (initial skeleton)
3. `LICENSE` (Modified BSD 3-clause)
4. `CLAUDE.md` (adapted from clipper2-rust)
5. `README.md` (initial project description)
6. `.gitignore`
7. `.claude/agents/rust-expert.md`
8. `.claude/agents/fix-test-failures.md`
9. `.claude/agents/code-reviewer.md`
10. `.claude/agents/test-writer.md`
11. `.claude/skills/checkin/SKILL.md`
12. `.claude/skills/fix-test-failures/SKILL.md`
13. `.claude/settings.local.json`
14. `.github/workflows/deploy-demo.yml`
15. `scripts/pre-commit-check.ps1`
16. `scripts/pre-commit-check.sh`
17. `demo/wasm/Cargo.toml` (skeleton)
18. `demo/wasm/src/lib.rs` (skeleton)
19. `demo/package.json`
20. `demo/tsconfig.json`

Then: `git init`, initial commit, create GitHub repo, push.

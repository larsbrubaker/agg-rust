# agg-rust porting plan

The open work only: what a crate user or a demo visitor would notice today, and what is
needed. Finished work leaves this file (git keeps the history); the reasons behind finished
work live in the code it concerns. The C++ headers named here are in
`cpp-references/agg-src/include/`, their demos in `cpp-references/agg-src/examples/`.

## Library

- **The rasterizer takes no gamma.** `RasterizerScanlineAa` is the C++ `nogamma` variant,
  so code ported from C++ that calls `ras.gamma(...)` (nine of the C++ examples do,
  `rasterizers`, `gamma_ctrl`, `idea` and `graph_test` among them) has no equivalent and
  must remap coverage itself, as the demo renderers do. Needed: the gamma-table variant of
  `agg_rasterizer_scanline_aa.h`, with a pixel test against a C++ demo that sets gamma.
- **No B-spline path converter.** `conv_bspline` / `vcgen_bspline` are missing, so a
  path can't be smoothed through its points as a converter; the `bspline` demo samples
  `Bspline` per axis by hand instead. Needed: `agg_conv_bspline.h` and
  `agg_vcgen_bspline.h`.
- **No clip-to-rectangle converters.** `conv_clip_polygon`, `conv_clip_polyline` and
  their generators `vpgen_clip_polygon` / `vpgen_clip_polyline` are missing, so a path
  can't be clipped to a box before rasterizing (the C++ `line_patterns` demo uses it).
  Needed: those four headers.
- **No terminal markers.** `vcgen_markers_term` and `conv_marker_adaptor` are missing,
  so arrowheads can't be placed at path ends through `ConvDash`/`ConvStroke` the way C++
  does. The `conv_dash_marker` demo rebuilds the marker positions by hand, and the
  `graph_test` demo draws its edges without arrowheads. Needed: both headers, then the
  arrowheads in `graph_test`.
- **Missing span generators.** `span_gradient_alpha` (the `alpha_gradient` demo
  emulates it), `span_gradient_image`, `span_gradient_contour`, `span_pattern_rgb`, and
  the grey-scale `span_image_filter_gray`, `span_pattern_gray` and `span_gouraud_gray`
  have no Rust counterpart. Needed: those headers, each with a test against the C++.
- **Missing small pieces.** `pixfmt_rgb_packed` (RGB 555/565 and the other packed
  formats), `conv_close_polygon`, `conv_unclose_polygon`, `conv_concat`,
  `conv_shorten_path` (only the `shorten_path` function exists), `path_length`,
  `path_storage_integer` and `rendering_buffer_dynarow` have no Rust counterpart.
  Needed: each header, with its tests.
- **The resample generators read past the span they request.** `SpanImageResampleRgbaAffine`,
  `SpanImageResampleRgba`, `SpanImageResampleRgbAffine` and `SpanImageResampleRgb` pass
  `len_x_lr` to `ImageSource::span`, then step `next_x` until the filter weight runs out,
  which with `rx_inv` rounded down can be more pixels than that (one more at moderate
  scales, such as bilinear at scale 2.5; possibly more at large downscales), so at the right
  edge of the image they read past the row they asked for. The accessors tolerate it:
  `ImageAccessorClone` clamps, `ImageAccessorClip` returns the background,
  `ImageAccessorWrap` wraps, and `ImageAccessorNoClip` panics when the stride has no
  padding and otherwise reads the padding bytes as a pixel. Needed: each generator asking
  `span` for the number of pixels it actually reads, with a test that no span steps past its
  `len`.
- **Removing the remaining `unsafe`.** The crate denies `unsafe_code`, and three places
  kept from the port still use it, each under an `#[expect(unsafe_code)]`. When all three
  are safe, `src/lib.rs` changes the `deny` to `#![forbid(unsafe_code)]`.
  1. *The pixel buffer.* `RowAccessor` and `RowPtrCache` (`src/rendering_buffer.rs`) keep
     a raw pointer with no lifetime, so `attach`, `new_with_buf` and `row_ptr` are
     `unsafe fn` and every user of the library writes `unsafe` to attach a buffer. Needed:
     a buffer that borrows `&mut [u8]` (or owns its pixels) with a lifetime, checks
     `height * |stride| <= len` when built, and gives the pixel formats their rows as
     slices. This changes the public API that the demos, tools, tests, agg-gui and
     MatterCAD use.
  2. *The vertical pass of `stack_blur_rgba32`* (`src/blur.rs`) walks columns with raw
     pointers. Needed: a safe version that is as fast.
  3. *`FastRow`* (`src/image_accessors.rs`), the cached-pointer fast path of
     `ImageAccessorClip` and `ImageAccessorClone`. Needed: a safe version that is as fast;
     the checked-slice attempt measured about 28% slower on `image_perspective`, so it
     needs a better design, such as holding the row as a slice.

## Demo site

- **The Line Thickness link opens "Page not found".** The sidebar links `#/line_thickness`
  and the WASM side still renders it, but the page script and its route in
  `demo/src/main.ts` are gone. Needed: a `demo/src/demos/line_thickness.ts` page with the
  controls of `line_thickness.cpp`, and its route.
- **No `gradients_contour` demo.** It is the one C++ example with neither a page nor a
  reason to leave it out. Needed: `span_gradient_contour` (above), then the page.

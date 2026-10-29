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

## Demo site

- **The Line Thickness link opens "Page not found".** The sidebar links `#/line_thickness`
  and the WASM side still renders it, but the page script and its route in
  `demo/src/main.ts` are gone. Needed: a `demo/src/demos/line_thickness.ts` page with the
  controls of `line_thickness.cpp`, and its route.
- **No `gradients_contour` demo.** It is the one C++ example with neither a page nor a
  reason to leave it out. Needed: `span_gradient_contour` (above), then the page.
- **Demo pages show no Rust code and no C++ picture.** A visitor sees the canvas and its
  controls, but not how to call the API from Rust nor the C++ original's picture beside
  it (the C++ pictures appear only as thumbnails on the home page). Needed: a code
  snippet and the reference picture on each demo page.

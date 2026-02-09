# Pixel-Perfect Demo Porting Checklist

Lessons learned from achieving byte-for-byte pixel matching on the `lion_outline` demo.

## The Comparison Infrastructure

The `pixel-compare` tool (`tools/pixel-compare/`) renders both Rust and C++ demos headlessly and compares raw RGBA output.

```bash
# Render a demo natively (Rust)
cargo run -p pixel-compare -- render <demo_name> <width> <height> -o output_rust.raw [params...]

# Render the same demo via C++ reference
.\tools\cpp-renderer\build\Release\agg-render.exe <demo_name> <width> <height> output_cpp.raw [params...]

# Compare the two
cargo run -p pixel-compare -- compare output_cpp.raw output_rust.raw
```

The compare command reports IDENTICAL or shows pixel diff stats (count, percentage, max deviation, mean deviation).

## Debugging Strategy: Work Top-Down

When the images don't match, debug from the highest level of abstraction down:

### 1. Isolate the problem area first

Use demo parameters to render subsets. For `lion_outline`, `max_paths=1` renders only the first path. `skip_controls=1` hides UI widgets. Getting a single path to match is far easier than debugging all paths at once.

### 2. Verify parameters match

Before looking at rendering code, confirm both sides see identical inputs:
- **Transform matrix** components (sx, shy, shx, sy, tx, ty)
- **Scale factor** and derived values (line width in subpixels)
- **Path data** (vertex count, first N vertex coordinates)
- **Bounding box** and base offset calculations

Add temporary `eprintln!` / `fprintf(stderr, ...)` to both Rust and C++ and compare output line-by-line. Remove all tracing when done.

### 3. Verify colors match

This was the #1 gotcha on `lion_outline`. The C++ AGG `parse_lion.cpp` stores hex colors into an `srgba8` array, but `rgb8_packed()` returns `rgba8` (linear). The C++ template system has an **implicit conversion constructor** (`rgba8T(const rgba8T<T>&)`) that applies `linear_to_sRGB()` gamma encoding when assigning across color space types. Then the renderer copies the sRGB-encoded bytes back into `rgba8` for blending — effectively a lossy roundtrip.

**Key insight**: Any place in C++ where a color is assigned between `rgba8` and `srgba8` (or vice versa) triggers a silent gamma conversion. The Rust port must replicate these conversions explicitly. Check `parse_lion.cpp` and any demo-specific color setup for `srgba8` ↔ `rgba8` assignments.

### 4. Verify blend operations match

If colors and parameters are identical but pixels still differ, instrument the blend calls:
- Log `(type, x, y, len, covers)` for every `blend_solid_hspan` and `blend_solid_vspan`
- Compare logs with a script — filter out zero-coverage operations (they're no-ops)
- Check both the content AND the interleaved ordering of blend operations

### 5. Check mathematical precision

- **`fast_sqrt` vs `f64::sqrt`**: C++ AGG uses integer square root approximation (`fast_sqrt`) in `semidot_hline` and `pie_hline`. Rust must use the same function, not `f64::sqrt`.
- **Truncated constants**: C++ uses `0.707106781` (truncated 1/sqrt(2)) in `trans_affine::scale()`. Rust must match this exact truncation, not use `std::f64::consts::FRAC_1_SQRT_2`.
- **Line profile width**: Even a 1-subpixel difference in the smoother width (255 vs 256) cascades into visible rendering differences.

### 6. Check loop boundary conditions

C++ `step_hor`/`step_ver` in `line_interpolator_aa` check `if(m_step >= m_count)` *at the end* of the function (blend first, then check). The Rust port must place the early-return check *at the beginning* to match C++'s "do work, then decide to stop" pattern — because the C++ callers check the return value to decide whether to continue.

## Common Pitfalls

| Pitfall | Symptom | Fix |
|---------|---------|-----|
| sRGB ↔ linear color conversion | Colors visibly wrong, ~3-7 value shift per channel | Apply `linear_to_srgb()` conversion to match C++ implicit `rgba8T` conversion |
| `f64::sqrt` instead of `fast_sqrt` | Subtle anti-aliasing differences in round caps/joins | Import and use `fast_sqrt` from the AGG math module |
| Truncated float constants | Tiny cascading precision differences | Match C++ constant values exactly, digit-for-digit |
| Missing final `render()` call | Last polyline segment not drawn | Call `self.render(ren, false)` after the `add_path` vertex loop |
| Controls rendering differences | Text/slider pixels differ | Use `skip_controls` param to isolate; ensure control text matches |
| Positive vs negative stride | Image vertically flipped | Both must use same stride sign (positive = top-down) |

## C++ Renderer Setup

The C++ reference renderer lives in `tools/cpp-renderer/`. To build:

```bash
cd tools/cpp-renderer
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

It links against the original C++ AGG headers in `cpp-references/agg-src/include/`. Any debug instrumentation added to those headers affects the reference renderer's output — always clean up after debugging.

## Script-Assisted Comparison

For detailed blend-level debugging, capture all blend operations to files and use Python to compare:

```bash
# Capture blend logs
cargo run -p pixel-compare -- render demo 512 512 -o rust.raw [params] 2>rust_blends.txt
.\tools\cpp-renderer\build\Release\agg-render.exe demo 512 512 cpp.raw [params] 2>cpp_blends.txt
```

Then parse and compare with a script that:
1. Extracts `(type, x, y, len, covers)` tuples from each log
2. Filters out zero-coverage no-ops
3. Compares in interleaved order to catch ordering differences
4. Expands to per-pixel touches to identify specific diverging pixels

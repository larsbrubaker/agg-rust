// Copyright 2025. Native demo render functions for pixel comparison.
//
// These functions mirror the WASM render functions but run natively.
// They use the agg-rust core library directly.

mod image_filters;
mod lion_outline;
mod rasterizers2;

use crate::PixelBuffer;

/// Render a named demo at the given dimensions with the given parameters.
///
/// Returns None if the demo name is not recognized.
pub fn render_demo(name: &str, width: u32, height: u32, params: &[f64]) -> Option<PixelBuffer> {
    let data = match name {
        "lion_outline" => lion_outline::render(width, height, params),
        "image_filters" => image_filters::render(width, height, params),
        "rasterizers2" => rasterizers2::render(width, height, params),
        "conv_dash_marker" => agg_wasm::render_demo("conv_dash_marker", width, height, params),
        "image_perspective" => agg_wasm::render_demo("image_perspective", width, height, params),
        "image_transforms" => agg_wasm::render_demo("image_transforms", width, height, params),
        "compositing" => agg_wasm::render_demo("compositing", width, height, params),
        "compositing2" => agg_wasm::render_demo("compositing2", width, height, params),
        "perspective" => agg_wasm::render_demo("perspective", width, height, params),
        "flash_rasterizer" => agg_wasm::render_demo("flash_rasterizer", width, height, params),
        "flash_rasterizer2" => agg_wasm::render_demo("flash_rasterizer2", width, height, params),
        "truetype_test" => agg_wasm::render_demo("truetype_test", width, height, params),
        "simple_line" => render_simple_line(width, height, params),
        _ => return None,
    };
    Some(PixelBuffer { width, height, data })
}

/// List all available demo names.
pub fn available_demos() -> &'static [&'static str] {
    &[
        "lion_outline",
        "image_filters",
        "rasterizers2",
        "conv_dash_marker",
        "image_perspective",
        "image_transforms",
        "compositing",
        "compositing2",
        "perspective",
        "flash_rasterizer",
        "flash_rasterizer2",
        "truetype_test",
        "simple_line",
    ]
}

/// Render a simple line test for comparison debugging.
fn render_simple_line(width: u32, height: u32, _params: &[f64]) -> Vec<u8> {
    use agg_rust::color::Rgba8;
    use agg_rust::pixfmt_rgba::PixfmtRgba32;
    use agg_rust::rasterizer_outline_aa::{OutlineAaJoin, RasterizerOutlineAa};
    use agg_rust::renderer_base::RendererBase;
    use agg_rust::renderer_outline_aa::{LineProfileAa, RendererOutlineAa};
    use agg_rust::rendering_buffer::RowAccessor;
    use agg_rust::path_storage::PathStorage;

    let stride = (width * 4) as i32;
    let mut buf = vec![255u8; (width * height * 4) as usize];
    let mut ra = RowAccessor::new();
    unsafe { ra.attach(buf.as_mut_ptr(), width, height, stride) };
    let pf = PixfmtRgba32::new(&mut ra);
    let mut rb = RendererBase::new(pf);
    rb.clear(&Rgba8::new(255, 255, 255, 255));

    let profile = LineProfileAa::with_width(1.0);
    let mut ren_oaa = RendererOutlineAa::new(&mut rb, &profile);
    let mut ras_oaa = RasterizerOutlineAa::new();
    ras_oaa.set_round_cap(false);
    ras_oaa.set_line_join(OutlineAaJoin::Round);

    use agg_rust::trans_affine::TransAffine;
    use agg_rust::conv_transform::ConvTransform;

    // Draw closed polygons (3+ vertices) through a transform, like lion_outline
    let mut path = PathStorage::new();
    // Triangle (closed polygon - exercises the closed polygon rendering path)
    path.move_to(50.0, 50.0);
    path.line_to(150.0, 50.0);
    path.line_to(100.0, 150.0);
    path.close_polygon(0);
    // Pentagon
    path.move_to(200.0, 100.0);
    path.line_to(250.0, 70.0);
    path.line_to(280.0, 110.0);
    path.line_to(260.0, 160.0);
    path.line_to(210.0, 150.0);
    path.close_polygon(0);
    // Simple 2-vertex line (open polyline path)
    path.move_to(50.0, 200.0);
    path.line_to(200.0, 250.0);
    path.close_polygon(0);

    // Apply a transform similar to lion (rotate PI + translate)
    let mut mtx = TransAffine::new();
    mtx.multiply(&TransAffine::new_rotation(std::f64::consts::PI));
    mtx.multiply(&TransAffine::new_translation(256.0, 256.5));
    let mut transformed = ConvTransform::new(&mut path, mtx);

    ren_oaa.set_color(Rgba8::new(0, 0, 0, 255));
    ras_oaa.add_path(&mut transformed, 0, &mut ren_oaa);

    drop(ren_oaa);
    buf
}

// ============================================================================
// Shared: Lion path data parser
// ============================================================================

use agg_rust::basics::{PATH_FLAGS_CLOSE, PATH_FLAGS_CW};
use agg_rust::color::Rgba8;
use agg_rust::gamma::{linear_to_srgb, srgb_to_linear};
use agg_rust::path_storage::PathStorage;

static LION_DATA: &str = include_str!("../../../../demo/wasm/src/lion.txt");

/// Convert a linear 8-bit value to sRGB 8-bit, matching C++ AGG's sRGB_lut<int8u>.
/// C++ formula: m_inv_table[i] = uround(255.0 * linear_to_sRGB(i / 255.0))
fn linear_u8_to_srgb_u8(v: u32) -> u32 {
    if v == 0 {
        return 0;
    }
    (255.0 * linear_to_srgb(v as f64 / 255.0) + 0.5) as u32
}

/// Convert an sRGB 8-bit value back to linear 8-bit, matching C++ AGG's
/// `sRGB_lut<int8u>` direct table: `m_dir_table[i] = uround(... sRGB_to_linear ...)`.
fn srgb_u8_to_linear_u8(v: u32) -> u32 {
    if v == 0 {
        return 0;
    }
    (255.0 * srgb_to_linear(v as f64 / 255.0) + 0.5) as u32
}

/// Replicate the C++ lion demo's full color roundtrip.
///
/// The C++ demo stores each hex color as linear `rgba8` (`rgb8_packed`), assigns
/// it into an `srgba8` array (linear -> sRGB), then renders through a pixfmt whose
/// `color_type` is linear `rgba8` (`pixfmt_bgr24`), which converts the color back
/// (sRGB -> linear) before blending. The net effect is a *lossy identity*: the
/// value returns close to the original hex but is off by a bit or two due to the
/// two 8-bit rounding steps. Both halves must be applied to match byte-for-byte;
/// applying only the linear->sRGB half leaves colors too light.
fn lion_color_roundtrip(v: u32) -> u32 {
    srgb_u8_to_linear_u8(linear_u8_to_srgb_u8(v))
}

/// Parse the lion vector data into a path storage with colors and path indices.
///
/// The C++ AGG demo (parse_lion.cpp) parses hex colors via `rgb8_packed()` which
/// returns `rgba8` (linear), stores them into an `srgba8` array (linear -> sRGB),
/// and finally renders through a linear-`rgba8` pixfmt (`pixfmt_bgr24`) which
/// converts the color back (sRGB -> linear) for blending. We replicate this full
/// lossy roundtrip via `lion_color_roundtrip` to match pixel-perfectly.
pub fn parse_lion() -> (PathStorage, Vec<Rgba8>, Vec<usize>) {
    let mut path = PathStorage::new();
    let mut colors: Vec<Rgba8> = Vec::new();
    let mut path_idx: Vec<usize> = Vec::new();

    for line in LION_DATA.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('M') || line.starts_with('L') {
            parse_path_line(line, &mut path);
        } else if line.chars().all(|c| c.is_ascii_hexdigit()) && line.len() == 6 {
            let c = u32::from_str_radix(line, 16).unwrap_or(0);
            let r = (c >> 16) & 0xFF;
            let g = (c >> 8) & 0xFF;
            let b = c & 0xFF;

            // Apply the full linear -> sRGB -> linear roundtrip to match C++ AGG's
            // implicit rgba8 -> srgba8 -> rgba8 conversion in the lion demo.
            let r = lion_color_roundtrip(r);
            let g = lion_color_roundtrip(g);
            let b = lion_color_roundtrip(b);

            // Must use PATH_FLAGS_CLOSE to match C++ close_polygon() default
            path.close_polygon(PATH_FLAGS_CLOSE);
            colors.push(Rgba8::new(r, g, b, 255));
            path_idx.push(path.start_new_path());
        }
    }

    path.arrange_orientations_all_paths(PATH_FLAGS_CW);
    (path, colors, path_idx)
}

#[cfg(test)]
mod demo_smoke_tests {
    use super::{available_demos, render_demo};

    #[test]
    fn compositing_section_demos_render() {
        for name in ["compositing", "compositing2", "flash_rasterizer", "flash_rasterizer2"] {
            let out = render_demo(name, 64, 48, &[]);
            assert!(out.is_some(), "missing renderer for {name}");
            let out = out.unwrap();
            assert_eq!(out.width, 64);
            assert_eq!(out.height, 48);
            assert_eq!(out.data.len(), 64 * 48 * 4);
        }
    }

    #[test]
    fn demos_list_includes_compositing_section() {
        let demos = available_demos();
        for name in ["compositing", "compositing2", "flash_rasterizer", "flash_rasterizer2"] {
            assert!(demos.contains(&name), "available_demos missing {name}");
        }
    }

    /// Byte-for-byte compare a `compositing2` render against a committed C++ AGG
    /// reference `.raw` (8-byte `[width:u32][height:u32]` header + RGBA data).
    fn assert_compositing2_matches(reference: &[u8], params: &[f64]) {
        let width = u32::from_le_bytes([reference[0], reference[1], reference[2], reference[3]]);
        let height = u32::from_le_bytes([reference[4], reference[5], reference[6], reference[7]]);
        assert_eq!((width, height), (600, 400), "reference dimensions");
        let cpp = &reference[8..];

        let out = render_demo("compositing2", width, height, params)
            .expect("compositing2 renderer missing");
        assert_eq!(out.data.len(), cpp.len(), "buffer length");

        let mut mismatches = 0usize;
        let mut first: Option<usize> = None;
        for (i, (r, c)) in out.data.iter().zip(cpp.iter()).enumerate() {
            if r != c {
                mismatches += 1;
                if first.is_none() {
                    first = Some(i);
                }
            }
        }
        assert_eq!(
            mismatches, 0,
            "compositing2 differs from C++ reference (params={params:?}): \
             {mismatches} byte(s) differ, first at byte {first:?}",
        );
    }

    /// Byte-for-byte regression against the C++ AGG reference render of the
    /// compositing2 demo (600x400, default params: comp-op src-over, alphas 1.0).
    #[test]
    fn compositing2_matches_cpp_reference_600x400() {
        const REF: &[u8] = include_bytes!("../../../../compositing2_cpp_600x400.raw");
        assert_compositing2_matches(REF, &[]);
    }

    /// Byte-for-byte regression at a non-default partial alpha (comp-op src-over,
    /// src alpha = dst alpha = 0.5). This locks in the color-ramp construction: at
    /// alpha < 1 the double-precision `rgba::gradient` used by the example/harness
    /// diverges from the fixed-point `rgba8::gradient` (10 segment-3 ramp entries
    /// differ, ~12260 output bytes), so this guards against a regression to the
    /// fixed-point path (the default-params test alone cannot catch it, since the
    /// two paths coincide at alpha = 1.0).
    #[test]
    fn compositing2_matches_cpp_reference_600x400_alpha050() {
        const REF: &[u8] = include_bytes!("../../../../compositing2_cpp_600x400_alpha050.raw");
        // params: [comp_op = 3 (src-over), src_alpha = 0.5, dst_alpha = 0.5]
        assert_compositing2_matches(REF, &[3.0, 0.5, 0.5]);
    }

    /// Byte-for-byte regression at comp-op src-atop (index 9), src alpha =
    /// dst alpha = 0.5. This locks in the port's replication of the upstream
    /// C++ AGG 2.6 src_atop blue-channel typo (agg_pixfmt_rgba.h:530): the
    /// blue channel is computed from the just-updated green (`d.g`), not blue,
    /// so byte-identity with the compiled C++ renderer requires the Rust port
    /// to carry the same typo rather than silently "fixing" it.
    #[test]
    fn compositing2_matches_cpp_reference_600x400_srcatop_a050() {
        const REF: &[u8] = include_bytes!("../../../../compositing2_cpp_600x400_srcatop_a050.raw");
        // params: [comp_op = 9 (src-atop), src_alpha = 0.5, dst_alpha = 0.5]
        assert_compositing2_matches(REF, &[9.0, 0.5, 0.5]);
    }
}

/// Byte-for-byte regression tests against the committed C++ AGG reference
/// renders. The reference `.raw` files use an 8-byte header
/// `[width:u32-le][height:u32-le]` followed by top-down RGBA pixels, matching
/// the format written by the headless C++ renderer and read by `load_raw`.
#[cfg(test)]
mod flash_reference_tests {
    use super::render_demo;

    /// Split a reference `.raw` blob into `(width, height, rgba_pixels)`.
    fn parse_raw(bytes: &[u8]) -> (u32, u32, &[u8]) {
        let w = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let h = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        (w, h, &bytes[8..])
    }

    fn assert_matches_reference(demo: &str, reference: &[u8]) {
        let (w, h, expected) = parse_raw(reference);
        let out = render_demo(demo, w, h, &[]).expect("demo should render");
        assert_eq!(out.width, w);
        assert_eq!(out.height, h);
        assert_eq!(
            out.data.len(),
            expected.len(),
            "{demo}: rendered byte length mismatch"
        );
        if out.data != expected {
            let first = out
                .data
                .iter()
                .zip(expected.iter())
                .position(|(a, b)| a != b)
                .unwrap();
            let px = first / 4;
            panic!(
                "{demo}: differs from C++ reference at byte {first} (pixel {}, {}): \
                 rust={:?} cpp={:?}",
                px % w as usize,
                px / w as usize,
                &out.data[px * 4..px * 4 + 4],
                &expected[px * 4..px * 4 + 4],
            );
        }
    }

    #[test]
    fn flash_rasterizer_matches_cpp_reference() {
        let reference = include_bytes!("../../../../flash_rasterizer_cpp_655x520.raw");
        assert_matches_reference("flash_rasterizer", reference);
    }

    #[test]
    fn flash_rasterizer2_matches_cpp_reference() {
        let reference = include_bytes!("../../../../flash_rasterizer2_cpp_655x520.raw");
        assert_matches_reference("flash_rasterizer2", reference);
    }
}

fn parse_path_line(line: &str, path: &mut PathStorage) {
    let mut tokens = line.split_whitespace();

    while let Some(token) = tokens.next() {
        let cmd = token.chars().next().unwrap_or(' ');
        let coords = if token.len() > 1 {
            &token[1..]
        } else if let Some(next) = tokens.next() {
            next
        } else {
            break;
        };

        let parts: Vec<&str> = coords.split(',').collect();
        if parts.len() < 2 {
            continue;
        }

        let x: f64 = parts[0].parse().unwrap_or(0.0);
        let y: f64 = parts[1].parse().unwrap_or(0.0);

        match cmd {
            'M' => {
                // C++ close_polygon() uses default path_flags_close
                path.close_polygon(PATH_FLAGS_CLOSE);
                path.move_to(x, y);
            }
            'L' => {
                path.line_to(x, y);
            }
            _ => {}
        }
    }
}

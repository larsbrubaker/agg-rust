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
        "simple_line" => render_simple_line(width, height, params),
        _ => return None,
    };
    Some(PixelBuffer { width, height, data })
}

/// List all available demo names.
pub fn available_demos() -> &'static [&'static str] {
    &["lion_outline", "image_filters", "rasterizers2", "simple_line"]
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
use agg_rust::gamma::linear_to_srgb;
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

/// Parse the lion vector data into a path storage with colors and path indices.
///
/// The C++ AGG demo (parse_lion.cpp) parses hex colors via `rgb8_packed()` which
/// returns `rgba8` (linear), then stores into `srgba8` which triggers an implicit
/// linear-to-sRGB conversion. The renderer then copies the sRGB-encoded bytes
/// back into `rgba8` for blending. We must replicate this conversion to match
/// pixel-perfectly.
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

            // Apply linear-to-sRGB conversion to match C++ AGG's implicit
            // rgba8 → srgba8 → rgba8 roundtrip in parse_lion.cpp
            let r = linear_u8_to_srgb_u8(r);
            let g = linear_u8_to_srgb_u8(g);
            let b = linear_u8_to_srgb_u8(b);

            // Must use PATH_FLAGS_CLOSE to match C++ close_polygon() default
            path.close_polygon(PATH_FLAGS_CLOSE);
            colors.push(Rgba8::new(r, g, b, 255));
            path_idx.push(path.start_new_path());
        }
    }

    path.arrange_orientations_all_paths(PATH_FLAGS_CW);
    (path, colors, path_idx)
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

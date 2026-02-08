//! Lion vector path data — the classic AGG demo graphic.
//!
//! Parsed from the original C++ `parse_lion.cpp` example. Contains colored
//! polygon regions that form a stylized lion face.

use agg_rust::basics::PATH_FLAGS_CW;
use agg_rust::color::Rgba8;
use agg_rust::path_storage::PathStorage;

static LION_DATA: &str = include_str!("lion.txt");

/// Parse the lion vector data into a path storage with colors and path indices.
///
/// Returns (path, colors, path_indices).
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
            // Path command line
            parse_path_line(line, &mut path);
        } else if line.chars().all(|c| c.is_ascii_hexdigit()) && line.len() == 6 {
            // Hex color code
            let c = u32::from_str_radix(line, 16).unwrap_or(0);
            let r = (c >> 16) & 0xFF;
            let g = (c >> 8) & 0xFF;
            let b = c & 0xFF;

            path.close_polygon(0);
            colors.push(Rgba8::new(r, g, b, 255));
            path_idx.push(path.start_new_path());
        }
    }

    path.arrange_orientations_all_paths(PATH_FLAGS_CW);
    (path, colors, path_idx)
}

fn parse_path_line(line: &str, path: &mut PathStorage) {
    // Split on whitespace-separated tokens: "M 69,18 L 82,8 L 99,3 ..."
    let mut tokens = line.split_whitespace();

    while let Some(token) = tokens.next() {
        let cmd = match token {
            "M" | "L" => token.chars().next().unwrap(),
            _ => {
                // Might be a coordinate pair like "69,18" following an implicit command
                if let Some((x, y)) = parse_coord_pair(token) {
                    path.line_to(x, y);
                }
                continue;
            }
        };

        // Next token should be "x,y"
        if let Some(coord_token) = tokens.next() {
            if let Some((x, y)) = parse_coord_pair(coord_token) {
                if cmd == 'M' {
                    path.close_polygon(0);
                    path.move_to(x, y);
                } else {
                    path.line_to(x, y);
                }
            }
        }
    }
}

fn parse_coord_pair(s: &str) -> Option<(f64, f64)> {
    let mut parts = s.split(',');
    let x: f64 = parts.next()?.parse().ok()?;
    let y: f64 = parts.next()?.parse().ok()?;
    Some((x, y))
}

// Copyright 2025. Pixel-perfect comparison library for AGG Rust vs C++ demos.
//
// Provides buffer comparison, BMP I/O, and diff image generation.

use std::fs::File;
use std::io::{self, Read as IoRead, Write as IoWrite};
use std::path::Path;

// ============================================================================
// Pixel Buffer
// ============================================================================

/// An RGBA pixel buffer with dimensions.
#[derive(Clone)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    /// RGBA pixel data, row-major, top-to-bottom. Length = width * height * 4.
    pub data: Vec<u8>,
}

impl PixelBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; (width * height * 4) as usize],
        }
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }

    /// Flip the buffer vertically (top-to-bottom becomes bottom-to-top).
    /// Needed when comparing C++ (flip_y=true, negative stride) output with
    /// Rust (positive stride) output.
    pub fn flip_vertical(&mut self) {
        let row_bytes = (self.width * 4) as usize;
        let h = self.height as usize;
        for y in 0..h / 2 {
            let top = y * row_bytes;
            let bot = (h - 1 - y) * row_bytes;
            for x in 0..row_bytes {
                self.data.swap(top + x, bot + x);
            }
        }
    }
}

// ============================================================================
// Comparison Result
// ============================================================================

/// Information about a single pixel difference.
#[derive(Debug, Clone)]
pub struct DiffInfo {
    pub x: u32,
    pub y: u32,
    pub pixel_a: [u8; 4],
    pub pixel_b: [u8; 4],
}

/// Result of comparing two pixel buffers.
#[derive(Debug, Clone)]
pub struct CompareResult {
    /// True if every pixel in both buffers is identical.
    pub identical: bool,
    /// Total number of pixels compared.
    pub total_pixels: u64,
    /// Number of pixels that differ by at least 1 in any channel.
    pub different_pixels: u64,
    /// Maximum absolute difference across any single channel of any pixel.
    pub max_channel_diff: u8,
    /// Mean absolute difference across all channels of all differing pixels.
    pub mean_channel_diff: f64,
    /// The first differing pixel found (scanning left-to-right, top-to-bottom).
    pub first_diff: Option<DiffInfo>,
    /// Per-channel histograms of differences (index = abs_diff, value = count).
    pub diff_histogram: [u64; 256],
}

impl std::fmt::Display for CompareResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.identical {
            write!(f, "IDENTICAL: {} pixels match perfectly", self.total_pixels)
        } else {
            write!(
                f,
                "DIFFERENT: {}/{} pixels differ ({:.2}%), max_diff={}, mean_diff={:.4}",
                self.different_pixels,
                self.total_pixels,
                self.different_pixels as f64 / self.total_pixels as f64 * 100.0,
                self.max_channel_diff,
                self.mean_channel_diff,
            )?;
            if let Some(ref d) = self.first_diff {
                write!(
                    f,
                    "\n  First diff at ({}, {}): A={:?} B={:?}",
                    d.x, d.y, d.pixel_a, d.pixel_b
                )?;
            }
            Ok(())
        }
    }
}

// ============================================================================
// Buffer Comparison
// ============================================================================

/// Compare two RGBA pixel buffers byte-by-byte.
///
/// Buffers must have the same dimensions. Returns detailed comparison results.
pub fn compare_buffers(a: &PixelBuffer, b: &PixelBuffer) -> CompareResult {
    assert_eq!(a.width, b.width, "Width mismatch");
    assert_eq!(a.height, b.height, "Height mismatch");
    assert_eq!(a.data.len(), b.data.len(), "Data length mismatch");

    let total_pixels = (a.width as u64) * (a.height as u64);
    let mut different_pixels = 0u64;
    let mut max_channel_diff = 0u8;
    let mut total_diff_sum = 0u64;
    let mut total_diff_channels = 0u64;
    let mut first_diff: Option<DiffInfo> = None;
    let mut diff_histogram = [0u64; 256];

    for y in 0..a.height {
        for x in 0..a.width {
            let i = ((y * a.width + x) * 4) as usize;
            let pa = [a.data[i], a.data[i + 1], a.data[i + 2], a.data[i + 3]];
            let pb = [b.data[i], b.data[i + 1], b.data[i + 2], b.data[i + 3]];

            let mut pixel_differs = false;
            for c in 0..4 {
                let diff = (pa[c] as i16 - pb[c] as i16).unsigned_abs() as u8;
                if diff > 0 {
                    pixel_differs = true;
                    if diff > max_channel_diff {
                        max_channel_diff = diff;
                    }
                    total_diff_sum += diff as u64;
                    total_diff_channels += 1;
                    diff_histogram[diff as usize] += 1;
                }
            }

            if pixel_differs {
                different_pixels += 1;
                if first_diff.is_none() {
                    first_diff = Some(DiffInfo {
                        x,
                        y,
                        pixel_a: pa,
                        pixel_b: pb,
                    });
                }
            }
        }
    }

    let mean_channel_diff = if total_diff_channels > 0 {
        total_diff_sum as f64 / total_diff_channels as f64
    } else {
        0.0
    };

    CompareResult {
        identical: different_pixels == 0,
        total_pixels,
        different_pixels,
        max_channel_diff,
        mean_channel_diff,
        first_diff,
        diff_histogram,
    }
}

/// Generate a visual diff image highlighting pixel differences.
///
/// - Identical pixels are shown as dark gray.
/// - Different pixels are shown in red, with brightness proportional to the
///   magnitude of the difference (amplified 10x for visibility).
pub fn generate_diff_image(a: &PixelBuffer, b: &PixelBuffer) -> PixelBuffer {
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);

    let mut diff = PixelBuffer::new(a.width, a.height);
    for y in 0..a.height {
        for x in 0..a.width {
            let i = ((y * a.width + x) * 4) as usize;
            let mut max_diff = 0u8;
            for c in 0..3 {
                let d = (a.data[i + c] as i16 - b.data[i + c] as i16).unsigned_abs() as u8;
                if d > max_diff {
                    max_diff = d;
                }
            }

            let oi = i;
            if max_diff == 0 {
                // Identical — dark gray
                diff.data[oi] = 40;
                diff.data[oi + 1] = 40;
                diff.data[oi + 2] = 40;
                diff.data[oi + 3] = 255;
            } else {
                // Different — red channel scaled by difference (amplified 10x)
                let v = (max_diff as u16 * 10).min(255) as u8;
                diff.data[oi] = v;
                diff.data[oi + 1] = 0;
                diff.data[oi + 2] = 0;
                diff.data[oi + 3] = 255;
            }
        }
    }
    diff
}

/// Generate a side-by-side comparison image: [A | Diff | B]
pub fn generate_sidebyside(a: &PixelBuffer, b: &PixelBuffer) -> PixelBuffer {
    let diff = generate_diff_image(a, b);
    let total_width = a.width * 3;
    let mut out = PixelBuffer::new(total_width, a.height);

    for y in 0..a.height {
        for x in 0..a.width {
            let src_i = ((y * a.width + x) * 4) as usize;
            // Left panel: image A
            let dst_a = ((y * total_width + x) * 4) as usize;
            out.data[dst_a..dst_a + 4].copy_from_slice(&a.data[src_i..src_i + 4]);
            // Center panel: diff
            let dst_d = ((y * total_width + a.width + x) * 4) as usize;
            out.data[dst_d..dst_d + 4].copy_from_slice(&diff.data[src_i..src_i + 4]);
            // Right panel: image B
            let dst_b = ((y * total_width + a.width * 2 + x) * 4) as usize;
            out.data[dst_b..dst_b + 4].copy_from_slice(&b.data[src_i..src_i + 4]);
        }
    }
    out
}

// ============================================================================
// BMP I/O (32-bit BGRA, top-down)
// ============================================================================

/// Save a pixel buffer as a 32-bit BMP file (top-down, BGRA).
pub fn save_bmp(path: &Path, buf: &PixelBuffer) -> io::Result<()> {
    let w = buf.width;
    let h = buf.height;
    let row_size = w * 4;
    let image_size = row_size * h;
    let file_size = 14 + 40 + image_size;

    let mut f = File::create(path)?;

    // BMP file header (14 bytes)
    f.write_all(b"BM")?;
    f.write_all(&file_size.to_le_bytes())?;
    f.write_all(&[0u8; 4])?; // reserved
    f.write_all(&(14u32 + 40).to_le_bytes())?; // pixel data offset

    // BITMAPINFOHEADER (40 bytes)
    f.write_all(&40u32.to_le_bytes())?; // header size
    f.write_all(&w.to_le_bytes())?; // width
    f.write_all(&(-(h as i32)).to_le_bytes())?; // negative height = top-down
    f.write_all(&1u16.to_le_bytes())?; // planes
    f.write_all(&32u16.to_le_bytes())?; // bits per pixel
    f.write_all(&0u32.to_le_bytes())?; // compression (BI_RGB)
    f.write_all(&image_size.to_le_bytes())?; // image size
    f.write_all(&[0u8; 4])?; // x pixels per meter
    f.write_all(&[0u8; 4])?; // y pixels per meter
    f.write_all(&0u32.to_le_bytes())?; // colors used
    f.write_all(&0u32.to_le_bytes())?; // important colors

    // Pixel data — convert RGBA to BGRA
    let mut row = vec![0u8; row_size as usize];
    for y in 0..h {
        for x in 0..w {
            let si = ((y * w + x) * 4) as usize;
            let di = (x * 4) as usize;
            row[di] = buf.data[si + 2]; // B
            row[di + 1] = buf.data[si + 1]; // G
            row[di + 2] = buf.data[si]; // R
            row[di + 3] = buf.data[si + 3]; // A
        }
        f.write_all(&row)?;
    }

    Ok(())
}

/// Load a BMP file into a pixel buffer. Handles 24-bit and 32-bit BMPs.
pub fn load_bmp(path: &Path) -> io::Result<PixelBuffer> {
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;

    if data.len() < 54 || &data[0..2] != b"BM" {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a valid BMP file"));
    }

    let pixel_offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
    let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;

    let width = w.unsigned_abs();
    let height = h.unsigned_abs();
    let top_down = h < 0;
    let bytes_pp = bpp / 8;
    let row_stride = ((width as usize * bytes_pp + 3) / 4) * 4;

    let mut buf = PixelBuffer::new(width, height);

    for y in 0..height as usize {
        let src_y = if top_down { y } else { height as usize - 1 - y };
        let row_offset = pixel_offset + src_y * row_stride;

        for x in 0..width as usize {
            let si = row_offset + x * bytes_pp;
            let di = (y * width as usize + x) * 4;
            if si + bytes_pp > data.len() {
                continue;
            }

            if bytes_pp == 4 {
                // BGRA -> RGBA
                buf.data[di] = data[si + 2];
                buf.data[di + 1] = data[si + 1];
                buf.data[di + 2] = data[si];
                buf.data[di + 3] = data[si + 3];
            } else if bytes_pp == 3 {
                // BGR -> RGBA
                buf.data[di] = data[si + 2];
                buf.data[di + 1] = data[si + 1];
                buf.data[di + 2] = data[si];
                buf.data[di + 3] = 255;
            }
        }
    }

    Ok(buf)
}

// ============================================================================
// Raw RGBA I/O (for precise byte-for-byte comparison)
// ============================================================================

/// Save pixel buffer as raw RGBA with a simple header: [width:u32][height:u32][rgba_data].
pub fn save_raw(path: &Path, buf: &PixelBuffer) -> io::Result<()> {
    let mut f = File::create(path)?;
    f.write_all(&buf.width.to_le_bytes())?;
    f.write_all(&buf.height.to_le_bytes())?;
    f.write_all(&buf.data)?;
    Ok(())
}

/// Load a raw RGBA file.
pub fn load_raw(path: &Path) -> io::Result<PixelBuffer> {
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;

    if data.len() < 8 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Raw file too small"));
    }

    let width = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let height = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let expected = (width * height * 4) as usize + 8;

    if data.len() < expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Raw file too small: expected {} bytes, got {}", expected, data.len()),
        ));
    }

    Ok(PixelBuffer {
        width,
        height,
        data: data[8..expected].to_vec(),
    })
}

/// Load an image file, detecting format by extension.
pub fn load_image(path: &Path) -> io::Result<PixelBuffer> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("bmp") => load_bmp(path),
        Some("raw") | Some("rgba") => load_raw(path),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported image format: {:?}", path),
        )),
    }
}

/// Save an image file, detecting format by extension.
pub fn save_image(path: &Path, buf: &PixelBuffer) -> io::Result<()> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("bmp") => save_bmp(path, buf),
        Some("raw") | Some("rgba") => save_raw(path, buf),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsupported image format: {:?}", path),
        )),
    }
}

// ============================================================================
// Demo rendering (re-exported from render module)
// ============================================================================

pub mod render;

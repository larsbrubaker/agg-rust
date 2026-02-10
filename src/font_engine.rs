//! TrueType font engine using `ttf-parser`.
//!
//! Port of the C++ `font_engine_freetype_base` concept, using `ttf-parser`
//! instead of FreeType. Provides glyph outline extraction and metrics.
//!
//! Copyright (c) 2025. BSD-3-Clause License.
//! Updated 2025 for LCD subpixel rendering support (scale_x).

use crate::basics::{
    PATH_CMD_CURVE3, PATH_CMD_CURVE4, PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO,
    PATH_FLAGS_CLOSE,
};

/// Glyph data types matching C++ `glyph_data_type` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphDataType {
    Invalid = 0,
    Mono = 1,
    Gray8 = 2,
    Outline = 3,
}

/// Prepared glyph data: outline vertices and metrics.
#[derive(Debug, Clone)]
pub struct GlyphData {
    /// Glyph index within the font.
    pub glyph_index: u16,
    /// Glyph data type (always Outline for this engine).
    pub data_type: GlyphDataType,
    /// Bounding box: (x_min, y_min, x_max, y_max) in scaled coordinates.
    pub bounds: (i32, i32, i32, i32),
    /// Horizontal advance in scaled coordinates.
    pub advance_x: f64,
    /// Vertical advance in scaled coordinates (usually 0 for horizontal text).
    pub advance_y: f64,
    /// Outline vertices as (x, y, cmd) tuples.
    /// Commands use AGG path constants: PATH_CMD_MOVE_TO, PATH_CMD_LINE_TO,
    /// PATH_CMD_CURVE3, PATH_CMD_CURVE4, PATH_CMD_END_POLY|PATH_FLAGS_CLOSE.
    pub outline: Vec<(f64, f64, u32)>,
}

/// TrueType font engine.
///
/// Loads a TTF/OTF font from raw bytes and extracts glyph outlines and metrics.
/// This is the Rust equivalent of C++ `font_engine_freetype_base`, using
/// `ttf-parser` instead of FreeType.
pub struct FontEngine {
    /// Owned font data bytes.
    face_data: Vec<u8>,
    /// Font face index (for font collections).
    face_index: u32,
    /// Desired em-height in pixels.
    height: f64,
    /// Horizontal scale factor applied to glyph outlines and advance values.
    ///
    /// Used for LCD subpixel rendering: set to `subpixel_scale` (3 for LCD, 1 for
    /// grayscale) to stretch glyph outlines horizontally. Matches C++
    /// `font_engine_win32_tt_base::scale_x()`.
    scale_x: f64,
    /// Whether to flip Y coordinates (for screen coordinate systems where y=0 is top).
    flip_y: bool,
    /// Whether hinting is enabled (informational; ttf-parser doesn't apply hinting).
    hinting: bool,
}

impl FontEngine {
    /// Create a font engine from raw TTF/OTF data.
    ///
    /// Validates that the data contains a parseable font face.
    /// `face_index` selects the face in a font collection (use 0 for single fonts).
    pub fn from_data(data: Vec<u8>, face_index: u32) -> Result<Self, String> {
        // Validate that the font can be parsed
        ttf_parser::Face::parse(&data, face_index)
            .map_err(|e| format!("Failed to parse font: {:?}", e))?;

        Ok(Self {
            face_data: data,
            face_index,
            height: 12.0,
            scale_x: 1.0,
            flip_y: false,
            hinting: false,
        })
    }

    /// Set the em-height in pixels.
    pub fn set_height(&mut self, h: f64) {
        self.height = h;
    }

    /// Get the current em-height in pixels.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Set whether to flip Y coordinates.
    ///
    /// When true, Y coordinates are negated so that y increases downward
    /// (matching screen coordinate systems). When false (default), Y coordinates
    /// follow the font's native upward direction.
    pub fn set_flip_y(&mut self, flip: bool) {
        self.flip_y = flip;
    }

    /// Get the flip_y setting.
    pub fn flip_y(&self) -> bool {
        self.flip_y
    }

    /// Set hinting (informational only — ttf-parser doesn't apply hinting).
    pub fn set_hinting(&mut self, h: bool) {
        self.hinting = h;
    }

    /// Get hinting setting.
    pub fn hinting(&self) -> bool {
        self.hinting
    }

    /// Set horizontal scale factor for glyph outlines.
    ///
    /// This scales X coordinates and advance_x of all glyphs. Used for LCD
    /// subpixel rendering where glyphs are stretched 3x horizontally.
    /// Matches C++ `font_engine_win32_tt_base::scale_x()`.
    pub fn set_scale_x(&mut self, s: f64) {
        self.scale_x = s;
    }

    /// Get the current horizontal scale factor.
    pub fn scale_x(&self) -> f64 {
        self.scale_x
    }

    /// Get the ascender in scaled coordinates.
    pub fn ascender(&self) -> f64 {
        let face = self.face();
        let scale = self.scale(&face);
        face.ascender() as f64 * scale
    }

    /// Get the descender in scaled coordinates (typically negative).
    pub fn descender(&self) -> f64 {
        let face = self.face();
        let scale = self.scale(&face);
        face.descender() as f64 * scale
    }

    /// Get the units-per-em value.
    pub fn units_per_em(&self) -> u16 {
        self.face().units_per_em()
    }

    /// Prepare a glyph: extract its outline and metrics.
    ///
    /// Returns `None` only if the character code is invalid or has no mapping
    /// in this font's cmap table. Characters like spaces that have a valid
    /// glyph but no outline will return `Some(GlyphData)` with an empty
    /// outline and `data_type == GlyphDataType::Invalid`, but valid `advance_x`.
    /// This matches C++ behavior where `glyph_cache` always has advance values
    /// even for non-outline glyphs.
    pub fn prepare_glyph(&self, char_code: u32) -> Option<GlyphData> {
        let ch = char::from_u32(char_code)?;
        let face = self.face();
        let glyph_id = face.glyph_index(ch)?;
        let scale = self.scale(&face);

        // Get horizontal advance (always available even for space characters)
        // Apply scale_x for LCD subpixel rendering
        let advance_x = face
            .glyph_hor_advance(glyph_id)
            .map(|a| a as f64 * scale * self.scale_x)
            .unwrap_or(0.0);

        // Try to extract outline — may be None for space, tab, etc.
        let mut builder = OutlineCollector::new(scale, self.flip_y, self.scale_x);
        let bbox_opt = face.outline_glyph(glyph_id, &mut builder);

        let (data_type, bounds) = if let Some(bbox) = bbox_opt {
            let y_sign = if self.flip_y { -1.0 } else { 1.0 };
            (
                GlyphDataType::Outline,
                (
                    (bbox.x_min as f64 * scale * self.scale_x) as i32,
                    (bbox.y_min as f64 * scale * y_sign) as i32,
                    (bbox.x_max as f64 * scale * self.scale_x) as i32,
                    (bbox.y_max as f64 * scale * y_sign) as i32,
                ),
            )
        } else {
            // No outline (e.g. space character) — still valid glyph with advance
            (GlyphDataType::Invalid, (0, 0, 0, 0))
        };

        Some(GlyphData {
            glyph_index: glyph_id.0,
            data_type,
            bounds,
            advance_x,
            advance_y: 0.0,
            outline: builder.vertices,
        })
    }

    /// Get kerning between two glyph indices in scaled coordinates.
    ///
    /// Returns the horizontal kerning adjustment, or 0.0 if no kerning data.
    pub fn kerning(&self, first_glyph: u16, second_glyph: u16) -> f64 {
        let face = self.face();
        let scale = self.scale(&face);
        let first = ttf_parser::GlyphId(first_glyph);
        let second = ttf_parser::GlyphId(second_glyph);

        // Try kern table subtables
        if let Some(kern) = face.tables().kern {
            for subtable in kern.subtables {
                if subtable.horizontal && !subtable.has_cross_stream {
                    if let Some(value) = subtable.glyphs_kerning(first, second) {
                        return value as f64 * scale;
                    }
                }
            }
        }

        0.0
    }

    // -- Internal helpers --

    /// Create a temporary Face from the stored data.
    fn face(&self) -> ttf_parser::Face<'_> {
        // Safe: we validated the data in from_data()
        ttf_parser::Face::parse(&self.face_data, self.face_index).unwrap()
    }

    /// Compute the scale factor: height / units_per_em.
    fn scale(&self, face: &ttf_parser::Face<'_>) -> f64 {
        self.height / face.units_per_em() as f64
    }
}

// ============================================================================
// OutlineCollector — implements ttf_parser::OutlineBuilder
// ============================================================================

/// Collects glyph outline commands into AGG-compatible vertex tuples.
struct OutlineCollector {
    vertices: Vec<(f64, f64, u32)>,
    scale: f64,
    scale_x: f64,
    flip_y: bool,
}

impl OutlineCollector {
    fn new(scale: f64, flip_y: bool, scale_x: f64) -> Self {
        Self {
            vertices: Vec::with_capacity(64),
            scale,
            scale_x,
            flip_y,
        }
    }

    #[inline]
    fn sx(&self, v: f32) -> f64 {
        v as f64 * self.scale * self.scale_x
    }

    #[inline]
    fn sy(&self, v: f32) -> f64 {
        let y = v as f64 * self.scale;
        if self.flip_y { -y } else { y }
    }
}

impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.vertices.push((self.sx(x), self.sy(y), PATH_CMD_MOVE_TO));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.vertices.push((self.sx(x), self.sy(y), PATH_CMD_LINE_TO));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // Quadratic Bezier: control point + endpoint, both with PATH_CMD_CURVE3
        self.vertices.push((self.sx(x1), self.sy(y1), PATH_CMD_CURVE3));
        self.vertices.push((self.sx(x), self.sy(y), PATH_CMD_CURVE3));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        // Cubic Bezier: two control points + endpoint, all with PATH_CMD_CURVE4
        self.vertices.push((self.sx(x1), self.sy(y1), PATH_CMD_CURVE4));
        self.vertices.push((self.sx(x2), self.sy(y2), PATH_CMD_CURVE4));
        self.vertices.push((self.sx(x), self.sy(y), PATH_CMD_CURVE4));
    }

    fn close(&mut self) {
        self.vertices
            .push((0.0, 0.0, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE));
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_data_type_values() {
        assert_eq!(GlyphDataType::Invalid as u32, 0);
        assert_eq!(GlyphDataType::Mono as u32, 1);
        assert_eq!(GlyphDataType::Gray8 as u32, 2);
        assert_eq!(GlyphDataType::Outline as u32, 3);
    }

    #[test]
    fn test_outline_collector_scale() {
        let c = OutlineCollector::new(2.0, false, 1.0);
        assert!((c.sx(10.0) - 20.0).abs() < 1e-10);
        assert!((c.sy(10.0) - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_outline_collector_scale_x() {
        let c = OutlineCollector::new(2.0, false, 3.0);
        assert!((c.sx(10.0) - 60.0).abs() < 1e-10); // 10 * 2.0 * 3.0
        assert!((c.sy(10.0) - 20.0).abs() < 1e-10); // scale_x doesn't affect Y
    }

    #[test]
    fn test_outline_collector_flip_y() {
        let c = OutlineCollector::new(1.0, true, 1.0);
        assert!((c.sy(10.0) - (-10.0)).abs() < 1e-10);

        let c2 = OutlineCollector::new(1.0, false, 1.0);
        assert!((c2.sy(10.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_outline_collector_commands() {
        let mut c = OutlineCollector::new(1.0, false, 1.0);
        ttf_parser::OutlineBuilder::move_to(&mut c, 10.0, 20.0);
        ttf_parser::OutlineBuilder::line_to(&mut c, 30.0, 40.0);
        ttf_parser::OutlineBuilder::quad_to(&mut c, 50.0, 60.0, 70.0, 80.0);
        ttf_parser::OutlineBuilder::curve_to(&mut c, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        ttf_parser::OutlineBuilder::close(&mut c);

        assert_eq!(c.vertices.len(), 8);
        assert_eq!(c.vertices[0].2, PATH_CMD_MOVE_TO);
        assert_eq!(c.vertices[1].2, PATH_CMD_LINE_TO);
        assert_eq!(c.vertices[2].2, PATH_CMD_CURVE3);
        assert_eq!(c.vertices[3].2, PATH_CMD_CURVE3);
        assert_eq!(c.vertices[4].2, PATH_CMD_CURVE4);
        assert_eq!(c.vertices[5].2, PATH_CMD_CURVE4);
        assert_eq!(c.vertices[6].2, PATH_CMD_CURVE4);
        assert_eq!(c.vertices[7].2, PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);
    }
}

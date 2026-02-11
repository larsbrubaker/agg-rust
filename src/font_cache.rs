//! Font cache manager and glyph path adaptor.
//!
//! Port of the C++ `font_cache_manager` concept. Caches glyph outlines and
//! metrics, and provides a `GlyphPathAdaptor` that implements `VertexSource`
//! for rendering cached glyphs through the AGG pipeline.
//!
//! Copyright (c) 2025. BSD-3-Clause License.

use crate::basics::{VertexSource, PATH_CMD_STOP};
use crate::font_engine::{FontEngine, GlyphData};
use std::collections::HashMap;

// ============================================================================
// GlyphPathAdaptor — VertexSource for a cached glyph
// ============================================================================

/// Replays a cached glyph outline as an AGG vertex source.
///
/// Equivalent of C++ `serialized_integer_path_adaptor` — stores pre-computed
/// outline vertices and replays them at a given (x, y) offset.
pub struct GlyphPathAdaptor {
    /// Pre-computed outline vertices at the origin: (x, y, cmd).
    vertices: Vec<(f64, f64, u32)>,
    /// Current replay index.
    vertex_idx: usize,
    /// Translation offset applied to all vertices.
    offset_x: f64,
    offset_y: f64,
}

impl GlyphPathAdaptor {
    /// Create an empty path adaptor.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            vertex_idx: 0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    /// Initialize the adaptor with a glyph's outline at position (x, y).
    ///
    /// This is the Rust equivalent of C++ `init_embedded_adaptors(glyph, x, y)`.
    pub fn init(&mut self, outline: &[(f64, f64, u32)], x: f64, y: f64) {
        self.vertices.clear();
        self.vertices.extend_from_slice(outline);
        self.offset_x = x;
        self.offset_y = y;
        self.vertex_idx = 0;
    }
}

impl Default for GlyphPathAdaptor {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for GlyphPathAdaptor {
    fn rewind(&mut self, _path_id: u32) {
        self.vertex_idx = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        if self.vertex_idx < self.vertices.len() {
            let (vx, vy, cmd) = self.vertices[self.vertex_idx];
            self.vertex_idx += 1;
            // Only offset vertex commands (move_to, line_to, curve3, curve4),
            // not end_poly/close/stop commands.
            if crate::basics::is_vertex(cmd) {
                *x = vx + self.offset_x;
                *y = vy + self.offset_y;
            } else {
                *x = 0.0;
                *y = 0.0;
            }
            cmd
        } else {
            PATH_CMD_STOP
        }
    }
}

// ============================================================================
// FontCacheManager
// ============================================================================

/// Font cache manager — caches glyph outlines and provides rendering adaptors.
///
/// Simplified port of C++ `font_cache_manager<FontEngine>`. Caches glyph
/// outlines and metrics in a HashMap, and provides a `GlyphPathAdaptor`
/// for rendering glyphs through the AGG vertex pipeline.
pub struct FontCacheManager {
    engine: FontEngine,
    cache: HashMap<u32, GlyphData>,
    /// Glyph index used as the left-side glyph for the next kerning lookup.
    prev_glyph_index: Option<u16>,
    /// Glyph index of the most recently requested glyph via `glyph()`.
    ///
    /// C++ `font_cache_manager` keeps "last glyph" separate from "previous glyph":
    /// callers do `glyph(ch)` first, then `add_kerning(...)`.
    last_glyph_index: Option<u16>,
    /// Path adaptor for the current glyph.
    path_adaptor: GlyphPathAdaptor,
}

impl FontCacheManager {
    /// Create a font cache manager from raw TTF/OTF data.
    pub fn from_data(data: Vec<u8>) -> Result<Self, String> {
        let engine = FontEngine::from_data(data, 0)?;
        Ok(Self {
            engine,
            cache: HashMap::new(),
            prev_glyph_index: None,
            last_glyph_index: None,
            path_adaptor: GlyphPathAdaptor::new(),
        })
    }

    /// Get mutable access to the font engine (for setting height, flip_y, etc.).
    pub fn engine_mut(&mut self) -> &mut FontEngine {
        // Changing engine settings invalidates the cache
        &mut self.engine
    }

    /// Get access to the font engine.
    pub fn engine(&self) -> &FontEngine {
        &self.engine
    }

    /// Clear the glyph cache (call after changing engine settings like height).
    pub fn reset_cache(&mut self) {
        self.cache.clear();
        self.prev_glyph_index = None;
        self.last_glyph_index = None;
    }

    /// Reset the kerning state (call at the start of a new text run).
    pub fn reset_last_glyph(&mut self) {
        self.prev_glyph_index = None;
        self.last_glyph_index = None;
    }

    /// Get a cached glyph, preparing it if not already cached.
    ///
    /// Returns `None` if the character has no glyph in this font.
    /// Updates the "current glyph" state for a subsequent `add_kerning()` call.
    pub fn glyph(&mut self, char_code: u32) -> Option<&GlyphData> {
        // Ensure the glyph is in the cache
        if !self.cache.contains_key(&char_code) {
            let data = self.engine.prepare_glyph(char_code)?;
            self.cache.insert(char_code, data);
        }

        let glyph = self.cache.get(&char_code)?;

        // Track "last glyph" (current glyph), not "previous glyph".
        // Kerning is applied later in add_kerning(), matching C++ call order.
        self.last_glyph_index = Some(glyph.glyph_index);

        Some(glyph)
    }

    /// Apply kerning between the previous and current glyphs.
    ///
    /// C++ call order is: `glyph(current)` then `add_kerning(...)`.
    /// This method follows that behavior by using the most recent `glyph()`
    /// result as the right-side glyph and the previously committed glyph as
    /// the left-side glyph.
    ///
    /// Returns `true` if kerning was applied.
    pub fn add_kerning(&mut self, char_code: u32, x: &mut f64, _y: &mut f64) -> bool {
        // Keep char_code in the API for compatibility with existing call sites
        // and to mirror C++ usage where the current glyph is identified by code.
        let _ = char_code;

        let mut applied = false;
        if let (Some(prev_idx), Some(last_idx)) = (self.prev_glyph_index, self.last_glyph_index) {
            let kern = self.engine.kerning(prev_idx, last_idx);
            if kern.abs() > 1e-10 {
                *x += kern;
                applied = true;
            }
        }

        // Advance kerning chain for the next glyph pair.
        self.prev_glyph_index = self.last_glyph_index;
        applied
    }

    /// Initialize the path adaptor for a glyph at position (x, y).
    ///
    /// After calling this, `path_adaptor()` returns a VertexSource that
    /// replays the glyph outline offset by (x, y).
    pub fn init_embedded_adaptors(&mut self, char_code: u32, x: f64, y: f64) {
        if let Some(glyph) = self.cache.get(&char_code) {
            self.path_adaptor.init(&glyph.outline, x, y);
        }
    }

    /// Get an immutable reference to the path adaptor.
    pub fn path_adaptor(&self) -> &GlyphPathAdaptor {
        &self.path_adaptor
    }

    /// Get a mutable reference to the path adaptor.
    ///
    /// Needed because `ConvCurve` etc. require `&mut VertexSource`.
    pub fn path_adaptor_mut(&mut self) -> &mut GlyphPathAdaptor {
        &mut self.path_adaptor
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_stop, is_vertex, PATH_CMD_MOVE_TO};

    #[test]
    fn test_glyph_path_adaptor_empty() {
        let mut adaptor = GlyphPathAdaptor::new();
        adaptor.rewind(0);
        let (mut x, mut y) = (0.0, 0.0);
        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_glyph_path_adaptor_offset() {
        let mut adaptor = GlyphPathAdaptor::new();
        let vertices = vec![
            (10.0, 20.0, PATH_CMD_MOVE_TO),
            (30.0, 40.0, crate::basics::PATH_CMD_LINE_TO),
        ];
        adaptor.init(&vertices, 100.0, 200.0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_vertex(cmd));
        assert!((x - 110.0).abs() < 1e-10);
        assert!((y - 220.0).abs() < 1e-10);

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_vertex(cmd));
        assert!((x - 130.0).abs() < 1e-10);
        assert!((y - 240.0).abs() < 1e-10);

        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_stop(cmd));
    }

    #[test]
    fn test_glyph_path_adaptor_rewind() {
        let mut adaptor = GlyphPathAdaptor::new();
        let vertices = vec![(5.0, 10.0, PATH_CMD_MOVE_TO)];
        adaptor.init(&vertices, 0.0, 0.0);

        // Read first vertex
        let (mut x, mut y) = (0.0, 0.0);
        adaptor.vertex(&mut x, &mut y);
        // Rewind and read again
        adaptor.rewind(0);
        let cmd = adaptor.vertex(&mut x, &mut y);
        assert!(is_vertex(cmd));
        assert!((x - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_kerning_uses_previous_and_current_glyph() {
        // Use the same embedded font as the demo code.
        let mut fman = FontCacheManager::from_data(
            include_bytes!("../demo/wasm/fonts/LiberationSerif-Regular.ttf").to_vec(),
        )
        .expect("font should load");
        fman.engine_mut().set_height(24.0);
        fman.reset_cache();

        // Find a pair with non-zero kerning in this font to ensure the test
        // actually verifies pairwise kerning semantics.
        let candidates = [
            ('A', 'V'),
            ('A', 'W'),
            ('T', 'o'),
            ('T', 'a'),
            ('Y', 'o'),
            ('L', 'T'),
        ];

        let mut chosen: Option<(u32, u32, f64)> = None;
        for (left, right) in candidates {
            let left_code = left as u32;
            let right_code = right as u32;

            let left_idx = fman.glyph(left_code).expect("left glyph").glyph_index;
            let right_idx = fman.glyph(right_code).expect("right glyph").glyph_index;
            let k = fman.engine().kerning(left_idx, right_idx);
            if k.abs() > 1e-10 {
                chosen = Some((left_code, right_code, k));
                break;
            }
        }

        let (left_code, right_code, expected_kern) =
            chosen.expect("expected at least one non-zero kerning pair");

        // Reproduce C++/demo call order:
        //   glyph(left); add_kerning(left)  -> no kerning (first glyph)
        //   glyph(right); add_kerning(right)-> applies kerning(left,right)
        fman.reset_last_glyph();
        let left_advance = fman.glyph(left_code).expect("left glyph").advance_x;
        let mut x = 0.0;
        let mut y = 0.0;
        assert!(
            !fman.add_kerning(left_code, &mut x, &mut y),
            "first glyph should not apply kerning",
        );
        x += left_advance;

        fman.glyph(right_code).expect("right glyph");
        let applied = fman.add_kerning(right_code, &mut x, &mut y);
        assert!(applied, "expected kerning to be applied for the pair");
        assert!(
            (x - (left_advance + expected_kern)).abs() < 1e-8,
            "x={} expected={}",
            x,
            left_advance + expected_kern
        );
    }
}

//! Dash converter for vertex sources.
//!
//! Port of `agg_conv_dash.h` — convenience wrapper that combines
//! `ConvAdaptorVcgen` with `VcgenDash` to produce dashed lines.

use crate::basics::VertexSource;
use crate::conv_adaptor_vcgen::ConvAdaptorVcgen;
use crate::vcgen_dash::VcgenDash;

// ============================================================================
// ConvDash
// ============================================================================

/// Dash converter: generates a dashed line from a continuous center-line path.
///
/// Port of C++ `conv_dash<VertexSource>`.
pub struct ConvDash<VS: VertexSource> {
    base: ConvAdaptorVcgen<VS, VcgenDash>,
}

impl<VS: VertexSource> ConvDash<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            base: ConvAdaptorVcgen::new(source, VcgenDash::new()),
        }
    }

    pub fn remove_all_dashes(&mut self) {
        self.base.generator_mut().remove_all_dashes();
    }

    pub fn add_dash(&mut self, dash_len: f64, gap_len: f64) {
        self.base.generator_mut().add_dash(dash_len, gap_len);
    }

    pub fn dash_start(&mut self, ds: f64) {
        self.base.generator_mut().dash_start(ds);
    }

    pub fn set_shorten(&mut self, s: f64) {
        self.base.generator_mut().set_shorten(s);
    }
    pub fn shorten(&self) -> f64 {
        self.base.generator().shorten()
    }

    pub fn source(&self) -> &VS {
        self.base.source()
    }

    pub fn source_mut(&mut self) -> &mut VS {
        self.base.source_mut()
    }
}

impl<VS: VertexSource> VertexSource for ConvDash<VS> {
    fn rewind(&mut self, path_id: u32) {
        self.base.rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.base.vertex(x, y)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_stop, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO};
    use crate::path_storage::PathStorage;

    fn collect_vertices<VS: VertexSource>(vs: &mut VS) -> Vec<(f64, f64, u32)> {
        let mut result = Vec::new();
        vs.rewind(0);
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            result.push((x, y, cmd));
        }
        result
    }

    #[test]
    fn test_dash_empty_path() {
        let path = PathStorage::new();
        let mut dash = ConvDash::new(path);
        dash.add_dash(10.0, 5.0);
        let verts = collect_vertices(&mut dash);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_dash_basic_pattern() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut dash = ConvDash::new(path);
        dash.add_dash(20.0, 10.0);
        let verts = collect_vertices(&mut dash);

        assert!(!verts.is_empty(), "Should produce dash vertices");
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_dash_has_gaps() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut dash = ConvDash::new(path);
        dash.add_dash(20.0, 10.0);
        let verts = collect_vertices(&mut dash);

        let move_count = verts.iter().filter(|v| v.2 == PATH_CMD_MOVE_TO).count();
        assert!(
            move_count >= 2,
            "Expected multiple dash segments, got {} move_to",
            move_count
        );
    }

    #[test]
    fn test_dash_no_pattern_no_output() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut dash = ConvDash::new(path);
        // No add_dash called
        let verts = collect_vertices(&mut dash);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_dash_rewind_replay() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut dash = ConvDash::new(path);
        dash.add_dash(15.0, 5.0);
        let v1 = collect_vertices(&mut dash);
        let v2 = collect_vertices(&mut dash);
        assert_eq!(v1.len(), v2.len());
    }

    #[test]
    fn test_dash_line_count() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut dash = ConvDash::new(path);
        dash.add_dash(20.0, 10.0);
        let verts = collect_vertices(&mut dash);

        let line_count = verts.iter().filter(|v| v.2 == PATH_CMD_LINE_TO).count();
        assert!(
            line_count >= 3,
            "Expected multiple line segments, got {}",
            line_count
        );
    }
}

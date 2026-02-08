//! Stroke converter for vertex sources.
//!
//! Port of `agg_conv_stroke.h` — convenience wrapper that combines
//! `ConvAdaptorVcgen` with `VcgenStroke` to stroke any vertex source.

use crate::basics::VertexSource;
use crate::conv_adaptor_vcgen::ConvAdaptorVcgen;
use crate::math_stroke::{InnerJoin, LineCap, LineJoin};
use crate::vcgen_stroke::VcgenStroke;

// ============================================================================
// ConvStroke
// ============================================================================

/// Stroke converter: generates a stroked outline from a center-line path.
///
/// Port of C++ `conv_stroke<VertexSource>`.
pub struct ConvStroke<VS: VertexSource> {
    base: ConvAdaptorVcgen<VS, VcgenStroke>,
}

impl<VS: VertexSource> ConvStroke<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            base: ConvAdaptorVcgen::new(source, VcgenStroke::new()),
        }
    }

    // Parameter forwarding
    pub fn set_line_cap(&mut self, lc: LineCap) {
        self.base.generator_mut().set_line_cap(lc);
    }
    pub fn line_cap(&self) -> LineCap {
        self.base.generator().line_cap()
    }

    pub fn set_line_join(&mut self, lj: LineJoin) {
        self.base.generator_mut().set_line_join(lj);
    }
    pub fn line_join(&self) -> LineJoin {
        self.base.generator().line_join()
    }

    pub fn set_inner_join(&mut self, ij: InnerJoin) {
        self.base.generator_mut().set_inner_join(ij);
    }
    pub fn inner_join(&self) -> InnerJoin {
        self.base.generator().inner_join()
    }

    pub fn set_width(&mut self, w: f64) {
        self.base.generator_mut().set_width(w);
    }
    pub fn width(&self) -> f64 {
        self.base.generator().width()
    }

    pub fn set_miter_limit(&mut self, ml: f64) {
        self.base.generator_mut().set_miter_limit(ml);
    }
    pub fn miter_limit(&self) -> f64 {
        self.base.generator().miter_limit()
    }

    pub fn set_miter_limit_theta(&mut self, t: f64) {
        self.base.generator_mut().set_miter_limit_theta(t);
    }

    pub fn set_inner_miter_limit(&mut self, ml: f64) {
        self.base.generator_mut().set_inner_miter_limit(ml);
    }
    pub fn inner_miter_limit(&self) -> f64 {
        self.base.generator().inner_miter_limit()
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.base.generator_mut().set_approximation_scale(s);
    }
    pub fn approximation_scale(&self) -> f64 {
        self.base.generator().approximation_scale()
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

impl<VS: VertexSource> VertexSource for ConvStroke<VS> {
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
    use crate::basics::{is_stop, is_vertex, PATH_CMD_MOVE_TO};
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
    fn test_stroke_horizontal_line() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut stroke = ConvStroke::new(path);
        stroke.set_width(10.0);
        let verts = collect_vertices(&mut stroke);

        assert!(
            verts.len() >= 4,
            "Expected at least 4 stroke vertices, got {}",
            verts.len()
        );
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_stroke_triangle() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 10.0);
        path.line_to(90.0, 10.0);
        path.line_to(50.0, 80.0);

        let mut stroke = ConvStroke::new(path);
        stroke.set_width(4.0);
        let verts = collect_vertices(&mut stroke);

        assert!(verts.len() >= 6, "Expected many stroke vertices");
    }

    #[test]
    fn test_stroke_width() {
        let path = PathStorage::new();
        let mut stroke = ConvStroke::new(path);
        stroke.set_width(5.0);
        assert!((stroke.width() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_stroke_empty_path() {
        let path = PathStorage::new();
        let mut stroke = ConvStroke::new(path);
        let verts = collect_vertices(&mut stroke);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_stroke_round_cap() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(50.0, 0.0);

        let mut stroke = ConvStroke::new(path);
        stroke.set_width(20.0);
        stroke.set_line_cap(LineCap::Round);
        let verts = collect_vertices(&mut stroke);

        // Round caps produce many vertices
        assert!(
            verts.len() > 10,
            "Round cap stroke should have many vertices, got {}",
            verts.len()
        );
    }

    #[test]
    fn test_stroke_y_extent() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 50.0);
        path.line_to(90.0, 50.0);

        let mut stroke = ConvStroke::new(path);
        stroke.set_width(20.0); // half-width = 10

        let verts = collect_vertices(&mut stroke);
        let max_y = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.1)
            .fold(f64::MIN, f64::max);
        let min_y = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.1)
            .fold(f64::MAX, f64::min);

        assert!(max_y >= 59.0, "Max y={} should be >= 59", max_y);
        assert!(min_y <= 41.0, "Min y={} should be <= 41", min_y);
    }

    #[test]
    fn test_stroke_rewind_replay() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(100.0, 0.0);

        let mut stroke = ConvStroke::new(path);
        stroke.set_width(4.0);
        let v1 = collect_vertices(&mut stroke);
        let v2 = collect_vertices(&mut stroke);
        assert_eq!(v1.len(), v2.len());
    }

    #[test]
    fn test_stroke_line_join_round() {
        let mut path = PathStorage::new();
        path.move_to(0.0, 0.0);
        path.line_to(50.0, 0.0);
        path.line_to(50.0, 50.0);

        let mut miter = ConvStroke::new(&mut path);
        miter.set_width(10.0);
        miter.set_line_join(LineJoin::Miter);
        let miter_verts = collect_vertices(&mut miter);

        let mut round = ConvStroke::new(&mut path);
        round.set_width(10.0);
        round.set_line_join(LineJoin::Round);
        let round_verts = collect_vertices(&mut round);

        // Round join produces more vertices than miter
        assert!(
            round_verts.len() >= miter_verts.len(),
            "Round ({}) should have >= vertices than miter ({})",
            round_verts.len(),
            miter_verts.len()
        );
    }
}

//! Contour converter for vertex sources.
//!
//! Port of `agg_conv_contour.h` — convenience wrapper that combines
//! `ConvAdaptorVcgen` with `VcgenContour` to produce offset contours.

use crate::basics::VertexSource;
use crate::conv_adaptor_vcgen::ConvAdaptorVcgen;
use crate::math_stroke::{InnerJoin, LineJoin};
use crate::vcgen_contour::VcgenContour;

// ============================================================================
// ConvContour
// ============================================================================

/// Contour converter: generates an offset contour from a closed polygon.
///
/// Port of C++ `conv_contour<VertexSource>`.
pub struct ConvContour<VS: VertexSource> {
    base: ConvAdaptorVcgen<VS, VcgenContour>,
}

impl<VS: VertexSource> ConvContour<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            base: ConvAdaptorVcgen::new(source, VcgenContour::new()),
        }
    }

    // Parameter forwarding
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

    pub fn set_auto_detect_orientation(&mut self, v: bool) {
        self.base.generator_mut().set_auto_detect_orientation(v);
    }
    pub fn auto_detect_orientation(&self) -> bool {
        self.base.generator().auto_detect_orientation()
    }

    pub fn source(&self) -> &VS {
        self.base.source()
    }

    pub fn source_mut(&mut self) -> &mut VS {
        self.base.source_mut()
    }
}

impl<VS: VertexSource> VertexSource for ConvContour<VS> {
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
    fn test_contour_empty_path() {
        let path = PathStorage::new();
        let mut contour = ConvContour::new(path);
        let verts = collect_vertices(&mut contour);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_contour_width() {
        let path = PathStorage::new();
        let mut contour = ConvContour::new(path);
        contour.set_width(5.0);
        assert!((contour.width() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_contour_closed_triangle() {
        let mut path = PathStorage::new();
        path.move_to(50.0, 10.0);
        path.line_to(90.0, 90.0);
        path.line_to(10.0, 90.0);
        path.close_polygon(0);

        let mut contour = ConvContour::new(path);
        contour.set_width(5.0);
        contour.set_auto_detect_orientation(true);
        let verts = collect_vertices(&mut contour);

        assert!(
            verts.len() >= 3,
            "Expected contour vertices, got {}",
            verts.len()
        );
        assert_eq!(verts[0].2, PATH_CMD_MOVE_TO);
    }

    #[test]
    fn test_contour_expands_polygon() {
        let mut path = PathStorage::new();
        path.move_to(20.0, 20.0);
        path.line_to(80.0, 20.0);
        path.line_to(80.0, 80.0);
        path.line_to(20.0, 80.0);
        path.close_polygon(0);

        let mut contour = ConvContour::new(path);
        contour.set_width(10.0);
        contour.set_auto_detect_orientation(true);
        let verts = collect_vertices(&mut contour);

        let max_x = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.0)
            .fold(f64::MIN, f64::max);
        let min_x = verts
            .iter()
            .filter(|v| is_vertex(v.2))
            .map(|v| v.0)
            .fold(f64::MAX, f64::min);

        assert!(max_x > 80.0, "Max x={} should exceed original 80", max_x);
        assert!(
            min_x < 20.0,
            "Min x={} should be less than original 20",
            min_x
        );
    }

    #[test]
    fn test_contour_auto_detect() {
        let path = PathStorage::new();
        let mut contour = ConvContour::new(path);
        contour.set_auto_detect_orientation(true);
        assert!(contour.auto_detect_orientation());
    }
}

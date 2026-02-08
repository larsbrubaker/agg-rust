//! Affine transform converter for vertex sources.
//!
//! Port of `agg_conv_transform.h` — wraps a `VertexSource` and applies
//! a `TransAffine` transformation to each vertex coordinate.

use crate::basics::{is_vertex, VertexSource};
use crate::trans_affine::TransAffine;

// ============================================================================
// ConvTransform
// ============================================================================

/// Applies an affine transform to each vertex from a source.
///
/// Port of C++ `conv_transform<VertexSource, Transformer>`.
/// Owns the source; use `ConvTransform<&mut PathStorage>` to borrow.
pub struct ConvTransform<VS: VertexSource> {
    source: VS,
    trans: TransAffine,
}

impl<VS: VertexSource> ConvTransform<VS> {
    pub fn new(source: VS, trans: TransAffine) -> Self {
        Self { source, trans }
    }

    pub fn set_transform(&mut self, trans: TransAffine) {
        self.trans = trans;
    }

    pub fn transform(&self) -> &TransAffine {
        &self.trans
    }

    pub fn source(&self) -> &VS {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut VS {
        &mut self.source
    }
}

impl<VS: VertexSource> VertexSource for ConvTransform<VS> {
    fn rewind(&mut self, path_id: u32) {
        self.source.rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        let cmd = self.source.vertex(x, y);
        if is_vertex(cmd) {
            self.trans.transform(x, y);
        }
        cmd
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
    use crate::path_storage::PathStorage;

    #[test]
    fn test_identity_transform() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 20.0);
        path.line_to(30.0, 40.0);

        let mut ct = ConvTransform::new(path, TransAffine::default());
        ct.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = ct.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert!((x - 10.0).abs() < 1e-10);
        assert!((y - 20.0).abs() < 1e-10);

        let cmd = ct.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_LINE_TO);
        assert!((x - 30.0).abs() < 1e-10);
        assert!((y - 40.0).abs() < 1e-10);
    }

    #[test]
    fn test_translation() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 20.0);

        let trans = TransAffine::new_translation(100.0, 200.0);
        let mut ct = ConvTransform::new(path, trans);
        ct.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        ct.vertex(&mut x, &mut y);
        assert!((x - 110.0).abs() < 1e-10);
        assert!((y - 220.0).abs() < 1e-10);
    }

    #[test]
    fn test_scaling() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 20.0);
        path.line_to(30.0, 40.0);

        let trans = TransAffine::new_scaling(2.0, 3.0);
        let mut ct = ConvTransform::new(path, trans);
        ct.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        ct.vertex(&mut x, &mut y);
        assert!((x - 20.0).abs() < 1e-10);
        assert!((y - 60.0).abs() < 1e-10);

        ct.vertex(&mut x, &mut y);
        assert!((x - 60.0).abs() < 1e-10);
        assert!((y - 120.0).abs() < 1e-10);
    }

    #[test]
    fn test_stop_not_transformed() {
        let path = PathStorage::new();
        // Empty path — first vertex is stop
        let mut ct = ConvTransform::new(path, TransAffine::new_scaling(2.0, 2.0));
        ct.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = ct.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    #[test]
    fn test_set_transform() {
        let mut path = PathStorage::new();
        path.move_to(10.0, 10.0);

        let mut ct = ConvTransform::new(path, TransAffine::default());
        ct.set_transform(TransAffine::new_translation(5.0, 5.0));
        ct.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        ct.vertex(&mut x, &mut y);
        assert!((x - 15.0).abs() < 1e-10);
        assert!((y - 15.0).abs() < 1e-10);
    }
}

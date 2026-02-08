//! Convenience line-segment subdivider.
//!
//! Port of `agg_conv_segmentator.h`.
//! Wraps `ConvAdaptorVpgen` with `VpgenSegmentator` for subdividing
//! long line segments into shorter ones.

use crate::basics::VertexSource;
use crate::conv_adaptor_vpgen::ConvAdaptorVpgen;
use crate::vpgen_segmentator::VpgenSegmentator;

/// Subdivides line segments of a vertex source for better curve approximation.
///
/// Port of C++ `conv_segmentator<VertexSource>`.
/// Thin wrapper around `ConvAdaptorVpgen<VS, VpgenSegmentator>`.
pub struct ConvSegmentator<VS> {
    inner: ConvAdaptorVpgen<VS, VpgenSegmentator>,
}

impl<VS: VertexSource> ConvSegmentator<VS> {
    pub fn new(source: VS) -> Self {
        Self {
            inner: ConvAdaptorVpgen::new(source, VpgenSegmentator::new()),
        }
    }

    pub fn approximation_scale(&self) -> f64 {
        self.inner.vpgen().approximation_scale()
    }

    pub fn set_approximation_scale(&mut self, s: f64) {
        self.inner.vpgen_mut().set_approximation_scale(s);
    }

    pub fn source(&self) -> &VS {
        self.inner.source()
    }

    pub fn source_mut(&mut self) -> &mut VS {
        self.inner.source_mut()
    }
}

impl<VS: VertexSource> VertexSource for ConvSegmentator<VS> {
    fn rewind(&mut self, path_id: u32) {
        self.inner.rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.inner.vertex(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{is_stop, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};

    /// Square path: (0,0)→(100,0)→(100,100)→(0,100)→close
    struct SquareSource {
        idx: usize,
    }

    impl SquareSource {
        fn new() -> Self {
            Self { idx: 0 }
        }
    }

    impl VertexSource for SquareSource {
        fn rewind(&mut self, _path_id: u32) {
            self.idx = 0;
        }

        fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
            self.idx += 1;
            match self.idx {
                1 => {
                    *x = 0.0;
                    *y = 0.0;
                    PATH_CMD_MOVE_TO
                }
                2 => {
                    *x = 100.0;
                    *y = 0.0;
                    PATH_CMD_LINE_TO
                }
                3 => {
                    *x = 100.0;
                    *y = 100.0;
                    PATH_CMD_LINE_TO
                }
                4 => {
                    *x = 0.0;
                    *y = 100.0;
                    PATH_CMD_LINE_TO
                }
                _ => PATH_CMD_STOP,
            }
        }
    }

    #[test]
    fn test_conv_segmentator_passthrough() {
        let mut seg = ConvSegmentator::new(SquareSource::new());
        seg.set_approximation_scale(1.0);
        seg.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let mut count = 0;
        loop {
            let cmd = seg.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        // With scale=1.0, short segments might not subdivide much
        assert!(count >= 4, "Expected at least 4 vertices, got {count}");
    }

    #[test]
    fn test_conv_segmentator_subdivides() {
        let mut seg = ConvSegmentator::new(SquareSource::new());
        seg.set_approximation_scale(10.0); // force fine subdivision
        seg.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let mut count = 0;
        loop {
            let cmd = seg.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(
            count > 10,
            "With scale=10, square should have many vertices: got {count}"
        );
    }

    #[test]
    fn test_approximation_scale_accessors() {
        let seg = ConvSegmentator::new(SquareSource::new());
        assert!((seg.approximation_scale() - 1.0).abs() < 1e-10);

        let mut seg = seg;
        seg.set_approximation_scale(5.0);
        assert!((seg.approximation_scale() - 5.0).abs() < 1e-10);
    }
}

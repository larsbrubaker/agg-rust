//! Generic vertex processor generator adaptor.
//!
//! Port of `agg_conv_adaptor_vpgen.h`.
//! Feeds source vertices through a vertex processor generator (vpgen),
//! handling move_to/line_to/close logic automatically.

use crate::basics::{
    is_closed, is_end_poly, is_move_to, is_stop, is_vertex, VertexSource, PATH_CMD_END_POLY,
    PATH_CMD_STOP, PATH_FLAGS_CLOSE,
};

/// Trait for vertex processor generators (vpgen).
///
/// A vpgen processes one line segment at a time: `move_to` sets the start
/// point, `line_to` feeds each subsequent point, and `vertex` produces
/// the output subdivided/processed vertices.
pub trait VpgenProcessor {
    fn reset(&mut self);
    fn move_to(&mut self, x: f64, y: f64);
    fn line_to(&mut self, x: f64, y: f64);
    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32;
    fn auto_close() -> bool;
    fn auto_unclose() -> bool;
}

/// Generic adaptor that feeds a `VertexSource` through a `VpgenProcessor`.
///
/// Port of C++ `conv_adaptor_vpgen<VertexSource, VPGen>`.
/// Handles the state machine for polygon close/open and auto-close logic.
pub struct ConvAdaptorVpgen<VS, Gen> {
    source: VS,
    vpgen: Gen,
    start_x: f64,
    start_y: f64,
    poly_flags: u32,
    vertices: i32,
}

impl<VS: VertexSource, Gen: VpgenProcessor> ConvAdaptorVpgen<VS, Gen> {
    pub fn new(source: VS, vpgen: Gen) -> Self {
        Self {
            source,
            vpgen,
            start_x: 0.0,
            start_y: 0.0,
            poly_flags: 0,
            vertices: 0,
        }
    }

    pub fn source(&self) -> &VS {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut VS {
        &mut self.source
    }

    pub fn vpgen(&self) -> &Gen {
        &self.vpgen
    }

    pub fn vpgen_mut(&mut self) -> &mut Gen {
        &mut self.vpgen
    }
}

impl<VS: VertexSource, Gen: VpgenProcessor> VertexSource for ConvAdaptorVpgen<VS, Gen> {
    fn rewind(&mut self, path_id: u32) {
        self.source.rewind(path_id);
        self.vpgen.reset();
        self.start_x = 0.0;
        self.start_y = 0.0;
        self.poly_flags = 0;
        self.vertices = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        loop {
            let cmd = self.vpgen.vertex(x, y);
            if !is_stop(cmd) {
                return cmd;
            }

            if self.poly_flags != 0 && !Gen::auto_unclose() {
                *x = 0.0;
                *y = 0.0;
                let cmd = self.poly_flags;
                self.poly_flags = 0;
                return cmd;
            }

            if self.vertices < 0 {
                if self.vertices < -1 {
                    self.vertices = 0;
                    return PATH_CMD_STOP;
                }
                self.vpgen.move_to(self.start_x, self.start_y);
                self.vertices = 1;
                continue;
            }

            let mut tx = 0.0;
            let mut ty = 0.0;
            let cmd = self.source.vertex(&mut tx, &mut ty);

            if is_vertex(cmd) {
                if is_move_to(cmd) {
                    if Gen::auto_close() && self.vertices > 2 {
                        self.vpgen.line_to(self.start_x, self.start_y);
                        self.poly_flags = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE;
                        self.start_x = tx;
                        self.start_y = ty;
                        self.vertices = -1;
                        continue;
                    }
                    self.vpgen.move_to(tx, ty);
                    self.start_x = tx;
                    self.start_y = ty;
                    self.vertices = 1;
                } else {
                    self.vpgen.line_to(tx, ty);
                    self.vertices += 1;
                }
            } else if is_end_poly(cmd) {
                self.poly_flags = cmd;
                if is_closed(cmd) || Gen::auto_close() {
                    if Gen::auto_close() {
                        self.poly_flags |= PATH_FLAGS_CLOSE;
                    }
                    if self.vertices > 2 {
                        self.vpgen.line_to(self.start_x, self.start_y);
                    }
                    self.vertices = 0;
                }
            } else {
                // PATH_CMD_STOP
                if Gen::auto_close() && self.vertices > 2 {
                    self.vpgen.line_to(self.start_x, self.start_y);
                    self.poly_flags = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE;
                    self.vertices = -2;
                    continue;
                }
                return PATH_CMD_STOP;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO};
    use crate::vpgen_segmentator::VpgenSegmentator;

    /// Simple vertex source that yields a single move_to + line_to pair.
    struct SimpleLineSource {
        idx: usize,
    }

    impl SimpleLineSource {
        fn new() -> Self {
            Self { idx: 0 }
        }
    }

    impl VertexSource for SimpleLineSource {
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
                _ => PATH_CMD_STOP,
            }
        }
    }

    #[test]
    fn test_adaptor_with_segmentator() {
        let source = SimpleLineSource::new();
        let vpgen = VpgenSegmentator::new();
        let mut adaptor = ConvAdaptorVpgen::new(source, vpgen);
        adaptor.vpgen_mut().set_approximation_scale(1.0);
        adaptor.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let mut count = 0;
        loop {
            let cmd = adaptor.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        // Should have at least move_to + line_to
        assert!(count >= 2, "Expected at least 2 vertices, got {count}");
    }

    #[test]
    fn test_adaptor_subdivides_long_segment() {
        let source = SimpleLineSource::new();
        let vpgen = VpgenSegmentator::new();
        let mut adaptor = ConvAdaptorVpgen::new(source, vpgen);
        adaptor.vpgen_mut().set_approximation_scale(10.0); // force subdivisions
        adaptor.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let mut count = 0;
        loop {
            let cmd = adaptor.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            count += 1;
        }
        assert!(
            count > 2,
            "Long segment with scale=10 should subdivide: got {count}"
        );
    }
}

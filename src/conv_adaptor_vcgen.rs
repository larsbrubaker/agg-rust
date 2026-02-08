//! Generic adapter connecting a vertex source to a vertex generator.
//!
//! Port of `agg_conv_adaptor_vcgen.h` — the core adapter pattern used by
//! `ConvStroke`, `ConvDash`, and `ConvContour`.

use crate::basics::{
    is_end_poly, is_move_to, is_stop, is_vertex, VertexSource, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
};

// ============================================================================
// VcgenGenerator trait
// ============================================================================

/// Vertex generator interface used by `ConvAdaptorVcgen`.
///
/// Port of the implicit C++ "Generator" concept used by `conv_adaptor_vcgen`.
pub trait VcgenGenerator {
    fn remove_all(&mut self);
    fn add_vertex(&mut self, x: f64, y: f64, cmd: u32);
    fn rewind(&mut self, path_id: u32);
    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32;
}

// ============================================================================
// ConvAdaptorVcgen
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    Accumulate,
    Generate,
}

/// Generic adapter connecting a `VertexSource` to a `VcgenGenerator`.
///
/// Reads vertices from the source, feeds them to the generator, then
/// yields the generated vertices. Handles path splitting on `move_to`.
///
/// Port of C++ `conv_adaptor_vcgen<VertexSource, Generator, Markers>`.
/// Markers omitted (null_markers behavior).
pub struct ConvAdaptorVcgen<VS: VertexSource, Gen: VcgenGenerator> {
    source: VS,
    generator: Gen,
    status: Status,
    last_cmd: u32,
    start_x: f64,
    start_y: f64,
}

impl<VS: VertexSource, Gen: VcgenGenerator> ConvAdaptorVcgen<VS, Gen> {
    pub fn new(source: VS, generator: Gen) -> Self {
        Self {
            source,
            generator,
            status: Status::Initial,
            last_cmd: 0,
            start_x: 0.0,
            start_y: 0.0,
        }
    }

    pub fn generator(&self) -> &Gen {
        &self.generator
    }

    pub fn generator_mut(&mut self) -> &mut Gen {
        &mut self.generator
    }

    pub fn source(&self) -> &VS {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut VS {
        &mut self.source
    }
}

impl<VS: VertexSource, Gen: VcgenGenerator> VertexSource for ConvAdaptorVcgen<VS, Gen> {
    fn rewind(&mut self, path_id: u32) {
        self.source.rewind(path_id);
        self.status = Status::Initial;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        loop {
            match self.status {
                Status::Initial => {
                    // null_markers.remove_all() — no-op
                    self.last_cmd = self.source.vertex(&mut self.start_x, &mut self.start_y);
                    self.status = Status::Accumulate;
                    // fall through to Accumulate
                }
                Status::Accumulate => {
                    if is_stop(self.last_cmd) {
                        return PATH_CMD_STOP;
                    }

                    self.generator.remove_all();
                    self.generator
                        .add_vertex(self.start_x, self.start_y, PATH_CMD_MOVE_TO);
                    // null_markers.add_vertex(...) — no-op

                    loop {
                        let cmd = self.source.vertex(x, y);
                        if is_vertex(cmd) {
                            self.last_cmd = cmd;
                            if is_move_to(cmd) {
                                self.start_x = *x;
                                self.start_y = *y;
                                break;
                            }
                            self.generator.add_vertex(*x, *y, cmd);
                            // null_markers.add_vertex(*x, *y, PATH_CMD_LINE_TO) — no-op
                        } else {
                            if is_stop(cmd) {
                                self.last_cmd = PATH_CMD_STOP;
                                break;
                            }
                            if is_end_poly(cmd) {
                                self.generator.add_vertex(*x, *y, cmd);
                                break;
                            }
                        }
                    }
                    self.generator.rewind(0);
                    self.status = Status::Generate;
                    // fall through to Generate
                }
                Status::Generate => {
                    let cmd = self.generator.vertex(x, y);
                    if is_stop(cmd) {
                        // Generator exhausted — go back to Accumulate for next sub-path
                        self.status = Status::Accumulate;
                        continue;
                    }
                    return cmd;
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::PATH_CMD_STOP;

    /// Minimal generator that echoes input as-is.
    struct EchoGenerator {
        vertices: Vec<(f64, f64, u32)>,
        idx: usize,
    }

    impl EchoGenerator {
        fn new() -> Self {
            Self {
                vertices: Vec::new(),
                idx: 0,
            }
        }
    }

    impl VcgenGenerator for EchoGenerator {
        fn remove_all(&mut self) {
            self.vertices.clear();
        }
        fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
            self.vertices.push((x, y, cmd));
        }
        fn rewind(&mut self, _path_id: u32) {
            self.idx = 0;
        }
        fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
            if self.idx >= self.vertices.len() {
                return PATH_CMD_STOP;
            }
            let (vx, vy, cmd) = self.vertices[self.idx];
            *x = vx;
            *y = vy;
            self.idx += 1;
            cmd
        }
    }

    #[test]
    fn test_empty_source() {
        use crate::path_storage::PathStorage;
        let path = PathStorage::new();
        let mut adaptor = ConvAdaptorVcgen::new(path, EchoGenerator::new());
        adaptor.rewind(0);

        let (mut x, mut y) = (0.0, 0.0);
        let cmd = adaptor.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_STOP);
    }

    #[test]
    fn test_passthrough_with_echo() {
        use crate::path_storage::PathStorage;

        let mut path = PathStorage::new();
        path.move_to(10.0, 20.0);
        path.line_to(30.0, 40.0);

        let mut adaptor = ConvAdaptorVcgen::new(path, EchoGenerator::new());
        adaptor.rewind(0);

        let mut verts = Vec::new();
        loop {
            let (mut x, mut y) = (0.0, 0.0);
            let cmd = adaptor.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            verts.push((x, y, cmd));
        }
        assert!(
            verts.len() >= 2,
            "Expected >= 2 vertices, got {}",
            verts.len()
        );
    }

    #[test]
    fn test_generator_access() {
        use crate::path_storage::PathStorage;
        let path = PathStorage::new();
        let adaptor = ConvAdaptorVcgen::new(path, EchoGenerator::new());
        let _ = adaptor.generator();
    }
}

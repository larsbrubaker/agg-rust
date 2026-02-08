//! Outline rasterizer.
//!
//! Port of `agg_rasterizer_outline.h` — simple wireframe rasterizer that feeds
//! vertices directly to a renderer (no scanline conversion). Suitable for
//! previewing paths or drawing non-anti-aliased outlines.

use crate::basics::{is_closed, is_end_poly, is_move_to, is_stop, VertexSource};

// ============================================================================
// RendererPrimitivesLike trait
// ============================================================================

/// Trait for renderers that can draw primitive lines.
///
/// Matches the subset of `RendererPrimitives` API used by `RasterizerOutline`.
pub trait RendererPrimitivesLike {
    type Color: Clone;

    fn coord(c: f64) -> i32;
    fn move_to(&mut self, x: i32, y: i32);
    fn line_to(&mut self, x: i32, y: i32, last: bool);
    fn set_line_color(&mut self, c: Self::Color);
}

// ============================================================================
// RasterizerOutline
// ============================================================================

/// Outline rasterizer.
///
/// Reads vertices from a `VertexSource` and feeds them directly to a
/// primitive renderer. No anti-aliasing or scanline conversion — just
/// Bresenham lines.
///
/// Port of C++ `rasterizer_outline<Renderer>`.
pub struct RasterizerOutline<'a, Ren: RendererPrimitivesLike> {
    ren: &'a mut Ren,
    start_x: i32,
    start_y: i32,
    vertices: u32,
}

impl<'a, Ren: RendererPrimitivesLike> RasterizerOutline<'a, Ren> {
    pub fn new(ren: &'a mut Ren) -> Self {
        Self {
            ren,
            start_x: 0,
            start_y: 0,
            vertices: 0,
        }
    }

    /// Move to a new position (subpixel coordinates).
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.vertices = 1;
        self.start_x = x;
        self.start_y = y;
        self.ren.move_to(x, y);
    }

    /// Draw a line to (x, y) (subpixel coordinates).
    pub fn line_to(&mut self, x: i32, y: i32) {
        self.vertices += 1;
        self.ren.line_to(x, y, false);
    }

    /// Move to a floating-point position (converts to subpixel).
    pub fn move_to_d(&mut self, x: f64, y: f64) {
        self.move_to(Ren::coord(x), Ren::coord(y));
    }

    /// Draw a line to a floating-point position (converts to subpixel).
    pub fn line_to_d(&mut self, x: f64, y: f64) {
        self.line_to(Ren::coord(x), Ren::coord(y));
    }

    /// Close the current polygon by drawing a line back to the start.
    pub fn close(&mut self) {
        if self.vertices > 2 {
            self.line_to(self.start_x, self.start_y);
        }
        self.vertices = 0;
    }

    /// Add a single vertex with a path command.
    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        if is_move_to(cmd) {
            self.move_to_d(x, y);
        } else if is_end_poly(cmd) {
            if is_closed(cmd) {
                self.close();
            }
        } else {
            self.line_to_d(x, y);
        }
    }

    /// Add all vertices from a vertex source.
    pub fn add_path<VS: VertexSource>(&mut self, vs: &mut VS, path_id: u32) {
        vs.rewind(path_id);
        let mut x = 0.0;
        let mut y = 0.0;
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            self.add_vertex(x, y, cmd);
        }
    }

    /// Render multiple paths with different colors.
    pub fn render_all_paths<VS: VertexSource>(
        &mut self,
        vs: &mut VS,
        colors: &[Ren::Color],
        path_ids: &[u32],
    ) {
        for i in 0..colors.len().min(path_ids.len()) {
            self.ren.set_line_color(colors[i].clone());
            self.add_path(vs, path_ids[i]);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};

    #[derive(Default)]
    struct MockRenderer {
        moves: Vec<(i32, i32)>,
        lines: Vec<(i32, i32)>,
    }

    impl RendererPrimitivesLike for MockRenderer {
        type Color = u32;

        fn coord(c: f64) -> i32 {
            (c * 256.0) as i32
        }
        fn move_to(&mut self, x: i32, y: i32) {
            self.moves.push((x, y));
        }
        fn line_to(&mut self, x: i32, y: i32, _last: bool) {
            self.lines.push((x, y));
        }
        fn set_line_color(&mut self, _c: u32) {}
    }

    #[test]
    fn test_move_and_line_to() {
        let mut ren = MockRenderer::default();
        let mut ras = RasterizerOutline::new(&mut ren);
        ras.move_to(10, 20);
        ras.line_to(30, 40);

        assert_eq!(ren.moves.len(), 1);
        assert_eq!(ren.moves[0], (10, 20));
        assert_eq!(ren.lines.len(), 1);
        assert_eq!(ren.lines[0], (30, 40));
    }

    #[test]
    fn test_close() {
        let mut ren = MockRenderer::default();
        let mut ras = RasterizerOutline::new(&mut ren);
        ras.move_to(0, 0);
        ras.line_to(100, 0);
        ras.line_to(100, 100);
        ras.close();

        // Should draw line back to (0, 0)
        assert_eq!(ren.lines.len(), 3);
        assert_eq!(ren.lines[2], (0, 0));
    }

    #[test]
    fn test_close_with_fewer_than_3_vertices() {
        let mut ren = MockRenderer::default();
        let mut ras = RasterizerOutline::new(&mut ren);
        ras.move_to(0, 0);
        ras.line_to(100, 0);
        ras.close();

        // Only 2 vertices — close() should not add a line
        assert_eq!(ren.lines.len(), 1);
    }

    #[test]
    fn test_add_path() {
        struct TrianglePath {
            idx: usize,
        }
        impl VertexSource for TrianglePath {
            fn rewind(&mut self, _path_id: u32) {
                self.idx = 0;
            }
            fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
                let verts: [(f64, f64, u32); 4] = [
                    (0.0, 0.0, PATH_CMD_MOVE_TO),
                    (10.0, 0.0, PATH_CMD_LINE_TO),
                    (5.0, 10.0, PATH_CMD_LINE_TO),
                    (0.0, 0.0, PATH_CMD_STOP),
                ];
                if self.idx >= verts.len() {
                    return PATH_CMD_STOP;
                }
                let v = verts[self.idx];
                *x = v.0;
                *y = v.1;
                self.idx += 1;
                v.2
            }
        }

        let mut ren = MockRenderer::default();
        let mut ras = RasterizerOutline::new(&mut ren);
        let mut path = TrianglePath { idx: 0 };
        ras.add_path(&mut path, 0);

        assert_eq!(ren.moves.len(), 1);
        assert_eq!(ren.lines.len(), 2);
    }

    #[test]
    fn test_move_to_d() {
        let mut ren = MockRenderer::default();
        let mut ras = RasterizerOutline::new(&mut ren);
        ras.move_to_d(1.5, 2.5);
        // coord(1.5) = 384, coord(2.5) = 640
        assert_eq!(ren.moves[0], (384, 640));
    }
}

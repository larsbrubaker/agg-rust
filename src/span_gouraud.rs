//! Gouraud shading base — triangle with per-vertex colors.
//!
//! Port of `agg_span_gouraud.h` — stores triangle vertices with associated
//! colors, sorts them by Y, and provides a `VertexSource` interface for
//! feeding the triangle outline to the rasterizer.

use crate::basics::{VertexSource, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
use crate::math::{calc_intersection, dilate_triangle};

// ============================================================================
// CoordType — vertex with position and color
// ============================================================================

/// Triangle vertex with position and color.
///
/// Port of C++ `span_gouraud::coord_type`.
#[derive(Clone)]
pub struct CoordType<C: Clone> {
    pub x: f64,
    pub y: f64,
    pub color: C,
}

impl<C: Clone + Default> Default for CoordType<C> {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            color: C::default(),
        }
    }
}

// ============================================================================
// SpanGouraud
// ============================================================================

/// Gouraud shading base class.
///
/// Stores a triangle with per-vertex colors. Implements `VertexSource`
/// to feed the (possibly dilated) triangle outline to the rasterizer.
///
/// Dilation produces a 6-vertex beveled polygon for numerical stability,
/// while the color interpolation coordinates use miter joins calculated
/// via `calc_intersection`.
///
/// Port of C++ `span_gouraud<ColorT>`.
pub struct SpanGouraud<C: Clone> {
    coord: [CoordType<C>; 3],
    x: [f64; 8],
    y: [f64; 8],
    cmd: [u32; 8],
    vertex: usize,
}

impl<C: Clone + Default> SpanGouraud<C> {
    pub fn new() -> Self {
        let mut s = Self {
            coord: [
                CoordType::default(),
                CoordType::default(),
                CoordType::default(),
            ],
            x: [0.0; 8],
            y: [0.0; 8],
            cmd: [PATH_CMD_STOP; 8],
            vertex: 0,
        };
        s.cmd[0] = PATH_CMD_STOP;
        s
    }

    /// Construct with colors and triangle geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_triangle(
        c1: C,
        c2: C,
        c3: C,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        d: f64,
    ) -> Self {
        let mut s = Self::new();
        s.colors(c1, c2, c3);
        s.triangle(x1, y1, x2, y2, x3, y3, d);
        s
    }

    /// Set the vertex colors.
    pub fn colors(&mut self, c1: C, c2: C, c3: C) {
        self.coord[0].color = c1;
        self.coord[1].color = c2;
        self.coord[2].color = c3;
    }

    /// Set the triangle geometry and optionally dilate it.
    ///
    /// When `d != 0`, the triangle is dilated to form a 6-vertex beveled
    /// polygon for numerical stability. The color interpolation coordinates
    /// are recalculated using miter-join intersections.
    #[allow(clippy::too_many_arguments)]
    pub fn triangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, d: f64) {
        self.coord[0].x = x1;
        self.coord[0].y = y1;
        self.coord[1].x = x2;
        self.coord[1].y = y2;
        self.coord[2].x = x3;
        self.coord[2].y = y3;

        self.x[0] = x1;
        self.y[0] = y1;
        self.x[1] = x2;
        self.y[1] = y2;
        self.x[2] = x3;
        self.y[2] = y3;

        self.cmd[0] = PATH_CMD_MOVE_TO;
        self.cmd[1] = PATH_CMD_LINE_TO;
        self.cmd[2] = PATH_CMD_LINE_TO;
        self.cmd[3] = PATH_CMD_STOP;

        if d != 0.0 {
            let (dx, dy) = dilate_triangle(
                self.coord[0].x,
                self.coord[0].y,
                self.coord[1].x,
                self.coord[1].y,
                self.coord[2].x,
                self.coord[2].y,
                d,
            );
            self.x[..6].copy_from_slice(&dx);
            self.y[..6].copy_from_slice(&dy);

            // Recalculate color interpolation coords using miter joins
            if let Some((ix, iy)) = calc_intersection(
                self.x[4], self.y[4], self.x[5], self.y[5], self.x[0], self.y[0], self.x[1],
                self.y[1],
            ) {
                self.coord[0].x = ix;
                self.coord[0].y = iy;
            }

            if let Some((ix, iy)) = calc_intersection(
                self.x[0], self.y[0], self.x[1], self.y[1], self.x[2], self.y[2], self.x[3],
                self.y[3],
            ) {
                self.coord[1].x = ix;
                self.coord[1].y = iy;
            }

            if let Some((ix, iy)) = calc_intersection(
                self.x[2], self.y[2], self.x[3], self.y[3], self.x[4], self.y[4], self.x[5],
                self.y[5],
            ) {
                self.coord[2].x = ix;
                self.coord[2].y = iy;
            }

            self.cmd[3] = PATH_CMD_LINE_TO;
            self.cmd[4] = PATH_CMD_LINE_TO;
            self.cmd[5] = PATH_CMD_LINE_TO;
            self.cmd[6] = PATH_CMD_STOP;
        }
    }

    /// Sort vertices by Y coordinate (top to bottom).
    ///
    /// Returns an array of three `CoordType` sorted so that
    /// `result[0].y <= result[1].y <= result[2].y`.
    pub fn arrange_vertices(&self) -> [CoordType<C>; 3] {
        let mut coord = [
            self.coord[0].clone(),
            self.coord[1].clone(),
            self.coord[2].clone(),
        ];

        if self.coord[0].y > self.coord[2].y {
            coord[0] = self.coord[2].clone();
            coord[2] = self.coord[0].clone();
        }

        if coord[0].y > coord[1].y {
            let tmp = coord[0].clone();
            coord[0] = coord[1].clone();
            coord[1] = tmp;
        }

        if coord[1].y > coord[2].y {
            let tmp = coord[1].clone();
            coord[1] = coord[2].clone();
            coord[2] = tmp;
        }

        coord
    }
}

impl<C: Clone + Default> Default for SpanGouraud<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Clone + Default> VertexSource for SpanGouraud<C> {
    fn rewind(&mut self, _path_id: u32) {
        self.vertex = 0;
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        let idx = self.vertex;
        *x = self.x[idx];
        *y = self.y[idx];
        let cmd = self.cmd[idx];
        self.vertex += 1;
        cmd
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;

    #[test]
    fn test_new_default() {
        let sg = SpanGouraud::<Rgba8>::new();
        assert_eq!(sg.cmd[0], PATH_CMD_STOP);
    }

    #[test]
    fn test_colors() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue = Rgba8::new(0, 0, 255, 255);
        sg.colors(red, green, blue);
        assert_eq!(sg.coord[0].color.r, 255);
        assert_eq!(sg.coord[1].color.g, 255);
        assert_eq!(sg.coord[2].color.b, 255);
    }

    #[test]
    fn test_triangle_no_dilation() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        sg.triangle(0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 0.0);

        assert_eq!(sg.cmd[0], PATH_CMD_MOVE_TO);
        assert_eq!(sg.cmd[1], PATH_CMD_LINE_TO);
        assert_eq!(sg.cmd[2], PATH_CMD_LINE_TO);
        assert_eq!(sg.cmd[3], PATH_CMD_STOP);

        assert_eq!(sg.x[0], 0.0);
        assert_eq!(sg.y[0], 0.0);
        assert_eq!(sg.x[1], 100.0);
        assert_eq!(sg.y[1], 0.0);
        assert_eq!(sg.x[2], 50.0);
        assert_eq!(sg.y[2], 100.0);
    }

    #[test]
    fn test_triangle_with_dilation() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        sg.triangle(0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 1.0);

        // Should have 6 vertices + stop
        assert_eq!(sg.cmd[0], PATH_CMD_MOVE_TO);
        assert_eq!(sg.cmd[5], PATH_CMD_LINE_TO);
        assert_eq!(sg.cmd[6], PATH_CMD_STOP);
    }

    #[test]
    fn test_vertex_source_no_dilation() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        sg.triangle(10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 0.0);
        sg.rewind(0);

        let mut x = 0.0;
        let mut y = 0.0;

        assert_eq!(sg.vertex(&mut x, &mut y), PATH_CMD_MOVE_TO);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);

        assert_eq!(sg.vertex(&mut x, &mut y), PATH_CMD_LINE_TO);
        assert_eq!(x, 30.0);
        assert_eq!(y, 40.0);

        assert_eq!(sg.vertex(&mut x, &mut y), PATH_CMD_LINE_TO);
        assert_eq!(x, 50.0);
        assert_eq!(y, 60.0);

        assert_eq!(sg.vertex(&mut x, &mut y), PATH_CMD_STOP);
    }

    #[test]
    fn test_arrange_vertices() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue = Rgba8::new(0, 0, 255, 255);
        sg.colors(red, green, blue);
        // Vertices: (50, 100), (0, 0), (100, 50) — intentionally unsorted
        sg.triangle(50.0, 100.0, 0.0, 0.0, 100.0, 50.0, 0.0);

        let sorted = sg.arrange_vertices();
        assert!(sorted[0].y <= sorted[1].y);
        assert!(sorted[1].y <= sorted[2].y);
        // Top vertex should have y=0 (originally vertex 1)
        assert_eq!(sorted[0].y, 0.0);
        assert_eq!(sorted[0].color.g, 255); // green vertex
    }

    #[test]
    fn test_new_with_triangle() {
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue = Rgba8::new(0, 0, 255, 255);
        let sg = SpanGouraud::new_with_triangle(
            red, green, blue, 0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 0.0,
        );
        assert_eq!(sg.coord[0].color.r, 255);
        assert_eq!(sg.coord[1].color.g, 255);
        assert_eq!(sg.x[2], 50.0);
    }

    #[test]
    fn test_arrange_already_sorted() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        let c = Rgba8::new(128, 128, 128, 255);
        sg.colors(c, c, c);
        sg.triangle(0.0, 0.0, 50.0, 50.0, 100.0, 100.0, 0.0);

        let sorted = sg.arrange_vertices();
        assert_eq!(sorted[0].y, 0.0);
        assert_eq!(sorted[1].y, 50.0);
        assert_eq!(sorted[2].y, 100.0);
    }

    #[test]
    fn test_rewind_resets_vertex() {
        let mut sg = SpanGouraud::<Rgba8>::new();
        sg.triangle(0.0, 0.0, 10.0, 10.0, 20.0, 0.0, 0.0);

        let mut x = 0.0;
        let mut y = 0.0;
        sg.rewind(0);
        sg.vertex(&mut x, &mut y);
        sg.vertex(&mut x, &mut y);

        // Rewind should reset
        sg.rewind(0);
        let cmd = sg.vertex(&mut x, &mut y);
        assert_eq!(cmd, PATH_CMD_MOVE_TO);
        assert_eq!(x, 0.0);
    }
}

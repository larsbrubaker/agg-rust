//! Bounding rectangle calculation.
//!
//! Port of `agg_bounding_rect.h` — computes the axis-aligned bounding box
//! of a vertex source.

use crate::basics::{is_stop, is_vertex, RectD, VertexSource};

/// Compute the bounding rectangle of a single path from a vertex source.
///
/// Rewinds the vertex source to `path_id`, iterates all vertices, and
/// returns the axis-aligned bounding box. Returns `None` if no vertices
/// are found.
///
/// Port of C++ `agg::bounding_rect_single`.
pub fn bounding_rect_single(vs: &mut dyn VertexSource, path_id: u32) -> Option<RectD> {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut first = true;
    let mut x1 = 1.0_f64;
    let mut y1 = 1.0_f64;
    let mut x2 = 0.0_f64;
    let mut y2 = 0.0_f64;

    vs.rewind(path_id);
    loop {
        let cmd = vs.vertex(&mut x, &mut y);
        if is_stop(cmd) {
            break;
        }
        if is_vertex(cmd) {
            if first {
                x1 = x;
                y1 = y;
                x2 = x;
                y2 = y;
                first = false;
            } else {
                if x < x1 {
                    x1 = x;
                }
                if y < y1 {
                    y1 = y;
                }
                if x > x2 {
                    x2 = x;
                }
                if y > y2 {
                    y2 = y;
                }
            }
        }
    }

    if x1 <= x2 && y1 <= y2 {
        Some(RectD::new(x1, y1, x2, y2))
    } else {
        None
    }
}

/// Compute the bounding rectangle across multiple paths from a vertex source.
///
/// Iterates paths from `start` to `start + num - 1`, rewinding each by
/// its path ID (obtained from `path_ids`), and returns the combined
/// bounding box. Returns `None` if no vertices are found.
///
/// Port of C++ `agg::bounding_rect`.
pub fn bounding_rect(
    vs: &mut dyn VertexSource,
    path_ids: &[u32],
    start: usize,
    num: usize,
) -> Option<RectD> {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut first = true;
    let mut x1 = 1.0_f64;
    let mut y1 = 1.0_f64;
    let mut x2 = 0.0_f64;
    let mut y2 = 0.0_f64;

    for i in 0..num {
        vs.rewind(path_ids[start + i]);
        loop {
            let cmd = vs.vertex(&mut x, &mut y);
            if is_stop(cmd) {
                break;
            }
            if is_vertex(cmd) {
                if first {
                    x1 = x;
                    y1 = y;
                    x2 = x;
                    y2 = y;
                    first = false;
                } else {
                    if x < x1 {
                        x1 = x;
                    }
                    if y < y1 {
                        y1 = y;
                    }
                    if x > x2 {
                        x2 = x;
                    }
                    if y > y2 {
                        y2 = y;
                    }
                }
            }
        }
    }

    if x1 <= x2 && y1 <= y2 {
        Some(RectD::new(x1, y1, x2, y2))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basics::{PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP};
    use crate::ellipse::Ellipse;

    /// Minimal test vertex source: a triangle.
    struct Triangle {
        vertices: [(f64, f64); 3],
        index: usize,
    }

    impl Triangle {
        fn new(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> Self {
            Self {
                vertices: [(x1, y1), (x2, y2), (x3, y3)],
                index: 0,
            }
        }
    }

    impl VertexSource for Triangle {
        fn rewind(&mut self, _path_id: u32) {
            self.index = 0;
        }

        fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
            if self.index < 3 {
                *x = self.vertices[self.index].0;
                *y = self.vertices[self.index].1;
                self.index += 1;
                if self.index == 1 {
                    PATH_CMD_MOVE_TO
                } else {
                    PATH_CMD_LINE_TO
                }
            } else {
                PATH_CMD_STOP
            }
        }
    }

    #[test]
    fn test_bounding_rect_single_triangle() {
        let mut tri = Triangle::new(10.0, 20.0, 50.0, 80.0, 30.0, 10.0);
        let r = bounding_rect_single(&mut tri, 0).unwrap();
        assert!((r.x1 - 10.0).abs() < 1e-10);
        assert!((r.y1 - 10.0).abs() < 1e-10);
        assert!((r.x2 - 50.0).abs() < 1e-10);
        assert!((r.y2 - 80.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_rect_single_ellipse() {
        let mut e = Ellipse::new(50.0, 50.0, 30.0, 20.0, 64, false);
        let r = bounding_rect_single(&mut e, 0).unwrap();
        // Ellipse center (50,50), rx=30, ry=20
        assert!((r.x1 - 20.0).abs() < 1.0); // ~20
        assert!((r.y1 - 30.0).abs() < 1.0); // ~30
        assert!((r.x2 - 80.0).abs() < 1.0); // ~80
        assert!((r.y2 - 70.0).abs() < 1.0); // ~70
    }

    #[test]
    fn test_bounding_rect_empty_returns_none() {
        struct Empty;
        impl VertexSource for Empty {
            fn rewind(&mut self, _: u32) {}
            fn vertex(&mut self, _x: &mut f64, _y: &mut f64) -> u32 {
                PATH_CMD_STOP
            }
        }
        let mut e = Empty;
        assert!(bounding_rect_single(&mut e, 0).is_none());
    }

    #[test]
    fn test_bounding_rect_single_point() {
        struct SinglePoint;
        impl VertexSource for SinglePoint {
            fn rewind(&mut self, _: u32) {}
            fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
                // Only return one vertex, then stop
                static mut CALLED: bool = false;
                unsafe {
                    if !CALLED {
                        CALLED = true;
                        *x = 42.0;
                        *y = 17.0;
                        PATH_CMD_MOVE_TO
                    } else {
                        CALLED = false; // reset for next test
                        PATH_CMD_STOP
                    }
                }
            }
        }
        let mut sp = SinglePoint;
        let r = bounding_rect_single(&mut sp, 0).unwrap();
        assert!((r.x1 - 42.0).abs() < 1e-10);
        assert!((r.y1 - 17.0).abs() < 1e-10);
        assert!((r.x2 - 42.0).abs() < 1e-10);
        assert!((r.y2 - 17.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_rect_multi_path() {
        let mut tri = Triangle::new(10.0, 20.0, 50.0, 80.0, 30.0, 10.0);
        let ids = [0u32];
        let r = bounding_rect(&mut tri, &ids, 0, 1).unwrap();
        assert!((r.x1 - 10.0).abs() < 1e-10);
        assert!((r.y1 - 10.0).abs() < 1e-10);
        assert!((r.x2 - 50.0).abs() < 1e-10);
        assert!((r.y2 - 80.0).abs() < 1e-10);
    }
}

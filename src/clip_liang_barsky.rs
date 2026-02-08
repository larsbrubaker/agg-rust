//! Liang-Barsky line clipping algorithm.
//!
//! Port of `agg_clip_liang_barsky.h` — parametric line clipping against
//! a rectangular clip box, plus Cohen-Sutherland outcode helpers.

use crate::basics::Rect;

// ============================================================================
// Clipping flags (Cohen-Sutherland outcodes)
// ============================================================================

pub const CLIPPING_FLAGS_X1_CLIPPED: u32 = 4;
pub const CLIPPING_FLAGS_X2_CLIPPED: u32 = 1;
pub const CLIPPING_FLAGS_Y1_CLIPPED: u32 = 8;
pub const CLIPPING_FLAGS_Y2_CLIPPED: u32 = 2;
pub const CLIPPING_FLAGS_X_CLIPPED: u32 = CLIPPING_FLAGS_X1_CLIPPED | CLIPPING_FLAGS_X2_CLIPPED;
pub const CLIPPING_FLAGS_Y_CLIPPED: u32 = CLIPPING_FLAGS_Y1_CLIPPED | CLIPPING_FLAGS_Y2_CLIPPED;

/// Compute Cohen-Sutherland outcode for point (x, y) against clip_box.
///
/// ```text
///        |        |
///  0110  |  0010  | 0011
///        |        |
/// -------+--------+-------- clip_box.y2
///        |        |
///  0100  |  0000  | 0001
///        |        |
/// -------+--------+-------- clip_box.y1
///        |        |
///  1100  |  1000  | 1001
///        |        |
///  clip_box.x1  clip_box.x2
/// ```
#[inline]
pub fn clipping_flags<T: Copy + PartialOrd>(x: T, y: T, clip_box: &Rect<T>) -> u32 {
    (x > clip_box.x2) as u32
        | (((y > clip_box.y2) as u32) << 1)
        | (((x < clip_box.x1) as u32) << 2)
        | (((y < clip_box.y1) as u32) << 3)
}

/// Compute x-axis clipping flags only.
#[inline]
pub fn clipping_flags_x<T: Copy + PartialOrd>(x: T, clip_box: &Rect<T>) -> u32 {
    (x > clip_box.x2) as u32 | (((x < clip_box.x1) as u32) << 2)
}

/// Compute y-axis clipping flags only.
#[inline]
pub fn clipping_flags_y<T: Copy + PartialOrd>(y: T, clip_box: &Rect<T>) -> u32 {
    (((y > clip_box.y2) as u32) << 1) | (((y < clip_box.y1) as u32) << 3)
}

// ============================================================================
// Liang-Barsky parametric line clipping
// ============================================================================

/// Clip a line segment against a rectangle using the Liang-Barsky algorithm.
///
/// Returns the number of output points (0 = fully clipped, 1-2 = partially visible).
/// Output coordinates are written to `x_out` and `y_out` arrays.
///
/// Port of C++ `clip_liang_barsky<T>`.
pub fn clip_liang_barsky_f64(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    clip_box: &Rect<f64>,
    x_out: &mut [f64],
    y_out: &mut [f64],
) -> u32 {
    const NEARZERO: f64 = 1e-30;

    let mut deltax = x2 - x1;
    let mut deltay = y2 - y1;

    if deltax == 0.0 {
        deltax = if x1 > clip_box.x1 {
            -NEARZERO
        } else {
            NEARZERO
        };
    }

    if deltay == 0.0 {
        deltay = if y1 > clip_box.y1 {
            -NEARZERO
        } else {
            NEARZERO
        };
    }

    let (xin, xout) = if deltax > 0.0 {
        (clip_box.x1, clip_box.x2)
    } else {
        (clip_box.x2, clip_box.x1)
    };

    let (yin, yout) = if deltay > 0.0 {
        (clip_box.y1, clip_box.y2)
    } else {
        (clip_box.y2, clip_box.y1)
    };

    let tinx = (xin - x1) / deltax;
    let tiny = (yin - y1) / deltay;

    let (tin1, tin2) = if tinx < tiny {
        (tinx, tiny)
    } else {
        (tiny, tinx)
    };

    let mut np: u32 = 0;
    let mut idx = 0;

    if tin1 <= 1.0 {
        if 0.0 < tin1 {
            x_out[idx] = xin;
            y_out[idx] = yin;
            idx += 1;
            np += 1;
        }

        if tin2 <= 1.0 {
            let toutx = (xout - x1) / deltax;
            let touty = (yout - y1) / deltay;
            let tout1 = if toutx < touty { toutx } else { touty };

            if tin2 > 0.0 || tout1 > 0.0 {
                if tin2 <= tout1 {
                    if tin2 > 0.0 {
                        if tinx > tiny {
                            x_out[idx] = xin;
                            y_out[idx] = y1 + tinx * deltay;
                        } else {
                            x_out[idx] = x1 + tiny * deltax;
                            y_out[idx] = yin;
                        }
                        idx += 1;
                        np += 1;
                    }

                    if tout1 < 1.0 {
                        if toutx < touty {
                            x_out[idx] = xout;
                            y_out[idx] = y1 + toutx * deltay;
                        } else {
                            x_out[idx] = x1 + touty * deltax;
                            y_out[idx] = yout;
                        }
                    } else {
                        x_out[idx] = x2;
                        y_out[idx] = y2;
                    }
                    np += 1;
                } else {
                    if tinx > tiny {
                        x_out[idx] = xin;
                        y_out[idx] = yout;
                    } else {
                        x_out[idx] = xout;
                        y_out[idx] = yin;
                    }
                    np += 1;
                }
            }
        }
    }
    np
}

// ============================================================================
// Cohen-Sutherland clip helpers
// ============================================================================

/// Move a clipped point to the clip boundary.
/// Returns `false` if the line is degenerate.
#[allow(clippy::too_many_arguments)]
pub fn clip_move_point_f64(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    clip_box: &Rect<f64>,
    x: &mut f64,
    y: &mut f64,
    flags: u32,
) -> bool {
    if (flags & CLIPPING_FLAGS_X_CLIPPED) != 0 {
        if x1 == x2 {
            return false;
        }
        let bound = if (flags & CLIPPING_FLAGS_X1_CLIPPED) != 0 {
            clip_box.x1
        } else {
            clip_box.x2
        };
        *y = (bound - x1) * (y2 - y1) / (x2 - x1) + y1;
        *x = bound;
    }

    let flags = clipping_flags_y(*y, clip_box);
    if (flags & CLIPPING_FLAGS_Y_CLIPPED) != 0 {
        if y1 == y2 {
            return false;
        }
        let bound = if (flags & CLIPPING_FLAGS_Y1_CLIPPED) != 0 {
            clip_box.y1
        } else {
            clip_box.y2
        };
        *x = (bound - y1) * (x2 - x1) / (y2 - y1) + x1;
        *y = bound;
    }
    true
}

/// Clip a line segment using Cohen-Sutherland quick rejection + point moving.
///
/// Returns:
/// - `>= 4` : fully clipped
/// - `bit 0 set` : first point was moved
/// - `bit 1 set` : second point was moved
/// - `0` : fully visible
pub fn clip_line_segment_f64(
    x1: &mut f64,
    y1: &mut f64,
    x2: &mut f64,
    y2: &mut f64,
    clip_box: &Rect<f64>,
) -> u32 {
    let f1 = clipping_flags(*x1, *y1, clip_box);
    let f2 = clipping_flags(*x2, *y2, clip_box);
    let mut ret: u32 = 0;

    if (f2 | f1) == 0 {
        return 0; // Fully visible
    }

    if (f1 & CLIPPING_FLAGS_X_CLIPPED) != 0
        && (f1 & CLIPPING_FLAGS_X_CLIPPED) == (f2 & CLIPPING_FLAGS_X_CLIPPED)
    {
        return 4; // Fully clipped
    }

    if (f1 & CLIPPING_FLAGS_Y_CLIPPED) != 0
        && (f1 & CLIPPING_FLAGS_Y_CLIPPED) == (f2 & CLIPPING_FLAGS_Y_CLIPPED)
    {
        return 4; // Fully clipped
    }

    let tx1 = *x1;
    let ty1 = *y1;
    let tx2 = *x2;
    let ty2 = *y2;

    if f1 != 0 {
        if !clip_move_point_f64(tx1, ty1, tx2, ty2, clip_box, x1, y1, f1) {
            return 4;
        }
        if *x1 == *x2 && *y1 == *y2 {
            return 4;
        }
        ret |= 1;
    }

    if f2 != 0 {
        if !clip_move_point_f64(tx1, ty1, tx2, ty2, clip_box, x2, y2, f2) {
            return 4;
        }
        if *x1 == *x2 && *y1 == *y2 {
            return 4;
        }
        ret |= 2;
    }

    ret
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn clip_box() -> Rect<f64> {
        Rect::new(10.0, 10.0, 100.0, 100.0)
    }

    #[test]
    fn test_clipping_flags_inside() {
        let cb = clip_box();
        assert_eq!(clipping_flags(50.0, 50.0, &cb), 0);
    }

    #[test]
    fn test_clipping_flags_right() {
        let cb = clip_box();
        assert_eq!(clipping_flags(110.0, 50.0, &cb), CLIPPING_FLAGS_X2_CLIPPED);
    }

    #[test]
    fn test_clipping_flags_left() {
        let cb = clip_box();
        assert_eq!(clipping_flags(5.0, 50.0, &cb), CLIPPING_FLAGS_X1_CLIPPED);
    }

    #[test]
    fn test_clipping_flags_above() {
        let cb = clip_box();
        assert_eq!(clipping_flags(50.0, 110.0, &cb), CLIPPING_FLAGS_Y2_CLIPPED);
    }

    #[test]
    fn test_clipping_flags_below() {
        let cb = clip_box();
        assert_eq!(clipping_flags(50.0, 5.0, &cb), CLIPPING_FLAGS_Y1_CLIPPED);
    }

    #[test]
    fn test_clipping_flags_corner() {
        let cb = clip_box();
        // Top-right corner
        assert_eq!(
            clipping_flags(110.0, 110.0, &cb),
            CLIPPING_FLAGS_X2_CLIPPED | CLIPPING_FLAGS_Y2_CLIPPED
        );
        // Bottom-left corner
        assert_eq!(
            clipping_flags(5.0, 5.0, &cb),
            CLIPPING_FLAGS_X1_CLIPPED | CLIPPING_FLAGS_Y1_CLIPPED
        );
    }

    #[test]
    fn test_liang_barsky_fully_inside() {
        let cb = clip_box();
        let mut x_out = [0.0; 4];
        let mut y_out = [0.0; 4];
        // For a fully-inside line, np=1: the output is the endpoint.
        // The start point (x1,y1) is implicitly unchanged.
        let np = clip_liang_barsky_f64(20.0, 20.0, 80.0, 80.0, &cb, &mut x_out, &mut y_out);
        assert_eq!(np, 1);
        assert!((x_out[0] - 80.0).abs() < 1e-6);
        assert!((y_out[0] - 80.0).abs() < 1e-6);
    }

    #[test]
    fn test_liang_barsky_fully_outside() {
        let cb = clip_box();
        let mut x_out = [0.0; 4];
        let mut y_out = [0.0; 4];
        // Line completely above the clip box
        let np = clip_liang_barsky_f64(20.0, 110.0, 80.0, 110.0, &cb, &mut x_out, &mut y_out);
        assert_eq!(np, 0);
    }

    #[test]
    fn test_liang_barsky_crossing() {
        let cb = clip_box();
        let mut x_out = [0.0; 4];
        let mut y_out = [0.0; 4];
        // Line from left of box through to right
        let np = clip_liang_barsky_f64(0.0, 50.0, 120.0, 50.0, &cb, &mut x_out, &mut y_out);
        assert_eq!(np, 2);
        assert!((x_out[0] - 10.0).abs() < 1e-6);
        assert!((y_out[0] - 50.0).abs() < 1e-6);
        assert!((x_out[1] - 100.0).abs() < 1e-6);
        assert!((y_out[1] - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_clip_line_segment_fully_visible() {
        let cb = clip_box();
        let mut x1 = 20.0;
        let mut y1 = 20.0;
        let mut x2 = 80.0;
        let mut y2 = 80.0;
        let ret = clip_line_segment_f64(&mut x1, &mut y1, &mut x2, &mut y2, &cb);
        assert_eq!(ret, 0);
    }

    #[test]
    fn test_clip_line_segment_fully_clipped() {
        let cb = clip_box();
        let mut x1 = 0.0;
        let mut y1 = 0.0;
        let mut x2 = 5.0;
        let mut y2 = 5.0;
        let ret = clip_line_segment_f64(&mut x1, &mut y1, &mut x2, &mut y2, &cb);
        assert!(ret >= 4);
    }

    #[test]
    fn test_clip_line_segment_partial() {
        let cb = clip_box();
        let mut x1 = 50.0;
        let mut y1 = 50.0;
        let mut x2 = 200.0;
        let mut y2 = 50.0;
        let ret = clip_line_segment_f64(&mut x1, &mut y1, &mut x2, &mut y2, &cb);
        // Point 1 inside, point 2 moved
        assert_eq!(ret & 1, 0); // first point not moved
        assert_ne!(ret & 2, 0); // second point moved
        assert!((x2 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_clipping_flags_x() {
        let cb = clip_box();
        assert_eq!(clipping_flags_x(5.0, &cb), CLIPPING_FLAGS_X1_CLIPPED);
        assert_eq!(clipping_flags_x(110.0, &cb), CLIPPING_FLAGS_X2_CLIPPED);
        assert_eq!(clipping_flags_x(50.0, &cb), 0);
    }

    #[test]
    fn test_clipping_flags_y() {
        let cb = clip_box();
        assert_eq!(clipping_flags_y(5.0, &cb), CLIPPING_FLAGS_Y1_CLIPPED);
        assert_eq!(clipping_flags_y(110.0, &cb), CLIPPING_FLAGS_Y2_CLIPPED);
        assert_eq!(clipping_flags_y(50.0, &cb), 0);
    }
}

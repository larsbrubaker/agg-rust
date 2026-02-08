//! Anti-aliased line rendering basics.
//!
//! Port of `agg_line_aa_basics.h` + `agg_line_aa_basics.cpp`.
//! Constants, coordinate converters, `LineParameters` struct, and bisectrix.

use crate::basics::iround;

// ============================================================================
// Constants
// ============================================================================

pub const LINE_SUBPIXEL_SHIFT: i32 = 8;
pub const LINE_SUBPIXEL_SCALE: i32 = 1 << LINE_SUBPIXEL_SHIFT; // 256
pub const LINE_SUBPIXEL_MASK: i32 = LINE_SUBPIXEL_SCALE - 1; // 255
pub const LINE_MAX_COORD: i32 = (1 << 28) - 1;
pub const LINE_MAX_LENGTH: i32 = 1 << 18; // ~262144

pub const LINE_MR_SUBPIXEL_SHIFT: i32 = 4;
pub const LINE_MR_SUBPIXEL_SCALE: i32 = 1 << LINE_MR_SUBPIXEL_SHIFT; // 16
pub const LINE_MR_SUBPIXEL_MASK: i32 = LINE_MR_SUBPIXEL_SCALE - 1; // 15

/// High-to-medium resolution.
#[inline]
pub fn line_mr(x: i32) -> i32 {
    x >> (LINE_SUBPIXEL_SHIFT - LINE_MR_SUBPIXEL_SHIFT)
}

/// Medium-to-high resolution.
#[inline]
pub fn line_hr(x: i32) -> i32 {
    x << (LINE_SUBPIXEL_SHIFT - LINE_MR_SUBPIXEL_SHIFT)
}

/// Integer to double-high resolution.
#[inline]
pub fn line_dbl_hr(x: i32) -> i32 {
    x << LINE_SUBPIXEL_SHIFT
}

// ============================================================================
// Coordinate converters
// ============================================================================

/// Convert f64 to subpixel integer coordinate.
#[inline]
pub fn line_coord(x: f64) -> i32 {
    iround(x * LINE_SUBPIXEL_SCALE as f64)
}

/// Convert f64 to subpixel integer coordinate with saturation.
#[inline]
pub fn line_coord_sat(x: f64) -> i32 {
    let v = iround(x * LINE_SUBPIXEL_SCALE as f64);
    v.max(-LINE_MAX_COORD).min(LINE_MAX_COORD)
}

// ============================================================================
// LineParameters
// ============================================================================

/// Orthogonal quadrant lookup by octant.
const S_ORTHOGONAL_QUADRANT: [u8; 8] = [0, 0, 1, 1, 3, 3, 2, 2];

/// Diagonal quadrant lookup by octant.
const S_DIAGONAL_QUADRANT: [u8; 8] = [0, 1, 2, 1, 0, 3, 2, 3];

/// Parameters describing a line segment for AA rendering.
///
/// Port of C++ `line_parameters`.
#[derive(Debug, Clone, Copy)]
pub struct LineParameters {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub dx: i32,
    pub dy: i32,
    pub sx: i32,
    pub sy: i32,
    pub vertical: bool,
    pub inc: i32,
    pub len: i32,
    pub octant: usize,
}

impl LineParameters {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, len: i32) -> Self {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let sx = if x2 > x1 { 1 } else { -1 };
        let sy = if y2 > y1 { 1 } else { -1 };
        let vertical = dy >= dx;
        let inc = if vertical { sy } else { sx };
        // octant = (sy & 4) | (sx & 2) | int(vertical)
        let octant = ((sy as u32 & 4) | (sx as u32 & 2) | (vertical as u32)) as usize;
        Self {
            x1, y1, x2, y2, dx, dy, sx, sy, vertical, inc, len, octant,
        }
    }

    pub fn orthogonal_quadrant(&self) -> u8 {
        S_ORTHOGONAL_QUADRANT[self.octant]
    }

    pub fn diagonal_quadrant(&self) -> u8 {
        S_DIAGONAL_QUADRANT[self.octant]
    }

    pub fn same_orthogonal_quadrant(&self, other: &LineParameters) -> bool {
        S_ORTHOGONAL_QUADRANT[self.octant] == S_ORTHOGONAL_QUADRANT[other.octant]
    }

    pub fn same_diagonal_quadrant(&self, other: &LineParameters) -> bool {
        S_DIAGONAL_QUADRANT[self.octant] == S_DIAGONAL_QUADRANT[other.octant]
    }

    /// Split line at midpoint into two halves.
    pub fn divide(&self) -> (LineParameters, LineParameters) {
        let xmid = (self.x1 + self.x2) >> 1;
        let ymid = (self.y1 + self.y2) >> 1;
        let len2 = self.len >> 1;

        let lp1 = LineParameters::new(self.x1, self.y1, xmid, ymid, len2);
        let lp2 = LineParameters::new(xmid, ymid, self.x2, self.y2, len2);

        (lp1, lp2)
    }
}

// ============================================================================
// Bisectrix
// ============================================================================

/// Calculate the angle bisector at the junction between two consecutive lines.
///
/// Port of C++ `bisectrix()`. Used for line joins.
pub fn bisectrix(l1: &LineParameters, l2: &LineParameters, x: &mut i32, y: &mut i32) {
    let k = l2.len as f64 / l1.len as f64;
    let mut tx = l2.x2 as f64 - (l2.x1 - l1.x1) as f64 * k;
    let mut ty = l2.y2 as f64 - (l2.y1 - l1.y1) as f64 * k;

    // All bisectrices must be on the right of the line.
    // If the next point is on the left (l1 => l2.2)
    // then the bisectrix should be rotated by 180 degrees.
    if ((l2.x2 - l2.x1) as f64 * (l2.y1 - l1.y1) as f64)
        < ((l2.y2 - l2.y1) as f64 * (l2.x1 - l1.x1) as f64 + 100.0)
    {
        tx -= (tx - l2.x1 as f64) * 2.0;
        ty -= (ty - l2.y1 as f64) * 2.0;
    }

    // Check if the bisectrix is too short
    let dx = tx - l2.x1 as f64;
    let dy = ty - l2.y1 as f64;
    if (dx * dx + dy * dy).sqrt() as i32 >= LINE_SUBPIXEL_SCALE {
        *x = iround(tx);
        *y = iround(ty);
    } else {
        *x = (l2.x1 + l2.x1 + (l2.y1 - l1.y1) + (l2.y2 - l2.y1)) >> 1;
        *y = (l2.y1 + l2.y1 - (l2.x1 - l1.x1) - (l2.x2 - l2.x1)) >> 1;
    }
}

/// Fix degenerate bisectrix at line start.
#[inline]
pub fn fix_degenerate_bisectrix_start(lp: &LineParameters, x: &mut i32, y: &mut i32) {
    let d = iround(
        ((*x - lp.x2) as f64 * (lp.y2 - lp.y1) as f64
            - (*y - lp.y2) as f64 * (lp.x2 - lp.x1) as f64)
            / lp.len as f64,
    );
    if d < LINE_SUBPIXEL_SCALE / 2 {
        *x = lp.x1 + (lp.y2 - lp.y1);
        *y = lp.y1 - (lp.x2 - lp.x1);
    }
}

/// Fix degenerate bisectrix at line end.
#[inline]
pub fn fix_degenerate_bisectrix_end(lp: &LineParameters, x: &mut i32, y: &mut i32) {
    let d = iround(
        ((*x - lp.x2) as f64 * (lp.y2 - lp.y1) as f64
            - (*y - lp.y2) as f64 * (lp.x2 - lp.x1) as f64)
            / lp.len as f64,
    );
    if d < LINE_SUBPIXEL_SCALE / 2 {
        *x = lp.x2 + (lp.y2 - lp.y1);
        *y = lp.y2 - (lp.x2 - lp.x1);
    }
}

/// Integer line segment clipping (Liang-Barsky).
///
/// Returns: >= 4 means fully clipped.
/// Bit 0: first point was moved. Bit 1: second point was moved.
pub fn clip_line_segment(
    x1: &mut i32,
    y1: &mut i32,
    x2: &mut i32,
    y2: &mut i32,
    clip: &crate::basics::RectI,
) -> u32 {
    // Use f64 version and convert
    let mut fx1 = *x1 as f64;
    let mut fy1 = *y1 as f64;
    let mut fx2 = *x2 as f64;
    let mut fy2 = *y2 as f64;
    let fclip = crate::basics::Rect::new(
        clip.x1 as f64,
        clip.y1 as f64,
        clip.x2 as f64,
        clip.y2 as f64,
    );
    let ret = crate::clip_liang_barsky::clip_line_segment_f64(
        &mut fx1, &mut fy1, &mut fx2, &mut fy2, &fclip,
    );
    *x1 = iround(fx1);
    *y1 = iround(fy1);
    *x2 = iround(fx2);
    *y2 = iround(fy2);
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(LINE_SUBPIXEL_SCALE, 256);
        assert_eq!(LINE_SUBPIXEL_MASK, 255);
        assert_eq!(LINE_MR_SUBPIXEL_SCALE, 16);
    }

    #[test]
    fn test_line_mr_hr() {
        assert_eq!(line_mr(256), 16); // 256 >> 4
        assert_eq!(line_hr(16), 256); // 16 << 4
        assert_eq!(line_dbl_hr(1), 256); // 1 << 8
    }

    #[test]
    fn test_line_coord() {
        assert_eq!(line_coord(1.0), 256);
        assert_eq!(line_coord(0.5), 128);
        assert_eq!(line_coord(0.0), 0);
    }

    #[test]
    fn test_line_parameters_horizontal() {
        let lp = LineParameters::new(0, 0, 1000, 0, 1000);
        assert!(!lp.vertical);
        assert_eq!(lp.dx, 1000);
        assert_eq!(lp.dy, 0);
        assert_eq!(lp.sx, 1);
        assert_eq!(lp.sy, -1); // y2 == y1, so y2 > y1 is false → sy = -1
        assert_eq!(lp.inc, 1); // sx since horizontal
    }

    #[test]
    fn test_line_parameters_vertical() {
        let lp = LineParameters::new(0, 0, 0, 1000, 1000);
        assert!(lp.vertical);
        assert_eq!(lp.dx, 0);
        assert_eq!(lp.dy, 1000);
        assert_eq!(lp.inc, 1); // sy since vertical
    }

    #[test]
    fn test_line_parameters_divide() {
        let lp = LineParameters::new(0, 0, 1000, 1000, 1414);
        let (lp1, lp2) = lp.divide();
        assert_eq!(lp1.x1, 0);
        assert_eq!(lp1.y1, 0);
        assert_eq!(lp1.x2, 500);
        assert_eq!(lp1.y2, 500);
        assert_eq!(lp2.x1, 500);
        assert_eq!(lp2.y1, 500);
        assert_eq!(lp2.x2, 1000);
        assert_eq!(lp2.y2, 1000);
    }

    #[test]
    fn test_quadrant_lookups() {
        let lp = LineParameters::new(0, 0, 100, 100, 141);
        // sx=1, sy=1, vertical(dy>=dx) → octant = (1&4)|(1&2)|1 = 0|0|1 = 1
        assert_eq!(lp.octant, 1);
        assert_eq!(lp.orthogonal_quadrant(), 0);
        assert_eq!(lp.diagonal_quadrant(), 1);
    }

    #[test]
    fn test_bisectrix_right_angle() {
        // Two perpendicular lines: (0,0)→(256,0) then (256,0)→(256,256)
        let l1 = LineParameters::new(0, 0, 256, 0, 256);
        let l2 = LineParameters::new(256, 0, 256, 256, 256);
        let (mut x, mut y) = (0, 0);
        bisectrix(&l1, &l2, &mut x, &mut y);
        // Bisector should point diagonally at ~45°
        assert!(x > 256, "bisectrix x={x} should be > 256");
        assert!(y < 0, "bisectrix y={y} should be < 0");
    }

    #[test]
    fn test_clip_line_segment_visible() {
        let clip = crate::basics::RectI::new(0, 0, 1000, 1000);
        let (mut x1, mut y1, mut x2, mut y2) = (100, 100, 500, 500);
        let ret = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &clip);
        assert_eq!(ret, 0); // fully visible
    }

    #[test]
    fn test_clip_line_segment_clipped() {
        let clip = crate::basics::RectI::new(100, 100, 400, 400);
        let (mut x1, mut y1, mut x2, mut y2) = (0, 0, 500, 500);
        let ret = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &clip);
        assert!(ret < 4); // not fully clipped
        assert_ne!(ret, 0); // at least one point moved
    }
}

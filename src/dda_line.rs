//! DDA (Digital Differential Analyzer) line interpolation algorithms.
//!
//! Port of `agg_dda_line.h` — efficient line interpolation using integer
//! arithmetic for rasterization.

// ============================================================================
// DDA line interpolator (fixed-point with configurable shift)
// ============================================================================

/// Fixed-point DDA line interpolator with configurable precision.
///
/// Port of C++ `dda_line_interpolator<FractionShift, YShift>`.
/// Uses bit-shift arithmetic for sub-pixel precision.
pub struct DdaLineInterpolator<const FRACTION_SHIFT: i32, const Y_SHIFT: i32 = 0> {
    y: i32,
    inc: i32,
    dy: i32,
}

impl<const FRACTION_SHIFT: i32, const Y_SHIFT: i32> DdaLineInterpolator<FRACTION_SHIFT, Y_SHIFT> {
    pub fn new(y1: i32, y2: i32, count: u32) -> Self {
        Self {
            y: y1,
            inc: ((y2 - y1) << FRACTION_SHIFT) / count as i32,
            dy: 0,
        }
    }

    /// Step forward one unit.
    #[inline]
    pub fn inc(&mut self) {
        self.dy += self.inc;
    }

    /// Step backward one unit.
    #[inline]
    pub fn dec(&mut self) {
        self.dy -= self.inc;
    }

    /// Step forward by `n` units.
    #[inline]
    pub fn inc_by(&mut self, n: u32) {
        self.dy += self.inc * n as i32;
    }

    /// Step backward by `n` units.
    #[inline]
    pub fn dec_by(&mut self, n: u32) {
        self.dy -= self.inc * n as i32;
    }

    /// Current Y value (shifted output).
    #[inline]
    pub fn y(&self) -> i32 {
        self.y + (self.dy >> (FRACTION_SHIFT - Y_SHIFT))
    }

    /// Raw accumulated delta.
    #[inline]
    pub fn dy(&self) -> i32 {
        self.dy
    }
}

// ============================================================================
// DDA2 line interpolator (Bresenham-style integer)
// ============================================================================

/// Integer DDA line interpolator using Bresenham-style remainder tracking.
///
/// Port of C++ `dda2_line_interpolator`.
/// Distributes rounding error evenly across all steps.
#[derive(Debug, Clone)]
pub struct Dda2LineInterpolator {
    cnt: i32,
    lft: i32,
    rem: i32,
    mod_val: i32,
    y: i32,
}

impl Dda2LineInterpolator {
    /// Forward-adjusted line from y1 to y2 over `count` steps.
    pub fn new_forward(y1: i32, y2: i32, count: i32) -> Self {
        let cnt = if count <= 0 { 1 } else { count };
        let mut lft = (y2 - y1) / cnt;
        let mut rem = (y2 - y1) % cnt;
        let mut mod_val = rem;

        if mod_val <= 0 {
            mod_val += count;
            rem += count;
            lft -= 1;
        }
        mod_val -= count;

        Self {
            cnt,
            lft,
            rem,
            mod_val,
            y: y1,
        }
    }

    /// Backward-adjusted line from y1 to y2 over `count` steps.
    pub fn new_backward(y1: i32, y2: i32, count: i32) -> Self {
        let cnt = if count <= 0 { 1 } else { count };
        let mut lft = (y2 - y1) / cnt;
        let mut rem = (y2 - y1) % cnt;
        let mut mod_val = rem;

        if mod_val <= 0 {
            mod_val += count;
            rem += count;
            lft -= 1;
        }

        Self {
            cnt,
            lft,
            rem,
            mod_val,
            y: y1,
        }
    }

    /// Relative delta over `count` steps (y starting at 0).
    pub fn new_relative(y: i32, count: i32) -> Self {
        let cnt = if count <= 0 { 1 } else { count };
        let mut lft = y / cnt;
        let mut rem = y % cnt;
        let mut mod_val = rem;

        if mod_val <= 0 {
            mod_val += count;
            rem += count;
            lft -= 1;
        }

        Self {
            cnt,
            lft,
            rem,
            mod_val,
            y: 0,
        }
    }

    /// Save state for later restoration.
    pub fn save(&self) -> [i32; 2] {
        [self.mod_val, self.y]
    }

    /// Load previously saved state.
    pub fn load(&mut self, data: &[i32; 2]) {
        self.mod_val = data[0];
        self.y = data[1];
    }

    /// Step forward one unit.
    #[inline]
    pub fn inc(&mut self) {
        self.mod_val += self.rem;
        self.y += self.lft;
        if self.mod_val > 0 {
            self.mod_val -= self.cnt;
            self.y += 1;
        }
    }

    /// Step backward one unit.
    #[inline]
    pub fn dec(&mut self) {
        if self.mod_val <= self.rem {
            self.mod_val += self.cnt;
            self.y -= 1;
        }
        self.mod_val -= self.rem;
        self.y -= self.lft;
    }

    /// Adjust forward (shift phase).
    #[inline]
    pub fn adjust_forward(&mut self) {
        self.mod_val -= self.cnt;
    }

    /// Adjust backward (shift phase).
    #[inline]
    pub fn adjust_backward(&mut self) {
        self.mod_val += self.cnt;
    }

    #[inline]
    pub fn mod_val(&self) -> i32 {
        self.mod_val
    }

    #[inline]
    pub fn rem(&self) -> i32 {
        self.rem
    }

    #[inline]
    pub fn lft(&self) -> i32 {
        self.lft
    }

    #[inline]
    pub fn y(&self) -> i32 {
        self.y
    }
}

// ============================================================================
// Bresenham line interpolator
// ============================================================================

/// Bresenham line interpolator with subpixel precision.
///
/// Port of C++ `line_bresenham_interpolator`.
/// Uses 8-bit subpixel scale (256x).
pub struct LineBresenhamInterpolator {
    x1_lr: i32,
    y1_lr: i32,
    #[allow(dead_code)]
    x2_lr: i32,
    #[allow(dead_code)]
    y2_lr: i32,
    ver: bool,
    len: u32,
    inc: i32,
    interpolator: Dda2LineInterpolator,
}

/// Subpixel constants for Bresenham interpolator.
pub const SUBPIXEL_SHIFT: i32 = 8;
pub const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;
#[allow(dead_code)]
pub const SUBPIXEL_MASK: i32 = SUBPIXEL_SCALE - 1;

/// Convert from high-resolution (subpixel) to low-resolution (pixel).
#[inline]
pub fn line_lr(v: i32) -> i32 {
    v >> SUBPIXEL_SHIFT
}

impl LineBresenhamInterpolator {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let x1_lr = line_lr(x1);
        let y1_lr = line_lr(y1);
        let x2_lr = line_lr(x2);
        let y2_lr = line_lr(y2);

        let ver = (x2_lr - x1_lr).abs() < (y2_lr - y1_lr).abs();
        let len = if ver {
            (y2_lr - y1_lr).unsigned_abs()
        } else {
            (x2_lr - x1_lr).unsigned_abs()
        };
        let inc = if ver {
            if y2 > y1 {
                1
            } else {
                -1
            }
        } else if x2 > x1 {
            1
        } else {
            -1
        };

        let interpolator = Dda2LineInterpolator::new_forward(
            if ver { x1 } else { y1 },
            if ver { x2 } else { y2 },
            len as i32,
        );

        Self {
            x1_lr,
            y1_lr,
            x2_lr,
            y2_lr,
            ver,
            len,
            inc,
            interpolator,
        }
    }

    /// True if the line is vertical-major.
    #[inline]
    pub fn is_ver(&self) -> bool {
        self.ver
    }

    /// Number of steps in the dominant axis.
    #[inline]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Direction increment (+1 or -1).
    #[inline]
    pub fn inc(&self) -> i32 {
        self.inc
    }

    /// Step along the horizontal-major axis.
    #[inline]
    pub fn hstep(&mut self) {
        self.interpolator.inc();
        self.x1_lr += self.inc;
    }

    /// Step along the vertical-major axis.
    #[inline]
    pub fn vstep(&mut self) {
        self.interpolator.inc();
        self.y1_lr += self.inc;
    }

    /// Current x1 in pixel coordinates.
    #[inline]
    pub fn x1(&self) -> i32 {
        self.x1_lr
    }

    /// Current y1 in pixel coordinates.
    #[inline]
    pub fn y1(&self) -> i32 {
        self.y1_lr
    }

    /// Current secondary axis value in pixel coordinates.
    #[inline]
    pub fn x2(&self) -> i32 {
        line_lr(self.interpolator.y())
    }

    /// Current secondary axis value in pixel coordinates.
    #[inline]
    pub fn y2(&self) -> i32 {
        line_lr(self.interpolator.y())
    }

    /// Current secondary axis value in subpixel coordinates.
    #[inline]
    pub fn x2_hr(&self) -> i32 {
        self.interpolator.y()
    }

    /// Current secondary axis value in subpixel coordinates.
    #[inline]
    pub fn y2_hr(&self) -> i32 {
        self.interpolator.y()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dda_line_interpolator_basic() {
        // Interpolate from 0 to 100 in 10 steps with 8-bit fraction
        let mut dda = DdaLineInterpolator::<8, 0>::new(0, 100, 10);
        assert_eq!(dda.y(), 0);
        for _ in 0..10 {
            dda.inc();
        }
        assert_eq!(dda.y(), 100);
    }

    #[test]
    fn test_dda_line_interpolator_midpoint() {
        let mut dda = DdaLineInterpolator::<8, 0>::new(0, 100, 10);
        for _ in 0..5 {
            dda.inc();
        }
        assert_eq!(dda.y(), 50);
    }

    #[test]
    fn test_dda_line_interpolator_backward() {
        let mut dda = DdaLineInterpolator::<8, 0>::new(0, 100, 10);
        dda.inc_by(10);
        assert_eq!(dda.y(), 100);
        dda.dec_by(10);
        assert_eq!(dda.y(), 0);
    }

    #[test]
    fn test_dda2_forward() {
        let mut dda = Dda2LineInterpolator::new_forward(0, 10, 10);
        for _ in 0..10 {
            dda.inc();
        }
        assert_eq!(dda.y(), 10);
    }

    #[test]
    fn test_dda2_forward_negative() {
        let mut dda = Dda2LineInterpolator::new_forward(10, 0, 10);
        for _ in 0..10 {
            dda.inc();
        }
        assert_eq!(dda.y(), 0);
    }

    #[test]
    fn test_dda2_backward() {
        let mut dda = Dda2LineInterpolator::new_backward(0, 10, 10);
        for _ in 0..10 {
            dda.inc();
        }
        assert_eq!(dda.y(), 10);
    }

    #[test]
    fn test_dda2_save_load() {
        let mut dda = Dda2LineInterpolator::new_forward(0, 100, 10);
        for _ in 0..5 {
            dda.inc();
        }
        let saved = dda.save();
        let y_at_5 = dda.y();

        for _ in 0..3 {
            dda.inc();
        }
        assert_ne!(dda.y(), y_at_5);

        dda.load(&saved);
        assert_eq!(dda.y(), y_at_5);
    }

    #[test]
    fn test_dda2_dec() {
        let mut dda = Dda2LineInterpolator::new_forward(0, 10, 10);
        for _ in 0..10 {
            dda.inc();
        }
        assert_eq!(dda.y(), 10);
        for _ in 0..10 {
            dda.dec();
        }
        assert_eq!(dda.y(), 0);
    }

    #[test]
    fn test_bresenham_horizontal() {
        let bi = LineBresenhamInterpolator::new(0, 0, 10 * SUBPIXEL_SCALE, 0);
        assert!(!bi.is_ver());
        assert_eq!(bi.len(), 10);
        assert_eq!(bi.inc(), 1);
    }

    #[test]
    fn test_bresenham_vertical() {
        let bi = LineBresenhamInterpolator::new(0, 0, 0, 10 * SUBPIXEL_SCALE);
        assert!(bi.is_ver());
        assert_eq!(bi.len(), 10);
        assert_eq!(bi.inc(), 1);
    }

    #[test]
    fn test_bresenham_diagonal() {
        let mut bi = LineBresenhamInterpolator::new(0, 0, 10 * SUBPIXEL_SCALE, 10 * SUBPIXEL_SCALE);
        // Diagonal: neither axis dominates, but due to < comparison, horizontal wins
        assert_eq!(bi.len(), 10);
        // Step all the way
        for _ in 0..bi.len() {
            if bi.is_ver() {
                bi.vstep();
            } else {
                bi.hstep();
            }
        }
    }

    #[test]
    fn test_bresenham_negative_direction() {
        let bi = LineBresenhamInterpolator::new(10 * SUBPIXEL_SCALE, 0, 0, 0);
        assert!(!bi.is_ver());
        assert_eq!(bi.inc(), -1);
    }
}

//! Foundation types, constants, and path command utilities.
//!
//! Port of `agg_basics.h` — the most fundamental types in AGG that
//! everything else depends on.

use core::ops::{Add, Sub};

// ============================================================================
// Rounding and conversion functions
// ============================================================================

/// Round a double to the nearest integer (round half away from zero).
/// Matches C++ AGG's `iround` (non-FISTP, non-QIFIST path).
#[inline]
pub fn iround(v: f64) -> i32 {
    if v < 0.0 {
        (v - 0.5) as i32
    } else {
        (v + 0.5) as i32
    }
}

/// Round a double to the nearest unsigned integer (round half up).
/// Matches C++ AGG's `uround`.
#[inline]
pub fn uround(v: f64) -> u32 {
    (v + 0.5) as u32
}

/// Floor a double to the nearest integer toward negative infinity.
/// Matches C++ AGG's `ifloor`.
#[inline]
pub fn ifloor(v: f64) -> i32 {
    let i = v as i32;
    i - (i as f64 > v) as i32
}

/// Floor a double to the nearest unsigned integer (truncation toward zero).
/// Matches C++ AGG's `ufloor`.
#[inline]
pub fn ufloor(v: f64) -> u32 {
    v as u32
}

/// Ceiling of a double as a signed integer.
/// Matches C++ AGG's `iceil`.
#[inline]
pub fn iceil(v: f64) -> i32 {
    v.ceil() as i32
}

/// Ceiling of a double as an unsigned integer.
/// Matches C++ AGG's `uceil`.
#[inline]
pub fn uceil(v: f64) -> u32 {
    v.ceil() as u32
}

// ============================================================================
// Saturation and fixed-point multiply
// ============================================================================

/// Round `v` to int, clamping to `[-limit, limit]`.
/// Replaces C++ template `saturation<Limit>::iround`.
#[inline]
pub fn saturation_iround(limit: i32, v: f64) -> i32 {
    if v < -(limit as f64) {
        return -limit;
    }
    if v > limit as f64 {
        return limit;
    }
    iround(v)
}

/// Fixed-point multiply: `(a * b + half) >> shift`, with rounding.
/// Replaces C++ template `mul_one<Shift>::mul`.
#[inline]
pub fn mul_one(a: u32, b: u32, shift: u32) -> u32 {
    let q = a * b + (1 << (shift - 1));
    (q + (q >> shift)) >> shift
}

// ============================================================================
// Cover (anti-aliasing) constants
// ============================================================================

/// The type used for anti-aliasing coverage values.
pub type CoverType = u8;

pub const COVER_SHIFT: u32 = 8;
pub const COVER_SIZE: u32 = 1 << COVER_SHIFT;
pub const COVER_MASK: u32 = COVER_SIZE - 1;
pub const COVER_NONE: CoverType = 0;
pub const COVER_FULL: CoverType = COVER_MASK as CoverType;

// ============================================================================
// Subpixel constants
// ============================================================================

/// These constants determine the subpixel accuracy (number of fractional bits).
/// With 8-bit fractional part and 32-bit integers, coordinate capacity is 24 bits.
pub const POLY_SUBPIXEL_SHIFT: u32 = 8;
pub const POLY_SUBPIXEL_SCALE: u32 = 1 << POLY_SUBPIXEL_SHIFT;
pub const POLY_SUBPIXEL_MASK: u32 = POLY_SUBPIXEL_SCALE - 1;

// ============================================================================
// Filling rule
// ============================================================================

/// Filling rule for polygon rasterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillingRule {
    NonZero,
    EvenOdd,
}

// ============================================================================
// Mathematical constants
// ============================================================================

pub const PI: f64 = std::f64::consts::PI;

/// Convert degrees to radians.
#[inline]
pub fn deg2rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

/// Convert radians to degrees.
#[inline]
pub fn rad2deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

// ============================================================================
// Rect
// ============================================================================

/// A rectangle defined by two corner points.
/// Port of C++ `rect_base<T>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect<T: Copy> {
    pub x1: T,
    pub y1: T,
    pub x2: T,
    pub y2: T,
}

impl<T: Copy + PartialOrd> Rect<T> {
    pub fn new(x1: T, y1: T, x2: T, y2: T) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn init(&mut self, x1: T, y1: T, x2: T, y2: T) {
        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;
    }

    /// Normalize so that x1 <= x2 and y1 <= y2, swapping if needed.
    pub fn normalize(&mut self) -> &Self {
        if self.x1 > self.x2 {
            core::mem::swap(&mut self.x1, &mut self.x2);
        }
        if self.y1 > self.y2 {
            core::mem::swap(&mut self.y1, &mut self.y2);
        }
        self
    }

    /// Clip this rectangle to the intersection with `r`.
    /// Returns `true` if the result is a valid (non-empty) rectangle.
    pub fn clip(&mut self, r: &Self) -> bool {
        if self.x2 > r.x2 {
            self.x2 = r.x2;
        }
        if self.y2 > r.y2 {
            self.y2 = r.y2;
        }
        if self.x1 < r.x1 {
            self.x1 = r.x1;
        }
        if self.y1 < r.y1 {
            self.y1 = r.y1;
        }
        self.x1 <= self.x2 && self.y1 <= self.y2
    }

    /// Returns `true` if the rectangle is valid (non-empty).
    pub fn is_valid(&self) -> bool {
        self.x1 <= self.x2 && self.y1 <= self.y2
    }

    /// Returns `true` if the point (x, y) is inside the rectangle.
    pub fn hit_test(&self, x: T, y: T) -> bool {
        x >= self.x1 && x <= self.x2 && y >= self.y1 && y <= self.y2
    }

    /// Returns `true` if this rectangle overlaps with `r`.
    pub fn overlaps(&self, r: &Self) -> bool {
        !(r.x1 > self.x2 || r.x2 < self.x1 || r.y1 > self.y2 || r.y2 < self.y1)
    }
}

/// Compute the intersection of two rectangles.
pub fn intersect_rectangles<T: Copy + PartialOrd>(r1: &Rect<T>, r2: &Rect<T>) -> Rect<T> {
    let mut r = *r1;
    // Process x2,y2 first (matches C++ order for MSVC compatibility comment)
    if r.x2 > r2.x2 {
        r.x2 = r2.x2;
    }
    if r.y2 > r2.y2 {
        r.y2 = r2.y2;
    }
    if r.x1 < r2.x1 {
        r.x1 = r2.x1;
    }
    if r.y1 < r2.y1 {
        r.y1 = r2.y1;
    }
    r
}

/// Compute the union (bounding box) of two rectangles.
pub fn unite_rectangles<T: Copy + PartialOrd>(r1: &Rect<T>, r2: &Rect<T>) -> Rect<T> {
    let mut r = *r1;
    if r.x2 < r2.x2 {
        r.x2 = r2.x2;
    }
    if r.y2 < r2.y2 {
        r.y2 = r2.y2;
    }
    if r.x1 > r2.x1 {
        r.x1 = r2.x1;
    }
    if r.y1 > r2.y1 {
        r.y1 = r2.y1;
    }
    r
}

/// Rectangle with `i32` coordinates.
pub type RectI = Rect<i32>;
/// Rectangle with `f32` coordinates.
pub type RectF = Rect<f32>;
/// Rectangle with `f64` coordinates.
pub type RectD = Rect<f64>;

// ============================================================================
// Path commands
// ============================================================================

pub const PATH_CMD_STOP: u32 = 0;
pub const PATH_CMD_MOVE_TO: u32 = 1;
pub const PATH_CMD_LINE_TO: u32 = 2;
pub const PATH_CMD_CURVE3: u32 = 3;
pub const PATH_CMD_CURVE4: u32 = 4;
pub const PATH_CMD_CURVE_N: u32 = 5;
pub const PATH_CMD_CATROM: u32 = 6;
pub const PATH_CMD_UBSPLINE: u32 = 7;
pub const PATH_CMD_END_POLY: u32 = 0x0F;
pub const PATH_CMD_MASK: u32 = 0x0F;

// ============================================================================
// Path flags
// ============================================================================

pub const PATH_FLAGS_NONE: u32 = 0;
pub const PATH_FLAGS_CCW: u32 = 0x10;
pub const PATH_FLAGS_CW: u32 = 0x20;
pub const PATH_FLAGS_CLOSE: u32 = 0x40;
pub const PATH_FLAGS_MASK: u32 = 0xF0;

// ============================================================================
// Path command query functions
// ============================================================================

/// Returns `true` if `c` is a vertex command (move_to through curveN).
#[inline]
pub fn is_vertex(c: u32) -> bool {
    (PATH_CMD_MOVE_TO..PATH_CMD_END_POLY).contains(&c)
}

/// Returns `true` if `c` is a drawing command (line_to through curveN).
#[inline]
pub fn is_drawing(c: u32) -> bool {
    (PATH_CMD_LINE_TO..PATH_CMD_END_POLY).contains(&c)
}

/// Returns `true` if `c` is the stop command.
#[inline]
pub fn is_stop(c: u32) -> bool {
    c == PATH_CMD_STOP
}

/// Returns `true` if `c` is a move_to command.
#[inline]
pub fn is_move_to(c: u32) -> bool {
    c == PATH_CMD_MOVE_TO
}

/// Returns `true` if `c` is a line_to command.
#[inline]
pub fn is_line_to(c: u32) -> bool {
    c == PATH_CMD_LINE_TO
}

/// Returns `true` if `c` is a curve command (curve3 or curve4).
#[inline]
pub fn is_curve(c: u32) -> bool {
    c == PATH_CMD_CURVE3 || c == PATH_CMD_CURVE4
}

/// Returns `true` if `c` is a quadratic curve command.
#[inline]
pub fn is_curve3(c: u32) -> bool {
    c == PATH_CMD_CURVE3
}

/// Returns `true` if `c` is a cubic curve command.
#[inline]
pub fn is_curve4(c: u32) -> bool {
    c == PATH_CMD_CURVE4
}

/// Returns `true` if `c` is an end_poly command (with any flags).
#[inline]
pub fn is_end_poly(c: u32) -> bool {
    (c & PATH_CMD_MASK) == PATH_CMD_END_POLY
}

/// Returns `true` if `c` is a close polygon command.
#[inline]
pub fn is_close(c: u32) -> bool {
    (c & !(PATH_FLAGS_CW | PATH_FLAGS_CCW)) == (PATH_CMD_END_POLY | PATH_FLAGS_CLOSE)
}

/// Returns `true` if `c` starts a new polygon (stop, move_to, or end_poly).
#[inline]
pub fn is_next_poly(c: u32) -> bool {
    is_stop(c) || is_move_to(c) || is_end_poly(c)
}

/// Returns `true` if `c` has the clockwise flag set.
#[inline]
pub fn is_cw(c: u32) -> bool {
    (c & PATH_FLAGS_CW) != 0
}

/// Returns `true` if `c` has the counter-clockwise flag set.
#[inline]
pub fn is_ccw(c: u32) -> bool {
    (c & PATH_FLAGS_CCW) != 0
}

/// Returns `true` if `c` has any orientation flag set.
#[inline]
pub fn is_oriented(c: u32) -> bool {
    (c & (PATH_FLAGS_CW | PATH_FLAGS_CCW)) != 0
}

/// Returns `true` if `c` has the close flag set.
#[inline]
pub fn is_closed(c: u32) -> bool {
    (c & PATH_FLAGS_CLOSE) != 0
}

/// Extract the close flag from a command.
#[inline]
pub fn get_close_flag(c: u32) -> u32 {
    c & PATH_FLAGS_CLOSE
}

/// Remove orientation flags from a command.
#[inline]
pub fn clear_orientation(c: u32) -> u32 {
    c & !(PATH_FLAGS_CW | PATH_FLAGS_CCW)
}

/// Extract the orientation flags from a command.
#[inline]
pub fn get_orientation(c: u32) -> u32 {
    c & (PATH_FLAGS_CW | PATH_FLAGS_CCW)
}

/// Set the orientation flags on a command.
#[inline]
pub fn set_orientation(c: u32, o: u32) -> u32 {
    clear_orientation(c) | o
}

// ============================================================================
// Point
// ============================================================================

/// A 2D point.
/// Port of C++ `point_base<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointBase<T: Copy> {
    pub x: T,
    pub y: T,
}

impl<T: Copy> PointBase<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

pub type PointI = PointBase<i32>;
pub type PointF = PointBase<f32>;
pub type PointD = PointBase<f64>;

// ============================================================================
// Vertex
// ============================================================================

/// A vertex with coordinates and a path command.
/// Port of C++ `vertex_base<T>`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VertexBase<T: Copy> {
    pub x: T,
    pub y: T,
    pub cmd: u32,
}

impl<T: Copy> VertexBase<T> {
    pub fn new(x: T, y: T, cmd: u32) -> Self {
        Self { x, y, cmd }
    }
}

pub type VertexI = VertexBase<i32>;
pub type VertexF = VertexBase<f32>;
pub type VertexD = VertexBase<f64>;

// ============================================================================
// Row info (for rendering buffer row access)
// ============================================================================

/// Information about a row in a rendering buffer.
/// Port of C++ `row_info<T>` — used with raw pointers for pixel buffer access.
#[derive(Debug, Clone, Copy)]
pub struct RowInfo<T> {
    pub x1: i32,
    pub x2: i32,
    pub ptr: *mut T,
}

impl<T> RowInfo<T> {
    pub fn new(x1: i32, x2: i32, ptr: *mut T) -> Self {
        Self { x1, x2, ptr }
    }
}

/// Const version of row info.
#[derive(Debug, Clone, Copy)]
pub struct ConstRowInfo<T> {
    pub x1: i32,
    pub x2: i32,
    pub ptr: *const T,
}

impl<T> ConstRowInfo<T> {
    pub fn new(x1: i32, x2: i32, ptr: *const T) -> Self {
        Self { x1, x2, ptr }
    }
}

// ============================================================================
// Approximate equality comparison
// ============================================================================

/// Compare two floating-point values for approximate equality using
/// relative comparison scaled by the smaller exponent.
/// Port of C++ `is_equal_eps`.
pub fn is_equal_eps<T>(v1: T, v2: T, epsilon: T) -> bool
where
    T: Copy + PartialOrd + Sub<Output = T> + Add<Output = T> + Into<f64> + From<f64>,
{
    let v1_f: f64 = v1.into();
    let v2_f: f64 = v2.into();
    let eps_f: f64 = epsilon.into();

    let neg1 = v1_f < 0.0;
    let neg2 = v2_f < 0.0;

    if neg1 != neg2 {
        return v1_f.abs() < eps_f && v2_f.abs() < eps_f;
    }

    let (_, exp1) = frexp(v1_f);
    let (_, exp2) = frexp(v2_f);
    let min_exp = exp1.min(exp2);

    let scaled1 = ldexp(v1_f, -min_exp);
    let scaled2 = ldexp(v2_f, -min_exp);

    (scaled1 - scaled2).abs() < eps_f
}

/// C-style frexp: decompose `x` into `(mantissa, exponent)` where
/// `x = mantissa * 2^exponent` and `0.5 <= |mantissa| < 1.0`.
#[inline]
fn frexp(x: f64) -> (f64, i32) {
    if x == 0.0 {
        return (0.0, 0);
    }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7FF) as i32 - 1022;
    let mantissa = f64::from_bits((bits & 0x800F_FFFF_FFFF_FFFF) | 0x3FE0_0000_0000_0000);
    (mantissa, exp)
}

/// C-style ldexp: compute `x * 2^exp`.
#[inline]
fn ldexp(x: f64, exp: i32) -> f64 {
    x * (2.0_f64).powi(exp)
}

// ============================================================================
// VertexSource trait
// ============================================================================

/// The fundamental vertex source interface. Every shape, path, and converter
/// in AGG implements this trait to produce a stream of vertices.
///
/// Port of the C++ "vertex source concept" — the implicit interface that
/// all AGG vertex sources implement via duck typing (template parameters).
pub trait VertexSource {
    /// Reset the vertex source to the beginning of the given path.
    /// `path_id` selects which sub-path to iterate (0 for the first/only path).
    fn rewind(&mut self, path_id: u32);

    /// Return the next vertex. Writes coordinates to `x` and `y`, returns a
    /// path command. Returns `PATH_CMD_STOP` when iteration is complete.
    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32;
}

/// Blanket implementation so `&mut T` can be used as a VertexSource.
/// This allows pipeline stages to borrow their source instead of owning it.
impl<T: VertexSource> VertexSource for &mut T {
    fn rewind(&mut self, path_id: u32) {
        (*self).rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        (*self).vertex(x, y)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iround() {
        assert_eq!(iround(0.5), 1);
        assert_eq!(iround(0.49), 0);
        assert_eq!(iround(-0.5), -1);
        assert_eq!(iround(-0.49), 0);
        assert_eq!(iround(1.5), 2);
        assert_eq!(iround(-1.5), -2);
        assert_eq!(iround(0.0), 0);
    }

    #[test]
    fn test_uround() {
        assert_eq!(uround(0.5), 1);
        assert_eq!(uround(0.49), 0);
        assert_eq!(uround(1.5), 2);
        assert_eq!(uround(0.0), 0);
    }

    #[test]
    fn test_ifloor() {
        assert_eq!(ifloor(1.7), 1);
        assert_eq!(ifloor(1.0), 1);
        assert_eq!(ifloor(-1.7), -2);
        assert_eq!(ifloor(-1.0), -1);
        assert_eq!(ifloor(0.0), 0);
    }

    #[test]
    fn test_ufloor() {
        assert_eq!(ufloor(1.7), 1);
        assert_eq!(ufloor(1.0), 1);
        assert_eq!(ufloor(0.0), 0);
    }

    #[test]
    fn test_iceil() {
        assert_eq!(iceil(1.1), 2);
        assert_eq!(iceil(1.0), 1);
        assert_eq!(iceil(-1.1), -1);
        assert_eq!(iceil(0.0), 0);
    }

    #[test]
    fn test_uceil() {
        assert_eq!(uceil(1.1), 2);
        assert_eq!(uceil(1.0), 1);
        assert_eq!(uceil(0.0), 0);
    }

    #[test]
    fn test_saturation_iround() {
        assert_eq!(saturation_iround(128, 200.0), 128);
        assert_eq!(saturation_iround(128, -200.0), -128);
        assert_eq!(saturation_iround(128, 50.7), 51);
    }

    #[test]
    fn test_mul_one_shift8() {
        // mul_one with shift=8: (a * b + 128) >> 8, with rounding correction
        // For a=255, b=255: should give 255
        assert_eq!(mul_one(255, 255, 8), 255);
        // For a=128, b=255: should give 128
        assert_eq!(mul_one(128, 255, 8), 128);
        // For a=0, b=255: should give 0
        assert_eq!(mul_one(0, 255, 8), 0);
    }

    #[test]
    fn test_cover_constants() {
        assert_eq!(COVER_SHIFT, 8);
        assert_eq!(COVER_SIZE, 256);
        assert_eq!(COVER_MASK, 255);
        assert_eq!(COVER_NONE, 0);
        assert_eq!(COVER_FULL, 255);
    }

    #[test]
    fn test_poly_subpixel_constants() {
        assert_eq!(POLY_SUBPIXEL_SHIFT, 8);
        assert_eq!(POLY_SUBPIXEL_SCALE, 256);
        assert_eq!(POLY_SUBPIXEL_MASK, 255);
    }

    #[test]
    fn test_deg2rad_rad2deg() {
        let epsilon = 1e-10;
        assert!((deg2rad(180.0) - PI).abs() < epsilon);
        assert!((rad2deg(PI) - 180.0).abs() < epsilon);
        assert!((deg2rad(90.0) - PI / 2.0).abs() < epsilon);
        assert!((deg2rad(0.0)).abs() < epsilon);
    }

    #[test]
    fn test_rect_new_and_is_valid() {
        let r = RectI::new(10, 20, 30, 40);
        assert!(r.is_valid());
        assert_eq!(r.x1, 10);
        assert_eq!(r.y1, 20);
        assert_eq!(r.x2, 30);
        assert_eq!(r.y2, 40);

        let r_invalid = RectI::new(30, 40, 10, 20);
        assert!(!r_invalid.is_valid());
    }

    #[test]
    fn test_rect_normalize() {
        let mut r = RectI::new(30, 40, 10, 20);
        r.normalize();
        assert_eq!(r.x1, 10);
        assert_eq!(r.y1, 20);
        assert_eq!(r.x2, 30);
        assert_eq!(r.y2, 40);
    }

    #[test]
    fn test_rect_clip() {
        let mut r = RectI::new(10, 20, 100, 200);
        let clip = RectI::new(50, 50, 80, 80);
        let valid = r.clip(&clip);
        assert!(valid);
        assert_eq!(r.x1, 50);
        assert_eq!(r.y1, 50);
        assert_eq!(r.x2, 80);
        assert_eq!(r.y2, 80);
    }

    #[test]
    fn test_rect_hit_test() {
        let r = RectI::new(10, 20, 30, 40);
        assert!(r.hit_test(15, 25));
        assert!(r.hit_test(10, 20));
        assert!(r.hit_test(30, 40));
        assert!(!r.hit_test(5, 25));
        assert!(!r.hit_test(15, 45));
    }

    #[test]
    fn test_rect_overlaps() {
        let r1 = RectI::new(10, 20, 30, 40);
        let r2 = RectI::new(25, 35, 50, 60);
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));

        let r3 = RectI::new(31, 41, 50, 60);
        assert!(!r1.overlaps(&r3));
    }

    #[test]
    fn test_intersect_rectangles() {
        let r1 = RectI::new(10, 20, 100, 200);
        let r2 = RectI::new(50, 50, 80, 80);
        let r = intersect_rectangles(&r1, &r2);
        assert_eq!(r.x1, 50);
        assert_eq!(r.y1, 50);
        assert_eq!(r.x2, 80);
        assert_eq!(r.y2, 80);
    }

    #[test]
    fn test_unite_rectangles() {
        let r1 = RectI::new(10, 20, 30, 40);
        let r2 = RectI::new(50, 60, 70, 80);
        let r = unite_rectangles(&r1, &r2);
        assert_eq!(r.x1, 10);
        assert_eq!(r.y1, 20);
        assert_eq!(r.x2, 70);
        assert_eq!(r.y2, 80);
    }

    #[test]
    fn test_path_command_classification() {
        assert!(is_stop(PATH_CMD_STOP));
        assert!(!is_stop(PATH_CMD_MOVE_TO));

        assert!(is_move_to(PATH_CMD_MOVE_TO));
        assert!(is_line_to(PATH_CMD_LINE_TO));

        assert!(is_vertex(PATH_CMD_MOVE_TO));
        assert!(is_vertex(PATH_CMD_LINE_TO));
        assert!(is_vertex(PATH_CMD_CURVE3));
        assert!(is_vertex(PATH_CMD_CURVE4));
        assert!(!is_vertex(PATH_CMD_STOP));
        assert!(!is_vertex(PATH_CMD_END_POLY));

        assert!(is_drawing(PATH_CMD_LINE_TO));
        assert!(is_drawing(PATH_CMD_CURVE3));
        assert!(!is_drawing(PATH_CMD_MOVE_TO));
        assert!(!is_drawing(PATH_CMD_END_POLY));

        assert!(is_curve(PATH_CMD_CURVE3));
        assert!(is_curve(PATH_CMD_CURVE4));
        assert!(!is_curve(PATH_CMD_LINE_TO));

        assert!(is_curve3(PATH_CMD_CURVE3));
        assert!(!is_curve3(PATH_CMD_CURVE4));

        assert!(is_curve4(PATH_CMD_CURVE4));
        assert!(!is_curve4(PATH_CMD_CURVE3));
    }

    #[test]
    fn test_path_end_poly_and_close() {
        assert!(is_end_poly(PATH_CMD_END_POLY));
        assert!(is_end_poly(PATH_CMD_END_POLY | PATH_FLAGS_CLOSE));
        assert!(is_end_poly(PATH_CMD_END_POLY | PATH_FLAGS_CW));
        assert!(!is_end_poly(PATH_CMD_LINE_TO));

        assert!(is_close(PATH_CMD_END_POLY | PATH_FLAGS_CLOSE));
        assert!(!is_close(PATH_CMD_END_POLY));
        // Close with orientation should still be close
        assert!(is_close(
            PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CW
        ));
    }

    #[test]
    fn test_path_flags() {
        let cmd = PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CW;
        assert!(is_cw(cmd));
        assert!(!is_ccw(cmd));
        assert!(is_oriented(cmd));
        assert!(is_closed(cmd));

        assert_eq!(get_close_flag(cmd), PATH_FLAGS_CLOSE);
        assert_eq!(get_orientation(cmd), PATH_FLAGS_CW);
        assert_eq!(clear_orientation(cmd), PATH_CMD_END_POLY | PATH_FLAGS_CLOSE);
        assert_eq!(
            set_orientation(cmd, PATH_FLAGS_CCW),
            PATH_CMD_END_POLY | PATH_FLAGS_CLOSE | PATH_FLAGS_CCW
        );
    }

    #[test]
    fn test_is_next_poly() {
        assert!(is_next_poly(PATH_CMD_STOP));
        assert!(is_next_poly(PATH_CMD_MOVE_TO));
        assert!(is_next_poly(PATH_CMD_END_POLY));
        assert!(!is_next_poly(PATH_CMD_LINE_TO));
        assert!(!is_next_poly(PATH_CMD_CURVE3));
    }

    #[test]
    fn test_point() {
        let p = PointD::new(1.5, 2.5);
        assert_eq!(p.x, 1.5);
        assert_eq!(p.y, 2.5);
    }

    #[test]
    fn test_vertex() {
        let v = VertexD::new(1.0, 2.0, PATH_CMD_LINE_TO);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.cmd, PATH_CMD_LINE_TO);
    }

    #[test]
    fn test_is_equal_eps() {
        assert!(is_equal_eps(1.0, 1.0, 1e-10));
        assert!(is_equal_eps(1.0, 1.0 + 1e-11, 1e-10));
        assert!(!is_equal_eps(1.0, 2.0, 1e-10));
        assert!(is_equal_eps(0.0, 0.0, 1e-10));
        // Different signs, both small
        assert!(is_equal_eps(1e-12, -1e-12, 1e-10));
        // Different signs, not small enough
        assert!(!is_equal_eps(0.1, -0.1, 1e-10));
    }
}

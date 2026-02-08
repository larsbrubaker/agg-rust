//! Gradient span generator and gradient functions.
//!
//! Port of `agg_span_gradient.h` — provides gradient shape functions
//! (linear, radial, diamond, conic, etc.) and the main `SpanGradient`
//! generator that combines an interpolator, gradient function, and color
//! function to produce gradient-colored spans.

use crate::basics::{iround, uround};
use crate::gradient_lut::ColorFunction;
use crate::math::fast_sqrt;
use crate::renderer_scanline::SpanGenerator;
use crate::span_interpolator_linear::{SpanInterpolatorLinear, SUBPIXEL_SHIFT};

// ============================================================================
// Constants
// ============================================================================

pub const GRADIENT_SUBPIXEL_SHIFT: i32 = 4;
pub const GRADIENT_SUBPIXEL_SCALE: i32 = 1 << GRADIENT_SUBPIXEL_SHIFT;
pub const GRADIENT_SUBPIXEL_MASK: i32 = GRADIENT_SUBPIXEL_SCALE - 1;

/// Downscale shift from interpolator subpixel to gradient subpixel.
const DOWNSCALE_SHIFT: i32 = SUBPIXEL_SHIFT as i32 - GRADIENT_SUBPIXEL_SHIFT;

// ============================================================================
// GradientFunction trait
// ============================================================================

/// Trait for gradient shape functions.
///
/// Maps a 2D point `(x, y)` to a scalar distance value. The `d` parameter
/// is the gradient diameter/range (used by some gradient types like XY, conic).
pub trait GradientFunction {
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32;
}

// ============================================================================
// Gradient functions — zero-sized types for stateless gradients
// ============================================================================

/// Linear gradient along the X axis.
///
/// Port of C++ `gradient_x`. Simply returns `x`.
pub struct GradientX;

impl GradientFunction for GradientX {
    #[inline]
    fn calculate(&self, x: i32, _y: i32, _d: i32) -> i32 {
        x
    }
}

/// Linear gradient along the Y axis.
///
/// Port of C++ `gradient_y`. Simply returns `y`.
pub struct GradientY;

impl GradientFunction for GradientY {
    #[inline]
    fn calculate(&self, _x: i32, y: i32, _d: i32) -> i32 {
        y
    }
}

/// Radial gradient using fast integer square root.
///
/// Port of C++ `gradient_radial` / `gradient_circle`.
pub struct GradientRadial;

impl GradientFunction for GradientRadial {
    #[inline]
    fn calculate(&self, x: i32, y: i32, _d: i32) -> i32 {
        fast_sqrt((x * x + y * y) as u32) as i32
    }
}

/// Radial gradient using f64 square root (higher precision).
///
/// Port of C++ `gradient_radial_d`.
pub struct GradientRadialD;

impl GradientFunction for GradientRadialD {
    #[inline]
    fn calculate(&self, x: i32, y: i32, _d: i32) -> i32 {
        uround(((x as f64) * (x as f64) + (y as f64) * (y as f64)).sqrt()) as i32
    }
}

/// Diamond-shaped gradient: max(|x|, |y|).
///
/// Port of C++ `gradient_diamond`.
pub struct GradientDiamond;

impl GradientFunction for GradientDiamond {
    #[inline]
    fn calculate(&self, x: i32, y: i32, _d: i32) -> i32 {
        let ax = x.abs();
        let ay = y.abs();
        if ax > ay {
            ax
        } else {
            ay
        }
    }
}

/// XY gradient: |x|*|y| / d.
///
/// Port of C++ `gradient_xy`.
pub struct GradientXY;

impl GradientFunction for GradientXY {
    #[inline]
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32 {
        x.abs() * y.abs() / d
    }
}

/// Square-root XY gradient: sqrt(|x|*|y|).
///
/// Port of C++ `gradient_sqrt_xy`.
pub struct GradientSqrtXY;

impl GradientFunction for GradientSqrtXY {
    #[inline]
    fn calculate(&self, x: i32, y: i32, _d: i32) -> i32 {
        fast_sqrt((x.abs() * y.abs()) as u32) as i32
    }
}

/// Conic (angular) gradient: |atan2(y,x)| * d / pi.
///
/// Port of C++ `gradient_conic`.
pub struct GradientConic;

impl GradientFunction for GradientConic {
    #[inline]
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32 {
        uround((y as f64).atan2(x as f64).abs() * (d as f64) / std::f64::consts::PI) as i32
    }
}

// ============================================================================
// Radial gradient with focal point
// ============================================================================

/// Radial gradient with an off-center focal point.
///
/// Port of C++ `gradient_radial_focus`. Has mutable state for the
/// precomputed invariant values.
pub struct GradientRadialFocus {
    r: i32,
    fx: i32,
    fy: i32,
    r2: f64,
    fx2: f64,
    fy2: f64,
    mul: f64,
}

impl GradientRadialFocus {
    pub fn new_default() -> Self {
        let mut s = Self {
            r: 100 * GRADIENT_SUBPIXEL_SCALE,
            fx: 0,
            fy: 0,
            r2: 0.0,
            fx2: 0.0,
            fy2: 0.0,
            mul: 0.0,
        };
        s.update_values();
        s
    }

    pub fn new(r: f64, fx: f64, fy: f64) -> Self {
        let mut s = Self {
            r: iround(r * GRADIENT_SUBPIXEL_SCALE as f64),
            fx: iround(fx * GRADIENT_SUBPIXEL_SCALE as f64),
            fy: iround(fy * GRADIENT_SUBPIXEL_SCALE as f64),
            r2: 0.0,
            fx2: 0.0,
            fy2: 0.0,
            mul: 0.0,
        };
        s.update_values();
        s
    }

    pub fn init(&mut self, r: f64, fx: f64, fy: f64) {
        self.r = iround(r * GRADIENT_SUBPIXEL_SCALE as f64);
        self.fx = iround(fx * GRADIENT_SUBPIXEL_SCALE as f64);
        self.fy = iround(fy * GRADIENT_SUBPIXEL_SCALE as f64);
        self.update_values();
    }

    pub fn radius(&self) -> f64 {
        self.r as f64 / GRADIENT_SUBPIXEL_SCALE as f64
    }

    pub fn focus_x(&self) -> f64 {
        self.fx as f64 / GRADIENT_SUBPIXEL_SCALE as f64
    }

    pub fn focus_y(&self) -> f64 {
        self.fy as f64 / GRADIENT_SUBPIXEL_SCALE as f64
    }

    fn update_values(&mut self) {
        // Calculate the invariant values. In case the focal center
        // lies exactly on the gradient circle the divisor degenerates
        // into zero. In this case we just move the focal center by
        // one subpixel unit possibly in the direction to the origin (0,0)
        // and calculate the values again.
        self.r2 = (self.r as f64) * (self.r as f64);
        self.fx2 = (self.fx as f64) * (self.fx as f64);
        self.fy2 = (self.fy as f64) * (self.fy as f64);
        let mut d = self.r2 - (self.fx2 + self.fy2);
        if d == 0.0 {
            if self.fx != 0 {
                if self.fx < 0 {
                    self.fx += 1;
                } else {
                    self.fx -= 1;
                }
            }
            if self.fy != 0 {
                if self.fy < 0 {
                    self.fy += 1;
                } else {
                    self.fy -= 1;
                }
            }
            self.fx2 = (self.fx as f64) * (self.fx as f64);
            self.fy2 = (self.fy as f64) * (self.fy as f64);
            d = self.r2 - (self.fx2 + self.fy2);
        }
        self.mul = self.r as f64 / d;
    }
}

impl GradientFunction for GradientRadialFocus {
    fn calculate(&self, x: i32, y: i32, _d: i32) -> i32 {
        let dx = x as f64 - self.fx as f64;
        let dy = y as f64 - self.fy as f64;
        let d2 = dx * self.fy as f64 - dy * self.fx as f64;
        let d3 = self.r2 * (dx * dx + dy * dy) - d2 * d2;
        iround((dx * self.fx as f64 + dy * self.fy as f64 + d3.abs().sqrt()) * self.mul)
    }
}

// ============================================================================
// Gradient adaptors — repeat and reflect
// ============================================================================

/// Repeating gradient adaptor — wraps gradient values with modulo.
///
/// Port of C++ `gradient_repeat_adaptor<GradientF>`.
pub struct GradientRepeatAdaptor<G> {
    gradient: G,
}

impl<G: GradientFunction> GradientRepeatAdaptor<G> {
    pub fn new(gradient: G) -> Self {
        Self { gradient }
    }
}

impl<G: GradientFunction> GradientFunction for GradientRepeatAdaptor<G> {
    #[inline]
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32 {
        let mut ret = self.gradient.calculate(x, y, d) % d;
        if ret < 0 {
            ret += d;
        }
        ret
    }
}

/// Reflecting gradient adaptor — mirrors gradient values at boundaries.
///
/// Port of C++ `gradient_reflect_adaptor<GradientF>`.
pub struct GradientReflectAdaptor<G> {
    gradient: G,
}

impl<G: GradientFunction> GradientReflectAdaptor<G> {
    pub fn new(gradient: G) -> Self {
        Self { gradient }
    }
}

impl<G: GradientFunction> GradientFunction for GradientReflectAdaptor<G> {
    #[inline]
    fn calculate(&self, x: i32, y: i32, d: i32) -> i32 {
        let d2 = d << 1;
        let mut ret = self.gradient.calculate(x, y, d) % d2;
        if ret < 0 {
            ret += d2;
        }
        if ret >= d {
            ret = d2 - ret;
        }
        ret
    }
}

// ============================================================================
// SpanGradient — the main gradient span generator
// ============================================================================

/// Main gradient span generator.
///
/// Combines an interpolator (for coordinate transformation), a gradient
/// function (for shape), and a color function (for color lookup) to
/// produce gradient-colored pixel spans.
///
/// Port of C++ `span_gradient<ColorT, Interpolator, GradientF, ColorF>`.
pub struct SpanGradient<'a, G, F> {
    interpolator: SpanInterpolatorLinear,
    gradient_function: G,
    color_function: &'a F,
    d1: i32,
    d2: i32,
}

impl<'a, G: GradientFunction, F: ColorFunction> SpanGradient<'a, G, F> {
    pub fn new(
        interpolator: SpanInterpolatorLinear,
        gradient_function: G,
        color_function: &'a F,
        d1: f64,
        d2: f64,
    ) -> Self {
        Self {
            interpolator,
            gradient_function,
            color_function,
            d1: iround(d1 * GRADIENT_SUBPIXEL_SCALE as f64),
            d2: iround(d2 * GRADIENT_SUBPIXEL_SCALE as f64),
        }
    }

    pub fn interpolator(&self) -> &SpanInterpolatorLinear {
        &self.interpolator
    }

    pub fn interpolator_mut(&mut self) -> &mut SpanInterpolatorLinear {
        &mut self.interpolator
    }

    pub fn gradient_function(&self) -> &G {
        &self.gradient_function
    }

    pub fn color_function(&self) -> &F {
        self.color_function
    }

    pub fn d1(&self) -> f64 {
        self.d1 as f64 / GRADIENT_SUBPIXEL_SCALE as f64
    }

    pub fn d2(&self) -> f64 {
        self.d2 as f64 / GRADIENT_SUBPIXEL_SCALE as f64
    }

    pub fn set_d1(&mut self, v: f64) {
        self.d1 = iround(v * GRADIENT_SUBPIXEL_SCALE as f64);
    }

    pub fn set_d2(&mut self, v: f64) {
        self.d2 = iround(v * GRADIENT_SUBPIXEL_SCALE as f64);
    }
}

impl<'a, G, F> SpanGenerator for SpanGradient<'a, G, F>
where
    G: GradientFunction,
    F: ColorFunction,
    F::Color: Copy,
{
    type Color = F::Color;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [F::Color], x: i32, y: i32, len: u32) {
        let dd = (self.d2 - self.d1).max(1);
        self.interpolator.begin(x as f64 + 0.5, y as f64 + 0.5, len);
        let color_size = self.color_function.size() as i32;
        for pixel in span.iter_mut().take(len as usize) {
            let mut ix = 0i32;
            let mut iy = 0i32;
            self.interpolator.coordinates(&mut ix, &mut iy);
            let d = self.gradient_function.calculate(
                ix >> DOWNSCALE_SHIFT,
                iy >> DOWNSCALE_SHIFT,
                self.d2,
            );
            let d = (((d - self.d1) * color_size) / dd).clamp(0, color_size - 1);
            *pixel = self.color_function.get(d as usize);
            self.interpolator.next();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::gradient_lut::{GradientLinearColor, GradientLut};
    use crate::trans_affine::TransAffine;

    // ===== Gradient function tests =====

    #[test]
    fn test_gradient_x() {
        let g = GradientX;
        assert_eq!(g.calculate(100, 200, 500), 100);
        assert_eq!(g.calculate(-50, 200, 500), -50);
    }

    #[test]
    fn test_gradient_y() {
        let g = GradientY;
        assert_eq!(g.calculate(100, 200, 500), 200);
        assert_eq!(g.calculate(100, -50, 500), -50);
    }

    #[test]
    fn test_gradient_radial() {
        let g = GradientRadial;
        // (3,4) → sqrt(9+16) = 5
        assert_eq!(g.calculate(3, 4, 100), 5);
        // Origin
        assert_eq!(g.calculate(0, 0, 100), 0);
    }

    #[test]
    fn test_gradient_radial_d() {
        let g = GradientRadialD;
        // (3,4) → sqrt(25) = 5
        assert_eq!(g.calculate(3, 4, 100), 5);
    }

    #[test]
    fn test_gradient_diamond() {
        let g = GradientDiamond;
        assert_eq!(g.calculate(3, 5, 100), 5);
        assert_eq!(g.calculate(7, 5, 100), 7);
        assert_eq!(g.calculate(-3, 5, 100), 5);
        assert_eq!(g.calculate(3, -8, 100), 8);
    }

    #[test]
    fn test_gradient_xy() {
        let g = GradientXY;
        assert_eq!(g.calculate(10, 20, 100), 2); // 200/100 = 2
        assert_eq!(g.calculate(-10, 20, 100), 2);
    }

    #[test]
    fn test_gradient_sqrt_xy() {
        let g = GradientSqrtXY;
        // sqrt(100 * 100) = 100
        assert_eq!(g.calculate(100, 100, 500), 100);
    }

    #[test]
    fn test_gradient_conic() {
        let g = GradientConic;
        // At (1,0): atan2(0,1) = 0 → 0
        assert_eq!(g.calculate(1, 0, 100), 0);
        // At (0,1): atan2(1,0) = pi/2 → d/2 = 50
        assert_eq!(g.calculate(0, 1, 100), 50);
        // At (-1,0): atan2(0,-1) = pi → d = 100
        assert_eq!(g.calculate(-1, 0, 100), 100);
    }

    #[test]
    fn test_gradient_radial_focus_default() {
        let g = GradientRadialFocus::new_default();
        assert_eq!(g.radius(), 100.0);
        assert_eq!(g.focus_x(), 0.0);
        assert_eq!(g.focus_y(), 0.0);
    }

    #[test]
    fn test_gradient_radial_focus_centered() {
        let g = GradientRadialFocus::new(100.0, 0.0, 0.0);
        // At center (0,0), distance should be 0
        assert_eq!(g.calculate(0, 0, 1600), 0);
        // At (1600,0) which is 100*16 (the radius in subpixel), should be ~1600
        let d = g.calculate(1600, 0, 1600);
        assert!((d - 1600).abs() <= 2, "d={}", d);
    }

    #[test]
    fn test_gradient_radial_focus_init() {
        let mut g = GradientRadialFocus::new_default();
        g.init(50.0, 10.0, 5.0);
        assert_eq!(g.radius(), 50.0);
        // focus_x/y are rounded to gradient subpixel
        assert!((g.focus_x() - 10.0).abs() < 0.1);
        assert!((g.focus_y() - 5.0).abs() < 0.1);
    }

    // ===== Adaptor tests =====

    #[test]
    fn test_gradient_repeat_adaptor() {
        let g = GradientRepeatAdaptor::new(GradientX);
        // 150 % 100 = 50
        assert_eq!(g.calculate(150, 0, 100), 50);
        // -50 % 100 → (-50 % 100) + 100 = 50
        assert_eq!(g.calculate(-50, 0, 100), 50);
        // 250 % 100 = 50
        assert_eq!(g.calculate(250, 0, 100), 50);
    }

    #[test]
    fn test_gradient_reflect_adaptor() {
        let g = GradientReflectAdaptor::new(GradientX);
        // 50 → 50 (within [0,d))
        assert_eq!(g.calculate(50, 0, 100), 50);
        // 150 → 150 % 200 = 150, >= 100 → 200 - 150 = 50
        assert_eq!(g.calculate(150, 0, 100), 50);
        // 250 → 250 % 200 = 50, < 100 → 50
        assert_eq!(g.calculate(250, 0, 100), 50);
    }

    // ===== SpanGradient tests =====

    #[test]
    fn test_span_gradient_linear_x() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let gc = GradientLinearColor::new(
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
            256,
        );
        let mut sg = SpanGradient::new(interp, GradientX, &gc, 0.0, 100.0);

        let mut span = vec![Rgba8::default(); 10];
        sg.generate(&mut span, 0, 0, 10);

        // Gradient should progress from dark to lighter
        assert!(span[0].r < span[9].r, "s0={} s9={}", span[0].r, span[9].r);
    }

    #[test]
    fn test_span_gradient_d1_d2() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let gc = GradientLinearColor::new(
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
            256,
        );
        let sg = SpanGradient::new(interp, GradientX, &gc, 10.0, 200.0);
        assert!((sg.d1() - 10.0).abs() < 0.1);
        assert!((sg.d2() - 200.0).abs() < 0.1);
    }

    #[test]
    fn test_span_gradient_with_lut() {
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let mut lut = GradientLut::new_default();
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));
        lut.build_lut();

        let mut sg = SpanGradient::new(interp, GradientX, &lut, 0.0, 100.0);

        let mut span = vec![Rgba8::default(); 5];
        sg.generate(&mut span, 0, 0, 5);

        // First pixel should be reddish (near start of gradient)
        assert!(span[0].r > 200, "r={}", span[0].r);
    }

    #[test]
    fn test_span_gradient_clamping() {
        // Test that out-of-range values are clamped to [0, size-1]
        let trans = TransAffine::new();
        let interp = SpanInterpolatorLinear::new(trans);
        let gc = GradientLinearColor::new(
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
            256,
        );
        // d1=90, d2=100 — most x values will be < d1 (clamped to 0)
        let mut sg = SpanGradient::new(interp, GradientX, &gc, 90.0, 100.0);

        let mut span = vec![Rgba8::default(); 5];
        sg.generate(&mut span, 0, 0, 5);
        // All should be black (clamped to index 0)
        for c in &span {
            assert_eq!(c.r, 0, "Expected black, got r={}", c.r);
        }
    }

    #[test]
    fn test_span_gradient_constants() {
        assert_eq!(GRADIENT_SUBPIXEL_SHIFT, 4);
        assert_eq!(GRADIENT_SUBPIXEL_SCALE, 16);
        assert_eq!(GRADIENT_SUBPIXEL_MASK, 15);
    }
}

//! Gradient color lookup table.
//!
//! Port of `agg_gradient_lut.h` — builds a LUT (lookup table) from SVG-style
//! color stops. Used by `SpanGradient` to map gradient distances to colors.

use crate::basics::uround;
use crate::color::Rgba8;
use crate::dda_line::DdaLineInterpolator;

// ============================================================================
// ColorFunction trait
// ============================================================================

/// Trait for color lookup functions used by span_gradient.
///
/// Provides indexed access to a color palette of known size.
pub trait ColorFunction {
    type Color;

    fn size(&self) -> usize;
    fn get(&self, index: usize) -> Self::Color;
}

// ============================================================================
// ColorInterpolator — generic version using gradient() method
// ============================================================================

/// Generic color interpolator using the color type's `gradient()` method.
///
/// Port of C++ `color_interpolator<ColorT>` (generic template).
struct ColorInterpolatorGeneric<C> {
    c1: C,
    c2: C,
    len: u32,
    count: u32,
}

impl<C: Clone> ColorInterpolatorGeneric<C> {
    fn new(c1: &C, c2: &C, len: u32) -> Self {
        Self {
            c1: c1.clone(),
            c2: c2.clone(),
            len,
            count: 0,
        }
    }

    fn inc(&mut self) {
        self.count += 1;
    }
}

impl ColorInterpolatorGeneric<Rgba8> {
    fn color(&self) -> Rgba8 {
        self.c1
            .gradient(&self.c2, self.count as f64 / self.len as f64)
    }
}

// ============================================================================
// ColorInterpolatorRgba8 — fast DDA specialization for Rgba8
// ============================================================================

/// Fast RGBA8 color interpolator using 14-bit DDA interpolation.
///
/// Port of C++ `color_interpolator<rgba8>` specialization.
struct ColorInterpolatorRgba8 {
    r: DdaLineInterpolator<14, 0>,
    g: DdaLineInterpolator<14, 0>,
    b: DdaLineInterpolator<14, 0>,
    a: DdaLineInterpolator<14, 0>,
}

impl ColorInterpolatorRgba8 {
    fn new(c1: &Rgba8, c2: &Rgba8, len: u32) -> Self {
        Self {
            r: DdaLineInterpolator::new(c1.r as i32, c2.r as i32, len),
            g: DdaLineInterpolator::new(c1.g as i32, c2.g as i32, len),
            b: DdaLineInterpolator::new(c1.b as i32, c2.b as i32, len),
            a: DdaLineInterpolator::new(c1.a as i32, c2.a as i32, len),
        }
    }

    fn inc(&mut self) {
        self.r.inc();
        self.g.inc();
        self.b.inc();
        self.a.inc();
    }

    fn color(&self) -> Rgba8 {
        Rgba8::new(
            self.r.y() as u32,
            self.g.y() as u32,
            self.b.y() as u32,
            self.a.y() as u32,
        )
    }
}

// ============================================================================
// GradientLut
// ============================================================================

/// Color stop for gradient definition.
#[derive(Clone)]
struct ColorPoint {
    offset: f64,
    color: Rgba8,
}

impl ColorPoint {
    fn new(offset: f64, color: Rgba8) -> Self {
        Self {
            offset: offset.clamp(0.0, 1.0),
            color,
        }
    }
}

/// Gradient color lookup table.
///
/// Builds a 256-entry (or custom size) color LUT from SVG-style color stops.
/// Supports arbitrary numbers of stops at positions [0..1].
///
/// Port of C++ `gradient_lut<ColorInterpolator, ColorLutSize>`.
pub struct GradientLut {
    color_profile: Vec<ColorPoint>,
    color_lut: Vec<Rgba8>,
    lut_size: usize,
    use_fast_interpolator: bool,
}

impl GradientLut {
    /// Create a new gradient LUT with the specified size (default 256).
    pub fn new(lut_size: usize) -> Self {
        Self {
            color_profile: Vec::new(),
            color_lut: vec![Rgba8::default(); lut_size],
            lut_size,
            use_fast_interpolator: true,
        }
    }

    /// Create a new gradient LUT with default size of 256.
    pub fn new_default() -> Self {
        Self::new(256)
    }

    /// Set whether to use the fast DDA interpolator (default: true).
    pub fn set_use_fast_interpolator(&mut self, fast: bool) {
        self.use_fast_interpolator = fast;
    }

    /// Remove all color stops.
    pub fn remove_all(&mut self) {
        self.color_profile.clear();
    }

    /// Add a color stop at the given offset (clamped to [0..1]).
    pub fn add_color(&mut self, offset: f64, color: Rgba8) {
        self.color_profile.push(ColorPoint::new(offset, color));
    }

    /// Build the lookup table by interpolating between color stops.
    ///
    /// Must have at least 2 color stops. Stops are sorted by offset
    /// and duplicates are removed.
    pub fn build_lut(&mut self) {
        // Sort by offset
        self.color_profile
            .sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
        // Remove duplicates (same offset)
        self.color_profile
            .dedup_by(|a, b| (a.offset - b.offset).abs() < 1e-10);

        if self.color_profile.len() < 2 {
            return;
        }

        let size = self.lut_size;
        let mut start = uround(self.color_profile[0].offset * size as f64) as usize;

        // Fill before first stop with first color
        let c = self.color_profile[0].color;
        for i in 0..start.min(size) {
            self.color_lut[i] = c;
        }

        // Interpolate between stops
        for i in 1..self.color_profile.len() {
            let end = uround(self.color_profile[i].offset * size as f64) as usize;
            let seg_len = if end > start { end - start + 1 } else { 1 };

            if self.use_fast_interpolator {
                let mut ci = ColorInterpolatorRgba8::new(
                    &self.color_profile[i - 1].color,
                    &self.color_profile[i].color,
                    seg_len as u32,
                );
                while start < end && start < size {
                    self.color_lut[start] = ci.color();
                    ci.inc();
                    start += 1;
                }
            } else {
                let mut ci = ColorInterpolatorGeneric::new(
                    &self.color_profile[i - 1].color,
                    &self.color_profile[i].color,
                    seg_len as u32,
                );
                while start < end && start < size {
                    self.color_lut[start] = ci.color();
                    ci.inc();
                    start += 1;
                }
            }
        }

        // Fill after last stop with last color
        let c = self.color_profile.last().unwrap().color;
        let mut end = start;
        while end < size {
            self.color_lut[end] = c;
            end += 1;
        }
    }
}

impl ColorFunction for GradientLut {
    type Color = Rgba8;

    fn size(&self) -> usize {
        self.lut_size
    }

    fn get(&self, index: usize) -> Rgba8 {
        self.color_lut[index]
    }
}

// ============================================================================
// GradientLinearColor — simple 2-color linear interpolation
// ============================================================================

/// Simple 2-color linear gradient color function.
///
/// Interpolates between two colors based on index/size ratio.
///
/// Port of C++ `gradient_linear_color<ColorT>`.
pub struct GradientLinearColor {
    c1: Rgba8,
    c2: Rgba8,
    size: usize,
}

impl GradientLinearColor {
    pub fn new(c1: Rgba8, c2: Rgba8, size: usize) -> Self {
        Self { c1, c2, size }
    }

    pub fn colors(&mut self, c1: Rgba8, c2: Rgba8) {
        self.c1 = c1;
        self.c2 = c2;
    }
}

impl ColorFunction for GradientLinearColor {
    type Color = Rgba8;

    fn size(&self) -> usize {
        self.size
    }

    fn get(&self, index: usize) -> Rgba8 {
        self.c1
            .gradient(&self.c2, index as f64 / (self.size - 1).max(1) as f64)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_lut_new() {
        let lut = GradientLut::new_default();
        assert_eq!(lut.size(), 256);
    }

    #[test]
    fn test_gradient_lut_two_stops() {
        let mut lut = GradientLut::new_default();
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));
        lut.build_lut();

        // First should be red
        let c0 = lut.get(0);
        assert_eq!(c0.r, 255);
        assert_eq!(c0.b, 0);

        // Last should be nearly blue (DDA with seg_len=257 over 256 entries
        // doesn't quite reach the endpoint — matches C++ AGG behavior)
        let c255 = lut.get(255);
        assert!(c255.r <= 3, "c255.r={}", c255.r);
        assert!(c255.b >= 252, "c255.b={}", c255.b);

        // Middle should be roughly equal
        let c128 = lut.get(128);
        assert!(c128.r > 50 && c128.r < 200, "Mid r={}", c128.r);
        assert!(c128.b > 50 && c128.b < 200, "Mid b={}", c128.b);
    }

    #[test]
    fn test_gradient_lut_three_stops() {
        let mut lut = GradientLut::new_default();
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.add_color(0.5, Rgba8::new(0, 255, 0, 255));
        lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));
        lut.build_lut();

        // Start: red
        assert_eq!(lut.get(0).r, 255);
        // End: nearly blue (multi-segment has ~2% error from DDA)
        assert!(lut.get(255).b >= 248, "last.b={}", lut.get(255).b);
        // Middle: should be mostly green
        let mid = lut.get(128);
        assert!(mid.g > 128, "Mid green={}", mid.g);
    }

    #[test]
    fn test_gradient_lut_remove_all() {
        let mut lut = GradientLut::new_default();
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.remove_all();
        lut.add_color(0.0, Rgba8::new(0, 255, 0, 255));
        lut.add_color(1.0, Rgba8::new(0, 255, 0, 255));
        lut.build_lut();
        assert_eq!(lut.get(0).g, 255);
    }

    #[test]
    fn test_gradient_lut_generic_interpolator() {
        let mut lut = GradientLut::new_default();
        lut.set_use_fast_interpolator(false);
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));
        lut.build_lut();

        let c0 = lut.get(0);
        assert_eq!(c0.r, 255);
        let c255 = lut.get(255);
        assert!(c255.b >= 252, "c255.b={}", c255.b);
    }

    #[test]
    fn test_gradient_lut_custom_size() {
        let mut lut = GradientLut::new(64);
        lut.add_color(0.0, Rgba8::new(0, 0, 0, 255));
        lut.add_color(1.0, Rgba8::new(255, 255, 255, 255));
        lut.build_lut();
        assert_eq!(lut.size(), 64);
        assert_eq!(lut.get(0).r, 0);
        assert!(lut.get(63).r >= 244, "last.r={}", lut.get(63).r);
    }

    #[test]
    fn test_gradient_linear_color() {
        let gc = GradientLinearColor::new(
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
            256,
        );
        assert_eq!(gc.size(), 256);

        let c0 = gc.get(0);
        assert_eq!(c0.r, 0);

        let c255 = gc.get(255);
        assert_eq!(c255.r, 255);

        let c128 = gc.get(128);
        assert!(c128.r > 100 && c128.r < 160, "c128.r={}", c128.r);
    }

    #[test]
    fn test_gradient_lut_unsorted_stops() {
        let mut lut = GradientLut::new_default();
        lut.add_color(1.0, Rgba8::new(0, 0, 255, 255));
        lut.add_color(0.0, Rgba8::new(255, 0, 0, 255));
        lut.build_lut();

        // Should still work — stops are sorted internally
        assert_eq!(lut.get(0).r, 255);
        assert!(lut.get(255).b >= 252, "last.b={}", lut.get(255).b);
    }

    #[test]
    fn test_color_interpolator_rgba8_fast() {
        let c1 = Rgba8::new(0, 0, 0, 255);
        let c2 = Rgba8::new(255, 255, 255, 255);
        let mut ci = ColorInterpolatorRgba8::new(&c1, &c2, 10);

        let first = ci.color();
        assert_eq!(first.r, 0);

        for _ in 0..10 {
            ci.inc();
        }
        let last = ci.color();
        assert_eq!(last.r, 255);
    }

    #[test]
    fn test_gradient_linear_color_set_colors() {
        let mut gc = GradientLinearColor::new(
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(255, 255, 255, 255),
            256,
        );
        gc.colors(Rgba8::new(255, 0, 0, 255), Rgba8::new(0, 255, 0, 255));
        assert_eq!(gc.get(0).r, 255);
        assert_eq!(gc.get(0).g, 0);
        assert_eq!(gc.get(255).r, 0);
        assert_eq!(gc.get(255).g, 255);
    }
}

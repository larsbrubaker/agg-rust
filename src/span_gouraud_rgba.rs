//! RGBA Gouraud shading span generator.
//!
//! Port of `agg_span_gouraud_rgba.h` — interpolates RGBA colors across a
//! triangle using DDA-based scanline interpolation.

use crate::basics::{iround, VertexSource};
use crate::color::Rgba8;
use crate::dda_line::DdaLineInterpolator;
use crate::math::cross_product;
use crate::renderer_scanline::SpanGenerator;
use crate::span_gouraud::{CoordType, SpanGouraud};

const SUBPIXEL_SHIFT: i32 = 4;
const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;

// ============================================================================
// RgbaCalc — per-edge color/position interpolator
// ============================================================================

/// Per-edge interpolation state for Gouraud shading.
///
/// Computes color and x-position at a given scanline y by linearly
/// interpolating between two triangle vertices.
///
/// Port of C++ `span_gouraud_rgba::rgba_calc`.
struct RgbaCalc {
    x1: f64,
    y1: f64,
    dx: f64,
    inv_dy: f64,
    r1: i32,
    g1: i32,
    b1: i32,
    a1: i32,
    dr: i32,
    dg: i32,
    db: i32,
    da: i32,
    r: i32,
    g: i32,
    b: i32,
    a: i32,
    x: i32,
}

impl RgbaCalc {
    fn new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            dx: 0.0,
            inv_dy: 0.0,
            r1: 0,
            g1: 0,
            b1: 0,
            a1: 0,
            dr: 0,
            dg: 0,
            db: 0,
            da: 0,
            r: 0,
            g: 0,
            b: 0,
            a: 0,
            x: 0,
        }
    }

    fn init(&mut self, c1: &CoordType<Rgba8>, c2: &CoordType<Rgba8>) {
        self.x1 = c1.x - 0.5;
        self.y1 = c1.y - 0.5;
        self.dx = c2.x - c1.x;
        let dy = c2.y - c1.y;
        self.inv_dy = if dy < 1e-5 { 1e5 } else { 1.0 / dy };
        self.r1 = c1.color.r as i32;
        self.g1 = c1.color.g as i32;
        self.b1 = c1.color.b as i32;
        self.a1 = c1.color.a as i32;
        self.dr = c2.color.r as i32 - self.r1;
        self.dg = c2.color.g as i32 - self.g1;
        self.db = c2.color.b as i32 - self.b1;
        self.da = c2.color.a as i32 - self.a1;
    }

    fn calc(&mut self, y: f64) {
        let k = ((y - self.y1) * self.inv_dy).clamp(0.0, 1.0);
        self.r = self.r1 + iround(self.dr as f64 * k);
        self.g = self.g1 + iround(self.dg as f64 * k);
        self.b = self.b1 + iround(self.db as f64 * k);
        self.a = self.a1 + iround(self.da as f64 * k);
        self.x = iround((self.x1 + self.dx * k) * SUBPIXEL_SCALE as f64);
    }
}

/// Adjust DDA by a signed step count (equivalent to C++ `r -= start`).
fn dda_sub(dda: &mut DdaLineInterpolator<14, 0>, n: i32) {
    if n >= 0 {
        dda.dec_by(n as u32);
    } else {
        dda.inc_by((-n) as u32);
    }
}

// ============================================================================
// SpanGouraudRgba
// ============================================================================

/// RGBA Gouraud shading span generator.
///
/// Composes `SpanGouraud<Rgba8>` for triangle storage and provides the
/// `SpanGenerator` implementation that interpolates RGBA colors across
/// scanlines using DDA.
///
/// Port of C++ `span_gouraud_rgba<ColorT>`.
pub struct SpanGouraudRgba {
    base: SpanGouraud<Rgba8>,
    swap: bool,
    y2: i32,
    rgba1: RgbaCalc,
    rgba2: RgbaCalc,
    rgba3: RgbaCalc,
}

impl SpanGouraudRgba {
    pub fn new() -> Self {
        Self {
            base: SpanGouraud::new(),
            swap: false,
            y2: 0,
            rgba1: RgbaCalc::new(),
            rgba2: RgbaCalc::new(),
            rgba3: RgbaCalc::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_triangle(
        c1: Rgba8,
        c2: Rgba8,
        c3: Rgba8,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        d: f64,
    ) -> Self {
        Self {
            base: SpanGouraud::new_with_triangle(c1, c2, c3, x1, y1, x2, y2, x3, y3, d),
            swap: false,
            y2: 0,
            rgba1: RgbaCalc::new(),
            rgba2: RgbaCalc::new(),
            rgba3: RgbaCalc::new(),
        }
    }

    /// Delegate to base: set vertex colors.
    pub fn colors(&mut self, c1: Rgba8, c2: Rgba8, c3: Rgba8) {
        self.base.colors(c1, c2, c3);
    }

    /// Delegate to base: set triangle geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn triangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, d: f64) {
        self.base.triangle(x1, y1, x2, y2, x3, y3, d);
    }
}

impl Default for SpanGouraudRgba {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexSource for SpanGouraudRgba {
    fn rewind(&mut self, path_id: u32) {
        self.base.rewind(path_id);
    }

    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.base.vertex(x, y)
    }
}

impl SpanGenerator for SpanGouraudRgba {
    type Color = Rgba8;

    fn prepare(&mut self) {
        let coord = self.base.arrange_vertices();

        self.y2 = coord[1].y as i32;

        self.swap = cross_product(
            coord[0].x, coord[0].y, coord[2].x, coord[2].y, coord[1].x, coord[1].y,
        ) < 0.0;

        self.rgba1.init(&coord[0], &coord[2]);
        self.rgba2.init(&coord[0], &coord[1]);
        self.rgba3.init(&coord[1], &coord[2]);
    }

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        self.rgba1.calc(y as f64);

        let (pc1_r, pc1_g, pc1_b, pc1_a, pc1_x, pc2_r, pc2_g, pc2_b, pc2_a, pc2_x);

        if y <= self.y2 {
            // Bottom part of the triangle (first sub-triangle)
            self.rgba2.calc(y as f64 + self.rgba2.inv_dy);
            if self.swap {
                pc1_r = self.rgba2.r;
                pc1_g = self.rgba2.g;
                pc1_b = self.rgba2.b;
                pc1_a = self.rgba2.a;
                pc1_x = self.rgba2.x;
                pc2_r = self.rgba1.r;
                pc2_g = self.rgba1.g;
                pc2_b = self.rgba1.b;
                pc2_a = self.rgba1.a;
                pc2_x = self.rgba1.x;
            } else {
                pc1_r = self.rgba1.r;
                pc1_g = self.rgba1.g;
                pc1_b = self.rgba1.b;
                pc1_a = self.rgba1.a;
                pc1_x = self.rgba1.x;
                pc2_r = self.rgba2.r;
                pc2_g = self.rgba2.g;
                pc2_b = self.rgba2.b;
                pc2_a = self.rgba2.a;
                pc2_x = self.rgba2.x;
            }
        } else {
            // Upper part (second sub-triangle)
            self.rgba3.calc(y as f64 - self.rgba3.inv_dy);
            if self.swap {
                pc1_r = self.rgba3.r;
                pc1_g = self.rgba3.g;
                pc1_b = self.rgba3.b;
                pc1_a = self.rgba3.a;
                pc1_x = self.rgba3.x;
                pc2_r = self.rgba1.r;
                pc2_g = self.rgba1.g;
                pc2_b = self.rgba1.b;
                pc2_a = self.rgba1.a;
                pc2_x = self.rgba1.x;
            } else {
                pc1_r = self.rgba1.r;
                pc1_g = self.rgba1.g;
                pc1_b = self.rgba1.b;
                pc1_a = self.rgba1.a;
                pc1_x = self.rgba1.x;
                pc2_r = self.rgba3.r;
                pc2_g = self.rgba3.g;
                pc2_b = self.rgba3.b;
                pc2_a = self.rgba3.a;
                pc2_x = self.rgba3.x;
            }
        }

        // Horizontal interpolation length with subpixel accuracy
        let mut nlen = (pc2_x - pc1_x).abs();
        if nlen <= 0 {
            nlen = 1;
        }

        let mut r = DdaLineInterpolator::<14, 0>::new(pc1_r, pc2_r, nlen as u32);
        let mut g = DdaLineInterpolator::<14, 0>::new(pc1_g, pc2_g, nlen as u32);
        let mut b = DdaLineInterpolator::<14, 0>::new(pc1_b, pc2_b, nlen as u32);
        let mut a = DdaLineInterpolator::<14, 0>::new(pc1_a, pc2_a, nlen as u32);

        // Roll back interpolators to span start
        let mut start = pc1_x - (x << SUBPIXEL_SHIFT);
        dda_sub(&mut r, start);
        dda_sub(&mut g, start);
        dda_sub(&mut b, start);
        dda_sub(&mut a, start);
        nlen += start;

        let lim = Rgba8::BASE_MASK as i32;
        let mut idx = 0usize;
        let mut remaining = len as i32;

        // Beginning part — check for overflow (typically 1-2 pixels)
        while remaining > 0 && start > 0 {
            let vr = r.y().clamp(0, lim);
            let vg = g.y().clamp(0, lim);
            let vb = b.y().clamp(0, lim);
            let va = a.y().clamp(0, lim);
            span[idx].r = vr as u8;
            span[idx].g = vg as u8;
            span[idx].b = vb as u8;
            span[idx].a = va as u8;
            r.inc_by(SUBPIXEL_SCALE as u32);
            g.inc_by(SUBPIXEL_SCALE as u32);
            b.inc_by(SUBPIXEL_SCALE as u32);
            a.inc_by(SUBPIXEL_SCALE as u32);
            nlen -= SUBPIXEL_SCALE;
            start -= SUBPIXEL_SCALE;
            idx += 1;
            remaining -= 1;
        }

        // Middle part — no overflow checking needed
        while remaining > 0 && nlen > 0 {
            span[idx].r = r.y() as u8;
            span[idx].g = g.y() as u8;
            span[idx].b = b.y() as u8;
            span[idx].a = a.y() as u8;
            r.inc_by(SUBPIXEL_SCALE as u32);
            g.inc_by(SUBPIXEL_SCALE as u32);
            b.inc_by(SUBPIXEL_SCALE as u32);
            a.inc_by(SUBPIXEL_SCALE as u32);
            nlen -= SUBPIXEL_SCALE;
            idx += 1;
            remaining -= 1;
        }

        // Ending part — check for overflow again
        while remaining > 0 {
            let vr = r.y().clamp(0, lim);
            let vg = g.y().clamp(0, lim);
            let vb = b.y().clamp(0, lim);
            let va = a.y().clamp(0, lim);
            span[idx].r = vr as u8;
            span[idx].g = vg as u8;
            span[idx].b = vb as u8;
            span[idx].a = va as u8;
            r.inc_by(SUBPIXEL_SCALE as u32);
            g.inc_by(SUBPIXEL_SCALE as u32);
            b.inc_by(SUBPIXEL_SCALE as u32);
            a.inc_by(SUBPIXEL_SCALE as u32);
            idx += 1;
            remaining -= 1;
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_default() {
        let sg = SpanGouraudRgba::new();
        assert_eq!(sg.y2, 0);
        assert!(!sg.swap);
    }

    #[test]
    fn test_prepare_simple_triangle() {
        let mut sg = SpanGouraudRgba::new();
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue = Rgba8::new(0, 0, 255, 255);
        sg.colors(red, green, blue);
        sg.triangle(0.0, 0.0, 100.0, 50.0, 50.0, 100.0, 0.0);
        sg.prepare();
        // y2 should be the middle vertex Y
        assert!(sg.y2 >= 0);
    }

    #[test]
    fn test_generate_horizontal_gradient() {
        // Triangle spanning the x-axis with red on left, green on right
        let mut sg = SpanGouraudRgba::new();
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue_ish = Rgba8::new(128, 128, 0, 255);
        sg.colors(red, green, blue_ish);
        sg.triangle(0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 0.0);
        sg.prepare();

        let mut span = vec![Rgba8::default(); 10];
        sg.generate(&mut span, 0, 50, 10);

        // Alpha should be non-zero for valid pixels
        // (Exact values depend on triangle geometry)
        let has_nonzero = span.iter().any(|c| c.a > 0);
        assert!(has_nonzero, "Expected some visible pixels");
    }

    #[test]
    fn test_generate_single_color() {
        // All three vertices same color — result should be uniform
        let c = Rgba8::new(100, 150, 200, 255);
        let mut sg = SpanGouraudRgba::new();
        sg.colors(c, c, c);
        sg.triangle(0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 0.0);
        sg.prepare();

        let mut span = vec![Rgba8::default(); 5];
        sg.generate(&mut span, 20, 25, 5);

        // All pixels in the span should have similar color values
        for pixel in &span {
            assert!(
                (pixel.r as i32 - 100).abs() <= 2,
                "r={} expected ~100",
                pixel.r
            );
            assert!(
                (pixel.g as i32 - 150).abs() <= 2,
                "g={} expected ~150",
                pixel.g
            );
            assert!(
                (pixel.b as i32 - 200).abs() <= 2,
                "b={} expected ~200",
                pixel.b
            );
        }
    }

    #[test]
    fn test_vertex_source_delegation() {
        let mut sg = SpanGouraudRgba::new();
        let c = Rgba8::new(128, 128, 128, 255);
        sg.colors(c, c, c);
        sg.triangle(10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 0.0);

        sg.rewind(0);
        let mut x = 0.0;
        let mut y = 0.0;
        let cmd = sg.vertex(&mut x, &mut y);
        assert_eq!(cmd, 1); // PATH_CMD_MOVE_TO
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn test_new_with_triangle() {
        let red = Rgba8::new(255, 0, 0, 255);
        let green = Rgba8::new(0, 255, 0, 255);
        let blue = Rgba8::new(0, 0, 255, 255);
        let mut sg = SpanGouraudRgba::new_with_triangle(
            red, green, blue, 0.0, 0.0, 100.0, 0.0, 50.0, 100.0, 0.0,
        );
        sg.prepare();
        // Should not panic
        let mut span = vec![Rgba8::default(); 3];
        sg.generate(&mut span, 40, 50, 3);
    }

    #[test]
    fn test_rgba_calc_init_and_calc() {
        let c1 = CoordType {
            x: 0.0,
            y: 0.0,
            color: Rgba8::new(0, 0, 0, 255),
        };
        let c2 = CoordType {
            x: 100.0,
            y: 100.0,
            color: Rgba8::new(255, 255, 255, 255),
        };

        let mut calc = RgbaCalc::new();
        calc.init(&c1, &c2);

        // At y=0 (start), should be near c1's color
        // (y1 is stored as c1.y - 0.5, so k is slightly > 0)
        calc.calc(0.0);
        assert!(calc.r <= 2, "r={}", calc.r);
        assert!(calc.g <= 2, "g={}", calc.g);

        // At y=100 (end), should be near c2's color
        calc.calc(100.0);
        assert!(calc.r >= 253, "r={}", calc.r);
        assert!(calc.g >= 253, "g={}", calc.g);

        // At y=50 (middle), should be halfway
        calc.calc(50.0);
        assert!(calc.r > 100 && calc.r < 160, "r={}", calc.r);
    }

    #[test]
    fn test_rgba_calc_zero_height() {
        // Degenerate case: zero height triangle edge
        let c1 = CoordType {
            x: 0.0,
            y: 50.0,
            color: Rgba8::new(100, 100, 100, 255),
        };
        let c2 = CoordType {
            x: 100.0,
            y: 50.0,
            color: Rgba8::new(200, 200, 200, 255),
        };

        let mut calc = RgbaCalc::new();
        calc.init(&c1, &c2);
        // Should not panic, inv_dy should be large
        calc.calc(50.0);
    }

    #[test]
    fn test_dda_sub_positive() {
        let mut dda = DdaLineInterpolator::<14, 0>::new(0, 255, 100);
        let y_before = dda.y();
        dda_sub(&mut dda, 10);
        // After subtracting 10 steps, y should decrease
        assert!(dda.y() <= y_before);
    }

    #[test]
    fn test_dda_sub_negative() {
        let mut dda = DdaLineInterpolator::<14, 0>::new(0, 255, 100);
        let y_before = dda.y();
        dda_sub(&mut dda, -10);
        // After subtracting negative steps (= adding), y should increase
        assert!(dda.y() >= y_before);
    }
}

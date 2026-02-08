//! SVG compositing operations and compositing pixel format.
//!
//! Port of the compositing portion of `agg_pixfmt_rgba.h`.
//! Provides 25 SVG compositing modes (the standard 24 plus `minus`)
//! and `PixfmtRgba32CompOp`, a pixel format that dispatches blending
//! through a runtime-selectable compositing operation.

use crate::basics::CoverType;
use crate::color::Rgba8;
use crate::pixfmt_rgba::PixelFormat;
use crate::rendering_buffer::RowAccessor;

// ============================================================================
// CompOp enum — 25 SVG/AGG compositing modes
// ============================================================================

/// SVG compositing operation.
///
/// Port of C++ `comp_op_e`. Each variant corresponds to a specific
/// alpha-compositing formula from the SVG Compositing specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompOp {
    Clear = 0,
    Src = 1,
    Dst = 2,
    SrcOver = 3,
    DstOver = 4,
    SrcIn = 5,
    DstIn = 6,
    SrcOut = 7,
    DstOut = 8,
    SrcAtop = 9,
    DstAtop = 10,
    Xor = 11,
    Plus = 12,
    Minus = 13,
    Multiply = 14,
    Screen = 15,
    Overlay = 16,
    Darken = 17,
    Lighten = 18,
    ColorDodge = 19,
    ColorBurn = 20,
    HardLight = 21,
    SoftLight = 22,
    Difference = 23,
    Exclusion = 24,
}

impl Default for CompOp {
    fn default() -> Self {
        CompOp::SrcOver
    }
}

// ============================================================================
// Premultiplied f64 RGBA working space (like C++ blender_base::rgba)
// ============================================================================

/// Premultiplied RGBA in f64 [0, 1] working space.
#[derive(Debug, Clone, Copy)]
struct PremulRgba {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl PremulRgba {
    /// Read from u8 pixel buffer as premultiplied f64, scaled by cover.
    #[inline]
    fn get(r: u8, g: u8, b: u8, a: u8, cover: u8) -> Self {
        if cover == 0 {
            return Self {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            };
        }
        let mut c = Self {
            r: Rgba8::to_double(r),
            g: Rgba8::to_double(g),
            b: Rgba8::to_double(b),
            a: Rgba8::to_double(a),
        };
        if cover < 255 {
            let x = cover as f64 / 255.0;
            c.r *= x;
            c.g *= x;
            c.b *= x;
            c.a *= x;
        }
        c
    }

    /// Read from pixel buffer slice (RGBA order) as premultiplied f64.
    #[inline]
    fn get_pix(p: &[u8], cover: u8) -> Self {
        Self::get(p[0], p[1], p[2], p[3], cover)
    }

    /// Write premultiplied f64 back to pixel buffer.
    #[inline]
    fn set(p: &mut [u8], c: &PremulRgba) {
        p[0] = Rgba8::from_double(c.r);
        p[1] = Rgba8::from_double(c.g);
        p[2] = Rgba8::from_double(c.b);
        p[3] = Rgba8::from_double(c.a);
    }

    /// Write direct RGBA values.
    #[inline]
    fn set_rgba(p: &mut [u8], r: u8, g: u8, b: u8, a: u8) {
        p[0] = r;
        p[1] = g;
        p[2] = b;
        p[3] = a;
    }

    /// Clamp all components to [0, 1].
    #[inline]
    fn clip(c: &mut PremulRgba) {
        c.r = c.r.clamp(0.0, 1.0);
        c.g = c.g.clamp(0.0, 1.0);
        c.b = c.b.clamp(0.0, 1.0);
        c.a = c.a.clamp(0.0, 1.0);
    }
}

// ============================================================================
// Per-operation blend functions
// ============================================================================

/// Blend pixel `p` (RGBA u8) with premultiplied source (r,g,b,a) using `op`.
///
/// Source colors are NON-premultiplied; this function premultiplies them
/// before dispatching (matching C++ `comp_op_adaptor_rgba`).
#[inline]
fn comp_op_blend(op: CompOp, p: &mut [u8], sr: u8, sg: u8, sb: u8, sa: u8, cover: u8) {
    // Premultiply source by alpha (matching comp_op_adaptor_rgba)
    let r = Rgba8::multiply(sr, sa);
    let g = Rgba8::multiply(sg, sa);
    let b = Rgba8::multiply(sb, sa);
    let a = sa;

    match op {
        CompOp::Clear => blend_clear(p, cover),
        CompOp::Src => blend_src(p, r, g, b, a, cover),
        CompOp::Dst => {} // no-op
        CompOp::SrcOver => blend_src_over(p, r, g, b, a, cover),
        CompOp::DstOver => blend_dst_over(p, r, g, b, a, cover),
        CompOp::SrcIn => blend_src_in(p, r, g, b, a, cover),
        CompOp::DstIn => blend_dst_in(p, a, cover),
        CompOp::SrcOut => blend_src_out(p, r, g, b, a, cover),
        CompOp::DstOut => blend_dst_out(p, a, cover),
        CompOp::SrcAtop => blend_src_atop(p, r, g, b, a, cover),
        CompOp::DstAtop => blend_dst_atop(p, r, g, b, a, cover),
        CompOp::Xor => blend_xor(p, r, g, b, a, cover),
        CompOp::Plus => blend_plus(p, r, g, b, a, cover),
        CompOp::Minus => blend_minus(p, r, g, b, a, cover),
        CompOp::Multiply => blend_multiply(p, r, g, b, a, cover),
        CompOp::Screen => blend_screen(p, r, g, b, a, cover),
        CompOp::Overlay => blend_overlay(p, r, g, b, a, cover),
        CompOp::Darken => blend_darken(p, r, g, b, a, cover),
        CompOp::Lighten => blend_lighten(p, r, g, b, a, cover),
        CompOp::ColorDodge => blend_color_dodge(p, r, g, b, a, cover),
        CompOp::ColorBurn => blend_color_burn(p, r, g, b, a, cover),
        CompOp::HardLight => blend_hard_light(p, r, g, b, a, cover),
        CompOp::SoftLight => blend_soft_light(p, r, g, b, a, cover),
        CompOp::Difference => blend_difference(p, r, g, b, a, cover),
        CompOp::Exclusion => blend_exclusion(p, r, g, b, a, cover),
    }
}

// ---- Clear: Dca' = 0, Da' = 0
#[inline]
fn blend_clear(p: &mut [u8], cover: u8) {
    if cover >= 255 {
        p[0] = 0;
        p[1] = 0;
        p[2] = 0;
        p[3] = 0;
    } else if cover > 0 {
        let d = PremulRgba::get_pix(p, 255 - cover);
        PremulRgba::set(p, &d);
    }
}

// ---- Src: Dca' = Sca, Da' = Sa
#[inline]
fn blend_src(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    if cover >= 255 {
        PremulRgba::set_rgba(p, r, g, b, a);
    } else {
        let s = PremulRgba::get(r, g, b, a, cover);
        let d = PremulRgba::get_pix(p, 255 - cover);
        let out = PremulRgba {
            r: d.r + s.r,
            g: d.g + s.g,
            b: d.b + s.b,
            a: d.a + s.a,
        };
        PremulRgba::set(p, &out);
    }
}

// ---- SrcOver: Dca' = Sca + Dca.(1 - Sa)
#[inline]
fn blend_src_over(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    let d = PremulRgba::get_pix(p, 255);
    let out = PremulRgba {
        r: d.r + s.r - d.r * s.a,
        g: d.g + s.g - d.g * s.a,
        b: d.b + s.b - d.b * s.a,
        a: d.a + s.a - d.a * s.a,
    };
    PremulRgba::set(p, &out);
}

// ---- DstOver: Dca' = Dca + Sca.(1 - Da)
#[inline]
fn blend_dst_over(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    let mut d = PremulRgba::get_pix(p, 255);
    let d1a = 1.0 - d.a;
    d.r += s.r * d1a;
    d.g += s.g * d1a;
    d.b += s.b * d1a;
    d.a += s.a * d1a;
    PremulRgba::set(p, &d);
}

// ---- SrcIn: Dca' = Sca.Da
#[inline]
fn blend_src_in(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let da = Rgba8::to_double(p[3]);
    if da > 0.0 {
        let s = PremulRgba::get(r, g, b, a, cover);
        let mut d = PremulRgba::get_pix(p, 255 - cover);
        d.r += s.r * da;
        d.g += s.g * da;
        d.b += s.b * da;
        d.a += s.a * da;
        PremulRgba::set(p, &d);
    }
}

// ---- DstIn: Dca' = Dca.Sa
#[inline]
fn blend_dst_in(p: &mut [u8], a: u8, cover: u8) {
    let sa = Rgba8::to_double(a);
    let mut d = PremulRgba::get_pix(p, 255 - cover);
    let d2 = PremulRgba::get_pix(p, cover);
    d.r += d2.r * sa;
    d.g += d2.g * sa;
    d.b += d2.b * sa;
    d.a += d2.a * sa;
    PremulRgba::set(p, &d);
}

// ---- SrcOut: Dca' = Sca.(1 - Da)
#[inline]
fn blend_src_out(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    let mut d = PremulRgba::get_pix(p, 255 - cover);
    let d1a = 1.0 - Rgba8::to_double(p[3]);
    d.r += s.r * d1a;
    d.g += s.g * d1a;
    d.b += s.b * d1a;
    d.a += s.a * d1a;
    PremulRgba::set(p, &d);
}

// ---- DstOut: Dca' = Dca.(1 - Sa)
#[inline]
fn blend_dst_out(p: &mut [u8], a: u8, cover: u8) {
    let mut d = PremulRgba::get_pix(p, 255 - cover);
    let dc = PremulRgba::get_pix(p, cover);
    let s1a = 1.0 - Rgba8::to_double(a);
    d.r += dc.r * s1a;
    d.g += dc.g * s1a;
    d.b += dc.b * s1a;
    d.a += dc.a * s1a;
    PremulRgba::set(p, &d);
}

// ---- SrcAtop: Dca' = Sca.Da + Dca.(1 - Sa), Da' = Da
#[inline]
fn blend_src_atop(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    let mut d = PremulRgba::get_pix(p, 255);
    let s1a = 1.0 - s.a;
    d.r = s.r * d.a + d.r * s1a;
    d.g = s.g * d.a + d.g * s1a;
    d.b = s.b * d.a + d.b * s1a;
    // Da' = Da (unchanged)
    PremulRgba::set(p, &d);
}

// ---- DstAtop: Dca' = Dca.Sa + Sca.(1 - Da), Da' = Sa
#[inline]
fn blend_dst_atop(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let sc = PremulRgba::get(r, g, b, a, cover);
    let dc = PremulRgba::get_pix(p, cover);
    let mut d = PremulRgba::get_pix(p, 255 - cover);
    let sa = Rgba8::to_double(a);
    let d1a = 1.0 - Rgba8::to_double(p[3]);
    d.r += dc.r * sa + sc.r * d1a;
    d.g += dc.g * sa + sc.g * d1a;
    d.b += dc.b * sa + sc.b * d1a;
    d.a += sc.a;
    PremulRgba::set(p, &d);
}

// ---- Xor: Dca' = Sca.(1 - Da) + Dca.(1 - Sa)
#[inline]
fn blend_xor(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    let mut d = PremulRgba::get_pix(p, 255);
    let s1a = 1.0 - s.a;
    let d1a = 1.0 - Rgba8::to_double(p[3]);
    d.r = s.r * d1a + d.r * s1a;
    d.g = s.g * d1a + d.g * s1a;
    d.b = s.b * d1a + d.b * s1a;
    d.a = s.a + d.a - 2.0 * s.a * d.a;
    PremulRgba::set(p, &d);
}

// ---- Plus: Dca' = Sca + Dca (clamped)
#[inline]
fn blend_plus(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        d.a = (d.a + s.a).min(1.0);
        d.r = (d.r + s.r).min(d.a);
        d.g = (d.g + s.g).min(d.a);
        d.b = (d.b + s.b).min(d.a);
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Minus: Dca' = Dca - Sca (clamped)
#[inline]
fn blend_minus(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        d.a += s.a - s.a * d.a;
        d.r = (d.r - s.r).max(0.0);
        d.g = (d.g - s.g).max(0.0);
        d.b = (d.b - s.b).max(0.0);
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Multiply: Dca' = Sca.Dca + Sca.(1 - Da) + Dca.(1 - Sa)
#[inline]
fn blend_multiply(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let s1a = 1.0 - s.a;
        let d1a = 1.0 - d.a;
        d.r = s.r * d.r + s.r * d1a + d.r * s1a;
        d.g = s.g * d.g + s.g * d1a + d.g * s1a;
        d.b = s.b * d.b + s.b * d1a + d.b * s1a;
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Screen: Dca' = Sca + Dca - Sca.Dca
#[inline]
fn blend_screen(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        d.r += s.r - s.r * d.r;
        d.g += s.g - s.g * d.g;
        d.b += s.b - s.b * d.b;
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Overlay
#[inline]
fn overlay_calc(dca: f64, sca: f64, da: f64, sa: f64, sada: f64, d1a: f64, s1a: f64) -> f64 {
    if 2.0 * dca <= da {
        2.0 * sca * dca + sca * d1a + dca * s1a
    } else {
        sada - 2.0 * (da - dca) * (sa - sca) + sca * d1a + dca * s1a
    }
}

#[inline]
fn blend_overlay(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let d1a = 1.0 - d.a;
        let s1a = 1.0 - s.a;
        let sada = s.a * d.a;
        d.r = overlay_calc(d.r, s.r, d.a, s.a, sada, d1a, s1a);
        d.g = overlay_calc(d.g, s.g, d.a, s.a, sada, d1a, s1a);
        d.b = overlay_calc(d.b, s.b, d.a, s.a, sada, d1a, s1a);
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Darken: min(Sca.Da, Dca.Sa) + Sca.(1 - Da) + Dca.(1 - Sa)
#[inline]
fn blend_darken(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let d1a = 1.0 - d.a;
        let s1a = 1.0 - s.a;
        d.r = (s.r * d.a).min(d.r * s.a) + s.r * d1a + d.r * s1a;
        d.g = (s.g * d.a).min(d.g * s.a) + s.g * d1a + d.g * s1a;
        d.b = (s.b * d.a).min(d.b * s.a) + s.b * d1a + d.b * s1a;
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Lighten: max(Sca.Da, Dca.Sa) + Sca.(1 - Da) + Dca.(1 - Sa)
#[inline]
fn blend_lighten(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let d1a = 1.0 - d.a;
        let s1a = 1.0 - s.a;
        d.r = (s.r * d.a).max(d.r * s.a) + s.r * d1a + d.r * s1a;
        d.g = (s.g * d.a).max(d.g * s.a) + s.g * d1a + d.g * s1a;
        d.b = (s.b * d.a).max(d.b * s.a) + s.b * d1a + d.b * s1a;
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- ColorDodge
#[inline]
fn color_dodge_calc(dca: f64, sca: f64, da: f64, sa: f64, sada: f64, d1a: f64, s1a: f64) -> f64 {
    if sca < sa {
        sada * (1.0f64).min((dca / da) * sa / (sa - sca)) + sca * d1a + dca * s1a
    } else if dca > 0.0 {
        sada + sca * d1a + dca * s1a
    } else {
        sca * d1a
    }
}

#[inline]
fn blend_color_dodge(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        if d.a > 0.0 {
            let sada = s.a * d.a;
            let s1a = 1.0 - s.a;
            let d1a = 1.0 - d.a;
            d.r = color_dodge_calc(d.r, s.r, d.a, s.a, sada, d1a, s1a);
            d.g = color_dodge_calc(d.g, s.g, d.a, s.a, sada, d1a, s1a);
            d.b = color_dodge_calc(d.b, s.b, d.a, s.a, sada, d1a, s1a);
            d.a += s.a - s.a * d.a;
            PremulRgba::clip(&mut d);
            PremulRgba::set(p, &d);
        } else {
            PremulRgba::set(p, &s);
        }
    }
}

// ---- ColorBurn
#[inline]
fn color_burn_calc(dca: f64, sca: f64, da: f64, sa: f64, sada: f64, d1a: f64, s1a: f64) -> f64 {
    if sca > 0.0 {
        sada * (1.0 - (1.0f64).min((1.0 - dca / da) * sa / sca)) + sca * d1a + dca * s1a
    } else if dca > da {
        sada + dca * s1a
    } else {
        dca * s1a
    }
}

#[inline]
fn blend_color_burn(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        if d.a > 0.0 {
            let sada = s.a * d.a;
            let s1a = 1.0 - s.a;
            let d1a = 1.0 - d.a;
            d.r = color_burn_calc(d.r, s.r, d.a, s.a, sada, d1a, s1a);
            d.g = color_burn_calc(d.g, s.g, d.a, s.a, sada, d1a, s1a);
            d.b = color_burn_calc(d.b, s.b, d.a, s.a, sada, d1a, s1a);
            d.a += s.a - sada;
            PremulRgba::clip(&mut d);
            PremulRgba::set(p, &d);
        } else {
            PremulRgba::set(p, &s);
        }
    }
}

// ---- HardLight
#[inline]
fn hard_light_calc(dca: f64, sca: f64, da: f64, sa: f64, sada: f64, d1a: f64, s1a: f64) -> f64 {
    if 2.0 * sca < sa {
        2.0 * sca * dca + sca * d1a + dca * s1a
    } else {
        sada - 2.0 * (da - dca) * (sa - sca) + sca * d1a + dca * s1a
    }
}

#[inline]
fn blend_hard_light(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let d1a = 1.0 - d.a;
        let s1a = 1.0 - s.a;
        let sada = s.a * d.a;
        d.r = hard_light_calc(d.r, s.r, d.a, s.a, sada, d1a, s1a);
        d.g = hard_light_calc(d.g, s.g, d.a, s.a, sada, d1a, s1a);
        d.b = hard_light_calc(d.b, s.b, d.a, s.a, sada, d1a, s1a);
        d.a += s.a - sada;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- SoftLight
#[inline]
fn soft_light_calc(dca: f64, sca: f64, da: f64, sa: f64, sada: f64, d1a: f64, s1a: f64) -> f64 {
    let dcasa = dca * sa;
    if 2.0 * sca <= sa {
        dcasa - (sada - 2.0 * sca * da) * dcasa * (sada - dcasa) + sca * d1a + dca * s1a
    } else if 4.0 * dca <= da {
        dcasa
            + (2.0 * sca * da - sada)
                * ((((16.0 * dcasa - 12.0) * dcasa + 4.0) * dca * da) - dca * da)
            + sca * d1a
            + dca * s1a
    } else {
        dcasa + (2.0 * sca * da - sada) * (dcasa.sqrt() - dcasa) + sca * d1a + dca * s1a
    }
}

#[inline]
fn blend_soft_light(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        if d.a > 0.0 {
            let sada = s.a * d.a;
            let s1a = 1.0 - s.a;
            let d1a = 1.0 - d.a;
            d.r = soft_light_calc(d.r, s.r, d.a, s.a, sada, d1a, s1a);
            d.g = soft_light_calc(d.g, s.g, d.a, s.a, sada, d1a, s1a);
            d.b = soft_light_calc(d.b, s.b, d.a, s.a, sada, d1a, s1a);
            d.a += s.a - sada;
            PremulRgba::clip(&mut d);
            PremulRgba::set(p, &d);
        } else {
            PremulRgba::set(p, &s);
        }
    }
}

// ---- Difference: Dca' = Sca + Dca - 2.min(Sca.Da, Dca.Sa)
#[inline]
fn blend_difference(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        d.r += s.r - 2.0 * (s.r * d.a).min(d.r * s.a);
        d.g += s.g - 2.0 * (s.g * d.a).min(d.g * s.a);
        d.b += s.b - 2.0 * (s.b * d.a).min(d.b * s.a);
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ---- Exclusion: (Sca.Da + Dca.Sa - 2.Sca.Dca) + Sca.(1 - Da) + Dca.(1 - Sa)
#[inline]
fn blend_exclusion(p: &mut [u8], r: u8, g: u8, b: u8, a: u8, cover: u8) {
    let s = PremulRgba::get(r, g, b, a, cover);
    if s.a > 0.0 {
        let mut d = PremulRgba::get_pix(p, 255);
        let d1a = 1.0 - d.a;
        let s1a = 1.0 - s.a;
        d.r = (s.r * d.a + d.r * s.a - 2.0 * s.r * d.r) + s.r * d1a + d.r * s1a;
        d.g = (s.g * d.a + d.g * s.a - 2.0 * s.g * d.g) + s.g * d1a + d.g * s1a;
        d.b = (s.b * d.a + d.b * s.a - 2.0 * s.b * d.b) + s.b * d1a + d.b * s1a;
        d.a += s.a - s.a * d.a;
        PremulRgba::clip(&mut d);
        PremulRgba::set(p, &d);
    }
}

// ============================================================================
// PixfmtRgba32CompOp — pixel format with runtime-selectable compositing
// ============================================================================

const BPP: usize = 4;

/// RGBA32 pixel format with runtime-selectable SVG compositing operations.
///
/// Port of C++ `pixfmt_custom_blend_rgba<comp_op_adaptor_rgba<rgba8, order_rgba>, rendering_buffer>`.
/// Wraps a `RowAccessor` and stores the current compositing operation.
/// All blending is dispatched through `comp_op_blend()`.
pub struct PixfmtRgba32CompOp<'a> {
    rbuf: &'a mut RowAccessor,
    comp_op: CompOp,
}

impl<'a> PixfmtRgba32CompOp<'a> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self {
            rbuf,
            comp_op: CompOp::SrcOver,
        }
    }

    pub fn new_with_op(rbuf: &'a mut RowAccessor, op: CompOp) -> Self {
        Self { rbuf, comp_op: op }
    }

    pub fn comp_op(&self) -> CompOp {
        self.comp_op
    }

    pub fn set_comp_op(&mut self, op: CompOp) {
        self.comp_op = op;
    }

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &Rgba8) {
        let w = self.rbuf.width();
        let h = self.rbuf.height();
        for y in 0..h {
            let row = unsafe {
                let ptr = self.rbuf.row_ptr(y as i32);
                std::slice::from_raw_parts_mut(ptr, (w as usize) * BPP)
            };
            for x in 0..w as usize {
                let off = x * BPP;
                row[off] = c.r;
                row[off + 1] = c.g;
                row[off + 2] = c.b;
                row[off + 3] = c.a;
            }
        }
    }
}

impl<'a> PixelFormat for PixfmtRgba32CompOp<'a> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.rbuf.width()
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        Rgba8::new(
            row[off] as u32,
            row[off + 1] as u32,
            row[off + 2] as u32,
            row[off + 3] as u32,
        )
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        row[off] = c.r;
        row[off + 1] = c.g;
        row[off + 2] = c.b;
        row[off + 3] = c.a;
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for i in 0..len as usize {
            let off = (x as usize + i) * BPP;
            row[off] = c.r;
            row[off + 1] = c.g;
            row[off + 2] = c.b;
            row[off + 3] = c.a;
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        let off = x as usize * BPP;
        comp_op_blend(self.comp_op, &mut row[off..off + BPP], c.r, c.g, c.b, c.a, cover);
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, cover: CoverType) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for i in 0..len as usize {
            let off = (x as usize + i) * BPP;
            comp_op_blend(self.comp_op, &mut row[off..off + BPP], c.r, c.g, c.b, c.a, cover);
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, covers: &[CoverType]) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for (i, &cov) in covers.iter().enumerate().take(len as usize) {
            let off = (x as usize + i) * BPP;
            comp_op_blend(self.comp_op, &mut row[off..off + BPP], c.r, c.g, c.b, c.a, cov);
        }
    }

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba8],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, (self.rbuf.width() as usize) * BPP)
        };
        for i in 0..len as usize {
            let c = &colors[i];
            let cov = if !covers.is_empty() { covers[i] } else { cover };
            let off = (x as usize + i) * BPP;
            comp_op_blend(self.comp_op, &mut row[off..off + BPP], c.r, c.g, c.b, c.a, cov);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering_buffer::RowAccessor;

    fn make_buffer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * 4) as i32;
        let buf = vec![0u8; (h * w * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    fn get_pixel(buf: &[u8], w: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let off = (y * w * 4 + x * 4) as usize;
        (buf[off], buf[off + 1], buf[off + 2], buf[off + 3])
    }

    #[test]
    fn test_comp_op_default() {
        assert_eq!(CompOp::default(), CompOp::SrcOver);
    }

    #[test]
    fn test_clear() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        // Set a pixel first
        pf.copy_pixel(5, 5, &Rgba8::new(255, 0, 0, 255));
        let p = pf.pixel(5, 5);
        assert_eq!(p.r, 255);

        // Clear it with comp_op clear
        pf.set_comp_op(CompOp::Clear);
        pf.blend_pixel(5, 5, &Rgba8::new(0, 0, 0, 255), 255);
        let p = pf.pixel(5, 5);
        assert_eq!(p.r, 0);
        assert_eq!(p.a, 0);
    }

    #[test]
    fn test_src() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(100, 100, 100, 255));

        pf.set_comp_op(CompOp::Src);
        pf.blend_pixel(0, 0, &Rgba8::new(200, 50, 0, 255), 255);
        let p = pf.pixel(0, 0);
        // With full cover and full alpha, Src just overwrites
        assert_eq!(p.r, 200);
        assert_eq!(p.g, 50);
        assert_eq!(p.b, 0);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_dst_is_noop() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(42, 43, 44, 200));

        pf.set_comp_op(CompOp::Dst);
        pf.blend_pixel(0, 0, &Rgba8::new(255, 255, 255, 255), 255);
        let p = pf.pixel(0, 0);
        assert_eq!(p.r, 42);
        assert_eq!(p.g, 43);
        assert_eq!(p.b, 44);
        assert_eq!(p.a, 200);
    }

    #[test]
    fn test_src_over_opaque() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(100, 100, 100, 255));

        pf.set_comp_op(CompOp::SrcOver);
        pf.blend_pixel(0, 0, &Rgba8::new(200, 50, 25, 255), 255);
        let p = pf.pixel(0, 0);
        // SrcOver with full alpha = complete replacement
        assert_eq!(p.r, 200);
        assert_eq!(p.g, 50);
        assert_eq!(p.b, 25);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_src_over_semitransparent() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(0, 0, 0, 255));

        pf.set_comp_op(CompOp::SrcOver);
        // Blend 50% transparent red over black
        pf.blend_pixel(0, 0, &Rgba8::new(255, 0, 0, 128), 255);
        let p = pf.pixel(0, 0);
        // Should get roughly half red
        assert!(p.r > 100 && p.r < 140, "r={}", p.r);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_multiply_mode() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        // White background
        pf.copy_pixel(0, 0, &Rgba8::new(255, 255, 255, 255));

        pf.set_comp_op(CompOp::Multiply);
        pf.blend_pixel(0, 0, &Rgba8::new(128, 128, 128, 255), 255);
        let p = pf.pixel(0, 0);
        // Multiply of white × 50% gray ≈ 128
        assert!(p.r > 120 && p.r < 140, "r={}", p.r);
    }

    #[test]
    fn test_screen_mode() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        // Black background
        pf.copy_pixel(0, 0, &Rgba8::new(0, 0, 0, 255));

        pf.set_comp_op(CompOp::Screen);
        pf.blend_pixel(0, 0, &Rgba8::new(128, 128, 128, 255), 255);
        let p = pf.pixel(0, 0);
        // Screen of black + gray = gray
        assert!(p.r > 120 && p.r < 140, "r={}", p.r);
    }

    #[test]
    fn test_xor_mode() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        // Fully opaque red
        pf.copy_pixel(0, 0, &Rgba8::new(255, 0, 0, 255));

        pf.set_comp_op(CompOp::Xor);
        // Xor with fully opaque blue → both fully opaque → everything cancels
        pf.blend_pixel(0, 0, &Rgba8::new(0, 0, 255, 255), 255);
        let p = pf.pixel(0, 0);
        // Xor of two fully opaque: Da'=Sa+Da-2*Sa*Da = 1+1-2 = 0
        assert_eq!(p.a, 0);
    }

    #[test]
    fn test_blend_hline() {
        let (_buf, mut ra) = make_buffer(20, 1);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.set_comp_op(CompOp::SrcOver);
        let red = Rgba8::new(255, 0, 0, 255);
        pf.blend_hline(5, 0, 10, &red, 255);
        // Pixel at x=10 should be red
        let p = pf.pixel(10, 0);
        assert_eq!(p.r, 255);
        // Pixel at x=0 should be transparent
        let p = pf.pixel(0, 0);
        assert_eq!(p.a, 0);
    }

    #[test]
    fn test_blend_solid_hspan() {
        let (_buf, mut ra) = make_buffer(10, 1);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.set_comp_op(CompOp::SrcOver);
        let green = Rgba8::new(0, 255, 0, 255);
        let covers = [255u8, 128, 64, 0];
        pf.blend_solid_hspan(0, 0, 4, &green, &covers);
        // Full cover pixel
        let p = pf.pixel(0, 0);
        assert_eq!(p.g, 255);
        // Zero cover pixel — unchanged
        let p = pf.pixel(3, 0);
        assert_eq!(p.g, 0);
    }

    #[test]
    fn test_all_ops_no_panic() {
        let ops = [
            CompOp::Clear,
            CompOp::Src,
            CompOp::Dst,
            CompOp::SrcOver,
            CompOp::DstOver,
            CompOp::SrcIn,
            CompOp::DstIn,
            CompOp::SrcOut,
            CompOp::DstOut,
            CompOp::SrcAtop,
            CompOp::DstAtop,
            CompOp::Xor,
            CompOp::Plus,
            CompOp::Minus,
            CompOp::Multiply,
            CompOp::Screen,
            CompOp::Overlay,
            CompOp::Darken,
            CompOp::Lighten,
            CompOp::ColorDodge,
            CompOp::ColorBurn,
            CompOp::HardLight,
            CompOp::SoftLight,
            CompOp::Difference,
            CompOp::Exclusion,
        ];
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        let c = Rgba8::new(128, 64, 32, 200);
        // Set some background
        pf.copy_pixel(0, 0, &Rgba8::new(100, 150, 200, 180));
        for &op in &ops {
            pf.set_comp_op(op);
            pf.blend_pixel(0, 0, &c, 128);
        }
    }

    #[test]
    fn test_difference_mode() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        // White opaque background
        pf.copy_pixel(0, 0, &Rgba8::new(255, 255, 255, 255));

        pf.set_comp_op(CompOp::Difference);
        pf.blend_pixel(0, 0, &Rgba8::new(255, 255, 255, 255), 255);
        let p = pf.pixel(0, 0);
        // Difference of same colors = black
        assert!(p.r < 5, "r={}", p.r);
        assert!(p.g < 5, "g={}", p.g);
        assert!(p.b < 5, "b={}", p.b);
    }

    #[test]
    fn test_plus_saturates() {
        let (_buf, mut ra) = make_buffer(10, 10);
        let mut pf = PixfmtRgba32CompOp::new(&mut ra);
        pf.copy_pixel(0, 0, &Rgba8::new(200, 200, 200, 255));

        pf.set_comp_op(CompOp::Plus);
        pf.blend_pixel(0, 0, &Rgba8::new(200, 200, 200, 255), 255);
        let p = pf.pixel(0, 0);
        // Should saturate at 255
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 255);
        assert_eq!(p.b, 255);
    }
}

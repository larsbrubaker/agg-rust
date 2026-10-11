//! RGB24 pixel formats with pluggable blenders: premultiplied and gamma.
//!
//! Port of `pixfmt_alpha_blend_rgb<Blender, rendering_buffer>` from
//! `agg_pixfmt_rgb.h` instantiated with `blender_rgb_pre` (C++
//! `pixfmt_rgb24_pre`) and `blender_rgb_gamma` (C++ `pixfmt_rgb24_gamma`).
//! The plain straight-alpha `pixfmt_rgb24` lives in `pixfmt_rgb.rs`; this
//! module carries the generic pixel format so further blenders can plug in.

use crate::basics::CoverType;
use crate::color::Rgba8;
use crate::gamma::GammaLut;
use crate::pixfmt_rgba::PixelFormat;
use crate::rendering_buffer::RowAccessor;

/// Bytes per pixel for RGB24.
const BPP: usize = 3;

/// Blender for a 3-byte RGB pixel. Port of the C++ blender concept used by
/// `pixfmt_alpha_blend_rgb` (`blend_pix` with and without a cover).
pub trait RgbBlender {
    /// Blend a color whose alpha already includes the coverage.
    fn blend_pix(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8);

    /// Blend a color at the given coverage.
    fn blend_pix_cover(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8, cover: CoverType);
}

/// Port of C++ `blender_rgb_pre`: the premultiplied form of Alvy-Ray Smith's
/// compositing function. Color channels are expected premultiplied by alpha;
/// a channel above its alpha saturates at 255 where C++ wraps (see
/// [`Rgba8::prelerp`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct BlenderRgbPre;

impl RgbBlender for BlenderRgbPre {
    #[inline]
    fn blend_pix(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        p[0] = Rgba8::prelerp(p[0], cr, alpha);
        p[1] = Rgba8::prelerp(p[1], cg, alpha);
        p[2] = Rgba8::prelerp(p[2], cb, alpha);
    }

    #[inline]
    fn blend_pix_cover(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8, cover: CoverType) {
        self.blend_pix(
            p,
            Rgba8::mult_cover(cr, cover),
            Rgba8::mult_cover(cg, cover),
            Rgba8::mult_cover(cb, cover),
            Rgba8::mult_cover(alpha, cover),
        );
    }
}

/// Port of C++ `blender_rgb_gamma`: blends in the gamma LUT's linear space,
/// `inv(downscale((dir(c) - dir(p)) * alpha) + dir(p))`.
#[derive(Clone, Copy)]
pub struct BlenderRgbGamma<'g> {
    gamma: &'g GammaLut,
}

impl<'g> BlenderRgbGamma<'g> {
    pub fn new(gamma: &'g GammaLut) -> Self {
        Self { gamma }
    }

    /// The gamma LUT the blender works through.
    pub fn gamma(&self) -> &'g GammaLut {
        self.gamma
    }

    #[inline]
    fn channel(&self, p: u8, c: u8, alpha: u8) -> u8 {
        let d = self.gamma.dir(p) as i32;
        // C++ computes in calc_type (int32u) and narrows to the LUT's int8u
        // argument; the arithmetic shift here agrees with that in the low byte.
        let v = (((self.gamma.dir(c) as i32 - d) * alpha as i32) >> 8) + d;
        self.gamma.inv(v as u8)
    }
}

impl RgbBlender for BlenderRgbGamma<'_> {
    #[inline]
    fn blend_pix(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8) {
        // Divergence from C++: blend_pix with alpha 0 writes inv(dir(p)), and
        // the LUT round trip is lossy at the dark end (gamma 2: dir(10) = 0,
        // inv(0) = 0), so a blend that should change nothing (a translucent
        // color whose cover brings its alpha to 0) turns dark pixels black.
        // A zero alpha is a no-op here. Pinned by
        // `tests/agg_sharp/blender_gamma_bgr_tests.rs::cover_that_zeroes_the_alpha_leaves_the_pixel`.
        if alpha == 0 {
            return;
        }
        p[0] = self.channel(p[0], cr, alpha);
        p[1] = self.channel(p[1], cg, alpha);
        p[2] = self.channel(p[2], cb, alpha);
    }

    #[inline]
    fn blend_pix_cover(&self, p: &mut [u8], cr: u8, cg: u8, cb: u8, alpha: u8, cover: CoverType) {
        self.blend_pix(p, cr, cg, cb, Rgba8::mult_cover(alpha, cover));
    }
}

/// Port of C++ `pixfmt_alpha_blend_rgb<Blender, rendering_buffer>` for
/// 3-byte RGB pixels (component order R, G, B).
pub struct PixfmtAlphaBlendRgb<'a, B: RgbBlender> {
    rbuf: &'a mut RowAccessor,
    blender: B,
}

/// C++ `pixfmt_rgb24_pre`.
pub type PixfmtRgb24Pre<'a> = PixfmtAlphaBlendRgb<'a, BlenderRgbPre>;

/// C++ `pixfmt_rgb24_gamma<gamma_lut<>>`.
pub type PixfmtRgb24Gamma<'a, 'g> = PixfmtAlphaBlendRgb<'a, BlenderRgbGamma<'g>>;

impl<'a> PixfmtAlphaBlendRgb<'a, BlenderRgbPre> {
    pub fn new(rbuf: &'a mut RowAccessor) -> Self {
        Self::new_with_blender(rbuf, BlenderRgbPre)
    }
}

impl<'a, 'g> PixfmtAlphaBlendRgb<'a, BlenderRgbGamma<'g>> {
    /// Port of the C++ `pixfmt_rgb24_gamma(rb, g)` constructor.
    pub fn new_with_gamma(rbuf: &'a mut RowAccessor, gamma: &'g GammaLut) -> Self {
        Self::new_with_blender(rbuf, BlenderRgbGamma::new(gamma))
    }
}

impl<'a, B: RgbBlender> PixfmtAlphaBlendRgb<'a, B> {
    pub fn new_with_blender(rbuf: &'a mut RowAccessor, blender: B) -> Self {
        Self { rbuf, blender }
    }

    /// The blender (C++ `blender()`).
    pub fn blender(&self) -> &B {
        &self.blender
    }

    #[inline]
    fn row(&self, y: i32) -> &[u8] {
        // SAFETY: `row_ptr(y)` points at a row of `width * BPP` bytes owned by
        // the attached buffer, which outlives `self.rbuf`.
        unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts(ptr, self.rbuf.width() as usize * BPP)
        }
    }

    #[inline]
    fn row_mut(&mut self, y: i32) -> &mut [u8] {
        // SAFETY: as in `row`; `&mut self` makes the slice unique.
        unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        }
    }

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &Rgba8) {
        for y in 0..self.rbuf.height() as i32 {
            for px in self.row_mut(y).as_chunks_mut::<BPP>().0 {
                set(px, c);
            }
        }
    }

    /// Port of C++ `copy_or_blend_pix(p, c, cover)`.
    #[inline]
    fn copy_or_blend_cover(blender: &B, p: &mut [u8], c: &Rgba8, cover: CoverType) {
        if c.a != 0 {
            if c.a == 255 && cover == 255 {
                set(p, c);
            } else {
                blender.blend_pix_cover(p, c.r, c.g, c.b, c.a, cover);
            }
        }
    }

    /// Port of C++ `copy_or_blend_pix(p, c)`.
    #[inline]
    fn copy_or_blend(blender: &B, p: &mut [u8], c: &Rgba8) {
        if c.a != 0 {
            if c.a == 255 {
                set(p, c);
            } else {
                blender.blend_pix(p, c.r, c.g, c.b, c.a);
            }
        }
    }

    /// Splits the borrow so the blender can be used while a row is mutated.
    #[inline]
    fn parts(&mut self, y: i32) -> (&B, &mut [u8]) {
        // SAFETY: as in `row_mut`; the blender is a separate field.
        let row = unsafe {
            let ptr = self.rbuf.row_ptr(y);
            std::slice::from_raw_parts_mut(ptr, self.rbuf.width() as usize * BPP)
        };
        (&self.blender, row)
    }
}

#[inline]
fn set(p: &mut [u8], c: &Rgba8) {
    p[0] = c.r;
    p[1] = c.g;
    p[2] = c.b;
}

#[inline]
fn px(row: &mut [u8], x: usize) -> &mut [u8] {
    &mut row[x * BPP..x * BPP + BPP]
}

impl<'a, B: RgbBlender> PixelFormat for PixfmtAlphaBlendRgb<'a, B> {
    type ColorType = Rgba8;

    fn width(&self) -> u32 {
        self.rbuf.width()
    }

    fn height(&self) -> u32 {
        self.rbuf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let row = self.row(y);
        let off = x as usize * BPP;
        Rgba8::new(
            row[off] as u32,
            row[off + 1] as u32,
            row[off + 2] as u32,
            255,
        )
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba8) {
        set(px(self.row_mut(y), x as usize), c);
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8) {
        let row = self.row_mut(y);
        for i in 0..len as usize {
            set(px(row, x as usize + i), c);
        }
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba8, cover: CoverType) {
        let (blender, row) = self.parts(y);
        Self::copy_or_blend_cover(blender, px(row, x as usize), c, cover);
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, cover: CoverType) {
        if c.a == 0 {
            return;
        }
        let (blender, row) = self.parts(y);
        for i in 0..len as usize {
            let p = px(row, x as usize + i);
            if c.a == 255 && cover == 255 {
                set(p, c);
            } else {
                blender.blend_pix_cover(p, c.r, c.g, c.b, c.a, cover);
            }
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba8, covers: &[CoverType]) {
        if c.a == 0 {
            return;
        }
        let (blender, row) = self.parts(y);
        for (i, &cover) in covers.iter().enumerate().take(len as usize) {
            let p = px(row, x as usize + i);
            if c.a == 255 && cover == 255 {
                set(p, c);
            } else {
                blender.blend_pix_cover(p, c.r, c.g, c.b, c.a, cover);
            }
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
        let (blender, row) = self.parts(y);
        for (i, c) in colors.iter().enumerate().take(len as usize) {
            let p = px(row, x as usize + i);
            if !covers.is_empty() {
                Self::copy_or_blend_cover(blender, p, c, covers[i]);
            } else if cover == 255 {
                Self::copy_or_blend(blender, p, c);
            } else {
                Self::copy_or_blend_cover(blender, p, c, cover);
            }
        }
    }
}

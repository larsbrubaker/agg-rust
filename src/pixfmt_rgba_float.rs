//! Floating-point RGBA pixel formats.
//!
//! Port of C++ `pixfmt_alpha_blend_rgba<Blender, rendering_buffer>` over
//! `rgba32` from `agg_pixfmt_rgba.h` (`pixfmt_rgba128`, `pixfmt_bgra128`,
//! `pixfmt_rgba128_pre`, ...), with its two blenders `blender_rgba` and
//! `blender_rgba_pre`. Pixels are four premultiplied `f32` channels.
//!
//! The byte-based [`RowAccessor`](crate::rendering_buffer::RowAccessor) does
//! not fit a float buffer, so these formats borrow a `&mut [f32]` with a width,
//! height and stride (in floats) instead of reinterpreting bytes.
//!
//! Float blending is not clamped, as in C++: the premultiplied blender can
//! push a channel above 1 when it is fed a color that is not premultiplied.
//! Clamping happens only where a float converts to a byte (`Rgba32::to_rgba8`).
//!
//! [`PixfmtCompOpRgbaF32`] is the float counterpart of `comp_op`'s
//! `PixfmtRgba32CompOp`: C++ `pixfmt_custom_blend_rgba` over
//! `comp_op_adaptor_rgba<rgba32>`, sharing that module's SVG operators.
//!
//! The 8-bit RGBA formats live in `pixfmt_rgba`, which also holds the
//! [`PixelFormat`] and [`PixfmtBlendFrom`] traits implemented here.

use std::marker::PhantomData;

use crate::basics::{CoverType, COVER_MASK};
use crate::color::{OrderAbgr, OrderArgb, OrderBgra, OrderRgba};
use crate::color_rgba32::Rgba32;
use crate::comp_op::{comp_op_blend, CompOp};
use crate::pixfmt_rgba::{PixelFormat, PixfmtBlendFrom};

const PIX_STEP: usize = 4;

/// Panics unless a buffer of `len` floats holds `height` rows of `width`
/// pixels `stride` floats apart.
fn check_buffer(len: usize, width: u32, height: u32, stride: usize) {
    let row = width as usize * PIX_STEP;
    assert!(
        stride >= row,
        "stride {stride} shorter than a row of {row} floats"
    );
    if height > 0 {
        let needed = (height as usize - 1) * stride + row;
        assert!(len >= needed, "buffer of {len} floats cannot hold {needed}");
    }
}

// ============================================================================
// Component order
// ============================================================================

/// Channel positions of a four-channel pixel (C++ `order_rgba` and friends as
/// a template parameter).
pub trait ComponentOrderRgba {
    const R: usize;
    const G: usize;
    const B: usize;
    const A: usize;
}

macro_rules! impl_order {
    ($t:ty) => {
        impl ComponentOrderRgba for $t {
            const R: usize = <$t>::R;
            const G: usize = <$t>::G;
            const B: usize = <$t>::B;
            const A: usize = <$t>::A;
        }
    };
}
impl_order!(OrderRgba);
impl_order!(OrderArgb);
impl_order!(OrderAbgr);
impl_order!(OrderBgra);

// ============================================================================
// Blenders
// ============================================================================

/// A C++ `rgba32` blender: one premultiplied float pixel at a time. The
/// covered and uncovered forms stay apart because C++ calls the uncovered one
/// at full cover, and `a * 255 / 255` in `f32` is not always `a`.
pub trait BlenderRgbaF32 {
    /// C++ `blend_pix(p, cr, cg, cb, alpha, cover)`.
    fn blend_pix_cover<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32, cover: CoverType);
    /// C++ `blend_pix(p, cr, cg, cb, alpha)`: the color at full cover.
    fn blend_pix<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32);
}

/// C++ `blender_rgba<rgba32, Order>`: a straight color into a premultiplied
/// buffer (the non-premultiplied form of Alvy-Ray Smith's compositing
/// function, the initial premultiply and final demultiply omitted). Color
/// channels `lerp` toward the source by its alpha; alpha `prelerp`s.
pub struct BlenderRgbaF32Straight;

impl BlenderRgbaF32Straight {
    fn blend<O: ComponentOrderRgba>(p: &mut [f32], cr: f32, cg: f32, cb: f32, alpha: f32) {
        p[O::R] = Rgba32::lerp(p[O::R], cr, alpha);
        p[O::G] = Rgba32::lerp(p[O::G], cg, alpha);
        p[O::B] = Rgba32::lerp(p[O::B], cb, alpha);
        p[O::A] = Rgba32::prelerp(p[O::A], alpha, alpha);
    }
}

impl BlenderRgbaF32 for BlenderRgbaF32Straight {
    fn blend_pix_cover<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32, cover: CoverType) {
        Self::blend::<O>(p, c.r, c.g, c.b, Rgba32::mult_cover(c.a, cover));
    }

    fn blend_pix<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32) {
        Self::blend::<O>(p, c.r, c.g, c.b, c.a);
    }
}

/// C++ `blender_rgba_pre<rgba32, Order>`: a premultiplied color into a
/// premultiplied buffer (src-over). A cover scales all four channels before
/// the `prelerp`.
pub struct BlenderRgbaF32Pre;

impl BlenderRgbaF32Pre {
    fn blend<O: ComponentOrderRgba>(p: &mut [f32], cr: f32, cg: f32, cb: f32, alpha: f32) {
        p[O::R] = Rgba32::prelerp(p[O::R], cr, alpha);
        p[O::G] = Rgba32::prelerp(p[O::G], cg, alpha);
        p[O::B] = Rgba32::prelerp(p[O::B], cb, alpha);
        p[O::A] = Rgba32::prelerp(p[O::A], alpha, alpha);
    }
}

impl BlenderRgbaF32 for BlenderRgbaF32Pre {
    fn blend_pix_cover<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32, cover: CoverType) {
        Self::blend::<O>(
            p,
            Rgba32::mult_cover(c.r, cover),
            Rgba32::mult_cover(c.g, cover),
            Rgba32::mult_cover(c.b, cover),
            Rgba32::mult_cover(c.a, cover),
        );
    }

    fn blend_pix<O: ComponentOrderRgba>(p: &mut [f32], c: &Rgba32) {
        Self::blend::<O>(p, c.r, c.g, c.b, c.a);
    }
}

// ============================================================================
// PixfmtAlphaBlendRgbaF32
// ============================================================================

/// C++ `pixfmt_alpha_blend_rgba<Blender, rendering_buffer>` over `rgba32`:
/// premultiplied float pixels in a borrowed `f32` buffer.
///
/// Row `y` starts at float `y * stride`; each pixel is four floats in the
/// order `O`. As in C++, the span methods do not clip (that is
/// `RendererBase`'s job) and expect `len >= 1`.
pub struct PixfmtAlphaBlendRgbaF32<'a, B: BlenderRgbaF32, O: ComponentOrderRgba> {
    buf: &'a mut [f32],
    width: u32,
    height: u32,
    stride: usize,
    _marker: PhantomData<(B, O)>,
}

/// C++ `pixfmt_rgba128`.
pub type PixfmtRgba128<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Straight, OrderRgba>;
/// C++ `pixfmt_argb128`.
pub type PixfmtArgb128<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Straight, OrderArgb>;
/// C++ `pixfmt_abgr128`.
pub type PixfmtAbgr128<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Straight, OrderAbgr>;
/// C++ `pixfmt_bgra128`.
pub type PixfmtBgra128<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Straight, OrderBgra>;
/// C++ `pixfmt_rgba128_pre`.
pub type PixfmtRgba128Pre<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Pre, OrderRgba>;
/// C++ `pixfmt_argb128_pre`.
pub type PixfmtArgb128Pre<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Pre, OrderArgb>;
/// C++ `pixfmt_abgr128_pre`.
pub type PixfmtAbgr128Pre<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Pre, OrderAbgr>;
/// C++ `pixfmt_bgra128_pre`.
pub type PixfmtBgra128Pre<'a> = PixfmtAlphaBlendRgbaF32<'a, BlenderRgbaF32Pre, OrderBgra>;

impl<'a, B: BlenderRgbaF32, O: ComponentOrderRgba> PixfmtAlphaBlendRgbaF32<'a, B, O> {
    /// Attach to `buf`: `height` rows of `width` pixels, row `y` starting at
    /// float `y * stride`.
    ///
    /// # Panics
    /// If `stride` is shorter than a row or `buf` cannot hold every row.
    pub fn new(buf: &'a mut [f32], width: u32, height: u32, stride: usize) -> Self {
        check_buffer(buf.len(), width, height, stride);
        Self {
            buf,
            width,
            height,
            stride,
            _marker: PhantomData,
        }
    }

    /// Stride in floats.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// The whole buffer.
    pub fn buf(&self) -> &[f32] {
        self.buf
    }

    #[inline]
    fn offset(&self, x: i32, y: i32) -> usize {
        y as usize * self.stride + x as usize * PIX_STEP
    }

    #[inline]
    fn pix_mut(&mut self, x: i32, y: i32) -> &mut [f32] {
        let o = self.offset(x, y);
        &mut self.buf[o..o + PIX_STEP]
    }

    #[inline]
    fn set(p: &mut [f32], c: &Rgba32) {
        p[O::R] = c.r;
        p[O::G] = c.g;
        p[O::B] = c.b;
        p[O::A] = c.a;
    }

    #[inline]
    fn get(p: &[f32]) -> Rgba32 {
        Rgba32::new(p[O::R], p[O::G], p[O::B], p[O::A])
    }

    /// C++ `copy_or_blend_pix(p, c, cover)`: nothing for a transparent
    /// color, a copy when opaque at full cover.
    #[inline]
    fn copy_or_blend_pix_cover(p: &mut [f32], c: &Rgba32, cover: CoverType) {
        if !c.is_transparent() {
            if c.is_opaque() && cover == COVER_MASK as CoverType {
                Self::set(p, c);
            } else {
                B::blend_pix_cover::<O>(p, c, cover);
            }
        }
    }

    /// C++ `copy_or_blend_pix(p, c)`.
    #[inline]
    fn copy_or_blend_pix(p: &mut [f32], c: &Rgba32) {
        if !c.is_transparent() {
            if c.is_opaque() {
                Self::set(p, c);
            } else {
                B::blend_pix::<O>(p, c);
            }
        }
    }

    /// C++ `blend_vline`.
    pub fn blend_vline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, cover: CoverType) {
        if c.is_transparent() {
            return;
        }
        for i in 0..len as i32 {
            let p = self.pix_mut(x, y + i);
            if c.is_opaque() && cover == COVER_MASK as CoverType {
                Self::set(p, c);
            } else if cover == COVER_MASK as CoverType {
                B::blend_pix::<O>(p, c);
            } else {
                B::blend_pix_cover::<O>(p, c, cover);
            }
        }
    }

    /// C++ `blend_solid_vspan`.
    pub fn blend_solid_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &Rgba32,
        covers: &[CoverType],
    ) {
        if c.is_transparent() {
            return;
        }
        for (i, &cover) in covers.iter().take(len as usize).enumerate() {
            let p = self.pix_mut(x, y + i as i32);
            if c.is_opaque() && cover == COVER_MASK as CoverType {
                Self::set(p, c);
            } else {
                B::blend_pix_cover::<O>(p, c, cover);
            }
        }
    }

    /// C++ `copy_color_hspan`.
    pub fn copy_color_hspan(&mut self, x: i32, y: i32, len: u32, colors: &[Rgba32]) {
        for (i, c) in colors.iter().take(len as usize).enumerate() {
            Self::set(self.pix_mut(x + i as i32, y), c);
        }
    }

    /// C++ `copy_color_vspan`.
    pub fn copy_color_vspan(&mut self, x: i32, y: i32, len: u32, colors: &[Rgba32]) {
        for (i, c) in colors.iter().take(len as usize).enumerate() {
            Self::set(self.pix_mut(x, y + i as i32), c);
        }
    }

    /// C++ `blend_color_vspan`: per-pixel covers when `covers` is non-empty,
    /// else `cover` for all.
    pub fn blend_color_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba32],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        for i in 0..len as usize {
            let p = self.pix_mut(x, y + i as i32);
            if !covers.is_empty() {
                Self::copy_or_blend_pix_cover(p, &colors[i], covers[i]);
            } else if cover == COVER_MASK as CoverType {
                Self::copy_or_blend_pix(p, &colors[i]);
            } else {
                Self::copy_or_blend_pix_cover(p, &colors[i], cover);
            }
        }
    }

    fn for_each_pixel(&mut self, f: impl Fn(&mut Rgba32)) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let p = self.pix_mut(x, y);
                let mut c = Self::get(p);
                f(&mut c);
                Self::set(p, &c);
            }
        }
    }

    /// C++ `premultiply`: every pixel through `rgba32::premultiply`.
    pub fn premultiply(&mut self) {
        self.for_each_pixel(|c| {
            c.premultiply();
        });
    }

    /// C++ `demultiply`: every pixel through `rgba32::demultiply`.
    pub fn demultiply(&mut self) {
        self.for_each_pixel(|c| {
            c.demultiply();
        });
    }
}

impl<B: BlenderRgbaF32, O: ComponentOrderRgba> PixelFormat for PixfmtAlphaBlendRgbaF32<'_, B, O> {
    type ColorType = Rgba32;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    /// C++ `blend_pixel`: `copy_or_blend_pix` with the cover.
    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba32, cover: CoverType) {
        Self::copy_or_blend_pix_cover(self.pix_mut(x, y), c, cover);
    }

    /// C++ `blend_hline`: a copy when opaque at full cover, the uncovered
    /// blend at full cover, the covered blend otherwise.
    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, cover: CoverType) {
        if c.is_transparent() {
            return;
        }
        for i in 0..len as i32 {
            let p = self.pix_mut(x + i, y);
            if c.is_opaque() && cover == COVER_MASK as CoverType {
                Self::set(p, c);
            } else if cover == COVER_MASK as CoverType {
                B::blend_pix::<O>(p, c);
            } else {
                B::blend_pix_cover::<O>(p, c, cover);
            }
        }
    }

    /// C++ `blend_solid_hspan`. Unlike `blend_hline`, a full cover on a
    /// translucent color still goes through the covered blend.
    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, covers: &[CoverType]) {
        if c.is_transparent() {
            return;
        }
        for (i, &cover) in covers.iter().take(len as usize).enumerate() {
            let p = self.pix_mut(x + i as i32, y);
            if c.is_opaque() && cover == COVER_MASK as CoverType {
                Self::set(p, c);
            } else {
                B::blend_pix_cover::<O>(p, c, cover);
            }
        }
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32) {
        for i in 0..len as i32 {
            Self::set(self.pix_mut(x + i, y), c);
        }
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba32) {
        Self::set(self.pix_mut(x, y), c);
    }

    /// C++ `blend_color_hspan`: per-pixel covers when `covers` is non-empty,
    /// else `cover` for all (the uncovered blend at full cover).
    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba32],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        for i in 0..len as usize {
            let p = self.pix_mut(x + i as i32, y);
            if !covers.is_empty() {
                Self::copy_or_blend_pix_cover(p, &colors[i], covers[i]);
            } else if cover == COVER_MASK as CoverType {
                Self::copy_or_blend_pix(p, &colors[i]);
            } else {
                Self::copy_or_blend_pix_cover(p, &colors[i], cover);
            }
        }
    }

    /// C++ `pixel`: the stored (premultiplied) channels.
    fn pixel(&self, x: i32, y: i32) -> Rgba32 {
        let o = self.offset(x, y);
        Self::get(&self.buf[o..o + PIX_STEP])
    }
}

impl<B, O, B2, O2> PixfmtBlendFrom<PixfmtAlphaBlendRgbaF32<'_, B2, O2>>
    for PixfmtAlphaBlendRgbaF32<'_, B, O>
where
    B: BlenderRgbaF32,
    O: ComponentOrderRgba,
    B2: BlenderRgbaF32,
    O2: ComponentOrderRgba,
{
    /// C++ `blend_from(from, xdst, ydst, xsrc, ysrc, len, cover)`: a row of
    /// source pixels blended as colors, walking backwards when shifting right
    /// so a copy within one buffer would not read what it just wrote.
    fn blend_from(
        &mut self,
        from: &PixfmtAlphaBlendRgbaF32<'_, B2, O2>,
        xdst: i32,
        ydst: i32,
        xsrc: i32,
        ysrc: i32,
        len: u32,
        cover: CoverType,
    ) {
        let backwards = xdst > xsrc;
        for k in 0..len as i32 {
            let i = if backwards { len as i32 - 1 - k } else { k };
            let c = from.pixel(xsrc + i, ysrc);
            let p = self.pix_mut(xdst + i, ydst);
            if cover == COVER_MASK as CoverType {
                Self::copy_or_blend_pix(p, &c);
            } else {
                Self::copy_or_blend_pix_cover(p, &c, cover);
            }
        }
    }
}

// ============================================================================
// PixfmtCompOpRgbaF32
// ============================================================================

/// C++ `pixfmt_custom_blend_rgba<comp_op_adaptor_rgba<rgba32, Order>,
/// rendering_buffer>`: every blend premultiplies the straight source color in
/// `f32` and composites it with a runtime-selectable SVG operator. Unlike
/// [`PixfmtAlphaBlendRgbaF32`] there is no shortcut for opaque or transparent
/// colors: an operator such as clear or src changes the destination even
/// where the source is transparent.
///
/// The operators are shared with the 8-bit [`PixfmtRgba32CompOp`]
/// (`comp_op.rs`): they work in `f64` and narrow back to `f32` with no
/// clamping beyond each operator's own `clip`.
///
/// [`PixfmtRgba32CompOp`]: crate::comp_op::PixfmtRgba32CompOp
pub struct PixfmtCompOpRgbaF32<'a, O: ComponentOrderRgba> {
    buf: &'a mut [f32],
    width: u32,
    height: u32,
    stride: usize,
    comp_op: CompOp,
    _marker: PhantomData<O>,
}

/// C++ `pixfmt_custom_blend_rgba<comp_op_adaptor_rgba<rgba32, order_rgba>>`.
pub type PixfmtRgba128CompOp<'a> = PixfmtCompOpRgbaF32<'a, OrderRgba>;
/// C++ `pixfmt_custom_blend_rgba<comp_op_adaptor_rgba<rgba32, order_bgra>>`.
pub type PixfmtBgra128CompOp<'a> = PixfmtCompOpRgbaF32<'a, OrderBgra>;

impl<'a, O: ComponentOrderRgba> PixfmtCompOpRgbaF32<'a, O> {
    /// Attach to `buf` as [`PixfmtAlphaBlendRgbaF32::new`] does, compositing
    /// with `op`.
    ///
    /// # Panics
    /// If `stride` is shorter than a row or `buf` cannot hold every row.
    pub fn new(buf: &'a mut [f32], width: u32, height: u32, stride: usize, op: CompOp) -> Self {
        check_buffer(buf.len(), width, height, stride);
        Self {
            buf,
            width,
            height,
            stride,
            comp_op: op,
            _marker: PhantomData,
        }
    }

    /// C++ `comp_op()`: the operator every blend uses.
    pub fn comp_op(&self) -> CompOp {
        self.comp_op
    }

    /// C++ `comp_op(op)`.
    pub fn set_comp_op(&mut self, op: CompOp) {
        self.comp_op = op;
    }

    /// Stride in floats.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// The whole buffer.
    pub fn buf(&self) -> &[f32] {
        self.buf
    }

    #[inline]
    fn offset(&self, x: i32, y: i32) -> usize {
        y as usize * self.stride + x as usize * PIX_STEP
    }

    /// C++ `blend_pix(p, c, cover)`: `comp_op_adaptor_rgba::blend_pix` on the
    /// pixel's channels, gathered into RGBA order and scattered back.
    #[inline]
    fn blend_pix(&mut self, x: i32, y: i32, c: &Rgba32, cover: CoverType) {
        let o = self.offset(x, y);
        let p = &mut self.buf[o..o + PIX_STEP];
        let mut px = [p[O::R], p[O::G], p[O::B], p[O::A]];
        comp_op_blend(self.comp_op, &mut px, c.r, c.g, c.b, c.a, cover);
        p[O::R] = px[0];
        p[O::G] = px[1];
        p[O::B] = px[2];
        p[O::A] = px[3];
    }

    #[inline]
    fn set(&mut self, x: i32, y: i32, c: &Rgba32) {
        let o = self.offset(x, y);
        let p = &mut self.buf[o..o + PIX_STEP];
        p[O::R] = c.r;
        p[O::G] = c.g;
        p[O::B] = c.b;
        p[O::A] = c.a;
    }

    /// C++ `blend_vline`.
    pub fn blend_vline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, cover: CoverType) {
        for i in 0..len as i32 {
            self.blend_pix(x, y + i, c, cover);
        }
    }

    /// C++ `blend_solid_vspan`.
    pub fn blend_solid_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &Rgba32,
        covers: &[CoverType],
    ) {
        for (i, &cover) in covers.iter().take(len as usize).enumerate() {
            self.blend_pix(x, y + i as i32, c, cover);
        }
    }

    /// C++ `blend_color_vspan`: per-pixel covers when `covers` is non-empty,
    /// else `cover` for all.
    pub fn blend_color_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba32],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        for (i, c) in colors.iter().take(len as usize).enumerate() {
            let cv = if covers.is_empty() { cover } else { covers[i] };
            self.blend_pix(x, y + i as i32, c, cv);
        }
    }
}

impl<O: ComponentOrderRgba> PixelFormat for PixfmtCompOpRgbaF32<'_, O> {
    type ColorType = Rgba32;

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &Rgba32, cover: CoverType) {
        self.blend_pix(x, y, c, cover);
    }

    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, cover: CoverType) {
        for i in 0..len as i32 {
            self.blend_pix(x + i, y, c, cover);
        }
    }

    fn blend_solid_hspan(&mut self, x: i32, y: i32, len: u32, c: &Rgba32, covers: &[CoverType]) {
        for (i, &cover) in covers.iter().take(len as usize).enumerate() {
            self.blend_pix(x + i as i32, y, c, cover);
        }
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &Rgba32) {
        for i in 0..len as i32 {
            self.set(x + i, y, c);
        }
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &Rgba32) {
        self.set(x, y, c);
    }

    /// C++ `blend_color_hspan`: `covers ? *covers++ : cover` per pixel.
    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[Rgba32],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        for (i, c) in colors.iter().take(len as usize).enumerate() {
            let cv = if covers.is_empty() { cover } else { covers[i] };
            self.blend_pix(x + i as i32, y, c, cv);
        }
    }

    fn pixel(&self, x: i32, y: i32) -> Rgba32 {
        let o = self.offset(x, y);
        let p = &self.buf[o..o + PIX_STEP];
        Rgba32::new(p[O::R], p[O::G], p[O::B], p[O::A])
    }
}

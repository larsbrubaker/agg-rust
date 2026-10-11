//! Pixel format adaptor that modulates every write by an alpha mask.
//!
//! Port of `agg_pixfmt_amask_adaptor.h`. Wraps any [`PixelFormat`] and an
//! [`AlphaMask`] (`alpha_mask_u8` and its no-clip variant): every copy turns
//! into a blend whose coverage comes from the mask, and every blend has the
//! mask combined into its coverage. Wrap the adaptor in a `RendererBase` to
//! clip, as C++ `renderer_base<pixfmt_amask_adaptor<...>>` does.

use crate::alpha_mask_u8::AlphaMask;
use crate::basics::{CoverType, COVER_FULL};
use crate::pixfmt_rgba::PixelFormat;

/// C++ `span_extra_tail`: the span grows by this much past the length that
/// outgrew it, so slightly longer spans don't reallocate again.
const SPAN_EXTRA_TAIL: usize = 256;

/// A pixel format that routes every write through an alpha mask.
///
/// Port of C++ `pixfmt_amask_adaptor<PixFmt, AlphaMask>`. C++ holds pointers
/// to the pixel format and mask; here the adaptor owns both (pixel formats and
/// `AlphaMaskU8` are themselves thin borrows of their buffers), with
/// [`attach_pixfmt`](Self::attach_pixfmt) / [`attach_alpha_mask`](Self::attach_alpha_mask)
/// to swap them. C++ `copy_from` and the sub-rectangle `attach_pixfmt` forward
/// to the wrapped format unchanged; reach them through [`pixf_mut`](Self::pixf_mut).
///
/// The covers a caller passes are copied into the adaptor's own span before
/// the mask is multiplied in, so the caller's array is never modified.
pub struct PixfmtAmaskAdaptor<PF: PixelFormat, AM: AlphaMask> {
    pixf: PF,
    mask: AM,
    span: Vec<CoverType>,
}

impl<PF: PixelFormat, AM: AlphaMask> PixfmtAmaskAdaptor<PF, AM> {
    pub fn new(pixf: PF, mask: AM) -> Self {
        Self {
            pixf,
            mask,
            span: Vec::new(),
        }
    }

    /// Replaces the wrapped pixel format, returning the previous one.
    pub fn attach_pixfmt(&mut self, pixf: PF) -> PF {
        std::mem::replace(&mut self.pixf, pixf)
    }

    /// Replaces the alpha mask, returning the previous one.
    pub fn attach_alpha_mask(&mut self, mask: AM) -> AM {
        std::mem::replace(&mut self.mask, mask)
    }

    pub fn pixf(&self) -> &PF {
        &self.pixf
    }

    pub fn pixf_mut(&mut self) -> &mut PF {
        &mut self.pixf
    }

    pub fn mask(&self) -> &AM {
        &self.mask
    }

    /// Consumes the adaptor, returning the pixel format and the mask.
    pub fn into_parts(self) -> (PF, AM) {
        (self.pixf, self.mask)
    }

    fn realloc_span(&mut self, len: usize) {
        if len > self.span.len() {
            self.span.resize(len + SPAN_EXTRA_TAIL, 0);
        }
    }

    fn init_span_full(&mut self, len: usize) {
        self.realloc_span(len);
        self.span[..len].fill(COVER_FULL);
    }

    fn init_span_covers(&mut self, len: usize, covers: &[CoverType]) {
        self.realloc_span(len);
        self.span[..len].copy_from_slice(&covers[..len]);
    }

    /// Blends `c` down the column at (x, y) with the per-pixel coverage in
    /// the span. C++ calls the wrapped format's `blend_solid_vspan`, which
    /// every AGG pixel format implements as one `copy_or_blend_pix` per row:
    /// this format's `blend_pixel`.
    fn blend_span_vertically(&mut self, x: i32, y: i32, len: usize, c: &PF::ColorType) {
        for (i, &cover) in self.span[..len].iter().enumerate() {
            self.pixf.blend_pixel(x, y + i as i32, c, cover);
        }
    }

    /// Blends `colors` down the column at (x, y) with the span's coverage,
    /// C++ `blend_color_vspan` of the wrapped format with covers: each row's
    /// coverage is its own (the uniform `cover` is ignored when covers exist).
    fn blend_colors_vertically(&mut self, x: i32, y: i32, len: usize, colors: &[PF::ColorType]) {
        for (i, (color, &cover)) in colors[..len].iter().zip(&self.span[..len]).enumerate() {
            self.pixf.blend_pixel(x, y + i as i32, color, cover);
        }
    }

    /// Copies `c` into `len` pixels down from (x, y), as blends at the
    /// mask's coverage.
    pub fn copy_vline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType) {
        let len = len as usize;
        self.realloc_span(len);
        self.mask.fill_vspan(x, y, &mut self.span[..len]);
        self.blend_span_vertically(x, y, len, c);
    }

    /// Blends `c` into `len` pixels down from (x, y). As in C++, the `cover`
    /// argument is not used: the line is drawn at the mask's coverage.
    pub fn blend_vline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType, _cover: CoverType) {
        let len = len as usize;
        self.init_span_full(len);
        self.mask.combine_vspan(x, y, &mut self.span[..len]);
        self.blend_span_vertically(x, y, len, c);
    }

    /// Blends `c` down the column at (x, y) with per-pixel `covers`, each
    /// combined with the mask.
    pub fn blend_solid_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &PF::ColorType,
        covers: &[CoverType],
    ) {
        let len = len as usize;
        self.init_span_covers(len, covers);
        self.mask.combine_vspan(x, y, &mut self.span[..len]);
        self.blend_span_vertically(x, y, len, c);
    }

    /// Copies `colors` into `len` pixels right of (x, y), as blends at the
    /// mask's coverage.
    pub fn copy_color_hspan(&mut self, x: i32, y: i32, len: u32, colors: &[PF::ColorType]) {
        let n = len as usize;
        self.realloc_span(n);
        self.mask.fill_hspan(x, y, &mut self.span[..n]);
        self.pixf
            .blend_color_hspan(x, y, len, colors, &self.span[..n], COVER_FULL);
    }

    /// Copies `colors` into `len` pixels down from (x, y), as blends at the
    /// mask's coverage.
    pub fn copy_color_vspan(&mut self, x: i32, y: i32, len: u32, colors: &[PF::ColorType]) {
        let n = len as usize;
        self.realloc_span(n);
        self.mask.fill_vspan(x, y, &mut self.span[..n]);
        self.blend_colors_vertically(x, y, n, colors);
    }

    /// Blends `colors` down the column at (x, y). With `covers` (non-empty)
    /// each is combined with the mask; without, the mask alone is the
    /// coverage. Either way the per-pixel span decides, as in C++, which
    /// passes the span to the wrapped format together with `cover`.
    pub fn blend_color_vspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[PF::ColorType],
        covers: &[CoverType],
        _cover: CoverType,
    ) {
        let n = len as usize;
        if !covers.is_empty() {
            self.init_span_covers(n, covers);
            self.mask.combine_vspan(x, y, &mut self.span[..n]);
        } else {
            self.realloc_span(n);
            self.mask.fill_vspan(x, y, &mut self.span[..n]);
        }
        self.blend_colors_vertically(x, y, n, colors);
    }
}

impl<PF: PixelFormat, AM: AlphaMask> PixelFormat for PixfmtAmaskAdaptor<PF, AM> {
    type ColorType = PF::ColorType;

    fn width(&self) -> u32 {
        self.pixf.width()
    }

    fn height(&self) -> u32 {
        self.pixf.height()
    }

    fn pixel(&self, x: i32, y: i32) -> PF::ColorType {
        self.pixf.pixel(x, y)
    }

    fn copy_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType) {
        let cover = self.mask.pixel(x, y);
        self.pixf.blend_pixel(x, y, c, cover);
    }

    fn blend_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType, cover: CoverType) {
        let cover = self.mask.combine_pixel(x, y, cover);
        self.pixf.blend_pixel(x, y, c, cover);
    }

    fn copy_hline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType) {
        let n = len as usize;
        self.realloc_span(n);
        self.mask.fill_hspan(x, y, &mut self.span[..n]);
        self.pixf.blend_solid_hspan(x, y, len, c, &self.span[..n]);
    }

    /// As in C++, the `cover` argument is not used: the line is drawn at the
    /// mask's coverage.
    fn blend_hline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType, _cover: CoverType) {
        let n = len as usize;
        self.init_span_full(n);
        self.mask.combine_hspan(x, y, &mut self.span[..n]);
        self.pixf.blend_solid_hspan(x, y, len, c, &self.span[..n]);
    }

    fn blend_solid_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        c: &PF::ColorType,
        covers: &[CoverType],
    ) {
        let n = len as usize;
        self.init_span_covers(n, covers);
        self.mask.combine_hspan(x, y, &mut self.span[..n]);
        self.pixf.blend_solid_hspan(x, y, len, c, &self.span[..n]);
    }

    fn copy_vline(&mut self, x: i32, y: i32, len: u32, c: &PF::ColorType) {
        PixfmtAmaskAdaptor::copy_vline(self, x, y, len, c);
    }

    fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: u32,
        colors: &[PF::ColorType],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        let n = len as usize;
        if !covers.is_empty() {
            self.init_span_covers(n, covers);
            self.mask.combine_hspan(x, y, &mut self.span[..n]);
        } else {
            self.realloc_span(n);
            self.mask.fill_hspan(x, y, &mut self.span[..n]);
        }
        self.pixf
            .blend_color_hspan(x, y, len, colors, &self.span[..n], cover);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha_mask_u8::{AlphaMaskGray8, OneComponentMask};
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
    use crate::rendering_buffer::RowAccessor;

    /// Column 0 of a 2x3 mask is 255, 128, 0 top to bottom; column 1 is 0, 255, 255.
    fn mask_bytes() -> Vec<u8> {
        vec![255, 0, 128, 255, 0, 255]
    }

    fn alphas(buf: &[u8]) -> Vec<u8> {
        buf.chunks_exact(4).map(|p| p[3]).collect()
    }

    #[test]
    fn vertical_and_copy_operations_take_the_mask_as_coverage() {
        let mut mbuf = mask_bytes();
        let mrows = unsafe { RowAccessor::new_with_buf(mbuf.as_mut_ptr(), 2, 3, 2) };
        let black = Rgba8::new(0, 0, 0, 255);

        let run = |op: &dyn Fn(&mut PixfmtAmaskAdaptor<PixfmtRgba32, AlphaMaskGray8>)| {
            let mut buf = vec![0u8; 2 * 3 * 4];
            let mut rows = unsafe { RowAccessor::new_with_buf(buf.as_mut_ptr(), 2, 3, 8) };
            {
                let mut a = PixfmtAmaskAdaptor::new(
                    PixfmtRgba32::new(&mut rows),
                    AlphaMaskGray8::new(&mrows, OneComponentMask),
                );
                op(&mut a);
            }
            alphas(&buf)
        };

        // copy_vline and blend_vline: the mask alone (the cover argument is unused, as in C++).
        assert_eq!(
            run(&|a| a.copy_vline(0, 0, 3, &black)),
            [255, 0, 128, 0, 0, 0]
        );
        assert_eq!(
            run(&|a| a.blend_vline(0, 0, 3, &black, 7)),
            [255, 0, 128, 0, 0, 0]
        );
        // blend_solid_vspan combines each cover with the mask: 128 * 255 -> 128, 128 * 128 -> 64.
        assert_eq!(
            run(&|a| a.blend_solid_vspan(0, 0, 3, &black, &[128, 128, 255])),
            [128, 0, 64, 0, 0, 0]
        );
        // Color spans with and without covers, both directions.
        let colors = [black, black, black];
        assert_eq!(
            run(&|a| a.blend_color_vspan(1, 0, 3, &colors, &[], 255)),
            [0, 0, 0, 255, 0, 255]
        );
        assert_eq!(
            run(&|a| a.copy_color_vspan(1, 0, 3, &colors)),
            [0, 0, 0, 255, 0, 255]
        );
        assert_eq!(
            run(&|a| a.blend_color_vspan(1, 0, 3, &colors, &[255, 128, 0], 255)),
            [0, 0, 0, 128, 0, 0]
        );
        assert_eq!(
            run(&|a| a.copy_color_hspan(0, 1, 2, &colors)),
            [0, 0, 128, 255, 0, 0]
        );
        assert_eq!(
            run(&|a| a.copy_hline(0, 0, 2, &black)),
            [255, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            run(&|a| a.blend_hline(0, 1, 2, &black, 9)),
            [0, 0, 128, 255, 0, 0]
        );
        assert_eq!(run(&|a| a.copy_pixel(0, 1, &black)), [0, 0, 128, 0, 0, 0]);
    }
}

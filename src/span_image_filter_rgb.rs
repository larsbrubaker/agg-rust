//! RGB (24-bit) image transformation span generators.
//!
//! Port of `agg_span_image_filter_rgb.h`: the 3-byte-pixel counterparts of
//! `span_image_filter_rgba` — nearest neighbor, bilinear, bilinear with a
//! background colour, custom 2x2, general N-tap and the two resamplers. They
//! read the first three bytes of each pixel as r, g, b (as the RGBA filters
//! read four) and output opaque colours, except `bilinear_clip`, whose alpha
//! falls with the share of background it takes outside the image.
//!
//! Divergence from C++ (rounding), the same as the RGBA filters' (see
//! `span_image_filter_rgba`): C++ starts every one of these sums at 0 and
//! truncates, darkening by half a level on average, and the resamplers divide
//! the bare sums by the total weight. Here every sum starts at half a unit and
//! the resamplers add half the total weight first, so all of them round.
//! agg-sharp's `SpanImageFilterRgbTests` and `SpanImageResampleRgbTests` pin
//! this (ported in `tests/agg_sharp/`). Channels are also clamped at 0 as
//! well as at 255 everywhere (colour arithmetic clamps, never wraps); C++'s
//! 2x2 clamps only the top.

use crate::color::Rgba8;
use crate::image_accessors::ImageSource;
use crate::image_filters::{
    ImageFilterLut, IMAGE_FILTER_SCALE, IMAGE_FILTER_SHIFT, IMAGE_SUBPIXEL_MASK,
    IMAGE_SUBPIXEL_SCALE, IMAGE_SUBPIXEL_SHIFT,
};
use crate::renderer_scanline::SpanGenerator;
use crate::rendering_buffer::RowAccessor;
use crate::span_image_filter::{SpanImageFilterBase, SpanImageResample, SpanImageResampleAffine};
use crate::span_interpolator_linear::{
    SpanInterpolator, SpanInterpolatorLinear, SpanInterpolatorLocalScale,
};
use crate::trans_affine::TransAffine;

/// Base mask for 8-bit color (255).
const BASE_MASK: i32 = 255;
const SUBPIX: i32 = IMAGE_SUBPIXEL_SCALE as i32;
const MASK: i32 = IMAGE_SUBPIXEL_MASK as i32;
/// Half a unit of a bilinear sum (weights total `IMAGE_SUBPIXEL_SCALE^2`).
const BILINEAR_HALF: i32 = SUBPIX * SUBPIX / 2;
const BILINEAR_SHIFT: u32 = IMAGE_SUBPIXEL_SHIFT * 2;

#[inline]
fn add_weighted(fg: &mut [i32; 3], weight: i32, p: &[u8]) {
    fg[0] += weight * p[0] as i32;
    fg[1] += weight * p[1] as i32;
    fg[2] += weight * p[2] as i32;
}

#[inline]
fn opaque(fg: [i32; 3]) -> Rgba8 {
    Rgba8::new(
        fg[0].clamp(0, BASE_MASK) as u32,
        fg[1].clamp(0, BASE_MASK) as u32,
        fg[2].clamp(0, BASE_MASK) as u32,
        BASE_MASK as u32,
    )
}

// ============================================================================
// SpanImageFilterRgbNn — nearest neighbor
// ============================================================================

/// Nearest-neighbor image filter for RGB images.
///
/// Port of C++ `span_image_filter_rgb_nn<Source, Interpolator>`.
pub struct SpanImageFilterRgbNn<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgbNn<'a, S, I> {
    pub fn new(source: &'a mut S, interpolator: &'a mut I) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, None),
            source,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }
}

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbNn<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx = self.base.filter_dx_dbl();
        let dy = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        for pixel in span.iter_mut().take(len as usize) {
            let (mut cx, mut cy) = (0, 0);
            self.base.interpolator().coordinates(&mut cx, &mut cy);
            let p = self
                .source
                .span(cx >> IMAGE_SUBPIXEL_SHIFT, cy >> IMAGE_SUBPIXEL_SHIFT, 1);
            *pixel = Rgba8::new(p[0] as u32, p[1] as u32, p[2] as u32, BASE_MASK as u32);
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgbBilinear — bilinear through the accessor
// ============================================================================

/// Bilinear image filter for RGB images, reading through its accessor (so a
/// clip, clone or wrap accessor decides what lies past the edges).
///
/// Port of C++ `span_image_filter_rgb_bilinear<Source, Interpolator>`.
pub struct SpanImageFilterRgbBilinear<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgbBilinear<'a, S, I> {
    pub fn new(source: &'a mut S, interpolator: &'a mut I) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, None),
            source,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }
}

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbBilinear<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        let dx = self.base.filter_dx_dbl();
        let dy = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        for pixel in span.iter_mut().take(len as usize) {
            let (mut x_hr, mut y_hr) = (0, 0);
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);
            x_hr -= dx_int;
            y_hr -= dy_int;
            let x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;
            let mut fg = [BILINEAR_HALF; 3];
            x_hr &= MASK;
            y_hr &= MASK;

            let p = self.source.span(x_lr, y_lr, 2);
            add_weighted(&mut fg, (SUBPIX - x_hr) * (SUBPIX - y_hr), p);
            let p = self.source.next_x();
            add_weighted(&mut fg, x_hr * (SUBPIX - y_hr), p);
            let p = self.source.next_y();
            add_weighted(&mut fg, (SUBPIX - x_hr) * y_hr, p);
            let p = self.source.next_x();
            add_weighted(&mut fg, x_hr * y_hr, p);

            *pixel = opaque(fg.map(|v| v >> BILINEAR_SHIFT));
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgbBilinearClip — bilinear with a background colour
// ============================================================================

/// Bilinear image filter for RGB images that blends in a background colour
/// (and its alpha) where the 2x2 footprint leaves the image.
///
/// Port of C++ `span_image_filter_rgb_bilinear_clip<Source, Interpolator>`.
pub struct SpanImageFilterRgbBilinearClip<'a, I> {
    base: SpanImageFilterBase<'a, I>,
    rbuf: &'a RowAccessor,
    back_color: Rgba8,
}

impl<'a, I> SpanImageFilterRgbBilinearClip<'a, I> {
    pub fn new(rbuf: &'a RowAccessor, back_color: Rgba8, interpolator: &'a mut I) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, None),
            rbuf,
            back_color,
        }
    }

    pub fn background_color(&self) -> &Rgba8 {
        &self.back_color
    }

    pub fn set_background_color(&mut self, v: Rgba8) {
        self.back_color = v;
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }
}

impl<I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbBilinearClip<'_, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        let dx = self.base.filter_dx_dbl();
        let dy = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);

        let back = [
            self.back_color.r as i32,
            self.back_color.g as i32,
            self.back_color.b as i32,
        ];
        let back_a = self.back_color.a as i32;
        let maxx = self.rbuf.width() as i32 - 1;
        let maxy = self.rbuf.height() as i32 - 1;
        let rbuf = self.rbuf;
        let pixel_at = |x_lr: i32, y_lr: i32| -> &[u8] {
            let off = x_lr as usize * 3;
            &rbuf.row_slice(y_lr as u32)[off..off + 3]
        };

        for pixel in span.iter_mut().take(len as usize) {
            let (mut x_hr, mut y_hr) = (0, 0);
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);
            x_hr -= dx_int;
            y_hr -= dy_int;
            let mut x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let mut y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;
            let fg: [i32; 3];
            let src_alpha: i32;

            if x_lr >= 0 && y_lr >= 0 && x_lr < maxx && y_lr < maxy {
                let mut sum = [BILINEAR_HALF; 3];
                x_hr &= MASK;
                y_hr &= MASK;
                add_weighted(
                    &mut sum,
                    (SUBPIX - x_hr) * (SUBPIX - y_hr),
                    pixel_at(x_lr, y_lr),
                );
                add_weighted(&mut sum, x_hr * (SUBPIX - y_hr), pixel_at(x_lr + 1, y_lr));
                add_weighted(&mut sum, (SUBPIX - x_hr) * y_hr, pixel_at(x_lr, y_lr + 1));
                add_weighted(&mut sum, x_hr * y_hr, pixel_at(x_lr + 1, y_lr + 1));
                fg = sum.map(|v| v >> BILINEAR_SHIFT);
                src_alpha = BASE_MASK;
            } else if x_lr < -1 || y_lr < -1 || x_lr > maxx || y_lr > maxy {
                fg = back;
                src_alpha = back_a;
            } else {
                let mut sum = [BILINEAR_HALF; 3];
                let mut alpha = BILINEAR_HALF;
                x_hr &= MASK;
                y_hr &= MASK;
                let mut take = |x_lr: i32, y_lr: i32, weight: i32| {
                    if x_lr >= 0 && y_lr >= 0 && x_lr <= maxx && y_lr <= maxy {
                        add_weighted(&mut sum, weight, pixel_at(x_lr, y_lr));
                        alpha += weight * BASE_MASK;
                    } else {
                        sum[0] += back[0] * weight;
                        sum[1] += back[1] * weight;
                        sum[2] += back[2] * weight;
                        alpha += back_a * weight;
                    }
                };
                take(x_lr, y_lr, (SUBPIX - x_hr) * (SUBPIX - y_hr));
                x_lr += 1;
                take(x_lr, y_lr, x_hr * (SUBPIX - y_hr));
                x_lr -= 1;
                y_lr += 1;
                take(x_lr, y_lr, (SUBPIX - x_hr) * y_hr);
                x_lr += 1;
                take(x_lr, y_lr, x_hr * y_hr);
                fg = sum.map(|v| v >> BILINEAR_SHIFT);
                src_alpha = alpha >> BILINEAR_SHIFT;
            }

            *pixel = Rgba8::new(fg[0] as u32, fg[1] as u32, fg[2] as u32, src_alpha as u32);
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgb2x2 — 2x2 lookup-table filter
// ============================================================================

/// 2x2 filter for RGB images, weighted by an `ImageFilterLut`.
///
/// Port of C++ `span_image_filter_rgb_2x2<Source, Interpolator>`.
pub struct SpanImageFilterRgb2x2<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgb2x2<'a, S, I> {
    pub fn new(source: &'a mut S, interpolator: &'a mut I, filter: &'a ImageFilterLut) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, Some(filter)),
            source,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }
}

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgb2x2<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        let dx = self.base.filter_dx_dbl();
        let dy = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        let Some(filter) = self.base.filter_lut() else {
            return;
        };
        let wa: &[i16] = filter.weight_array();
        let offset = ((filter.diameter() / 2 - 1) << IMAGE_SUBPIXEL_SHIFT) as usize;
        let weight = |xi: usize, yi: usize| -> i32 {
            (wa[offset + xi] as i32 * wa[offset + yi] as i32 + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT
        };

        for pixel in span.iter_mut().take(len as usize) {
            let (mut x_hr, mut y_hr) = (0, 0);
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);
            x_hr -= dx_int;
            y_hr -= dy_int;
            let x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;
            let xf = (x_hr & MASK) as usize;
            let yf = (y_hr & MASK) as usize;
            let s = SUBPIX as usize;
            let mut fg = [IMAGE_FILTER_SCALE / 2; 3];

            let p = self.source.span(x_lr, y_lr, 2);
            add_weighted(&mut fg, weight(xf + s, yf + s), p);
            let p = self.source.next_x();
            add_weighted(&mut fg, weight(xf, yf + s), p);
            let p = self.source.next_y();
            add_weighted(&mut fg, weight(xf + s, yf), p);
            let p = self.source.next_x();
            add_weighted(&mut fg, weight(xf, yf), p);

            *pixel = opaque(fg.map(|v| v >> IMAGE_FILTER_SHIFT));
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgb — general N-tap lookup-table filter
// ============================================================================

/// General filter for RGB images over the `ImageFilterLut`'s full diameter.
///
/// Port of C++ `span_image_filter_rgb<Source, Interpolator>`.
pub struct SpanImageFilterRgb<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgb<'a, S, I> {
    pub fn new(source: &'a mut S, interpolator: &'a mut I, filter: &'a ImageFilterLut) -> Self {
        Self {
            base: SpanImageFilterBase::new(interpolator, Some(filter)),
            source,
        }
    }

    pub fn base(&self) -> &SpanImageFilterBase<'a, I> {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut SpanImageFilterBase<'a, I> {
        &mut self.base
    }
}

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgb<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        let dx = self.base.filter_dx_dbl();
        let dy = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        let Some(filter) = self.base.filter_lut() else {
            return;
        };
        let diameter = filter.diameter() as i32;
        let start = filter.start();
        let wa: &[i16] = filter.weight_array();

        for pixel in span.iter_mut().take(len as usize) {
            let (mut cx, mut cy) = (0, 0);
            self.base.interpolator().coordinates(&mut cx, &mut cy);
            cx -= dx_int;
            cy -= dy_int;
            let x_lr = cx >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = cy >> IMAGE_SUBPIXEL_SHIFT;
            let mut fg = [IMAGE_FILTER_SCALE / 2; 3];
            let x_fract = cx & MASK;
            let mut y_hr = MASK - (cy & MASK);

            let mut fg_ptr = self
                .source
                .span(x_lr + start, y_lr + start, diameter as u32);
            let mut y_count = diameter;
            loop {
                let weight_y = wa[y_hr as usize] as i32;
                let mut x_hr = MASK - x_fract;
                let mut x_count = diameter;
                loop {
                    let weight = (weight_y * wa[x_hr as usize] as i32 + IMAGE_FILTER_SCALE / 2)
                        >> IMAGE_FILTER_SHIFT;
                    add_weighted(&mut fg, weight, fg_ptr);
                    x_count -= 1;
                    if x_count == 0 {
                        break;
                    }
                    x_hr += SUBPIX;
                    fg_ptr = self.source.next_x();
                }
                y_count -= 1;
                if y_count == 0 {
                    break;
                }
                y_hr += SUBPIX;
                fg_ptr = self.source.next_y();
            }

            *pixel = opaque(fg.map(|v| v >> IMAGE_FILTER_SHIFT));
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// Resampling
// ============================================================================

/// One resampled pixel: the filter footprint `rx` x `ry` (subpixel units)
/// walked from (`cx`, `cy`), the sums divided by their total weight with
/// half the total added first, so the division rounds (module docs).
#[allow(clippy::too_many_arguments)]
fn resample_pixel<S: ImageSource>(
    source: &mut S,
    wa: &[i16],
    diameter: i32,
    mut cx: i32,
    mut cy: i32,
    rx: i32,
    ry: i32,
    rx_inv: i32,
    ry_inv: i32,
) -> Rgba8 {
    let filter_scale = diameter << IMAGE_SUBPIXEL_SHIFT;
    let radius_x = (diameter * rx) >> 1;
    let radius_y = (diameter * ry) >> 1;
    let len_x_lr = (diameter * rx + MASK) >> IMAGE_SUBPIXEL_SHIFT;
    cx -= radius_x;
    cy -= radius_y;

    let mut fg = [0i32; 3];
    let y_lr = cy >> IMAGE_SUBPIXEL_SHIFT;
    let mut y_hr = ((MASK - (cy & MASK)) * ry_inv) >> IMAGE_SUBPIXEL_SHIFT;
    let mut total_weight = 0i32;
    let x_lr = cx >> IMAGE_SUBPIXEL_SHIFT;
    let x_hr2 = ((MASK - (cx & MASK)) * rx_inv) >> IMAGE_SUBPIXEL_SHIFT;

    let mut fg_ptr = source.span(x_lr, y_lr, len_x_lr as u32);
    loop {
        let weight_y = wa[y_hr as usize] as i32;
        let mut x_hr = x_hr2;
        loop {
            let weight = (weight_y * wa[x_hr as usize] as i32 + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT;
            add_weighted(&mut fg, weight, fg_ptr);
            total_weight += weight;
            x_hr += rx_inv;
            if x_hr >= filter_scale {
                break;
            }
            fg_ptr = source.next_x();
        }
        y_hr += ry_inv;
        if y_hr >= filter_scale {
            break;
        }
        fg_ptr = source.next_y();
    }

    // C++ divides unguarded; a zero total (no weight reached) leaves 0.
    if total_weight > 0 {
        let half = total_weight / 2;
        fg = fg.map(|v| (v + half) / total_weight);
    }
    opaque(fg)
}

/// Affine resampling for RGB images: the footprint follows the transform's
/// scale, computed once in `prepare`.
///
/// Port of C++ `span_image_resample_rgb_affine<Source>`.
pub struct SpanImageResampleRgbAffine<'a, S: ImageSource> {
    base: SpanImageResampleAffine<'a>,
    source: &'a mut S,
}

impl<'a, S: ImageSource> SpanImageResampleRgbAffine<'a, S> {
    pub fn new(
        source: &'a mut S,
        interpolator: &'a mut SpanInterpolatorLinear<TransAffine>,
        filter: &'a ImageFilterLut,
    ) -> Self {
        Self {
            base: SpanImageResampleAffine::new(interpolator, filter),
            source,
        }
    }

    pub fn resample_base(&self) -> &SpanImageResampleAffine<'a> {
        &self.base
    }

    pub fn resample_base_mut(&mut self) -> &mut SpanImageResampleAffine<'a> {
        &mut self.base
    }
}

impl<S: ImageSource> SpanGenerator for SpanImageResampleRgbAffine<'_, S> {
    type Color = Rgba8;

    fn prepare(&mut self) {
        self.base.prepare();
    }

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.base().filter_dx_int() as i32;
        let dy_int = self.base.base().filter_dy_int() as i32;
        let dx = self.base.base().filter_dx_dbl();
        let dy = self.base.base().filter_dy_dbl();
        self.base
            .base_mut()
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        let Some(filter) = self.base.base().filter_lut() else {
            return;
        };
        let diameter = filter.diameter() as i32;
        let wa: &[i16] = filter.weight_array();
        let (rx, ry, rx_inv, ry_inv) = (
            self.base.rx(),
            self.base.ry(),
            self.base.rx_inv(),
            self.base.ry_inv(),
        );

        for pixel in span.iter_mut().take(len as usize) {
            let (mut cx, mut cy) = (0, 0);
            self.base
                .base()
                .interpolator()
                .coordinates(&mut cx, &mut cy);
            *pixel = resample_pixel(
                self.source,
                wa,
                diameter,
                cx + dx_int,
                cy + dy_int,
                rx,
                ry,
                rx_inv,
                ry_inv,
            );
            self.base.base_mut().interpolator_mut().next();
        }
    }
}

/// Resampling for RGB images under any interpolator that reports its local
/// scale (a perspective, usually under `SpanSubdivAdaptor`): the footprint is
/// recomputed per pixel.
///
/// Port of C++ `span_image_resample_rgb<Source, Interpolator>`.
pub struct SpanImageResampleRgb<'a, S: ImageSource, I> {
    base: SpanImageResample<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageResampleRgb<'a, S, I> {
    pub fn new(source: &'a mut S, interpolator: &'a mut I, filter: &'a ImageFilterLut) -> Self {
        Self {
            base: SpanImageResample::new(interpolator, filter),
            source,
        }
    }

    pub fn resample_base(&self) -> &SpanImageResample<'a, I> {
        &self.base
    }

    pub fn resample_base_mut(&mut self) -> &mut SpanImageResample<'a, I> {
        &mut self.base
    }
}

impl<S: ImageSource, I: SpanInterpolatorLocalScale> SpanGenerator
    for SpanImageResampleRgb<'_, S, I>
{
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_int = self.base.base().filter_dx_int() as i32;
        let dy_int = self.base.base().filter_dy_int() as i32;
        let dx = self.base.base().filter_dx_dbl();
        let dy = self.base.base().filter_dy_dbl();
        self.base
            .base_mut()
            .interpolator_mut()
            .begin(x as f64 + dx, y as f64 + dy, len);
        let Some(filter) = self.base.base().filter_lut() else {
            return;
        };
        let diameter = filter.diameter() as i32;
        let wa: &[i16] = filter.weight_array();

        for pixel in span.iter_mut().take(len as usize) {
            let (mut cx, mut cy) = (0, 0);
            let (mut rx, mut ry) = (0, 0);
            let interpolator = self.base.base().interpolator();
            interpolator.coordinates(&mut cx, &mut cy);
            interpolator.local_scale(&mut rx, &mut ry);
            self.base.adjust_scale(&mut rx, &mut ry);
            let rx_inv = SUBPIX * SUBPIX / rx;
            let ry_inv = SUBPIX * SUBPIX / ry;
            *pixel = resample_pixel(
                self.source,
                wa,
                diameter,
                cx + dx_int,
                cy + dy_int,
                rx,
                ry,
                rx_inv,
                ry_inv,
            );
            self.base.base_mut().interpolator_mut().next();
        }
    }
}

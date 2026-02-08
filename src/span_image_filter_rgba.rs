//! RGBA image transformation span generators.
//!
//! Port of `agg_span_image_filter_rgba.h` — span generators for transforming
//! RGBA images with various filter kernels: nearest neighbor, bilinear,
//! custom 2x2, general N-tap, and resampling variants.

use crate::color::Rgba8;
use crate::image_accessors::ImageSource;
use crate::image_filters::{
    ImageFilterLut, IMAGE_FILTER_SCALE, IMAGE_FILTER_SHIFT, IMAGE_SUBPIXEL_MASK,
    IMAGE_SUBPIXEL_SCALE, IMAGE_SUBPIXEL_SHIFT,
};
use crate::renderer_scanline::SpanGenerator;
use crate::rendering_buffer::RowAccessor;
use crate::span_image_filter::{SpanImageFilterBase, SpanImageResampleAffine};
use crate::span_interpolator_linear::{SpanInterpolator, SpanInterpolatorLinear};
use crate::trans_affine::TransAffine;

/// Base mask for 8-bit color (255).
const BASE_MASK: i32 = 255;

// ============================================================================
// SpanImageFilterRgbaNn — nearest neighbor
// ============================================================================

/// Nearest-neighbor image filter for RGBA images.
///
/// Simplest and fastest: picks the nearest pixel without interpolation.
///
/// Port of C++ `span_image_filter_rgba_nn<Source, Interpolator>`.
pub struct SpanImageFilterRgbaNn<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgbaNn<'a, S, I> {
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

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbaNn<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_dbl = self.base.filter_dx_dbl();
        let dy_dbl = self.base.filter_dy_dbl();
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        for pixel in span.iter_mut().take(len as usize) {
            let mut x_hr = 0i32;
            let mut y_hr = 0i32;
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);

            let fg_ptr = self.source.span(
                x_hr >> IMAGE_SUBPIXEL_SHIFT,
                y_hr >> IMAGE_SUBPIXEL_SHIFT,
                1,
            );
            *pixel = Rgba8::new(
                fg_ptr[0] as u32,
                fg_ptr[1] as u32,
                fg_ptr[2] as u32,
                fg_ptr[3] as u32,
            );
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgbaBilinear — bilinear interpolation
// ============================================================================

/// Bilinear image filter for RGBA images.
///
/// 2x2 weighted blend using subpixel fractions from the interpolator.
/// Source must use an image accessor that handles boundary conditions.
///
/// Port of C++ `span_image_filter_rgba_bilinear<Source, Interpolator>`.
pub struct SpanImageFilterRgbaBilinear<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgbaBilinear<'a, S, I> {
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

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbaBilinear<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_dbl = self.base.filter_dx_dbl();
        let dy_dbl = self.base.filter_dy_dbl();
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        let half = (IMAGE_SUBPIXEL_SCALE * IMAGE_SUBPIXEL_SCALE / 2) as i32;

        for pixel in span.iter_mut().take(len as usize) {
            let mut x_hr = 0i32;
            let mut y_hr = 0i32;
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);

            x_hr -= dx_int;
            y_hr -= dy_int;

            let x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;

            let mut fg = [half; 4];

            let x_frac = x_hr & IMAGE_SUBPIXEL_MASK as i32;
            let y_frac = y_hr & IMAGE_SUBPIXEL_MASK as i32;
            let subpix = IMAGE_SUBPIXEL_SCALE as i32;

            // Top-left
            let p = self.source.span(x_lr, y_lr, 2);
            let weight = (subpix - x_frac) * (subpix - y_frac);
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Top-right
            let p = self.source.next_x();
            let weight = x_frac * (subpix - y_frac);
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Bottom-left
            let p = self.source.next_y();
            let weight = (subpix - x_frac) * y_frac;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Bottom-right
            let p = self.source.next_x();
            let weight = x_frac * y_frac;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            let shift = IMAGE_SUBPIXEL_SHIFT * 2;
            *pixel = Rgba8::new(
                (fg[0] >> shift) as u32,
                (fg[1] >> shift) as u32,
                (fg[2] >> shift) as u32,
                (fg[3] >> shift) as u32,
            );

            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgbaBilinearClip — bilinear with background color
// ============================================================================

/// Bilinear image filter with background color for out-of-bounds pixels.
///
/// Accesses the rendering buffer directly (no image accessor) and
/// returns a background color for pixels that fall outside the image.
///
/// Port of C++ `span_image_filter_rgba_bilinear_clip<Source, Interpolator>`.
pub struct SpanImageFilterRgbaBilinearClip<'a, I> {
    base: SpanImageFilterBase<'a, I>,
    rbuf: &'a RowAccessor,
    back_color: Rgba8,
}

impl<'a, I> SpanImageFilterRgbaBilinearClip<'a, I> {
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

impl<I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbaBilinearClip<'_, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_dbl = self.base.filter_dx_dbl();
        let dy_dbl = self.base.filter_dy_dbl();
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        let back_r = self.back_color.r as i32;
        let back_g = self.back_color.g as i32;
        let back_b = self.back_color.b as i32;
        let back_a = self.back_color.a as i32;

        let maxx = self.rbuf.width() as i32 - 1;
        let maxy = self.rbuf.height() as i32 - 1;

        for pixel in span.iter_mut().take(len as usize) {
            let mut x_hr = 0i32;
            let mut y_hr = 0i32;
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);

            x_hr -= dx_int;
            y_hr -= dy_int;

            let mut x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let mut y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;

            let mut fg: [i32; 4];
            let subpix = IMAGE_SUBPIXEL_SCALE as i32;

            if x_lr >= 0 && y_lr >= 0 && x_lr < maxx && y_lr < maxy {
                // All 4 pixels in bounds — fast path
                fg = [0; 4];
                let x_frac = x_hr & IMAGE_SUBPIXEL_MASK as i32;
                let y_frac = y_hr & IMAGE_SUBPIXEL_MASK as i32;

                let row = self.rbuf.row_slice(y_lr as u32);
                let off = (x_lr as usize) << 2;

                let weight = (subpix - x_frac) * (subpix - y_frac);
                fg[0] += weight * row[off] as i32;
                fg[1] += weight * row[off + 1] as i32;
                fg[2] += weight * row[off + 2] as i32;
                fg[3] += weight * row[off + 3] as i32;

                let weight = x_frac * (subpix - y_frac);
                fg[0] += weight * row[off + 4] as i32;
                fg[1] += weight * row[off + 5] as i32;
                fg[2] += weight * row[off + 6] as i32;
                fg[3] += weight * row[off + 7] as i32;

                let row2 = self.rbuf.row_slice((y_lr + 1) as u32);
                let weight = (subpix - x_frac) * y_frac;
                fg[0] += weight * row2[off] as i32;
                fg[1] += weight * row2[off + 1] as i32;
                fg[2] += weight * row2[off + 2] as i32;
                fg[3] += weight * row2[off + 3] as i32;

                let weight = x_frac * y_frac;
                fg[0] += weight * row2[off + 4] as i32;
                fg[1] += weight * row2[off + 5] as i32;
                fg[2] += weight * row2[off + 6] as i32;
                fg[3] += weight * row2[off + 7] as i32;

                let shift = IMAGE_SUBPIXEL_SHIFT * 2;
                fg[0] >>= shift;
                fg[1] >>= shift;
                fg[2] >>= shift;
                fg[3] >>= shift;
            } else if x_lr < -1 || y_lr < -1 || x_lr > maxx || y_lr > maxy {
                // Completely outside — use background
                fg = [back_r, back_g, back_b, back_a];
            } else {
                // Partially outside — blend with background
                fg = [0; 4];
                let x_frac = x_hr & IMAGE_SUBPIXEL_MASK as i32;
                let y_frac = y_hr & IMAGE_SUBPIXEL_MASK as i32;

                let weight = (subpix - x_frac) * (subpix - y_frac);
                if x_lr >= 0 && y_lr >= 0 && x_lr <= maxx && y_lr <= maxy {
                    let row = self.rbuf.row_slice(y_lr as u32);
                    let off = (x_lr as usize) << 2;
                    fg[0] += weight * row[off] as i32;
                    fg[1] += weight * row[off + 1] as i32;
                    fg[2] += weight * row[off + 2] as i32;
                    fg[3] += weight * row[off + 3] as i32;
                } else {
                    fg[0] += back_r * weight;
                    fg[1] += back_g * weight;
                    fg[2] += back_b * weight;
                    fg[3] += back_a * weight;
                }

                x_lr += 1;
                let weight = x_frac * (subpix - y_frac);
                if x_lr >= 0 && y_lr >= 0 && x_lr <= maxx && y_lr <= maxy {
                    let row = self.rbuf.row_slice(y_lr as u32);
                    let off = (x_lr as usize) << 2;
                    fg[0] += weight * row[off] as i32;
                    fg[1] += weight * row[off + 1] as i32;
                    fg[2] += weight * row[off + 2] as i32;
                    fg[3] += weight * row[off + 3] as i32;
                } else {
                    fg[0] += back_r * weight;
                    fg[1] += back_g * weight;
                    fg[2] += back_b * weight;
                    fg[3] += back_a * weight;
                }

                x_lr -= 1;
                y_lr += 1;
                let weight = (subpix - x_frac) * y_frac;
                if x_lr >= 0 && y_lr >= 0 && x_lr <= maxx && y_lr <= maxy {
                    let row = self.rbuf.row_slice(y_lr as u32);
                    let off = (x_lr as usize) << 2;
                    fg[0] += weight * row[off] as i32;
                    fg[1] += weight * row[off + 1] as i32;
                    fg[2] += weight * row[off + 2] as i32;
                    fg[3] += weight * row[off + 3] as i32;
                } else {
                    fg[0] += back_r * weight;
                    fg[1] += back_g * weight;
                    fg[2] += back_b * weight;
                    fg[3] += back_a * weight;
                }

                x_lr += 1;
                let weight = x_frac * y_frac;
                if x_lr >= 0 && y_lr >= 0 && x_lr <= maxx && y_lr <= maxy {
                    let row = self.rbuf.row_slice(y_lr as u32);
                    let off = (x_lr as usize) << 2;
                    fg[0] += weight * row[off] as i32;
                    fg[1] += weight * row[off + 1] as i32;
                    fg[2] += weight * row[off + 2] as i32;
                    fg[3] += weight * row[off + 3] as i32;
                } else {
                    fg[0] += back_r * weight;
                    fg[1] += back_g * weight;
                    fg[2] += back_b * weight;
                    fg[3] += back_a * weight;
                }

                let shift = IMAGE_SUBPIXEL_SHIFT * 2;
                fg[0] >>= shift;
                fg[1] >>= shift;
                fg[2] >>= shift;
                fg[3] >>= shift;
            }

            *pixel = Rgba8::new(fg[0] as u32, fg[1] as u32, fg[2] as u32, fg[3] as u32);
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgba2x2 — 2x2 custom filter kernel
// ============================================================================

/// 2x2 custom filter kernel for RGBA images.
///
/// Uses the `ImageFilterLut` weights at the subpixel offset for a
/// separable 2x2 filter kernel.
///
/// Port of C++ `span_image_filter_rgba_2x2<Source, Interpolator>`.
pub struct SpanImageFilterRgba2x2<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgba2x2<'a, S, I> {
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

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgba2x2<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_dbl = self.base.filter_dx_dbl();
        let dy_dbl = self.base.filter_dy_dbl();
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        let filter = self.base.filter().unwrap();
        let weight_array: Vec<i16> = filter.weight_array().to_vec();
        let wa_offset = ((filter.diameter() / 2 - 1) << IMAGE_SUBPIXEL_SHIFT) as usize;

        for pixel in span.iter_mut().take(len as usize) {
            let mut x_hr = 0i32;
            let mut y_hr = 0i32;
            self.base.interpolator().coordinates(&mut x_hr, &mut y_hr);

            x_hr -= dx_int;
            y_hr -= dy_int;

            let x_lr = x_hr >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = y_hr >> IMAGE_SUBPIXEL_SHIFT;

            let mut fg = [0i32; 4];
            let x_frac = (x_hr & IMAGE_SUBPIXEL_MASK as i32) as usize;
            let y_frac = (y_hr & IMAGE_SUBPIXEL_MASK as i32) as usize;
            let subpix = IMAGE_SUBPIXEL_SCALE as usize;

            // Top-left
            let p = self.source.span(x_lr, y_lr, 2);
            let weight = ((weight_array[wa_offset + x_frac + subpix] as i32
                * weight_array[wa_offset + y_frac + subpix] as i32)
                + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Top-right
            let p = self.source.next_x();
            let weight = ((weight_array[wa_offset + x_frac] as i32
                * weight_array[wa_offset + y_frac + subpix] as i32)
                + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Bottom-left
            let p = self.source.next_y();
            let weight = ((weight_array[wa_offset + x_frac + subpix] as i32
                * weight_array[wa_offset + y_frac] as i32)
                + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            // Bottom-right
            let p = self.source.next_x();
            let weight = ((weight_array[wa_offset + x_frac] as i32
                * weight_array[wa_offset + y_frac] as i32)
                + IMAGE_FILTER_SCALE / 2)
                >> IMAGE_FILTER_SHIFT;
            fg[0] += weight * p[0] as i32;
            fg[1] += weight * p[1] as i32;
            fg[2] += weight * p[2] as i32;
            fg[3] += weight * p[3] as i32;

            fg[0] >>= IMAGE_FILTER_SHIFT;
            fg[1] >>= IMAGE_FILTER_SHIFT;
            fg[2] >>= IMAGE_FILTER_SHIFT;
            fg[3] >>= IMAGE_FILTER_SHIFT;

            // Clamp: alpha to full_value, RGB to alpha
            if fg[3] > BASE_MASK {
                fg[3] = BASE_MASK;
            }
            if fg[0] > fg[3] {
                fg[0] = fg[3];
            }
            if fg[1] > fg[3] {
                fg[1] = fg[3];
            }
            if fg[2] > fg[3] {
                fg[2] = fg[3];
            }

            *pixel = Rgba8::new(fg[0] as u32, fg[1] as u32, fg[2] as u32, fg[3] as u32);
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageFilterRgbaGen — general N-tap filter
// ============================================================================

/// General N-tap image filter for RGBA images.
///
/// Walks a `diameter × diameter` kernel from the filter LUT, accumulating
/// weighted pixel values. Handles negative weights correctly.
///
/// Port of C++ `span_image_filter_rgba<Source, Interpolator>`.
pub struct SpanImageFilterRgbaGen<'a, S: ImageSource, I> {
    base: SpanImageFilterBase<'a, I>,
    source: &'a mut S,
}

impl<'a, S: ImageSource, I> SpanImageFilterRgbaGen<'a, S, I> {
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

impl<S: ImageSource, I: SpanInterpolator> SpanGenerator for SpanImageFilterRgbaGen<'_, S, I> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let dx_dbl = self.base.filter_dx_dbl();
        let dy_dbl = self.base.filter_dy_dbl();
        let dx_int = self.base.filter_dx_int() as i32;
        let dy_int = self.base.filter_dy_int() as i32;
        self.base
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        let filter = self.base.filter().unwrap();
        let diameter = filter.diameter() as i32;
        let start = filter.start();
        let weight_array: Vec<i16> = filter.weight_array().to_vec();

        for pixel in span.iter_mut().take(len as usize) {
            let mut cx = 0i32;
            let mut cy = 0i32;
            self.base.interpolator().coordinates(&mut cx, &mut cy);

            cx -= dx_int;
            cy -= dy_int;

            let x_lr = cx >> IMAGE_SUBPIXEL_SHIFT;
            let y_lr = cy >> IMAGE_SUBPIXEL_SHIFT;

            let mut fg = [0i32; 4];
            let x_fract = cx & IMAGE_SUBPIXEL_MASK as i32;

            let mut y_count = diameter as u32;
            let mut y_wa = IMAGE_SUBPIXEL_MASK as i32 - (cy & IMAGE_SUBPIXEL_MASK as i32);

            let mut fg_ptr = self
                .source
                .span(x_lr + start, y_lr + start, diameter as u32);

            loop {
                let mut x_count = diameter;
                let weight_y = weight_array[y_wa as usize] as i32;
                let mut x_wa = IMAGE_SUBPIXEL_MASK as i32 - x_fract;

                loop {
                    let weight = (weight_y * weight_array[x_wa as usize] as i32
                        + IMAGE_FILTER_SCALE / 2)
                        >> IMAGE_FILTER_SHIFT;

                    fg[0] += weight * fg_ptr[0] as i32;
                    fg[1] += weight * fg_ptr[1] as i32;
                    fg[2] += weight * fg_ptr[2] as i32;
                    fg[3] += weight * fg_ptr[3] as i32;

                    x_count -= 1;
                    if x_count == 0 {
                        break;
                    }
                    x_wa += IMAGE_SUBPIXEL_SCALE as i32;
                    fg_ptr = self.source.next_x();
                }

                y_count -= 1;
                if y_count == 0 {
                    break;
                }
                y_wa += IMAGE_SUBPIXEL_SCALE as i32;
                fg_ptr = self.source.next_y();
            }

            fg[0] >>= IMAGE_FILTER_SHIFT;
            fg[1] >>= IMAGE_FILTER_SHIFT;
            fg[2] >>= IMAGE_FILTER_SHIFT;
            fg[3] >>= IMAGE_FILTER_SHIFT;

            // Clamp negative values and alpha to full_value, RGB to alpha
            fg[3] = fg[3].clamp(0, BASE_MASK);
            fg[0] = fg[0].max(0).min(fg[3]);
            fg[1] = fg[1].max(0).min(fg[3]);
            fg[2] = fg[2].max(0).min(fg[3]);

            *pixel = Rgba8::new(fg[0] as u32, fg[1] as u32, fg[2] as u32, fg[3] as u32);
            self.base.interpolator_mut().next();
        }
    }
}

// ============================================================================
// SpanImageResampleRgbaAffine — affine resampling
// ============================================================================

/// Affine resampling for RGBA images.
///
/// Uses precomputed scale factors (rx/ry) from `SpanImageResampleAffine`
/// to walk a variable-size filter kernel that adapts to the magnification.
///
/// Port of C++ `span_image_resample_rgba_affine<Source>`.
pub struct SpanImageResampleRgbaAffine<'a, S: ImageSource> {
    base: SpanImageResampleAffine<'a>,
    source: &'a mut S,
}

impl<'a, S: ImageSource> SpanImageResampleRgbaAffine<'a, S> {
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

impl<S: ImageSource> SpanGenerator for SpanImageResampleRgbaAffine<'_, S> {
    type Color = Rgba8;

    fn prepare(&mut self) {
        self.base.prepare();
    }

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let filter = self.base.base().filter().unwrap();
        let diameter = filter.diameter() as i32;
        let filter_scale = diameter << IMAGE_SUBPIXEL_SHIFT;
        let rx = self.base.rx();
        let ry = self.base.ry();
        let rx_inv = self.base.rx_inv();
        let ry_inv = self.base.ry_inv();
        let radius_x = (diameter * rx) >> 1;
        let radius_y = (diameter * ry) >> 1;
        let len_x_lr = (diameter * rx + IMAGE_SUBPIXEL_MASK as i32) >> IMAGE_SUBPIXEL_SHIFT;
        let weight_array: Vec<i16> = filter.weight_array().to_vec();

        let dx_dbl = self.base.base().filter_dx_dbl();
        let dy_dbl = self.base.base().filter_dy_dbl();
        let dx_int = self.base.base().filter_dx_int() as i32;
        let dy_int = self.base.base().filter_dy_int() as i32;

        self.base
            .base_mut()
            .interpolator_mut()
            .begin(x as f64 + dx_dbl, y as f64 + dy_dbl, len);

        for pixel in span.iter_mut().take(len as usize) {
            let mut cx = 0i32;
            let mut cy = 0i32;
            self.base
                .base()
                .interpolator()
                .coordinates(&mut cx, &mut cy);

            cx += dx_int - radius_x;
            cy += dy_int - radius_y;

            let mut fg = [0i32; 4];

            let y_lr = cy >> IMAGE_SUBPIXEL_SHIFT;
            let mut y_wa = ((IMAGE_SUBPIXEL_MASK as i32 - (cy & IMAGE_SUBPIXEL_MASK as i32))
                * ry_inv)
                >> IMAGE_SUBPIXEL_SHIFT;
            let mut total_weight = 0i32;
            let x_lr = cx >> IMAGE_SUBPIXEL_SHIFT;
            let x_wa_start = ((IMAGE_SUBPIXEL_MASK as i32 - (cx & IMAGE_SUBPIXEL_MASK as i32))
                * rx_inv)
                >> IMAGE_SUBPIXEL_SHIFT;

            let mut fg_ptr = self.source.span(x_lr, y_lr, len_x_lr as u32);

            loop {
                let weight_y = weight_array[y_wa as usize] as i32;
                let mut x_wa = x_wa_start;

                loop {
                    let weight = (weight_y * weight_array[x_wa as usize] as i32
                        + IMAGE_FILTER_SCALE / 2)
                        >> IMAGE_FILTER_SHIFT;

                    fg[0] += fg_ptr[0] as i32 * weight;
                    fg[1] += fg_ptr[1] as i32 * weight;
                    fg[2] += fg_ptr[2] as i32 * weight;
                    fg[3] += fg_ptr[3] as i32 * weight;
                    total_weight += weight;

                    x_wa += rx_inv;
                    if x_wa >= filter_scale {
                        break;
                    }
                    fg_ptr = self.source.next_x();
                }

                y_wa += ry_inv;
                if y_wa >= filter_scale {
                    break;
                }
                fg_ptr = self.source.next_y();
            }

            if total_weight > 0 {
                fg[0] /= total_weight;
                fg[1] /= total_weight;
                fg[2] /= total_weight;
                fg[3] /= total_weight;
            }

            fg[3] = fg[3].clamp(0, BASE_MASK);
            fg[0] = fg[0].max(0).min(fg[3]);
            fg[1] = fg[1].max(0).min(fg[3]);
            fg[2] = fg[2].max(0).min(fg[3]);

            *pixel = Rgba8::new(fg[0] as u32, fg[1] as u32, fg[2] as u32, fg[3] as u32);
            self.base.base_mut().interpolator_mut().next();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_accessors::{ImageAccessorClip, ImageAccessorClone};
    use crate::image_filters::ImageFilterBilinear;
    use crate::rendering_buffer::RowAccessor;

    fn make_rgba_buffer(width: u32, height: u32, data: &mut Vec<u8>) -> RowAccessor {
        let stride = width as usize * 4;
        data.resize(stride * height as usize, 0);
        unsafe { RowAccessor::new_with_buf(data.as_mut_ptr(), width, height, stride as i32) }
    }

    fn set_pixel(data: &mut [u8], width: u32, x: u32, y: u32, rgba: [u8; 4]) {
        let off = (y * width * 4 + x * 4) as usize;
        data[off..off + 4].copy_from_slice(&rgba);
    }

    // -- Nearest Neighbor tests --

    #[test]
    fn test_nn_identity() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 1, [100, 150, 200, 255]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaNn::new(&mut acc, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 1, 1, 1);
        assert_eq!(span[0].r, 100);
        assert_eq!(span[0].g, 150);
        assert_eq!(span[0].b, 200);
        assert_eq!(span[0].a, 255);
    }

    #[test]
    fn test_nn_multiple_pixels() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 0, 0, [10, 20, 30, 40]);
        set_pixel(&mut data, 4, 1, 0, [50, 60, 70, 80]);
        set_pixel(&mut data, 4, 2, 0, [90, 100, 110, 120]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaNn::new(&mut acc, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 3];
        filter.generate(&mut span, 0, 0, 3);
        assert_eq!(span[0].r, 10);
        assert_eq!(span[1].r, 50);
        assert_eq!(span[2].r, 90);
    }

    #[test]
    fn test_nn_with_translation() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 2, 2, [77, 88, 99, 255]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        // Translate by (-2, -2): pixel at device (0,0) reads source (2,2)
        let trans = TransAffine::new_translation(2.0, 2.0);
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaNn::new(&mut acc, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 0, 0, 1);
        assert_eq!(span[0].r, 77);
        assert_eq!(span[0].g, 88);
    }

    // -- Bilinear tests --

    #[test]
    fn test_bilinear_integer_position() {
        // At integer positions, bilinear should approximate the pixel value
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        // Fill a 2x2 region with the same color
        set_pixel(&mut data, 4, 0, 0, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 1, 0, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 0, 1, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 1, 1, [100, 100, 100, 255]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaBilinear::new(&mut acc, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 0, 0, 1);
        // With uniform color, bilinear should produce ~100
        assert!((span[0].r as i32 - 100).abs() <= 1);
    }

    #[test]
    fn test_bilinear_blend() {
        // 2x2 image: top=white, bottom=black
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(2, 2, &mut data);
        set_pixel(&mut data, 2, 0, 0, [255, 255, 255, 255]);
        set_pixel(&mut data, 2, 1, 0, [255, 255, 255, 255]);
        set_pixel(&mut data, 2, 0, 1, [0, 0, 0, 255]);
        set_pixel(&mut data, 2, 1, 1, [0, 0, 0, 255]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaBilinear::new(&mut acc, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 0, 0, 1);
        // At (0, 0), bilinear samples (0,0),(1,0),(0,1),(1,1) with half-pixel offset
        // Result depends on filter offset (default 0.5), meaning it samples at (0.5, 0.5)
        // which is the center of the 4 pixels = average
        assert_eq!(span[0].a, 255);
    }

    // -- Bilinear Clip tests --

    #[test]
    fn test_bilinear_clip_in_bounds() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 1, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 2, 1, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 1, 2, [100, 100, 100, 255]);
        set_pixel(&mut data, 4, 2, 2, [100, 100, 100, 255]);

        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let bg = Rgba8::new(0, 0, 0, 0);
        let mut filter = SpanImageFilterRgbaBilinearClip::new(&rbuf, bg, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 1, 1, 1);
        // All 4 neighbors are (100,100,100,255), result should be ~100
        assert!((span[0].r as i32 - 100).abs() <= 1);
    }

    #[test]
    fn test_bilinear_clip_out_of_bounds() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);

        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let bg = Rgba8::new(55, 66, 77, 88);
        let mut filter = SpanImageFilterRgbaBilinearClip::new(&rbuf, bg, &mut interp);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        // Far outside — should return background
        filter.generate(&mut span, -10, -10, 1);
        assert_eq!(span[0].r, 55);
        assert_eq!(span[0].g, 66);
        assert_eq!(span[0].b, 77);
        assert_eq!(span[0].a, 88);
    }

    #[test]
    fn test_bilinear_clip_background_setter() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let bg = Rgba8::new(0, 0, 0, 0);
        let mut filter = SpanImageFilterRgbaBilinearClip::new(&rbuf, bg, &mut interp);
        filter.set_background_color(Rgba8::new(10, 20, 30, 40));
        assert_eq!(filter.background_color().r, 10);
    }

    // -- 2x2 Custom Filter tests --

    #[test]
    fn test_2x2_with_bilinear_filter() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 1, [200, 200, 200, 255]);
        set_pixel(&mut data, 4, 2, 1, [200, 200, 200, 255]);
        set_pixel(&mut data, 4, 1, 2, [200, 200, 200, 255]);
        set_pixel(&mut data, 4, 2, 2, [200, 200, 200, 255]);

        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgba2x2::new(&mut acc, &mut interp, &lut);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 1, 1, 1);
        // Uniform region should produce ~200
        assert!((span[0].r as i32 - 200).abs() <= 2);
    }

    // -- General N-tap Filter tests --

    #[test]
    fn test_general_with_bilinear_kernel() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 1, [150, 150, 150, 255]);
        set_pixel(&mut data, 4, 2, 1, [150, 150, 150, 255]);
        set_pixel(&mut data, 4, 1, 2, [150, 150, 150, 255]);
        set_pixel(&mut data, 4, 2, 2, [150, 150, 150, 255]);

        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaGen::new(&mut acc, &mut interp, &lut);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.generate(&mut span, 1, 1, 1);
        // Uniform region should produce ~150
        assert!((span[0].r as i32 - 150).abs() <= 2);
    }

    #[test]
    fn test_general_clamps_negative() {
        // With some filter kernels, negative weights can produce negative intermediate
        // values. The general filter clamps these to 0.
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        // All zeros — result should be 0, never negative
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaGen::new(&mut acc, &mut interp, &lut);

        let mut span = vec![Rgba8::new(99, 99, 99, 99); 1];
        filter.generate(&mut span, 0, 0, 1);
        assert_eq!(span[0].r, 0);
        assert_eq!(span[0].g, 0);
        assert_eq!(span[0].b, 0);
    }

    // -- Resample Affine tests --

    #[test]
    fn test_resample_affine_identity() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 1, [120, 130, 140, 255]);
        set_pixel(&mut data, 4, 2, 1, [120, 130, 140, 255]);
        set_pixel(&mut data, 4, 1, 2, [120, 130, 140, 255]);
        set_pixel(&mut data, 4, 2, 2, [120, 130, 140, 255]);

        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageResampleRgbaAffine::new(&mut acc, &mut interp, &lut);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        filter.prepare();
        filter.generate(&mut span, 1, 1, 1);
        // At 1x scale with bilinear, uniform region → ~120
        assert!((span[0].r as i32 - 120).abs() <= 2);
    }

    #[test]
    fn test_resample_affine_clamps() {
        // All black — result should be 0, never negative
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        let lut = ImageFilterLut::new_with_filter(&ImageFilterBilinear, true);
        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageResampleRgbaAffine::new(&mut acc, &mut interp, &lut);

        let mut span = vec![Rgba8::new(99, 99, 99, 99); 1];
        filter.prepare();
        filter.generate(&mut span, 0, 0, 1);
        assert_eq!(span[0].r, 0);
    }

    // -- ImageSource trait tests --

    #[test]
    fn test_image_source_trait_clip() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 0, 0, [11, 22, 33, 44]);

        let mut acc = ImageAccessorClip::<4>::new(&rbuf, &[0, 0, 0, 0]);
        let p: &[u8] = ImageSource::span(&mut acc, 0, 0, 1);
        assert_eq!(p[0], 11);
        assert_eq!(p[3], 44);
    }

    #[test]
    fn test_image_source_trait_clone() {
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(4, 4, &mut data);
        set_pixel(&mut data, 4, 1, 0, [55, 66, 77, 88]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let p = ImageSource::span(&mut acc, 1, 0, 1);
        assert_eq!(p[0], 55);
        let _ = ImageSource::next_x(&mut acc);
    }

    #[test]
    fn test_span_generator_trait_compliance() {
        // Verify that the NN filter implements SpanGenerator correctly
        let mut data = Vec::new();
        let rbuf = make_rgba_buffer(2, 2, &mut data);
        set_pixel(&mut data, 2, 0, 0, [10, 20, 30, 255]);
        set_pixel(&mut data, 2, 1, 0, [40, 50, 60, 255]);

        let mut acc = ImageAccessorClone::<4>::new(&rbuf);
        let trans = TransAffine::new();
        let mut interp = SpanInterpolatorLinear::new(trans);
        let mut filter = SpanImageFilterRgbaNn::new(&mut acc, &mut interp);

        // Test prepare() + generate() as SpanGenerator
        SpanGenerator::prepare(&mut filter);
        let mut span = vec![Rgba8::new(0, 0, 0, 0); 2];
        SpanGenerator::generate(&mut filter, &mut span, 0, 0, 2);
        assert_eq!(span[0].r, 10);
        assert_eq!(span[1].r, 40);
    }
}

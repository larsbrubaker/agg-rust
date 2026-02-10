//! Image-patterned outline renderer.
//!
//! Port of `agg_renderer_outline_image.h`.
//! Renders anti-aliased lines with image patterns applied along the stroke.
//! The pattern is sampled from a source image and mapped onto each line segment.
//!
//! Copyright 2025-2026.

use crate::basics::{iround, RectI};
use crate::color::{Rgba, Rgba8};
use crate::dda_line::Dda2LineInterpolator;
use crate::line_aa_basics::*;
use crate::pattern_filters_rgba::PatternFilter;
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;
use crate::renderer_outline_aa::OutlineAaRenderer;

// ============================================================================
// Pattern source trait
// ============================================================================

/// Trait for image pattern sources.
///
/// Any type implementing this can be used as input to `LineImagePattern::create()`.
/// The C++ equivalent uses a template parameter with `width()`, `height()`, `pixel()`.
pub trait ImagePatternSource {
    fn width(&self) -> f64;
    fn height(&self) -> f64;
    fn pixel(&self, x: i32, y: i32) -> Rgba8;
}

// ============================================================================
// line_image_scale — scales a pattern source to a different height
// ============================================================================

/// Helper that wraps a pattern source and scales its height.
///
/// Port of C++ `line_image_scale<Source>`.
/// Uses float (Rgba) arithmetic for both branches, matching C++ exactly.
pub struct LineImageScale<'a, S: ImagePatternSource> {
    source: &'a S,
    height: f64,
    scale: f64,
    scale_inv: f64,
}

/// Convert Rgba8 to Rgba (float) — matching C++ `rgba(rgba8)` conversion.
#[inline]
fn rgba8_to_rgba(c: Rgba8) -> Rgba {
    Rgba::new(
        c.r as f64 / 255.0,
        c.g as f64 / 255.0,
        c.b as f64 / 255.0,
        c.a as f64 / 255.0,
    )
}

/// Convert Rgba (float) back to Rgba8 — matching C++ `rgba8(rgba)` conversion.
/// Uses uround (v + 0.5) as i32, clamped to [0, 255].
#[inline]
fn rgba_to_rgba8(c: &Rgba) -> Rgba8 {
    #[inline]
    fn clamp_u8(v: f64) -> u32 {
        let i = (v * 255.0 + 0.5) as i32;
        i.clamp(0, 255) as u32
    }
    Rgba8::new(clamp_u8(c.r), clamp_u8(c.g), clamp_u8(c.b), clamp_u8(c.a))
}

impl<'a, S: ImagePatternSource> LineImageScale<'a, S> {
    pub fn new(source: &'a S, height: f64) -> Self {
        let sh = source.height();
        Self {
            source,
            height,
            scale: sh / height,
            scale_inv: height / sh,
        }
    }
}

impl<'a, S: ImagePatternSource> ImagePatternSource for LineImageScale<'a, S> {
    fn width(&self) -> f64 {
        self.source.width()
    }

    fn height(&self) -> f64 {
        self.height
    }

    /// Sample the scaled pattern at (x, y).
    ///
    /// Port of C++ `line_image_scale::pixel`.
    /// Uses float (Rgba) arithmetic for both branches, matching C++ exactly:
    /// - scale < 1.0: gradient interpolation between two rows
    /// - scale >= 1.0: area-weighted average of multiple rows
    fn pixel(&self, x: i32, y: i32) -> Rgba8 {
        let h = self.source.height() as i32 - 1;

        if self.scale < 1.0 {
            // Interpolate between two nearest source rows
            let src_y = (y as f64 + 0.5) * self.scale - 0.5;
            let y1 = src_y.floor() as i32;
            let y2 = y1 + 1;
            let pix1 = if y1 < 0 {
                Rgba::no_color()
            } else {
                rgba8_to_rgba(self.source.pixel(x, y1))
            };
            let pix2 = if y2 > h {
                Rgba::no_color()
            } else {
                rgba8_to_rgba(self.source.pixel(x, y2))
            };
            let k = src_y - y1 as f64;
            rgba_to_rgba8(&pix1.gradient(&pix2, k))
        } else {
            // Area-weighted average of source rows covering [src_y1, src_y2)
            let src_y1 = (y as f64 + 0.5) * self.scale - 0.5;
            let src_y2 = src_y1 + self.scale;
            let mut y1 = src_y1.floor() as i32;
            let y2 = src_y2.floor() as i32;

            let mut c = Rgba::no_color();

            // First partial row
            if y1 >= 0 {
                let weight = (y1 + 1) as f64 - src_y1;
                let p = rgba8_to_rgba(self.source.pixel(x, y1));
                c += p * weight;
            }

            // Full middle rows
            y1 += 1;
            while y1 < y2 {
                if y1 <= h {
                    c += rgba8_to_rgba(self.source.pixel(x, y1));
                }
                y1 += 1;
            }

            // Last partial row
            if y2 <= h {
                let weight = src_y2 - y2 as f64;
                let p = rgba8_to_rgba(self.source.pixel(x, y2));
                c += p * weight;
            }

            c *= self.scale_inv;
            rgba_to_rgba8(&c)
        }
    }
}

// ============================================================================
// line_image_pattern — the main pattern container
// ============================================================================

/// Ceiling of float to unsigned — port of C++ `uceil`.
#[inline]
fn uceil(v: f64) -> u32 {
    v.ceil() as u32
}

/// Round float to signed int — port of C++ `uround`.
#[inline]
fn uround(v: f64) -> i32 {
    (v + 0.5) as i32
}

/// Image pattern for line rendering.
///
/// Port of C++ `line_image_pattern<Filter>`.
/// Stores a copy of the pattern image extended with dilation borders
/// for the filter to access neighboring pixels.
pub struct LineImagePattern<F: PatternFilter> {
    _phantom: std::marker::PhantomData<F>,
    /// 2D pixel buffer: rows[y][x]
    buf: Vec<Vec<Rgba8>>,
    dilation: u32,
    dilation_hr: i32,
    width: u32,
    height: u32,
    width_hr: i32,
    half_height_hr: i32,
    offset_y_hr: i32,
}

impl<F: PatternFilter> LineImagePattern<F> {
    /// Create an empty pattern with the specified filter type.
    pub fn new() -> Self {
        let dilation = F::dilation() + 1;
        Self {
            _phantom: std::marker::PhantomData,
            buf: Vec::new(),
            dilation,
            dilation_hr: (dilation as i32) << LINE_SUBPIXEL_SHIFT,
            width: 0,
            height: 0,
            width_hr: 0,
            half_height_hr: 0,
            offset_y_hr: 0,
        }
    }

    /// Create a pattern initialized from a source.
    pub fn with_source<S: ImagePatternSource>(src: &S) -> Self {
        let mut p = Self::new();
        p.create(src);
        p
    }

    /// Initialize or reinitialize the pattern from a source image.
    ///
    /// Port of C++ `line_image_pattern::create`.
    pub fn create<S: ImagePatternSource>(&mut self, src: &S) {
        self.height = uceil(src.height());
        self.width = uceil(src.width());
        self.width_hr = uround(src.width() * LINE_SUBPIXEL_SCALE as f64);
        self.half_height_hr = uround(src.height() * LINE_SUBPIXEL_SCALE as f64 / 2.0);
        self.offset_y_hr =
            self.dilation_hr + self.half_height_hr - LINE_SUBPIXEL_SCALE / 2;
        self.half_height_hr += LINE_SUBPIXEL_SCALE / 2;

        let total_w = (self.width + self.dilation * 2) as usize;
        let total_h = (self.height + self.dilation * 2) as usize;

        // Allocate buffer
        self.buf = vec![vec![Rgba8::new(0, 0, 0, 0); total_w]; total_h];

        // Copy source pixels into center region
        for y in 0..self.height as usize {
            let row = &mut self.buf[y + self.dilation as usize];
            for x in 0..self.width as usize {
                row[x + self.dilation as usize] = src.pixel(x as i32, y as i32);
            }
        }

        // Fill top/bottom dilation borders with no_color (transparent)
        let no_color = Rgba8::new(0, 0, 0, 0);
        for dy in 0..self.dilation as usize {
            // Bottom border
            let row_bot = &mut self.buf[self.dilation as usize + self.height as usize + dy];
            for x in 0..self.width as usize {
                row_bot[x + self.dilation as usize] = no_color;
            }
            // Top border
            let row_top = &mut self.buf[self.dilation as usize - dy - 1];
            for x in 0..self.width as usize {
                row_top[x + self.dilation as usize] = no_color;
            }
        }

        // Fill left/right dilation borders (wrap from opposite side)
        // C++ wraps: right border gets left-edge pixels, left border gets right-edge pixels
        for y in 0..total_h {
            for dx in 0..self.dilation as usize {
                // Right border: copy from left edge of center
                let src_val = self.buf[y][self.dilation as usize + dx];
                self.buf[y][self.dilation as usize + self.width as usize + dx] = src_val;

                // Left border: copy from right edge of center
                let src_val = self.buf[y]
                    [self.dilation as usize + self.width as usize - 1 - dx];
                self.buf[y][self.dilation as usize - 1 - dx] = src_val;
            }
        }
    }

    /// Pattern width in subpixel coordinates (for repeating).
    pub fn pattern_width(&self) -> i32 {
        self.width_hr
    }

    /// Line width in subpixel coordinates (half-height of pattern).
    pub fn line_width(&self) -> i32 {
        self.half_height_hr
    }

    /// Width in floating-point (returns height, matching C++ behavior).
    pub fn width(&self) -> f64 {
        self.height as f64
    }

    /// Get a pixel from the pattern at the given subpixel coordinates.
    ///
    /// Port of C++ `line_image_pattern::pixel`.
    #[inline]
    pub fn pixel(&self, p: &mut Rgba8, x: i32, y: i32) {
        F::pixel_high_res(
            &self.buf,
            p,
            x % self.width_hr + self.dilation_hr,
            y + self.offset_y_hr,
        );
    }
}

// ============================================================================
// line_image_pattern_pow2 — optimized version using power-of-2 masking
// ============================================================================

/// Power-of-2 optimized image pattern for line rendering.
///
/// Port of C++ `line_image_pattern_pow2<Filter>`.
/// Uses bit masking instead of modulo for pattern wrapping.
pub struct LineImagePatternPow2<F: PatternFilter> {
    base: LineImagePattern<F>,
    mask: u32,
}

impl<F: PatternFilter> LineImagePatternPow2<F> {
    pub fn new() -> Self {
        Self {
            base: LineImagePattern::new(),
            mask: LINE_SUBPIXEL_MASK as u32,
        }
    }

    pub fn with_source<S: ImagePatternSource>(src: &S) -> Self {
        let mut p = Self::new();
        p.create(src);
        p
    }

    pub fn create<S: ImagePatternSource>(&mut self, src: &S) {
        self.base.create(src);
        self.mask = 1;
        while self.mask < self.base.width {
            self.mask <<= 1;
            self.mask |= 1;
        }
        self.mask <<= LINE_SUBPIXEL_SHIFT as u32 - 1;
        self.mask |= LINE_SUBPIXEL_MASK as u32;
        self.base.width_hr = self.mask as i32 + 1;
    }

    pub fn pattern_width(&self) -> i32 {
        self.base.width_hr
    }

    pub fn line_width(&self) -> i32 {
        self.base.half_height_hr
    }

    pub fn width(&self) -> f64 {
        self.base.height as f64
    }

    #[inline]
    pub fn pixel(&self, p: &mut Rgba8, x: i32, y: i32) {
        F::pixel_high_res(
            &self.base.buf,
            p,
            (x & self.mask as i32) + self.base.dilation_hr,
            y + self.base.offset_y_hr,
        );
    }
}

// ============================================================================
// ImageLinePattern trait — common interface for line image patterns
// ============================================================================

/// Trait for image-line patterns (both standard and pow2-optimized).
///
/// This abstracts over `LineImagePattern` and `LineImagePatternPow2` so that
/// `RendererOutlineImage` can work with either type.
pub trait ImageLinePattern {
    /// Pattern width in subpixel coordinates (for repeating).
    fn pattern_width(&self) -> i32;
    /// Line width in subpixel coordinates (half-height of pattern).
    fn line_width(&self) -> i32;
    /// Width in floating-point (returns height, matching C++ behavior).
    fn width(&self) -> f64;
    /// Get a pixel from the pattern at the given subpixel coordinates.
    fn pixel(&self, p: &mut Rgba8, x: i32, y: i32);
}

impl<F: PatternFilter> ImageLinePattern for LineImagePattern<F> {
    fn pattern_width(&self) -> i32 { self.pattern_width() }
    fn line_width(&self) -> i32 { self.line_width() }
    fn width(&self) -> f64 { self.width() }
    fn pixel(&self, p: &mut Rgba8, x: i32, y: i32) { self.pixel(p, x, y) }
}

impl<F: PatternFilter> ImageLinePattern for LineImagePatternPow2<F> {
    fn pattern_width(&self) -> i32 { self.pattern_width() }
    fn line_width(&self) -> i32 { self.line_width() }
    fn width(&self) -> f64 { self.width() }
    fn pixel(&self, p: &mut Rgba8, x: i32, y: i32) { self.pixel(p, x, y) }
}

// ============================================================================
// distance_interpolator4 — for image pattern rendering
// ============================================================================

/// Distance interpolator for image-patterned lines.
///
/// Port of C++ `distance_interpolator4`.
/// Tracks perpendicular distance, start/end join distances, and pattern offset distance.
pub struct DistanceInterpolator4 {
    dx: i32,
    dy: i32,
    dx_start: i32,
    dy_start: i32,
    dx_pict: i32,
    dy_pict: i32,
    dx_end: i32,
    dy_end: i32,
    dist: i32,
    dist_start: i32,
    dist_pict: i32,
    dist_end: i32,
    len: i32,
}

impl DistanceInterpolator4 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x1: i32, y1: i32, x2: i32, y2: i32,
        sx: i32, sy: i32, ex: i32, ey: i32,
        len: i32, scale: f64, x: i32, y: i32,
    ) -> Self {
        let mut dx = x2 - x1;
        let mut dy = y2 - y1;
        let mut dx_start = line_mr(sx) - line_mr(x1);
        let mut dy_start = line_mr(sy) - line_mr(y1);
        let mut dx_end = line_mr(ex) - line_mr(x2);
        let mut dy_end = line_mr(ey) - line_mr(y2);

        let dist = iround(
            (x + LINE_SUBPIXEL_SCALE / 2 - x2) as f64 * dy as f64
                - (y + LINE_SUBPIXEL_SCALE / 2 - y2) as f64 * dx as f64,
        );

        let dist_start = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(sx)) * dy_start
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(sy)) * dx_start;

        let dist_end = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(ex)) * dy_end
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(ey)) * dx_end;

        let ilen = uround(len as f64 / scale);

        let d = len as f64 * scale;
        let dx_f = iround(((x2 - x1) << LINE_SUBPIXEL_SHIFT) as f64 / d);
        let dy_f = iround(((y2 - y1) << LINE_SUBPIXEL_SHIFT) as f64 / d);
        let dx_pict = -dy_f;
        let dy_pict = dx_f;
        let dist_pict = ((x + LINE_SUBPIXEL_SCALE / 2 - (x1 - dy_f)) as i64
            * dy_pict as i64
            - (y + LINE_SUBPIXEL_SCALE / 2 - (y1 + dx_f)) as i64
                * dx_pict as i64)
            >> LINE_SUBPIXEL_SHIFT;

        dx <<= LINE_SUBPIXEL_SHIFT;
        dy <<= LINE_SUBPIXEL_SHIFT;
        dx_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dx_end <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_end <<= LINE_MR_SUBPIXEL_SHIFT;

        Self {
            dx, dy, dx_start, dy_start, dx_pict, dy_pict, dx_end, dy_end,
            dist, dist_start, dist_pict: dist_pict as i32, dist_end,
            len: ilen,
        }
    }

    #[inline]
    pub fn inc_x(&mut self, dy: i32) {
        self.dist += self.dy;
        self.dist_start += self.dy_start;
        self.dist_pict += self.dy_pict;
        self.dist_end += self.dy_end;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
            self.dist_pict -= self.dx_pict;
            self.dist_end -= self.dx_end;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
            self.dist_pict += self.dx_pict;
            self.dist_end += self.dx_end;
        }
    }

    #[inline]
    pub fn dec_x(&mut self, dy: i32) {
        self.dist -= self.dy;
        self.dist_start -= self.dy_start;
        self.dist_pict -= self.dy_pict;
        self.dist_end -= self.dy_end;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
            self.dist_pict -= self.dx_pict;
            self.dist_end -= self.dx_end;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
            self.dist_pict += self.dx_pict;
            self.dist_end += self.dx_end;
        }
    }

    #[inline]
    pub fn inc_y(&mut self, dx: i32) {
        self.dist -= self.dx;
        self.dist_start -= self.dx_start;
        self.dist_pict -= self.dx_pict;
        self.dist_end -= self.dx_end;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
            self.dist_pict += self.dy_pict;
            self.dist_end += self.dy_end;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
            self.dist_pict -= self.dy_pict;
            self.dist_end -= self.dy_end;
        }
    }

    #[inline]
    pub fn dec_y(&mut self, dx: i32) {
        self.dist += self.dx;
        self.dist_start += self.dx_start;
        self.dist_pict += self.dx_pict;
        self.dist_end += self.dx_end;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
            self.dist_pict += self.dy_pict;
            self.dist_end += self.dy_end;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
            self.dist_pict -= self.dy_pict;
            self.dist_end -= self.dy_end;
        }
    }

    #[inline] pub fn dist(&self) -> i32 { self.dist }
    #[inline] pub fn dist_start(&self) -> i32 { self.dist_start }
    #[inline] pub fn dist_pict(&self) -> i32 { self.dist_pict }
    #[inline] pub fn dist_end(&self) -> i32 { self.dist_end }
    #[inline] pub fn dx_start(&self) -> i32 { self.dx_start }
    #[inline] pub fn dy_start(&self) -> i32 { self.dy_start }
    #[inline] pub fn dx_pict(&self) -> i32 { self.dx_pict }
    #[inline] pub fn dy_pict(&self) -> i32 { self.dy_pict }
    #[inline] pub fn dx_end(&self) -> i32 { self.dx_end }
    #[inline] pub fn dy_end(&self) -> i32 { self.dy_end }
    #[inline] pub fn len(&self) -> i32 { self.len }
}

// ============================================================================
// renderer_outline_image — renders lines with image patterns
// ============================================================================

/// Image-patterned outline renderer.
///
/// Port of C++ `renderer_outline_image<BaseRenderer, ImagePattern>`.
/// Renders anti-aliased lines using an image pattern sampled along the stroke.
///
/// The generic parameter `P` is the image line pattern type (e.g.,
/// `LineImagePattern<PatternFilterBilinearRgba>` or
/// `LineImagePatternPow2<PatternFilterBilinearRgba>`).
pub struct RendererOutlineImage<'a, PF: PixelFormat, P: ImageLinePattern> {
    ren: &'a mut RendererBase<PF>,
    pattern: &'a P,
    start: i32,
    scale_x: f64,
    clip_box: RectI,
    clipping: bool,
}

impl<'a, PF: PixelFormat, P: ImageLinePattern> RendererOutlineImage<'a, PF, P>
where
    PF::ColorType: Default + Clone + From<Rgba8>,
{
    pub fn new(ren: &'a mut RendererBase<PF>, pattern: &'a P) -> Self {
        Self {
            ren,
            pattern,
            start: 0,
            scale_x: 1.0,
            clip_box: RectI::new(0, 0, 0, 0),
            clipping: false,
        }
    }

    pub fn reset_clipping(&mut self) {
        self.clipping = false;
    }

    pub fn set_clip_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.clip_box = RectI::new(
            line_coord_sat(x1),
            line_coord_sat(y1),
            line_coord_sat(x2),
            line_coord_sat(y2),
        );
        self.clipping = true;
    }

    pub fn set_scale_x(&mut self, s: f64) {
        self.scale_x = s;
    }

    pub fn scale_x(&self) -> f64 {
        self.scale_x
    }

    pub fn set_start_x(&mut self, s: f64) {
        self.start = iround(s * LINE_SUBPIXEL_SCALE as f64);
    }

    pub fn start_x(&self) -> f64 {
        self.start as f64 / LINE_SUBPIXEL_SCALE as f64
    }

    pub fn subpixel_width(&self) -> i32 {
        self.pattern.line_width()
    }

    pub fn pattern_width(&self) -> i32 {
        self.pattern.pattern_width()
    }

    pub fn width(&self) -> f64 {
        self.subpixel_width() as f64 / LINE_SUBPIXEL_SCALE as f64
    }

    /// Render a line segment with image pattern (no clipping).
    ///
    /// Port of C++ `renderer_outline_image::line3_no_clip`.
    fn line3_no_clip(
        &mut self,
        lp: &LineParameters,
        sx: i32, sy: i32,
        ex: i32, ey: i32,
    ) {
        if lp.len > LINE_MAX_LENGTH {
            let (lp1, lp2) = lp.divide();
            let mx = lp1.x2 + (lp1.y2 - lp1.y1);
            let my = lp1.y2 - (lp1.x2 - lp1.x1);
            self.line3_no_clip(
                &lp1,
                (lp.x1 + sx) >> 1, (lp.y1 + sy) >> 1,
                mx, my,
            );
            self.line3_no_clip(
                &lp2,
                mx, my,
                (lp.x2 + ex) >> 1, (lp.y2 + ey) >> 1,
            );
            return;
        }

        let mut sx = sx;
        let mut sy = sy;
        let mut ex = ex;
        let mut ey = ey;
        fix_degenerate_bisectrix_start(lp, &mut sx, &mut sy);
        fix_degenerate_bisectrix_end(lp, &mut ex, &mut ey);

        // Run the line interpolator inline, sampling from the pattern
        // and blending into the renderer.
        self.render_line_image(lp, sx, sy, ex, ey);

        self.start += uround(lp.len as f64 / self.scale_x);
    }

    /// Core rendering: walk along the line segment and sample the pattern.
    ///
    /// This combines the C++ `line_interpolator_image` constructor and
    /// stepping methods into a single function to avoid borrow conflicts.
    #[allow(clippy::too_many_arguments)]
    fn render_line_image(
        &mut self,
        lp: &LineParameters,
        sx: i32, sy: i32,
        ex: i32, ey: i32,
    ) {
        const MAX_HW: usize = 64;
        const DIST_SIZE: usize = MAX_HW + 1;
        const COLOR_SIZE: usize = MAX_HW * 2 + 4;

        let li_init = if lp.vertical {
            Dda2LineInterpolator::new_relative(
                line_dbl_hr(lp.x2 - lp.x1),
                (lp.y2 - lp.y1).abs(),
            )
        } else {
            Dda2LineInterpolator::new_relative(
                line_dbl_hr(lp.y2 - lp.y1),
                (lp.x2 - lp.x1).abs() + 1,
            )
        };

        let di_init = DistanceInterpolator4::new(
            lp.x1, lp.y1, lp.x2, lp.y2,
            sx, sy, ex, ey,
            lp.len, self.scale_x,
            lp.x1 & !LINE_SUBPIXEL_MASK,
            lp.y1 & !LINE_SUBPIXEL_MASK,
        );

        let ix = lp.x1 >> LINE_SUBPIXEL_SHIFT;
        let iy = lp.y1 >> LINE_SUBPIXEL_SHIFT;

        let count = if lp.vertical {
            ((lp.y2 >> LINE_SUBPIXEL_SHIFT) - iy).abs()
        } else {
            ((lp.x2 >> LINE_SUBPIXEL_SHIFT) - ix).abs()
        };

        let width = self.pattern.line_width();
        let max_extent = (width + LINE_SUBPIXEL_SCALE) >> LINE_SUBPIXEL_SHIFT;
        let start = self.start + (max_extent + 2) * self.pattern.pattern_width();

        // Pre-compute distance table
        let mut dist_pos = [0i32; DIST_SIZE];
        {
            let mut dd = Dda2LineInterpolator::new_forward(
                0,
                if lp.vertical { lp.dy << LINE_SUBPIXEL_SHIFT } else { lp.dx << LINE_SUBPIXEL_SHIFT },
                lp.len,
            );
            let stop = width + LINE_SUBPIXEL_SCALE * 2;
            let mut i = 0;
            while i < MAX_HW {
                dist_pos[i] = dd.y();
                if dist_pos[i] >= stop { break; }
                dd.inc();
                i += 1;
            }
            if i <= MAX_HW {
                dist_pos[i] = 0x7FFF_0000;
            }
        }

        let mut li = li_init;
        let mut di = di_init;
        let mut x = ix;
        let mut y = iy;
        let mut old_x = ix;
        let mut old_y = iy;
        let mut step = 0i32;

        // ---- Backward stepping ----
        let mut npix = 1i32;
        if lp.vertical {
            loop {
                li.dec();
                y -= lp.inc;
                x = (lp.x1 + li.y()) >> LINE_SUBPIXEL_SHIFT;
                if lp.inc > 0 { di.dec_y(x - old_x); }
                else { di.inc_y(x - old_x); }
                old_x = x;

                let mut d1 = di.dist_start();
                let mut d2 = d1;
                let mut dx = 0usize;
                if d1 < 0 { npix += 1; }
                loop {
                    d1 += di.dy_start();
                    d2 -= di.dy_start();
                    if d1 < 0 { npix += 1; }
                    if d2 < 0 { npix += 1; }
                    dx += 1;
                    if dist_pos[dx] > width { break; }
                }
                if npix == 0 { break; }
                npix = 0;
                step -= 1;
                if step < -max_extent { break; }
            }
        } else {
            loop {
                li.dec();
                x -= lp.inc;
                y = (lp.y1 + li.y()) >> LINE_SUBPIXEL_SHIFT;
                if lp.inc > 0 { di.dec_x(y - old_y); }
                else { di.inc_x(y - old_y); }
                old_y = y;

                let mut d1 = di.dist_start();
                let mut d2 = d1;
                let mut dy = 0usize;
                if d1 < 0 { npix += 1; }
                loop {
                    d1 -= di.dx_start();
                    d2 += di.dx_start();
                    if d1 < 0 { npix += 1; }
                    if d2 < 0 { npix += 1; }
                    dy += 1;
                    if dist_pos[dy] > width { break; }
                }
                if npix == 0 { break; }
                npix = 0;
                step -= 1;
                if step < -max_extent { break; }
            }
        }

        li.adjust_forward();

        step -= max_extent;

        // ---- Forward stepping ----
        let mut colors = [Rgba8::new(0, 0, 0, 0); COLOR_SIZE];

        if lp.vertical {
            loop {
                // step_ver
                li.inc();
                y += lp.inc;
                x = (lp.x1 + li.y()) >> LINE_SUBPIXEL_SHIFT;
                if lp.inc > 0 { di.inc_y(x - old_x); }
                else { di.dec_y(x - old_x); }
                old_x = x;

                let s1 = di.dist() / lp.len;
                let s2 = -s1;
                let s1_adj = if lp.inc > 0 { -s1 } else { s1 };

                let mut dist_start = di.dist_start();
                let mut dist_pict = di.dist_pict() + start;
                let mut dist_end = di.dist_end();

                let center = MAX_HW + 2;
                let mut p0 = center;
                let mut p1 = center;

                let mut n = 0;
                colors[p1] = Rgba8::new(0, 0, 0, 0);
                if dist_end > 0 {
                    if dist_start <= 0 {
                        self.pattern.pixel(&mut colors[p1], dist_pict, s2);
                    }
                    n += 1;
                }
                p1 += 1;

                let mut dx = 1usize;
                while dx < DIST_SIZE && dist_pos[dx] - s1_adj <= width {
                    dist_start += di.dy_start();
                    dist_pict += di.dy_pict();
                    dist_end += di.dy_end();
                    colors[p1] = Rgba8::new(0, 0, 0, 0);
                    if dist_end > 0 && dist_start <= 0 {
                        let mut d = dist_pos[dx];
                        if lp.inc > 0 { d = -d; }
                        self.pattern.pixel(&mut colors[p1], dist_pict, s2 + d);
                        n += 1;
                    }
                    p1 += 1;
                    dx += 1;
                }

                dx = 1;
                dist_start = di.dist_start();
                dist_pict = di.dist_pict() + start;
                dist_end = di.dist_end();
                while dx < DIST_SIZE && dist_pos[dx] + s1_adj <= width {
                    dist_start -= di.dy_start();
                    dist_pict -= di.dy_pict();
                    dist_end -= di.dy_end();
                    p0 -= 1;
                    colors[p0] = Rgba8::new(0, 0, 0, 0);
                    if dist_end > 0 && dist_start <= 0 {
                        let mut d = dist_pos[dx];
                        if lp.inc > 0 { d = -d; }
                        self.pattern.pixel(&mut colors[p0], dist_pict, s2 - d);
                        n += 1;
                    }
                    dx += 1;
                }

                // Blend horizontal span
                let len = p1 - p0;
                if len > 0 {
                    let cvec: Vec<PF::ColorType> = colors[p0..p1]
                        .iter()
                        .map(|c| PF::ColorType::from(*c))
                        .collect();
                    self.ren.blend_color_hspan(
                        x - dx as i32 + 1, y, len as i32,
                        &cvec, &[], 255,
                    );
                }

                step += 1;
                if n == 0 || step >= count {
                    break;
                }
            }
        } else {
            loop {
                // step_hor
                li.inc();
                x += lp.inc;
                y = (lp.y1 + li.y()) >> LINE_SUBPIXEL_SHIFT;
                if lp.inc > 0 { di.inc_x(y - old_y); }
                else { di.dec_x(y - old_y); }
                old_y = y;

                let s1 = di.dist() / lp.len;
                let s2 = -s1;
                let s1_adj = if lp.inc < 0 { -s1 } else { s1 };

                let mut dist_start = di.dist_start();
                let mut dist_pict = di.dist_pict() + start;
                let mut dist_end = di.dist_end();

                let center = MAX_HW + 2;
                let mut p0 = center;
                let mut p1 = center;

                let mut n = 0;
                colors[p1] = Rgba8::new(0, 0, 0, 0);
                if dist_end > 0 {
                    if dist_start <= 0 {
                        self.pattern.pixel(&mut colors[p1], dist_pict, s2);
                    }
                    n += 1;
                }
                p1 += 1;

                let mut dy = 1usize;
                while dy < DIST_SIZE && dist_pos[dy] - s1_adj <= width {
                    dist_start -= di.dx_start();
                    dist_pict -= di.dx_pict();
                    dist_end -= di.dx_end();
                    colors[p1] = Rgba8::new(0, 0, 0, 0);
                    if dist_end > 0 && dist_start <= 0 {
                        let mut d = dist_pos[dy];
                        if lp.inc > 0 { d = -d; }
                        self.pattern.pixel(&mut colors[p1], dist_pict, s2 - d);
                        n += 1;
                    }
                    p1 += 1;
                    dy += 1;
                }

                dy = 1;
                dist_start = di.dist_start();
                dist_pict = di.dist_pict() + start;
                dist_end = di.dist_end();
                while dy < DIST_SIZE && dist_pos[dy] + s1_adj <= width {
                    dist_start += di.dx_start();
                    dist_pict += di.dx_pict();
                    dist_end += di.dx_end();
                    p0 -= 1;
                    colors[p0] = Rgba8::new(0, 0, 0, 0);
                    if dist_end > 0 && dist_start <= 0 {
                        let mut d = dist_pos[dy];
                        if lp.inc > 0 { d = -d; }
                        self.pattern.pixel(&mut colors[p0], dist_pict, s2 + d);
                        n += 1;
                    }
                    dy += 1;
                }

                // Blend vertical span
                let len = p1 - p0;
                if len > 0 {
                    let cvec: Vec<PF::ColorType> = colors[p0..p1]
                        .iter()
                        .map(|c| PF::ColorType::from(*c))
                        .collect();
                    self.ren.blend_color_vspan(
                        x, y - dy as i32 + 1, len as i32,
                        &cvec, &[], 255,
                    );
                }

                step += 1;
                if n == 0 || step >= count {
                    break;
                }
            }
        }
    }
}

impl<'a, PF: PixelFormat, P: ImageLinePattern> OutlineAaRenderer
    for RendererOutlineImage<'a, PF, P>
where
    PF::ColorType: Default + Clone + From<Rgba8>,
{
    fn accurate_join_only(&self) -> bool {
        true
    }

    // Image pattern renderer only supports line3 (with both joins).
    fn line0(&mut self, _lp: &LineParameters) {}
    fn line1(&mut self, _lp: &LineParameters, _sx: i32, _sy: i32) {}
    fn line2(&mut self, _lp: &LineParameters, _ex: i32, _ey: i32) {}

    fn line3(&mut self, lp: &LineParameters, sx: i32, sy: i32, ex: i32, ey: i32) {
        if self.clipping {
            let mut x1 = lp.x1;
            let mut y1 = lp.y1;
            let mut x2 = lp.x2;
            let mut y2 = lp.y2;
            let flags = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &self.clip_box);
            let start = self.start;
            if (flags & 4) == 0 {
                if flags != 0 {
                    let lp2 = LineParameters::new(
                        x1, y1, x2, y2,
                        uround(calc_distance_i(x1, y1, x2, y2)),
                    );
                    let mut sx = sx;
                    let mut sy = sy;
                    let mut ex = ex;
                    let mut ey = ey;
                    if flags & 1 != 0 {
                        self.start += uround(
                            calc_distance_i(lp.x1, lp.y1, x1, y1) / self.scale_x,
                        );
                        sx = x1 + (y2 - y1);
                        sy = y1 - (x2 - x1);
                    } else {
                        while (sx - lp.x1).abs() + (sy - lp.y1).abs() > lp2.len {
                            sx = (lp.x1 + sx) >> 1;
                            sy = (lp.y1 + sy) >> 1;
                        }
                    }
                    if flags & 2 != 0 {
                        ex = x2 + (y2 - y1);
                        ey = y2 - (x2 - x1);
                    } else {
                        while (ex - lp.x2).abs() + (ey - lp.y2).abs() > lp2.len {
                            ex = (lp.x2 + ex) >> 1;
                            ey = (lp.y2 + ey) >> 1;
                        }
                    }
                    self.line3_no_clip(&lp2, sx, sy, ex, ey);
                } else {
                    self.line3_no_clip(lp, sx, sy, ex, ey);
                }
            }
            self.start = start + uround(lp.len as f64 / self.scale_x);
        } else {
            self.line3_no_clip(lp, sx, sy, ex, ey);
        }
    }

    fn semidot(&mut self, _cmp: fn(i32) -> bool, _xc1: i32, _yc1: i32, _xc2: i32, _yc2: i32) {}
    fn pie(&mut self, _xc: i32, _yc: i32, _x1: i32, _y1: i32, _x2: i32, _y2: i32) {}
}

// ============================================================================
// Helper
// ============================================================================

fn calc_distance_i(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    (dx * dx + dy * dy).sqrt()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple solid-color pattern source for testing.
    struct SolidPatternSource {
        w: u32,
        h: u32,
        color: Rgba8,
    }

    impl ImagePatternSource for SolidPatternSource {
        fn width(&self) -> f64 { self.w as f64 }
        fn height(&self) -> f64 { self.h as f64 }
        fn pixel(&self, _x: i32, _y: i32) -> Rgba8 { self.color }
    }

    #[test]
    fn test_line_image_pattern_creation() {
        use crate::pattern_filters_rgba::PatternFilterBilinearRgba;
        let src = SolidPatternSource { w: 16, h: 8, color: Rgba8::new(255, 0, 0, 255) };
        let pat = LineImagePattern::<PatternFilterBilinearRgba>::with_source(&src);
        assert!(pat.pattern_width() > 0);
        assert!(pat.line_width() > 0);
    }

    #[test]
    fn test_line_image_pattern_pixel() {
        use crate::pattern_filters_rgba::PatternFilterNn;
        let src = SolidPatternSource { w: 16, h: 8, color: Rgba8::new(255, 128, 64, 200) };
        let pat = LineImagePattern::<PatternFilterNn>::with_source(&src);
        let mut p = Rgba8::new(0, 0, 0, 0);
        pat.pixel(&mut p, 0, 0);
        assert!(p.a > 0, "Expected non-zero alpha");
    }

    #[test]
    fn test_distance_interpolator4() {
        let di = DistanceInterpolator4::new(
            0, 0, 256, 0,
            -256, 0, 512, 0,
            256, 1.0, 0, 0,
        );
        assert_ne!(di.len(), 0);
    }

    #[test]
    fn test_line_image_pattern_pow2() {
        use crate::pattern_filters_rgba::PatternFilterBilinearRgba;
        let src = SolidPatternSource { w: 16, h: 8, color: Rgba8::new(0, 255, 0, 255) };
        let pat = LineImagePatternPow2::<PatternFilterBilinearRgba>::with_source(&src);
        assert!(pat.pattern_width() > 0);
        assert!(pat.line_width() > 0);
        let mut p = Rgba8::new(0, 0, 0, 0);
        pat.pixel(&mut p, 0, 0);
        assert!(p.a > 0);
    }

    #[test]
    fn test_renderer_outline_image_basic() {
        use crate::pattern_filters_rgba::PatternFilterBilinearRgba;
        use crate::pixfmt_rgba::PixfmtRgba32;
        use crate::rendering_buffer::RowAccessor;

        let w = 100u32;
        let h = 100u32;
        let stride = (w * 4) as i32;
        let mut buf = vec![255u8; (h * w * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe { ra.attach(buf.as_mut_ptr(), w, h, stride); }
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);

        let src = SolidPatternSource { w: 32, h: 16, color: Rgba8::new(255, 0, 0, 255) };
        let pat = LineImagePattern::<PatternFilterBilinearRgba>::with_source(&src);

        let mut ren = RendererOutlineImage::new(&mut rb, &pat);
        ren.set_scale_x(1.0);
        ren.set_start_x(0.0);

        // Draw a horizontal line
        let lp = LineParameters::new(
            10 * 256, 50 * 256,
            90 * 256, 50 * 256,
            80 * 256,
        );
        ren.line3(
            &lp,
            10 * 256 + (0), 50 * 256 - (80 * 256),
            90 * 256 + (0), 50 * 256 - (80 * 256),
        );

        // Check some pixels were drawn (red channel should be non-zero near the line)
        let mut found = false;
        let pf2 = PixfmtRgba32::new(&mut ra);
        let rb2 = RendererBase::new(pf2);
        for y in 42..=58 {
            for x in 10..90 {
                let p = rb2.pixel(x, y);
                if p.r > 100 {
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        assert!(found, "Expected red pixels near row 50");
    }
}

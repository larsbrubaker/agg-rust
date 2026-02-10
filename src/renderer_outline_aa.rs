//! Anti-aliased outline renderer.
//!
//! Port of `agg_renderer_outline_aa.h` + `agg_line_profile_aa.cpp`.
//! Renders anti-aliased lines with sub-pixel precision using distance
//! interpolation and a configurable width profile.
//!
//! Copyright 2025.

use crate::basics::{iround, RectI};
use crate::dda_line::Dda2LineInterpolator;
use crate::ellipse_bresenham::EllipseBresenhamInterpolator;
use crate::line_aa_basics::*;
use crate::math::fast_sqrt;
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;

// ============================================================================
// Line Profile
// ============================================================================

// These must match C++ line_profile_aa::subpixel_scale_e
const PROFILE_SUBPIXEL_SHIFT: i32 = LINE_SUBPIXEL_SHIFT; // 8
const PROFILE_SUBPIXEL_SCALE: i32 = 1 << PROFILE_SUBPIXEL_SHIFT; // 256
const PROFILE_AA_SHIFT: i32 = 8;
const PROFILE_AA_SCALE: i32 = 1 << PROFILE_AA_SHIFT; // 256
const PROFILE_AA_MASK: i32 = PROFILE_AA_SCALE - 1; // 255

/// Anti-aliased line width profile.
///
/// Port of C++ `line_profile_aa`. Builds a lookup table mapping perpendicular
/// distance from line center → coverage value, with configurable width and
/// gamma correction.
pub struct LineProfileAa {
    profile: Vec<u8>,
    gamma: [u8; 256],
    subpixel_width: i32,
    min_width: f64,
    smoother_width: f64,
}

impl LineProfileAa {
    pub fn new() -> Self {
        let mut s = Self {
            profile: Vec::new(),
            gamma: [0u8; 256],
            subpixel_width: 0,
            min_width: 1.0,
            smoother_width: 1.0,
        };
        // Identity gamma
        for i in 0..256 {
            s.gamma[i] = i as u8;
        }
        s
    }

    /// Create with a specific width.
    pub fn with_width(w: f64) -> Self {
        let mut s = Self::new();
        s.set_width(w);
        s
    }

    pub fn min_width(&self) -> f64 {
        self.min_width
    }
    pub fn smoother_width(&self) -> f64 {
        self.smoother_width
    }
    pub fn subpixel_width(&self) -> i32 {
        self.subpixel_width
    }

    pub fn set_min_width(&mut self, w: f64) {
        self.min_width = w;
    }
    pub fn set_smoother_width(&mut self, w: f64) {
        self.smoother_width = w;
    }

    /// Set the line width (in pixels).
    /// Port of C++ `line_profile_aa::width`.
    pub fn set_width(&mut self, mut w: f64) {
        if w < 0.0 { w = 0.0; }

        if w < self.smoother_width {
            w += w;
        } else {
            w += self.smoother_width;
        }

        w *= 0.5;
        w -= self.smoother_width;
        let mut s = self.smoother_width;
        if w < 0.0 {
            s += w;
            w = 0.0;
        }
        self.build_profile(w, s);
    }

    /// Apply gamma function.
    pub fn set_gamma<F: Fn(f64) -> f64>(&mut self, gamma_fn: F) {
        for i in 0..256 {
            self.gamma[i] = iround(gamma_fn(i as f64 / 255.0) * 255.0) as u8;
        }
    }

    /// Lookup coverage for a given perpendicular distance.
    #[inline]
    pub fn value(&self, dist: i32) -> u8 {
        let idx = (dist + PROFILE_SUBPIXEL_SCALE * 2) as usize;
        if idx < self.profile.len() {
            self.profile[idx]
        } else {
            0
        }
    }

    fn profile_size(&self) -> usize {
        self.profile.len()
    }

    /// Port of C++ `line_profile_aa::set` + `line_profile_aa::profile`.
    fn build_profile(&mut self, center_width: f64, smoother_width: f64) {
        let mut base_val = 1.0f64;
        let mut cw = center_width;
        let mut sw = smoother_width;

        if cw == 0.0 {
            cw = 1.0 / PROFILE_SUBPIXEL_SCALE as f64;
        }
        if sw == 0.0 {
            sw = 1.0 / PROFILE_SUBPIXEL_SCALE as f64;
        }

        let width = cw + sw;
        if width < self.min_width {
            let k = width / self.min_width;
            base_val *= k;
            cw /= k;
            sw /= k;
        }

        // C++ profile(): m_subpixel_width = uround(w * subpixel_scale)
        self.subpixel_width = iround((cw + sw) * PROFILE_SUBPIXEL_SCALE as f64);
        let size = self.subpixel_width as usize + PROFILE_SUBPIXEL_SCALE as usize * 6;
        self.profile.resize(size, 0);

        let subpixel_center_width = (cw * PROFILE_SUBPIXEL_SCALE as f64) as usize;
        let subpixel_smoother_width = (sw * PROFILE_SUBPIXEL_SCALE as f64) as usize;

        let ch_center = PROFILE_SUBPIXEL_SCALE as usize * 2;

        // Fill center region with full-alpha value
        let val = self.gamma[(base_val * PROFILE_AA_MASK as f64) as usize];
        for i in 0..subpixel_center_width {
            self.profile[ch_center + i] = val;
        }

        // Fill smoother region with falloff
        let ch_smoother = ch_center + subpixel_center_width;
        for i in 0..subpixel_smoother_width {
            let k = base_val - base_val * (i as f64 / subpixel_smoother_width as f64);
            self.profile[ch_smoother + i] =
                self.gamma[(k * PROFILE_AA_MASK as f64) as usize];
        }

        // Fill remaining with gamma[0]
        let n_smoother = size
            - subpixel_smoother_width
            - subpixel_center_width
            - PROFILE_SUBPIXEL_SCALE as usize * 2;
        let gamma_zero = self.gamma[0];
        for i in 0..n_smoother {
            self.profile[ch_smoother + subpixel_smoother_width + i] = gamma_zero;
        }

        // Mirror to the left (C++: *--ch = *ch_center++)
        let mut src = ch_center;
        let mut dst = ch_center;
        for _ in 0..(PROFILE_SUBPIXEL_SCALE as usize * 2) {
            if dst == 0 || src >= self.profile.len() {
                break;
            }
            dst -= 1;
            let v = self.profile[src];
            self.profile[dst] = v;
            src += 1;
        }
    }
}

impl Default for LineProfileAa {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Distance Interpolators
// ============================================================================

/// Distance interpolator 0 — for semidot/pie (distance from point).
///
/// Port of C++ `distance_interpolator0`.
/// Uses `line_mr()` (medium resolution) for dx/dy.
pub struct DistanceInterpolator0 {
    dx: i32,
    dy: i32,
    dist: i32,
}

impl DistanceInterpolator0 {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, x: i32, y: i32) -> Self {
        let mut dx = line_mr(x2) - line_mr(x1);
        let mut dy = line_mr(y2) - line_mr(y1);
        let dist = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(x2)) * dy
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(y2)) * dx;
        dx <<= LINE_MR_SUBPIXEL_SHIFT;
        dy <<= LINE_MR_SUBPIXEL_SHIFT;
        Self { dx, dy, dist }
    }

    #[inline]
    pub fn inc_x(&mut self) {
        self.dist += self.dy;
    }

    #[inline]
    pub fn dist(&self) -> i32 {
        self.dist
    }
}

/// Distance interpolator 00 — for pie (two rays).
///
/// Port of C++ `distance_interpolator00`.
/// Uses `line_mr()` (medium resolution) for dx/dy.
pub struct DistanceInterpolator00 {
    dx1: i32,
    dy1: i32,
    dx2: i32,
    dy2: i32,
    dist1: i32,
    dist2: i32,
}

impl DistanceInterpolator00 {
    pub fn new(
        xc: i32, yc: i32,
        x1: i32, y1: i32,
        x2: i32, y2: i32,
        x: i32, y: i32,
    ) -> Self {
        let mut dx1 = line_mr(x1) - line_mr(xc);
        let mut dy1 = line_mr(y1) - line_mr(yc);
        let mut dx2 = line_mr(x2) - line_mr(xc);
        let mut dy2 = line_mr(y2) - line_mr(yc);
        let dist1 = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(x1)) * dy1
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(y1)) * dx1;
        let dist2 = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(x2)) * dy2
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(y2)) * dx2;
        dx1 <<= LINE_MR_SUBPIXEL_SHIFT;
        dy1 <<= LINE_MR_SUBPIXEL_SHIFT;
        dx2 <<= LINE_MR_SUBPIXEL_SHIFT;
        dy2 <<= LINE_MR_SUBPIXEL_SHIFT;
        Self { dx1, dy1, dx2, dy2, dist1, dist2 }
    }

    #[inline]
    pub fn inc_x(&mut self) {
        self.dist1 += self.dy1;
        self.dist2 += self.dy2;
    }

    #[inline]
    pub fn dist1(&self) -> i32 {
        self.dist1
    }
    #[inline]
    pub fn dist2(&self) -> i32 {
        self.dist2
    }
}

/// Distance interpolator 1 — basic perpendicular distance tracker.
///
/// Port of C++ `distance_interpolator1`.
pub struct DistanceInterpolator1 {
    dx: i32,
    dy: i32,
    dist: i32,
}

impl DistanceInterpolator1 {
    pub fn new(x1: i32, y1: i32, x2: i32, y2: i32, x: i32, y: i32) -> Self {
        let mut dx = x2 - x1;
        let mut dy = y2 - y1;
        let dist = iround(
            (x + LINE_SUBPIXEL_SCALE / 2 - x2) as f64 * dy as f64
                - (y + LINE_SUBPIXEL_SCALE / 2 - y2) as f64 * dx as f64,
        );
        dx <<= LINE_SUBPIXEL_SHIFT;
        dy <<= LINE_SUBPIXEL_SHIFT;
        Self { dx, dy, dist }
    }

    #[inline]
    pub fn inc_x(&mut self, dy: i32) {
        self.dist += self.dy;
        if dy > 0 {
            self.dist -= self.dx;
        }
        if dy < 0 {
            self.dist += self.dx;
        }
    }

    #[inline]
    pub fn dec_x(&mut self, dy: i32) {
        self.dist -= self.dy;
        if dy > 0 {
            self.dist -= self.dx;
        }
        if dy < 0 {
            self.dist += self.dx;
        }
    }

    #[inline]
    pub fn inc_y(&mut self, dx: i32) {
        self.dist -= self.dx;
        if dx > 0 {
            self.dist += self.dy;
        }
        if dx < 0 {
            self.dist -= self.dy;
        }
    }

    #[inline]
    pub fn dec_y(&mut self, dx: i32) {
        self.dist += self.dx;
        if dx > 0 {
            self.dist += self.dy;
        }
        if dx < 0 {
            self.dist -= self.dy;
        }
    }

    #[inline]
    pub fn dist(&self) -> i32 {
        self.dist
    }
    #[inline]
    pub fn dx(&self) -> i32 {
        self.dx
    }
    #[inline]
    pub fn dy(&self) -> i32 {
        self.dy
    }
}

/// Distance interpolator 2 — tracks main distance + start or end join distance.
///
/// Port of C++ `distance_interpolator2`.
pub struct DistanceInterpolator2 {
    dx: i32,
    dy: i32,
    dx_start: i32,
    dy_start: i32,
    dist: i32,
    dist_start: i32,
}

impl DistanceInterpolator2 {
    /// Start join variant.
    pub fn new_start(
        x1: i32, y1: i32, x2: i32, y2: i32, sx: i32, sy: i32, x: i32, y: i32,
    ) -> Self {
        let mut dx = x2 - x1;
        let mut dy = y2 - y1;
        let mut dx_start = line_mr(sx) - line_mr(x1);
        let mut dy_start = line_mr(sy) - line_mr(y1);
        let dist = iround(
            (x + LINE_SUBPIXEL_SCALE / 2 - x2) as f64 * dy as f64
                - (y + LINE_SUBPIXEL_SCALE / 2 - y2) as f64 * dx as f64,
        );
        let dist_start = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(sx)) * dy_start
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(sy)) * dx_start;
        dx <<= LINE_SUBPIXEL_SHIFT;
        dy <<= LINE_SUBPIXEL_SHIFT;
        dx_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_start <<= LINE_MR_SUBPIXEL_SHIFT;
        Self { dx, dy, dx_start, dy_start, dist, dist_start }
    }

    /// End join variant.
    pub fn new_end(
        x1: i32, y1: i32, x2: i32, y2: i32, ex: i32, ey: i32, x: i32, y: i32,
    ) -> Self {
        let mut dx = x2 - x1;
        let mut dy = y2 - y1;
        let mut dx_start = line_mr(ex) - line_mr(x2);
        let mut dy_start = line_mr(ey) - line_mr(y2);
        let dist = iround(
            (x + LINE_SUBPIXEL_SCALE / 2 - x2) as f64 * dy as f64
                - (y + LINE_SUBPIXEL_SCALE / 2 - y2) as f64 * dx as f64,
        );
        let dist_start = (line_mr(x + LINE_SUBPIXEL_SCALE / 2) - line_mr(ex)) * dy_start
            - (line_mr(y + LINE_SUBPIXEL_SCALE / 2) - line_mr(ey)) * dx_start;
        dx <<= LINE_SUBPIXEL_SHIFT;
        dy <<= LINE_SUBPIXEL_SHIFT;
        dx_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_start <<= LINE_MR_SUBPIXEL_SHIFT;
        Self { dx, dy, dx_start, dy_start, dist, dist_start }
    }

    #[inline]
    pub fn inc_x(&mut self, dy: i32) {
        self.dist += self.dy;
        self.dist_start += self.dy_start;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
        }
    }

    #[inline]
    pub fn dec_x(&mut self, dy: i32) {
        self.dist -= self.dy;
        self.dist_start -= self.dy_start;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
        }
    }

    #[inline]
    pub fn inc_y(&mut self, dx: i32) {
        self.dist -= self.dx;
        self.dist_start -= self.dx_start;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
        }
    }

    #[inline]
    pub fn dec_y(&mut self, dx: i32) {
        self.dist += self.dx;
        self.dist_start += self.dx_start;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
        }
    }

    #[inline]
    pub fn dist(&self) -> i32 {
        self.dist
    }
    #[inline]
    pub fn dist_start(&self) -> i32 {
        self.dist_start
    }
    #[inline]
    pub fn dist_end(&self) -> i32 {
        self.dist_start
    }
    #[inline]
    pub fn dx_start(&self) -> i32 {
        self.dx_start
    }
    #[inline]
    pub fn dy_start(&self) -> i32 {
        self.dy_start
    }
    #[inline]
    pub fn dx_end(&self) -> i32 {
        self.dx_start
    }
    #[inline]
    pub fn dy_end(&self) -> i32 {
        self.dy_start
    }
}

/// Distance interpolator 3 — tracks main + start + end join distances.
///
/// Port of C++ `distance_interpolator3`.
pub struct DistanceInterpolator3 {
    dx: i32,
    dy: i32,
    dx_start: i32,
    dy_start: i32,
    dx_end: i32,
    dy_end: i32,
    dist: i32,
    dist_start: i32,
    dist_end: i32,
}

impl DistanceInterpolator3 {
    pub fn new(
        x1: i32, y1: i32, x2: i32, y2: i32,
        sx: i32, sy: i32, ex: i32, ey: i32,
        x: i32, y: i32,
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

        dx <<= LINE_SUBPIXEL_SHIFT;
        dy <<= LINE_SUBPIXEL_SHIFT;
        dx_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_start <<= LINE_MR_SUBPIXEL_SHIFT;
        dx_end <<= LINE_MR_SUBPIXEL_SHIFT;
        dy_end <<= LINE_MR_SUBPIXEL_SHIFT;

        Self {
            dx, dy, dx_start, dy_start, dx_end, dy_end, dist, dist_start, dist_end,
        }
    }

    #[inline]
    pub fn inc_x(&mut self, dy: i32) {
        self.dist += self.dy;
        self.dist_start += self.dy_start;
        self.dist_end += self.dy_end;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
            self.dist_end -= self.dx_end;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
            self.dist_end += self.dx_end;
        }
    }

    #[inline]
    pub fn dec_x(&mut self, dy: i32) {
        self.dist -= self.dy;
        self.dist_start -= self.dy_start;
        self.dist_end -= self.dy_end;
        if dy > 0 {
            self.dist -= self.dx;
            self.dist_start -= self.dx_start;
            self.dist_end -= self.dx_end;
        }
        if dy < 0 {
            self.dist += self.dx;
            self.dist_start += self.dx_start;
            self.dist_end += self.dx_end;
        }
    }

    #[inline]
    pub fn inc_y(&mut self, dx: i32) {
        self.dist -= self.dx;
        self.dist_start -= self.dx_start;
        self.dist_end -= self.dx_end;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
            self.dist_end += self.dy_end;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
            self.dist_end -= self.dy_end;
        }
    }

    #[inline]
    pub fn dec_y(&mut self, dx: i32) {
        self.dist += self.dx;
        self.dist_start += self.dx_start;
        self.dist_end += self.dx_end;
        if dx > 0 {
            self.dist += self.dy;
            self.dist_start += self.dy_start;
            self.dist_end += self.dy_end;
        }
        if dx < 0 {
            self.dist -= self.dy;
            self.dist_start -= self.dy_start;
            self.dist_end -= self.dy_end;
        }
    }

    #[inline]
    pub fn dist(&self) -> i32 {
        self.dist
    }
    #[inline]
    pub fn dist_start(&self) -> i32 {
        self.dist_start
    }
    #[inline]
    pub fn dist_end(&self) -> i32 {
        self.dist_end
    }
    #[inline]
    pub fn dx_start(&self) -> i32 {
        self.dx_start
    }
    #[inline]
    pub fn dy_start(&self) -> i32 {
        self.dy_start
    }
    #[inline]
    pub fn dx_end(&self) -> i32 {
        self.dx_end
    }
    #[inline]
    pub fn dy_end(&self) -> i32 {
        self.dy_end
    }
}

// ============================================================================
// Outline AA Renderer Trait
// ============================================================================

pub const MAX_HALF_WIDTH: usize = 64;

/// Trait for renderers used with `RasterizerOutlineAa`.
///
/// Both `RendererOutlineAa` (solid color) and `RendererOutlineImage`
/// (image pattern) implement this trait, allowing the rasterizer to
/// work with either renderer type.
///
/// Port of the C++ template interface used by `rasterizer_outline_aa`.
pub trait OutlineAaRenderer {
    /// Returns true if this renderer only supports accurate (miter) joins.
    /// Image pattern renderers return true; solid AA renderers return false.
    fn accurate_join_only(&self) -> bool;

    /// Render a simple line segment (no join information).
    fn line0(&mut self, lp: &LineParameters);

    /// Render a line segment with start join bisectrix.
    fn line1(&mut self, lp: &LineParameters, sx: i32, sy: i32);

    /// Render a line segment with end join bisectrix.
    fn line2(&mut self, lp: &LineParameters, ex: i32, ey: i32);

    /// Render a line segment with both start and end join bisectrices.
    fn line3(&mut self, lp: &LineParameters, sx: i32, sy: i32, ex: i32, ey: i32);

    /// Render a semi-circular dot (for round caps).
    fn semidot(&mut self, cmp: fn(i32) -> bool, xc1: i32, yc1: i32, xc2: i32, yc2: i32);

    /// Render a pie slice (for round joins).
    fn pie(&mut self, xc: i32, yc: i32, x1: i32, y1: i32, x2: i32, y2: i32);
}

// ============================================================================
// Renderer Outline AA
// ============================================================================

/// Anti-aliased outline renderer.
///
/// Port of C++ `renderer_outline_aa<BaseRenderer>`.
/// Renders anti-aliased lines using a distance interpolation technique
/// with configurable width profile.
pub struct RendererOutlineAa<'a, PF: PixelFormat> {
    ren: &'a mut RendererBase<PF>,
    profile: &'a LineProfileAa,
    color: PF::ColorType,
    clip_box: RectI,
    clipping: bool,
}

impl<'a, PF: PixelFormat> RendererOutlineAa<'a, PF>
where
    PF::ColorType: Default + Clone,
{
    pub fn new(ren: &'a mut RendererBase<PF>, profile: &'a LineProfileAa) -> Self {
        Self {
            ren,
            profile,
            color: PF::ColorType::default(),
            clip_box: RectI::new(0, 0, 0, 0),
            clipping: false,
        }
    }

    pub fn ren(&self) -> &RendererBase<PF> {
        self.ren
    }

    pub fn set_color(&mut self, c: PF::ColorType) {
        self.color = c;
    }

    pub fn color(&self) -> &PF::ColorType {
        &self.color
    }

    pub fn subpixel_width(&self) -> i32 {
        self.profile.subpixel_width()
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

    pub fn reset_clipping(&mut self) {
        self.clipping = false;
    }

    #[inline]
    fn cover(&self, d: i32) -> u8 {
        self.profile.value(d)
    }

    /// Render a simple line (no joins).
    pub fn line0(&mut self, lp: &LineParameters) {
        if self.clipping {
            let (mut x1, mut y1, mut x2, mut y2) = (lp.x1, lp.y1, lp.x2, lp.y2);
            let flags = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &self.clip_box);
            if flags >= 4 {
                return;
            }
            if flags != 0 {
                let lp2 = LineParameters::new(
                    x1, y1, x2, y2,
                    uround(calc_distance_i(x1, y1, x2, y2)),
                );
                self.line0_no_clip(&lp2);
                return;
            }
        }
        self.line0_no_clip(lp);
    }

    fn line0_no_clip(&mut self, lp: &LineParameters) {
        if lp.len > LINE_MAX_LENGTH {
            let (lp1, lp2) = lp.divide();
            self.line0_no_clip(&lp1);
            self.line0_no_clip(&lp2);
            return;
        }

        let li = LineInterpolatorAa0::new(lp, self.profile.subpixel_width());
        if li.count > 0 {
            if lp.vertical {
                self.draw_line0_ver(li, lp);
            } else {
                self.draw_line0_hor(li, lp);
            }
        }
    }

    fn draw_line0_hor(&mut self, mut li: LineInterpolatorAa0, lp: &LineParameters) {
        while let Some(span) = li.step_hor(self.profile, lp) {
            let x = li.x();
            let y = li.y() - span.offset as i32 + 1;
            self.ren.blend_solid_vspan(
                x, y, span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    fn draw_line0_ver(&mut self, mut li: LineInterpolatorAa0, lp: &LineParameters) {
        while let Some(span) = li.step_ver(self.profile, lp) {
            let x = li.x() - span.offset as i32 + 1;
            let y = li.y();
            self.ren.blend_solid_hspan(
                x, y, span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    /// Render line with start join.
    pub fn line1(&mut self, lp: &LineParameters, sx: i32, sy: i32) {
        if self.clipping {
            let (mut x1, mut y1, mut x2, mut y2) = (lp.x1, lp.y1, lp.x2, lp.y2);
            let flags = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &self.clip_box);
            if flags >= 4 {
                return;
            }
            if flags != 0 {
                let lp2 = LineParameters::new(
                    x1, y1, x2, y2,
                    uround(calc_distance_i(x1, y1, x2, y2)),
                );
                if flags & 1 != 0 {
                    // Start was clipped — use line0 instead
                    self.line0_no_clip(&lp2);
                } else {
                    self.line1_no_clip(&lp2, sx, sy);
                }
                return;
            }
        }
        self.line1_no_clip(lp, sx, sy);
    }

    fn line1_no_clip(&mut self, lp: &LineParameters, mut sx: i32, mut sy: i32) {
        if lp.len > LINE_MAX_LENGTH {
            let (lp1, lp2) = lp.divide();
            self.line1_no_clip(
                &lp1,
                (lp.x1 + sx) >> 1,
                (lp.y1 + sy) >> 1,
            );
            self.line1_no_clip(
                &lp2,
                lp1.x2 + (lp1.y2 - lp1.y1),
                lp1.y2 - (lp1.x2 - lp1.x1),
            );
            return;
        }

        fix_degenerate_bisectrix_start(lp, &mut sx, &mut sy);
        let li = LineInterpolatorAa1::new(lp, sx, sy, self.profile.subpixel_width());
        if lp.vertical {
            self.draw_line1_ver(li, lp);
        } else {
            self.draw_line1_hor(li, lp);
        }
    }

    fn draw_line1_hor(&mut self, mut li: LineInterpolatorAa1, lp: &LineParameters) {
        while let Some(span) = li.step_hor(self.profile, lp) {
            self.ren.blend_solid_vspan(
                li.x(), li.y() - span.offset as i32 + 1, span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    fn draw_line1_ver(&mut self, mut li: LineInterpolatorAa1, lp: &LineParameters) {
        while let Some(span) = li.step_ver(self.profile, lp) {
            self.ren.blend_solid_hspan(
                li.x() - span.offset as i32 + 1, li.y(), span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    /// Render line with end join.
    pub fn line2(&mut self, lp: &LineParameters, ex: i32, ey: i32) {
        if self.clipping {
            let (mut x1, mut y1, mut x2, mut y2) = (lp.x1, lp.y1, lp.x2, lp.y2);
            let flags = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &self.clip_box);
            if flags >= 4 {
                return;
            }
            if flags != 0 {
                let lp2 = LineParameters::new(
                    x1, y1, x2, y2,
                    uround(calc_distance_i(x1, y1, x2, y2)),
                );
                if flags & 2 != 0 {
                    self.line0_no_clip(&lp2);
                } else {
                    self.line2_no_clip(&lp2, ex, ey);
                }
                return;
            }
        }
        self.line2_no_clip(lp, ex, ey);
    }

    fn line2_no_clip(&mut self, lp: &LineParameters, mut ex: i32, mut ey: i32) {
        if lp.len > LINE_MAX_LENGTH {
            let (lp1, lp2) = lp.divide();
            self.line2_no_clip(
                &lp1,
                lp1.x2 + (lp1.y2 - lp1.y1),
                lp1.y2 - (lp1.x2 - lp1.x1),
            );
            self.line2_no_clip(
                &lp2,
                (lp.x2 + ex) >> 1,
                (lp.y2 + ey) >> 1,
            );
            return;
        }

        fix_degenerate_bisectrix_end(lp, &mut ex, &mut ey);
        let li = LineInterpolatorAa2::new(lp, ex, ey, self.profile.subpixel_width());
        if lp.vertical {
            self.draw_line2_ver(li, lp);
        } else {
            self.draw_line2_hor(li, lp);
        }
    }

    fn draw_line2_hor(&mut self, mut li: LineInterpolatorAa2, lp: &LineParameters) {
        while let Some(span) = li.step_hor(self.profile, lp) {
            self.ren.blend_solid_vspan(
                li.x(), li.y() - span.offset as i32 + 1, span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    fn draw_line2_ver(&mut self, mut li: LineInterpolatorAa2, lp: &LineParameters) {
        while let Some(span) = li.step_ver(self.profile, lp) {
            self.ren.blend_solid_hspan(
                li.x() - span.offset as i32 + 1, li.y(), span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    /// Render line with both joins.
    pub fn line3(
        &mut self,
        lp: &LineParameters,
        sx: i32,
        sy: i32,
        ex: i32,
        ey: i32,
    ) {
        if self.clipping {
            let (mut x1, mut y1, mut x2, mut y2) = (lp.x1, lp.y1, lp.x2, lp.y2);
            let flags = clip_line_segment(&mut x1, &mut y1, &mut x2, &mut y2, &self.clip_box);
            if flags >= 4 {
                return;
            }
            if flags != 0 {
                let lp2 = LineParameters::new(
                    x1, y1, x2, y2,
                    uround(calc_distance_i(x1, y1, x2, y2)),
                );
                match flags & 3 {
                    3 => self.line0_no_clip(&lp2),
                    1 => self.line2_no_clip(&lp2, ex, ey),
                    2 => self.line1_no_clip(&lp2, sx, sy),
                    _ => self.line3_no_clip(&lp2, sx, sy, ex, ey),
                }
                return;
            }
        }
        self.line3_no_clip(lp, sx, sy, ex, ey);
    }

    fn line3_no_clip(
        &mut self,
        lp: &LineParameters,
        mut sx: i32,
        mut sy: i32,
        mut ex: i32,
        mut ey: i32,
    ) {
        if lp.len > LINE_MAX_LENGTH {
            let (lp1, lp2) = lp.divide();
            let mx = lp1.x2 + (lp1.y2 - lp1.y1);
            let my = lp1.y2 - (lp1.x2 - lp1.x1);
            self.line3_no_clip(
                &lp1,
                (lp.x1 + sx) >> 1,
                (lp.y1 + sy) >> 1,
                mx, my,
            );
            self.line3_no_clip(
                &lp2,
                mx, my,
                (lp.x2 + ex) >> 1,
                (lp.y2 + ey) >> 1,
            );
            return;
        }

        fix_degenerate_bisectrix_start(lp, &mut sx, &mut sy);
        fix_degenerate_bisectrix_end(lp, &mut ex, &mut ey);
        let li = LineInterpolatorAa3::new(lp, sx, sy, ex, ey, self.profile.subpixel_width());
        if lp.vertical {
            self.draw_line3_ver(li, lp);
        } else {
            self.draw_line3_hor(li, lp);
        }
    }

    fn draw_line3_hor(&mut self, mut li: LineInterpolatorAa3, lp: &LineParameters) {
        while let Some(span) = li.step_hor(self.profile, lp) {
            self.ren.blend_solid_vspan(
                li.x(), li.y() - span.offset as i32 + 1, span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    fn draw_line3_ver(&mut self, mut li: LineInterpolatorAa3, lp: &LineParameters) {
        while let Some(span) = li.step_ver(self.profile, lp) {
            self.ren.blend_solid_hspan(
                li.x() - span.offset as i32 + 1, li.y(), span.len as i32, &self.color,
                &li.covers[span.p0..span.p0 + span.len],
            );
        }
    }

    /// Render a semi-circular dot (for round caps).
    /// Port of C++ `semidot`.
    pub fn semidot<F: Fn(i32) -> bool>(
        &mut self,
        cmp: F,
        xc1: i32,
        yc1: i32,
        xc2: i32,
        yc2: i32,
    ) {
        let r = ((self.profile.subpixel_width() + LINE_SUBPIXEL_MASK) >> LINE_SUBPIXEL_SHIFT) as i32;
        if r < 1 {
            return;
        }
        let mut ei = EllipseBresenhamInterpolator::new(r, r);
        let mut dx = 0i32;
        let mut dy = -r;
        let mut dy0 = dy;
        let mut dx0 = dx;

        let x = xc1 >> LINE_SUBPIXEL_SHIFT;
        let y = yc1 >> LINE_SUBPIXEL_SHIFT;

        loop {
            dx += ei.dx();
            dy += ei.dy();
            if dy != dy0 {
                self.semidot_hline(&cmp, xc1, yc1, xc2, yc2, x - dx0, y + dy0, x + dx0);
                self.semidot_hline(&cmp, xc1, yc1, xc2, yc2, x - dx0, y - dy0, x + dx0);
            }
            dx0 = dx;
            dy0 = dy;
            ei.next();
            if dy >= 0 {
                break;
            }
        }
        self.semidot_hline(&cmp, xc1, yc1, xc2, yc2, x - dx0, y + dy0, x + dx0);
    }

    /// Port of C++ `semidot_hline`.
    /// x1, y1, x2 are in pixel coordinates; xc1/yc1/xc2/yc2 are subpixel.
    fn semidot_hline<F: Fn(i32) -> bool>(
        &mut self,
        cmp: &F,
        xc1: i32,
        yc1: i32,
        xc2: i32,
        yc2: i32,
        mut x1: i32,
        y1: i32,
        x2: i32,
    ) {
        let mut covers = [0u8; MAX_HALF_WIDTH * 2 + 4];
        let mut p0 = 0usize;
        let mut p1 = 0usize;

        // C++ passes pixel coords << subpixel_shift to DI0
        let x = x1 << LINE_SUBPIXEL_SHIFT;
        let y = y1 << LINE_SUBPIXEL_SHIFT;
        let w = self.profile.subpixel_width();

        let mut di = DistanceInterpolator0::new(xc1, yc1, xc2, yc2, x, y);

        // Offset to pixel center for distance calculation
        let mut dx = x + LINE_SUBPIXEL_SCALE / 2 - xc1;
        let dy = y + LINE_SUBPIXEL_SCALE / 2 - yc1;

        loop {
            let d = fast_sqrt((dx * dx + dy * dy) as u32) as i32;
            covers[p1] = 0;
            if cmp(di.dist()) && d <= w {
                covers[p1] = self.cover(d);
            }
            p1 += 1;
            dx += LINE_SUBPIXEL_SCALE;
            di.inc_x();
            x1 += 1;
            if x1 > x2 {
                break;
            }
        }

        self.ren.blend_solid_hspan(
            x1 - (p1 as i32), y1, (p1 - p0) as i32, &self.color, &covers[p0..p1],
        );
    }

    /// Render a pie slice (for round joins between two line segments).
    /// Port of C++ `pie`.
    pub fn pie(
        &mut self,
        xc: i32,
        yc: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    ) {
        let r = ((self.profile.subpixel_width() + LINE_SUBPIXEL_MASK) >> LINE_SUBPIXEL_SHIFT) as i32;
        if r < 1 {
            return;
        }
        let mut ei = EllipseBresenhamInterpolator::new(r, r);
        let mut dx = 0i32;
        let mut dy = -r;
        let mut dy0 = dy;
        let mut dx0 = dx;

        let x = xc >> LINE_SUBPIXEL_SHIFT;
        let y = yc >> LINE_SUBPIXEL_SHIFT;

        loop {
            dx += ei.dx();
            dy += ei.dy();
            if dy != dy0 {
                self.pie_hline(xc, yc, x1, y1, x2, y2, x - dx0, y + dy0, x + dx0);
                self.pie_hline(xc, yc, x1, y1, x2, y2, x - dx0, y - dy0, x + dx0);
            }
            dx0 = dx;
            dy0 = dy;
            ei.next();
            if dy >= 0 {
                break;
            }
        }
        self.pie_hline(xc, yc, x1, y1, x2, y2, x - dx0, y + dy0, x + dx0);
    }

    /// Port of C++ `pie_hline`.
    /// xh1, yh1, xh2 are in pixel coordinates; xc/yc/xp1/yp1/xp2/yp2 are subpixel.
    fn pie_hline(
        &mut self,
        xc: i32,
        yc: i32,
        xp1: i32,
        yp1: i32,
        xp2: i32,
        yp2: i32,
        mut xh1: i32,
        yh1: i32,
        xh2: i32,
    ) {
        let mut covers = [0u8; MAX_HALF_WIDTH * 2 + 4];
        let mut p0 = 0usize;
        let mut p1 = 0usize;

        let x = xh1 << LINE_SUBPIXEL_SHIFT;
        let y = yh1 << LINE_SUBPIXEL_SHIFT;
        let w = self.profile.subpixel_width();

        let mut di = DistanceInterpolator00::new(
            xc, yc, xp1, yp1, xp2, yp2, x, y,
        );

        let mut dx = x + LINE_SUBPIXEL_SCALE / 2 - xc;
        let dy = y + LINE_SUBPIXEL_SCALE / 2 - yc;

        let xh0 = xh1;
        loop {
            let d = fast_sqrt((dx * dx + dy * dy) as u32) as i32;
            covers[p1] = 0;
            if di.dist1() <= 0 && di.dist2() > 0 && d <= w {
                covers[p1] = self.cover(d);
            }
            p1 += 1;
            dx += LINE_SUBPIXEL_SCALE;
            di.inc_x();
            xh1 += 1;
            if xh1 > xh2 {
                break;
            }
        }

        self.ren.blend_solid_hspan(
            xh0, yh1, (p1 - p0) as i32, &self.color, &covers[p0..p1],
        );
    }
}

// Implementation of OutlineAaRenderer for RendererOutlineAa.
impl<'a, PF: PixelFormat> OutlineAaRenderer for RendererOutlineAa<'a, PF>
where
    PF::ColorType: Default + Clone,
{
    fn accurate_join_only(&self) -> bool {
        false
    }

    fn line0(&mut self, lp: &LineParameters) {
        self.line0(lp);
    }

    fn line1(&mut self, lp: &LineParameters, sx: i32, sy: i32) {
        self.line1(lp, sx, sy);
    }

    fn line2(&mut self, lp: &LineParameters, ex: i32, ey: i32) {
        self.line2(lp, ex, ey);
    }

    fn line3(&mut self, lp: &LineParameters, sx: i32, sy: i32, ex: i32, ey: i32) {
        self.line3(lp, sx, sy, ex, ey);
    }

    fn semidot(&mut self, cmp: fn(i32) -> bool, xc1: i32, yc1: i32, xc2: i32, yc2: i32) {
        self.semidot(cmp, xc1, yc1, xc2, yc2);
    }

    fn pie(&mut self, xc: i32, yc: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.pie(xc, yc, x1, y1, x2, y2);
    }
}

// ============================================================================
// Helpers
// ============================================================================

#[inline]
fn calc_distance_i(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    (dx * dx + dy * dy).sqrt()
}

#[inline]
fn uround(v: f64) -> i32 {
    (v + 0.5) as i32
}

// ============================================================================
// Line Interpolator AA base functionality
// ============================================================================

const COVER_SIZE: usize = MAX_HALF_WIDTH * 2 + 4;
const DIST_SIZE: usize = MAX_HALF_WIDTH + 1;

/// Span result from a line interpolator step.
struct LineSpan {
    /// Index into covers array where the span starts.
    p0: usize,
    /// Number of cover values in the span.
    len: usize,
    /// For step_hor: vertical offset (dy) for blend_solid_vspan positioning.
    /// For step_ver: horizontal offset (dx) for blend_solid_hspan positioning.
    offset: i32,
}

// Common initialization for all line interpolator types
fn init_line_interpolator_base(lp: &LineParameters, width: i32) -> (
    Dda2LineInterpolator, // li
    i32, // x
    i32, // y
    i32, // count
    i32, // len
    i32, // max_extent
    [i32; DIST_SIZE], // dist table
) {
    let max_extent = (width + LINE_SUBPIXEL_MASK) >> LINE_SUBPIXEL_SHIFT;

    let x;
    let y;
    let count;
    let li;

    if lp.vertical {
        x = lp.x1 >> LINE_SUBPIXEL_SHIFT;
        y = lp.y1 >> LINE_SUBPIXEL_SHIFT;
        count = ((lp.y2 >> LINE_SUBPIXEL_SHIFT) - y).abs();
        li = Dda2LineInterpolator::new_relative(
            line_dbl_hr(lp.x2 - lp.x1),
            (lp.y2 - lp.y1).abs(),
        );
    } else {
        x = lp.x1 >> LINE_SUBPIXEL_SHIFT;
        y = lp.y1 >> LINE_SUBPIXEL_SHIFT;
        count = ((lp.x2 >> LINE_SUBPIXEL_SHIFT) - x).abs();
        li = Dda2LineInterpolator::new_relative(
            line_dbl_hr(lp.y2 - lp.y1),
            (lp.x2 - lp.x1).abs() + 1,
        );
    };

    let len = if lp.vertical == (lp.inc > 0) { -lp.len } else { lp.len };

    // Pre-compute distance table
    let mut dist = [0i32; DIST_SIZE];
    let mut dd = Dda2LineInterpolator::new_forward(
        0,
        if lp.vertical { lp.dy << LINE_SUBPIXEL_SHIFT } else { lp.dx << LINE_SUBPIXEL_SHIFT },
        lp.len,
    );
    let stop = width + LINE_SUBPIXEL_SCALE * 2;
    let mut i = 0;
    while i < MAX_HALF_WIDTH {
        dist[i] = dd.y();
        if dist[i] >= stop {
            break;
        }
        dd.inc();
        i += 1;
    }
    if i < DIST_SIZE {
        dist[i] = 0x7FFF_0000;
    }

    (li, x, y, count, len, max_extent, dist)
}

/// Line interpolator for AA line type 0 (no joins).
/// Port of C++ `line_interpolator_aa0`.
struct LineInterpolatorAa0 {
    di: DistanceInterpolator1,
    li: Dda2LineInterpolator,
    x: i32,
    y: i32,
    old_x: i32,
    old_y: i32,
    count: i32,
    width: i32,
    max_extent: i32,
    len: i32,
    step: i32,
    dist: [i32; DIST_SIZE],
    pub covers: [u8; COVER_SIZE],
}

impl LineInterpolatorAa0 {
    fn new(lp: &LineParameters, subpixel_width: i32) -> Self {
        let (mut li, x, y, count, len, max_extent, dist) =
            init_line_interpolator_base(lp, subpixel_width);

        // C++: m_di(lp.x1, lp.y1, lp.x2, lp.y2,
        //          lp.x1 & ~line_subpixel_mask, lp.y1 & ~line_subpixel_mask)
        let di = DistanceInterpolator1::new(
            lp.x1, lp.y1, lp.x2, lp.y2,
            lp.x1 & !LINE_SUBPIXEL_MASK,
            lp.y1 & !LINE_SUBPIXEL_MASK,
        );

        li.adjust_forward();

        Self {
            di,
            li,
            x,
            y,
            old_x: x,
            old_y: y,
            count,
            width: subpixel_width,
            max_extent,
            len,
            step: 0,
            dist,
            covers: [0u8; COVER_SIZE],
        }
    }

    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }

    fn step_hor(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        // Check at the BEGINNING — C++ does blend first, then `return ++step < count`.
        // We must check before work so that the LAST step still returns Some(span).
        if self.step >= self.count { return None; }

        self.li.inc();
        self.x += lp.inc;
        self.y = (lp.y1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;

        if lp.inc > 0 {
            self.di.inc_x(self.y - self.old_y);
        } else {
            self.di.dec_x(self.y - self.old_y);
        }
        self.old_y = self.y;

        let s1 = self.di.dist() / self.len;

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        self.covers[p1] = profile.value(s1) as u8;
        p1 += 1;

        let mut dy = 1usize;
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] - s1;
            if dist > self.width { break; }
            self.covers[p1] = profile.value(dist) as u8;
            p1 += 1;
            dy += 1;
        }

        let mut dy = 1usize;
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] + s1;
            if dist > self.width { break; }
            p0 -= 1;
            self.covers[p0] = profile.value(dist) as u8;
            dy += 1;
        }

        self.step += 1;

        Some(LineSpan {
            p0,
            len: p1 - p0,
            offset: dy as i32,
        })
    }

    fn step_ver(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.y += lp.inc;
        self.x = (lp.x1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;

        if lp.inc > 0 {
            self.di.inc_y(self.x - self.old_x);
        } else {
            self.di.dec_y(self.x - self.old_x);
        }
        self.old_x = self.x;

        let s1 = self.di.dist() / self.len;

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        self.covers[p1] = profile.value(s1) as u8;
        p1 += 1;

        let mut dx = 1usize;
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] - s1;
            if dist > self.width { break; }
            self.covers[p1] = profile.value(dist) as u8;
            p1 += 1;
            dx += 1;
        }

        let mut dx = 1usize;
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] + s1;
            if dist > self.width { break; }
            p0 -= 1;
            self.covers[p0] = profile.value(dist) as u8;
            dx += 1;
        }

        self.step += 1;

        Some(LineSpan {
            p0,
            len: p1 - p0,
            offset: dx as i32,
        })
    }
}

/// Line interpolator for AA line type 1 (start join).
/// Port of C++ `line_interpolator_aa1`.
struct LineInterpolatorAa1 {
    di: DistanceInterpolator2,
    li: Dda2LineInterpolator,
    x: i32,
    y: i32,
    old_x: i32,
    old_y: i32,
    count: i32,
    width: i32,
    max_extent: i32,
    len: i32,
    step: i32,
    dist: [i32; DIST_SIZE],
    pub covers: [u8; COVER_SIZE],
}

impl LineInterpolatorAa1 {
    fn new(lp: &LineParameters, sx: i32, sy: i32, subpixel_width: i32) -> Self {
        let (mut li, mut x, mut y, count, len, max_extent, dist) =
            init_line_interpolator_base(lp, subpixel_width);

        let mut di = DistanceInterpolator2::new_start(
            lp.x1, lp.y1, lp.x2, lp.y2, sx, sy,
            lp.x1 & !LINE_SUBPIXEL_MASK,
            lp.y1 & !LINE_SUBPIXEL_MASK,
        );

        let mut old_x = x;
        let mut old_y = y;
        let mut step = 0i32;

        // Backward stepping to find where start join begins
        let mut npix = 1i32;

        if lp.vertical {
            loop {
                li.dec();
                y -= lp.inc;
                x = (lp.x1 + li.y()) >> LINE_SUBPIXEL_SHIFT;

                if lp.inc > 0 {
                    di.dec_y(x - old_x);
                } else {
                    di.inc_y(x - old_x);
                }
                old_x = x;

                let mut dist1_start = di.dist_start();
                let mut dist2_start = dist1_start;

                let mut dx = 0;
                if dist1_start < 0 { npix += 1; }
                loop {
                    dist1_start += di.dy_start();
                    dist2_start -= di.dy_start();
                    if dist1_start < 0 { npix += 1; }
                    if dist2_start < 0 { npix += 1; }
                    dx += 1;
                    if dist[dx as usize] > subpixel_width { break; }
                }
                step -= 1;
                if npix == 0 { break; }
                npix = 0;
                if step < -max_extent { break; }
            }
        } else {
            loop {
                li.dec();
                x -= lp.inc;
                y = (lp.y1 + li.y()) >> LINE_SUBPIXEL_SHIFT;

                if lp.inc > 0 {
                    di.dec_x(y - old_y);
                } else {
                    di.inc_x(y - old_y);
                }
                old_y = y;

                let mut dist1_start = di.dist_start();
                let mut dist2_start = dist1_start;

                let mut dy = 0;
                if dist1_start < 0 { npix += 1; }
                loop {
                    dist1_start -= di.dx_start();
                    dist2_start += di.dx_start();
                    if dist1_start < 0 { npix += 1; }
                    if dist2_start < 0 { npix += 1; }
                    dy += 1;
                    if dist[dy as usize] > subpixel_width { break; }
                }
                step -= 1;
                if npix == 0 { break; }
                npix = 0;
                if step < -max_extent { break; }
            }
        }

        li.adjust_forward();

        Self {
            di, li, x, y, old_x, old_y,
            count, width: subpixel_width, max_extent, len, step,
            dist, covers: [0u8; COVER_SIZE],
        }
    }

    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }

    fn step_hor(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.x += lp.inc;
        self.y = (lp.y1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_x(self.y - self.old_y); }
        else { self.di.dec_x(self.y - self.old_y); }
        self.old_y = self.y;

        let s1 = self.di.dist() / self.len;
        let mut dist_start = self.di.dist_start();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        self.covers[p1] = 0;
        if dist_start <= 0 {
            self.covers[p1] = profile.value(s1) as u8;
        }
        p1 += 1;

        let mut dy = 1usize;
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] - s1;
            if dist > self.width { break; }
            dist_start -= self.di.dx_start();
            self.covers[p1] = 0;
            if dist_start <= 0 {
                self.covers[p1] = profile.value(dist) as u8;
            }
            p1 += 1;
            dy += 1;
        }

        let mut dy = 1usize;
        dist_start = self.di.dist_start();
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] + s1;
            if dist > self.width { break; }
            dist_start += self.di.dx_start();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_start <= 0 {
                self.covers[p0] = profile.value(dist) as u8;
            }
            dy += 1;
        }

        self.step += 1;

        Some(LineSpan { p0, len: p1 - p0, offset: dy as i32 })
    }

    fn step_ver(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.y += lp.inc;
        self.x = (lp.x1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_y(self.x - self.old_x); }
        else { self.di.dec_y(self.x - self.old_x); }
        self.old_x = self.x;

        let s1 = self.di.dist() / self.len;
        let mut dist_start = self.di.dist_start();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        self.covers[p1] = 0;
        if dist_start <= 0 {
            self.covers[p1] = profile.value(s1) as u8;
        }
        p1 += 1;

        let mut dx = 1usize;
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] - s1;
            if dist > self.width { break; }
            dist_start += self.di.dy_start();
            self.covers[p1] = 0;
            if dist_start <= 0 {
                self.covers[p1] = profile.value(dist) as u8;
            }
            p1 += 1;
            dx += 1;
        }

        let mut dx = 1usize;
        dist_start = self.di.dist_start();
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] + s1;
            if dist > self.width { break; }
            dist_start -= self.di.dy_start();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_start <= 0 {
                self.covers[p0] = profile.value(dist) as u8;
            }
            dx += 1;
        }

        self.step += 1;

        Some(LineSpan { p0, len: p1 - p0, offset: dx as i32 })
    }
}

/// Line interpolator for AA line type 2 (end join).
/// Port of C++ `line_interpolator_aa2`.
struct LineInterpolatorAa2 {
    di: DistanceInterpolator2,
    li: Dda2LineInterpolator,
    x: i32,
    y: i32,
    old_x: i32,
    old_y: i32,
    count: i32,
    width: i32,
    max_extent: i32,
    len: i32,
    step: i32,
    dist: [i32; DIST_SIZE],
    pub covers: [u8; COVER_SIZE],
}

impl LineInterpolatorAa2 {
    fn new(lp: &LineParameters, ex: i32, ey: i32, subpixel_width: i32) -> Self {
        let (mut li, x, y, count, len, max_extent, dist) =
            init_line_interpolator_base(lp, subpixel_width);

        let di = DistanceInterpolator2::new_end(
            lp.x1, lp.y1, lp.x2, lp.y2, ex, ey,
            lp.x1 & !LINE_SUBPIXEL_MASK,
            lp.y1 & !LINE_SUBPIXEL_MASK,
        );

        li.adjust_forward();
        let step = 0 - max_extent;

        Self {
            di, li, x, y, old_x: x, old_y: y,
            count, width: subpixel_width, max_extent, len, step,
            dist, covers: [0u8; COVER_SIZE],
        }
    }

    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }

    fn step_hor(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.x += lp.inc;
        self.y = (lp.y1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_x(self.y - self.old_y); }
        else { self.di.dec_x(self.y - self.old_y); }
        self.old_y = self.y;

        let s1 = self.di.dist() / self.len;
        let mut dist_end = self.di.dist_end();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        let mut npix = 0;
        self.covers[p1] = 0;
        if dist_end > 0 {
            self.covers[p1] = profile.value(s1) as u8;
            npix += 1;
        }
        p1 += 1;

        let mut dy = 1usize;
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] - s1;
            if dist > self.width { break; }
            dist_end -= self.di.dx_end();
            self.covers[p1] = 0;
            if dist_end > 0 {
                self.covers[p1] = profile.value(dist) as u8;
                npix += 1;
            }
            p1 += 1;
            dy += 1;
        }

        let mut dy = 1usize;
        dist_end = self.di.dist_end();
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] + s1;
            if dist > self.width { break; }
            dist_end += self.di.dx_end();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_end > 0 {
                self.covers[p0] = profile.value(dist) as u8;
                npix += 1;
            }
            dy += 1;
        }

        self.step += 1;
        if npix == 0 { return None; }

        Some(LineSpan { p0, len: p1 - p0, offset: dy as i32 })
    }

    fn step_ver(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.y += lp.inc;
        self.x = (lp.x1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_y(self.x - self.old_x); }
        else { self.di.dec_y(self.x - self.old_x); }
        self.old_x = self.x;

        let s1 = self.di.dist() / self.len;
        let mut dist_end = self.di.dist_end();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        let mut npix = 0;
        self.covers[p1] = 0;
        if dist_end > 0 {
            self.covers[p1] = profile.value(s1) as u8;
            npix += 1;
        }
        p1 += 1;

        let mut dx = 1usize;
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] - s1;
            if dist > self.width { break; }
            dist_end += self.di.dy_end();
            self.covers[p1] = 0;
            if dist_end > 0 {
                self.covers[p1] = profile.value(dist) as u8;
                npix += 1;
            }
            p1 += 1;
            dx += 1;
        }

        let mut dx = 1usize;
        dist_end = self.di.dist_end();
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] + s1;
            if dist > self.width { break; }
            dist_end -= self.di.dy_end();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_end > 0 {
                self.covers[p0] = profile.value(dist) as u8;
                npix += 1;
            }
            dx += 1;
        }

        self.step += 1;
        if npix == 0 { return None; }

        Some(LineSpan { p0, len: p1 - p0, offset: dx as i32 })
    }
}

/// Line interpolator for AA line type 3 (both joins).
/// Port of C++ `line_interpolator_aa3`.
struct LineInterpolatorAa3 {
    di: DistanceInterpolator3,
    li: Dda2LineInterpolator,
    x: i32,
    y: i32,
    old_x: i32,
    old_y: i32,
    count: i32,
    width: i32,
    max_extent: i32,
    len: i32,
    step: i32,
    dist: [i32; DIST_SIZE],
    pub covers: [u8; COVER_SIZE],
}

impl LineInterpolatorAa3 {
    fn new(
        lp: &LineParameters,
        sx: i32, sy: i32, ex: i32, ey: i32,
        subpixel_width: i32,
    ) -> Self {
        let (mut li, mut x, mut y, count, len, max_extent, dist) =
            init_line_interpolator_base(lp, subpixel_width);

        let mut di = DistanceInterpolator3::new(
            lp.x1, lp.y1, lp.x2, lp.y2,
            sx, sy, ex, ey,
            lp.x1 & !LINE_SUBPIXEL_MASK,
            lp.y1 & !LINE_SUBPIXEL_MASK,
        );

        let mut old_x = x;
        let mut old_y = y;
        let mut step = 0i32;

        // Backward stepping (same as AA1 but uses DI3)
        let mut npix = 1i32;

        if lp.vertical {
            loop {
                li.dec();
                y -= lp.inc;
                x = (lp.x1 + li.y()) >> LINE_SUBPIXEL_SHIFT;

                if lp.inc > 0 {
                    di.dec_y(x - old_x);
                } else {
                    di.inc_y(x - old_x);
                }
                old_x = x;

                let mut dist1_start = di.dist_start();
                let mut dist2_start = dist1_start;

                let mut dx = 0;
                if dist1_start < 0 { npix += 1; }
                loop {
                    dist1_start += di.dy_start();
                    dist2_start -= di.dy_start();
                    if dist1_start < 0 { npix += 1; }
                    if dist2_start < 0 { npix += 1; }
                    dx += 1;
                    if dist[dx as usize] > subpixel_width { break; }
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

                if lp.inc > 0 {
                    di.dec_x(y - old_y);
                } else {
                    di.inc_x(y - old_y);
                }
                old_y = y;

                let mut dist1_start = di.dist_start();
                let mut dist2_start = dist1_start;

                let mut dy = 0;
                if dist1_start < 0 { npix += 1; }
                loop {
                    dist1_start -= di.dx_start();
                    dist2_start += di.dx_start();
                    if dist1_start < 0 { npix += 1; }
                    if dist2_start < 0 { npix += 1; }
                    dy += 1;
                    if dist[dy as usize] > subpixel_width { break; }
                }
                if npix == 0 { break; }
                npix = 0;
                step -= 1;
                if step < -max_extent { break; }
            }
        }

        li.adjust_forward();
        step -= max_extent;

        Self {
            di, li, x, y, old_x, old_y,
            count, width: subpixel_width, max_extent, len, step,
            dist, covers: [0u8; COVER_SIZE],
        }
    }

    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }

    fn step_hor(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.x += lp.inc;
        self.y = (lp.y1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_x(self.y - self.old_y); }
        else { self.di.dec_x(self.y - self.old_y); }
        self.old_y = self.y;

        let s1 = self.di.dist() / self.len;
        let mut dist_start = self.di.dist_start();
        let mut dist_end = self.di.dist_end();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        let mut npix = 0;
        self.covers[p1] = 0;
        if dist_end > 0 {
            if dist_start <= 0 {
                self.covers[p1] = profile.value(s1) as u8;
            }
            npix += 1;
        }
        p1 += 1;

        let mut dy = 1usize;
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] - s1;
            if dist > self.width { break; }
            dist_start -= self.di.dx_start();
            dist_end -= self.di.dx_end();
            self.covers[p1] = 0;
            if dist_end > 0 && dist_start <= 0 {
                self.covers[p1] = profile.value(dist) as u8;
                npix += 1;
            }
            p1 += 1;
            dy += 1;
        }

        let mut dy = 1usize;
        dist_start = self.di.dist_start();
        dist_end = self.di.dist_end();
        loop {
            if dy >= DIST_SIZE { break; }
            let dist = self.dist[dy] + s1;
            if dist > self.width { break; }
            dist_start += self.di.dx_start();
            dist_end += self.di.dx_end();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_end > 0 && dist_start <= 0 {
                self.covers[p0] = profile.value(dist) as u8;
                npix += 1;
            }
            dy += 1;
        }

        self.step += 1;
        if npix == 0 { return None; }

        Some(LineSpan { p0, len: p1 - p0, offset: dy as i32 })
    }

    fn step_ver(&mut self, profile: &LineProfileAa, lp: &LineParameters) -> Option<LineSpan> {
        if self.step >= self.count { return None; }

        self.li.inc();
        self.y += lp.inc;
        self.x = (lp.x1 + self.li.y()) >> LINE_SUBPIXEL_SHIFT;
        if lp.inc > 0 { self.di.inc_y(self.x - self.old_x); }
        else { self.di.dec_y(self.x - self.old_x); }
        self.old_x = self.x;

        let s1 = self.di.dist() / self.len;
        let mut dist_start = self.di.dist_start();
        let mut dist_end = self.di.dist_end();

        let center = MAX_HALF_WIDTH + 2;
        let mut p0 = center;
        let mut p1 = center;

        let mut npix = 0;
        self.covers[p1] = 0;
        if dist_end > 0 {
            if dist_start <= 0 {
                self.covers[p1] = profile.value(s1) as u8;
            }
            npix += 1;
        }
        p1 += 1;

        let mut dx = 1usize;
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] - s1;
            if dist > self.width { break; }
            dist_start += self.di.dy_start();
            dist_end += self.di.dy_end();
            self.covers[p1] = 0;
            if dist_end > 0 && dist_start <= 0 {
                self.covers[p1] = profile.value(dist) as u8;
                npix += 1;
            }
            p1 += 1;
            dx += 1;
        }

        let mut dx = 1usize;
        dist_start = self.di.dist_start();
        dist_end = self.di.dist_end();
        loop {
            if dx >= DIST_SIZE { break; }
            let dist = self.dist[dx] + s1;
            if dist > self.width { break; }
            dist_start -= self.di.dy_start();
            dist_end -= self.di.dy_end();
            p0 -= 1;
            self.covers[p0] = 0;
            if dist_end > 0 && dist_start <= 0 {
                self.covers[p0] = profile.value(dist) as u8;
                npix += 1;
            }
            dx += 1;
        }

        self.step += 1;
        if npix == 0 { return None; }

        Some(LineSpan { p0, len: p1 - p0, offset: dx as i32 })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
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

    #[test]
    fn test_line_profile_creation() {
        let p = LineProfileAa::with_width(2.0);
        assert!(p.subpixel_width() > 0);
        assert!(p.profile_size() > 0);
    }

    #[test]
    fn test_line_profile_value_center() {
        let p = LineProfileAa::with_width(3.0);
        // Center should have high coverage
        let center = p.value(0);
        assert!(center > 200, "center coverage={center} should be > 200");
    }

    #[test]
    fn test_line_profile_value_edge() {
        let p = LineProfileAa::with_width(3.0);
        // Far from center should have zero coverage.
        // Width=3.0 → half-width in subpixel is ~512, so dist=800 should be zero.
        let far = p.value(800);
        assert_eq!(far, 0);
    }

    #[test]
    fn test_distance_interpolator1() {
        let di = DistanceInterpolator1::new(0, 0, 256, 0, 128, 128);
        // Distance from (128,128) to line (0,0)-(256,0) should be negative
        // (below the line) since line goes right and point is below
        assert_ne!(di.dist(), 0);
    }

    #[test]
    fn test_render_line0() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);
        let prof = LineProfileAa::with_width(2.0);
        let mut ren_aa = RendererOutlineAa::new(&mut ren, &prof);
        ren_aa.set_color(Rgba8::new(255, 0, 0, 255));

        // Draw a horizontal line
        let lp = LineParameters::new(
            10 * 256, 50 * 256,
            90 * 256, 50 * 256,
            80 * 256,
        );
        ren_aa.line0(&lp);

        // Check that some pixels were drawn somewhere near the line
        let mut found = false;
        for y in 48..=52 {
            for x in 0..100 {
                let p = ren_aa.ren().pixel(x, y);
                if p.r > 0 {
                    found = true;
                    break;
                }
            }
            if found { break; }
        }
        assert!(found, "Expected red pixels near row 50");
    }

    #[test]
    fn test_render_line_diagonal() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);
        let prof = LineProfileAa::with_width(1.5);
        let mut ren_aa = RendererOutlineAa::new(&mut ren, &prof);
        ren_aa.set_color(Rgba8::new(0, 255, 0, 255));

        let lp = LineParameters::new(
            10 * 256, 10 * 256,
            90 * 256, 90 * 256,
            uround(calc_distance_i(10 * 256, 10 * 256, 90 * 256, 90 * 256)),
        );
        ren_aa.line0(&lp);

        let p = ren_aa.ren().pixel(50, 50);
        assert!(p.g > 0, "Expected green pixel at (50,50), got g={}", p.g);
    }
}

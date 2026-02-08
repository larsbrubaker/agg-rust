//! Perspective span interpolators.
//!
//! Port of `agg_span_interpolator_persp.h`.
//! Two variants for perspective coordinate interpolation:
//! - `SpanInterpolatorPerspExact` — uses exact perspective iterator
//! - `SpanInterpolatorPerspLerp` — uses linear interpolation with DDA

use crate::basics::{iround, uround};
use crate::dda_line::Dda2LineInterpolator;
use crate::span_interpolator_linear::SpanInterpolator;
use crate::trans_perspective::{PerspectiveIteratorX, TransPerspective};

/// Subpixel precision for perspective interpolation.
const SUBPIXEL_SHIFT: u32 = 8;
const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;

/// Helper: compute local scale from perspective transform at a point.
/// Returns the scale factor as a subpixel-shifted integer, then right-shifted.
fn calc_scale(
    xt: f64,
    yt: f64,
    x_src: f64,
    y_src: f64,
    trans_inv: &TransPerspective,
    dx_offset: f64,
    dy_offset: f64,
) -> i32 {
    let mut dx = xt + dx_offset;
    let mut dy = yt + dy_offset;
    trans_inv.transform(&mut dx, &mut dy);
    dx -= x_src;
    dy -= y_src;
    (uround(SUBPIXEL_SCALE as f64 / (dx * dx + dy * dy).sqrt()) >> SUBPIXEL_SHIFT) as i32
}

// ============================================================================
// SpanInterpolatorPerspExact
// ============================================================================

/// Exact perspective span interpolator.
///
/// Port of C++ `span_interpolator_persp_exact<SubpixelShift>`.
/// Uses `PerspectiveIteratorX` for exact perspective division at each pixel.
/// Scale factors are linearly interpolated via DDA.
pub struct SpanInterpolatorPerspExact {
    trans_dir: TransPerspective,
    trans_inv: TransPerspective,
    iterator: PerspectiveIteratorX,
    scale_x: Dda2LineInterpolator,
    scale_y: Dda2LineInterpolator,
}

impl SpanInterpolatorPerspExact {
    pub fn new() -> Self {
        Self {
            trans_dir: TransPerspective::new(),
            trans_inv: TransPerspective::new(),
            iterator: PerspectiveIteratorX::default_new(),
            scale_x: Dda2LineInterpolator::new_forward(0, 0, 1),
            scale_y: Dda2LineInterpolator::new_forward(0, 0, 1),
        }
    }

    pub fn new_quad_to_quad(src: &[f64; 8], dst: &[f64; 8]) -> Self {
        let mut s = Self::new();
        s.quad_to_quad(src, dst);
        s
    }

    pub fn new_rect_to_quad(x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) -> Self {
        let mut s = Self::new();
        s.rect_to_quad(x1, y1, x2, y2, quad);
        s
    }

    pub fn new_quad_to_rect(quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut s = Self::new();
        s.quad_to_rect(quad, x1, y1, x2, y2);
        s
    }

    pub fn quad_to_quad(&mut self, src: &[f64; 8], dst: &[f64; 8]) {
        self.trans_dir.quad_to_quad(src, dst);
        self.trans_inv.quad_to_quad(dst, src);
    }

    pub fn rect_to_quad(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) {
        let src = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(&src, quad);
    }

    pub fn quad_to_rect(&mut self, quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) {
        let dst = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(quad, &dst);
    }

    pub fn is_valid(&self) -> bool {
        self.trans_dir.is_valid()
    }

    pub fn local_scale(&self, x: &mut i32, y: &mut i32) {
        *x = self.scale_x.y();
        *y = self.scale_y.y();
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        self.trans_dir.transform(x, y);
    }

    pub fn trans_dir(&self) -> &TransPerspective {
        &self.trans_dir
    }

    pub fn trans_inv(&self) -> &TransPerspective {
        &self.trans_inv
    }
}

impl SpanInterpolator for SpanInterpolatorPerspExact {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        self.iterator = self.trans_dir.begin(x, y, 1.0);
        let xt = self.iterator.x;
        let yt = self.iterator.y;

        let delta = 1.0 / SUBPIXEL_SCALE as f64;

        let sx1 = calc_scale(xt, yt, x, y, &self.trans_inv, delta, 0.0);
        let sy1 = calc_scale(xt, yt, x, y, &self.trans_inv, 0.0, delta);

        let x2 = x + len as f64;
        let mut xt2 = x2;
        let mut yt2 = y;
        self.trans_dir.transform(&mut xt2, &mut yt2);

        let sx2 = calc_scale(xt2, yt2, x2, y, &self.trans_inv, delta, 0.0);
        let sy2 = calc_scale(xt2, yt2, x2, y, &self.trans_inv, 0.0, delta);

        self.scale_x = Dda2LineInterpolator::new_forward(sx1, sx2, len as i32);
        self.scale_y = Dda2LineInterpolator::new_forward(sy1, sy2, len as i32);
    }

    fn next(&mut self) {
        self.iterator.next();
        self.scale_x.inc();
        self.scale_y.inc();
    }

    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        *x = iround(self.iterator.x * SUBPIXEL_SCALE as f64);
        *y = iround(self.iterator.y * SUBPIXEL_SCALE as f64);
    }
}

// ============================================================================
// SpanInterpolatorPerspLerp
// ============================================================================

/// Linear-interpolation perspective span interpolator.
///
/// Port of C++ `span_interpolator_persp_lerp<SubpixelShift>`.
/// Transforms only the endpoints and linearly interpolates coordinates
/// between them using DDA. Faster but less accurate than the exact variant.
pub struct SpanInterpolatorPerspLerp {
    trans_dir: TransPerspective,
    trans_inv: TransPerspective,
    coord_x: Dda2LineInterpolator,
    coord_y: Dda2LineInterpolator,
    scale_x: Dda2LineInterpolator,
    scale_y: Dda2LineInterpolator,
}

impl SpanInterpolatorPerspLerp {
    pub fn new() -> Self {
        Self {
            trans_dir: TransPerspective::new(),
            trans_inv: TransPerspective::new(),
            coord_x: Dda2LineInterpolator::new_forward(0, 0, 1),
            coord_y: Dda2LineInterpolator::new_forward(0, 0, 1),
            scale_x: Dda2LineInterpolator::new_forward(0, 0, 1),
            scale_y: Dda2LineInterpolator::new_forward(0, 0, 1),
        }
    }

    pub fn new_quad_to_quad(src: &[f64; 8], dst: &[f64; 8]) -> Self {
        let mut s = Self::new();
        s.quad_to_quad(src, dst);
        s
    }

    pub fn new_rect_to_quad(x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) -> Self {
        let mut s = Self::new();
        s.rect_to_quad(x1, y1, x2, y2, quad);
        s
    }

    pub fn new_quad_to_rect(quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        let mut s = Self::new();
        s.quad_to_rect(quad, x1, y1, x2, y2);
        s
    }

    pub fn quad_to_quad(&mut self, src: &[f64; 8], dst: &[f64; 8]) {
        self.trans_dir.quad_to_quad(src, dst);
        self.trans_inv.quad_to_quad(dst, src);
    }

    pub fn rect_to_quad(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, quad: &[f64; 8]) {
        let src = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(&src, quad);
    }

    pub fn quad_to_rect(&mut self, quad: &[f64; 8], x1: f64, y1: f64, x2: f64, y2: f64) {
        let dst = [x1, y1, x2, y1, x2, y2, x1, y2];
        self.quad_to_quad(quad, &dst);
    }

    pub fn is_valid(&self) -> bool {
        self.trans_dir.is_valid()
    }

    pub fn local_scale(&self, x: &mut i32, y: &mut i32) {
        *x = self.scale_x.y();
        *y = self.scale_y.y();
    }

    pub fn transform(&self, x: &mut f64, y: &mut f64) {
        self.trans_dir.transform(x, y);
    }

    pub fn trans_dir(&self) -> &TransPerspective {
        &self.trans_dir
    }

    pub fn trans_inv(&self) -> &TransPerspective {
        &self.trans_inv
    }
}

impl SpanInterpolator for SpanInterpolatorPerspLerp {
    fn begin(&mut self, x: f64, y: f64, len: u32) {
        // Transform start point
        let mut xt = x;
        let mut yt = y;
        self.trans_dir.transform(&mut xt, &mut yt);
        let x1 = iround(xt * SUBPIXEL_SCALE as f64);
        let y1 = iround(yt * SUBPIXEL_SCALE as f64);

        let delta = 1.0 / SUBPIXEL_SCALE as f64;

        let sx1 = calc_scale(xt, yt, x, y, &self.trans_inv, delta, 0.0);
        let sy1 = calc_scale(xt, yt, x, y, &self.trans_inv, 0.0, delta);

        // Transform end point
        let x_end = x + len as f64;
        let mut xt2 = x_end;
        let mut yt2 = y;
        self.trans_dir.transform(&mut xt2, &mut yt2);
        let x2 = iround(xt2 * SUBPIXEL_SCALE as f64);
        let y2 = iround(yt2 * SUBPIXEL_SCALE as f64);

        let sx2 = calc_scale(xt2, yt2, x_end, y, &self.trans_inv, delta, 0.0);
        let sy2 = calc_scale(xt2, yt2, x_end, y, &self.trans_inv, 0.0, delta);

        self.coord_x = Dda2LineInterpolator::new_forward(x1, x2, len as i32);
        self.coord_y = Dda2LineInterpolator::new_forward(y1, y2, len as i32);
        self.scale_x = Dda2LineInterpolator::new_forward(sx1, sx2, len as i32);
        self.scale_y = Dda2LineInterpolator::new_forward(sy1, sy2, len as i32);
    }

    fn next(&mut self) {
        self.coord_x.inc();
        self.coord_y.inc();
        self.scale_x.inc();
        self.scale_y.inc();
    }

    fn coordinates(&self, x: &mut i32, y: &mut i32) {
        *x = self.coord_x.y();
        *y = self.coord_y.y();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_identity() {
        let src = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let dst = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let mut interp = SpanInterpolatorPerspExact::new_quad_to_quad(&src, &dst);
        assert!(interp.is_valid());

        interp.begin(50.0, 50.0, 10);
        let (mut x, mut y) = (0, 0);
        interp.coordinates(&mut x, &mut y);
        // Should be approximately (50*256, 50*256) = (12800, 12800)
        assert!((x - 12800).abs() < 5, "x={x}");
        assert!((y - 12800).abs() < 5, "y={y}");
    }

    #[test]
    fn test_exact_next_advances() {
        let src = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let dst = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let mut interp = SpanInterpolatorPerspExact::new_quad_to_quad(&src, &dst);

        interp.begin(0.0, 50.0, 10);
        let (mut x1, mut y1) = (0, 0);
        interp.coordinates(&mut x1, &mut y1);

        interp.next();
        let (mut x2, mut y2) = (0, 0);
        interp.coordinates(&mut x2, &mut y2);

        // x should advance by roughly 256 (1 pixel in subpixel coords)
        assert!(x2 > x1, "x2={x2} should be > x1={x1}");
        assert!((x2 - x1 - 256).abs() < 5, "step={}", x2 - x1);
    }

    #[test]
    fn test_lerp_identity() {
        let src = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let dst = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let mut interp = SpanInterpolatorPerspLerp::new_quad_to_quad(&src, &dst);
        assert!(interp.is_valid());

        interp.begin(50.0, 50.0, 10);
        let (mut x, mut y) = (0, 0);
        interp.coordinates(&mut x, &mut y);
        assert!((x - 12800).abs() < 5, "x={x}");
        assert!((y - 12800).abs() < 5, "y={y}");
    }

    #[test]
    fn test_lerp_next_advances() {
        let src = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let dst = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        let mut interp = SpanInterpolatorPerspLerp::new_quad_to_quad(&src, &dst);

        interp.begin(0.0, 50.0, 10);
        let (mut x1, mut y1) = (0, 0);
        interp.coordinates(&mut x1, &mut y1);

        interp.next();
        let (mut x2, mut y2) = (0, 0);
        interp.coordinates(&mut x2, &mut y2);

        assert!(x2 > x1, "x2={x2} should be > x1={x1}");
        assert!((x2 - x1 - 256).abs() < 5, "step={}", x2 - x1);
    }

    #[test]
    fn test_exact_rect_to_quad() {
        let quad = [10.0, 10.0, 110.0, 10.0, 110.0, 110.0, 10.0, 110.0];
        let mut interp = SpanInterpolatorPerspExact::new_rect_to_quad(0.0, 0.0, 100.0, 100.0, &quad);
        assert!(interp.is_valid());

        // Point (0,0) in rect space → (10,10) in quad space
        interp.begin(0.0, 0.0, 1);
        let (mut x, mut y) = (0, 0);
        interp.coordinates(&mut x, &mut y);
        assert!((x - 10 * 256).abs() < 5, "x={x}");
        assert!((y - 10 * 256).abs() < 5, "y={y}");
    }

    #[test]
    fn test_lerp_rect_to_quad() {
        let quad = [10.0, 10.0, 110.0, 10.0, 110.0, 110.0, 10.0, 110.0];
        let mut interp = SpanInterpolatorPerspLerp::new_rect_to_quad(0.0, 0.0, 100.0, 100.0, &quad);
        assert!(interp.is_valid());

        interp.begin(0.0, 0.0, 1);
        let (mut x, mut y) = (0, 0);
        interp.coordinates(&mut x, &mut y);
        assert!((x - 10 * 256).abs() < 5, "x={x}");
        assert!((y - 10 * 256).abs() < 5, "y={y}");
    }

    #[test]
    fn test_exact_and_lerp_agree_on_identity() {
        let src = [0.0, 0.0, 200.0, 0.0, 200.0, 200.0, 0.0, 200.0];
        let dst = src;

        let mut exact = SpanInterpolatorPerspExact::new_quad_to_quad(&src, &dst);
        let mut lerp = SpanInterpolatorPerspLerp::new_quad_to_quad(&src, &dst);

        exact.begin(10.0, 10.0, 5);
        lerp.begin(10.0, 10.0, 5);

        for _ in 0..5 {
            let (mut ex, mut ey) = (0, 0);
            let (mut lx, mut ly) = (0, 0);
            exact.coordinates(&mut ex, &mut ey);
            lerp.coordinates(&mut lx, &mut ly);
            assert!((ex - lx).abs() < 3, "ex={ex} lx={lx}");
            assert!((ey - ly).abs() < 3, "ey={ey} ly={ly}");

            exact.next();
            lerp.next();
        }
    }
}

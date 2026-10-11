//! Marker renderer: small symbols (squares, circles, triangles, crosses, ...)
//! centred on integer points.
//!
//! Port of `agg_renderer_markers.h` (agg-sharp `RendererMarkers.cs`). C++
//! derives `renderer_markers` from `renderer_primitives`; here it holds a
//! [`RendererPrimitives`] and derefs to it, so the fill and line colours and
//! the primitives' shapes are reached the same way. Generic over the base
//! renderer, so it draws through `RendererBase` or `RendererMclip`.

use crate::basics::{RectI, COVER_FULL};
use crate::ellipse_bresenham::EllipseBresenhamInterpolator;
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;
use crate::renderer_primitives::{BaseRenderer, RendererPrimitives};
use std::ops::{Deref, DerefMut};

/// C++ `marker_e`: the marker shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkerType {
    Square,
    Diamond,
    Circle,
    CrossedCircle,
    SemiellipseLeft,
    SemiellipseRight,
    SemiellipseUp,
    SemiellipseDown,
    TriangleLeft,
    TriangleRight,
    TriangleUp,
    TriangleDown,
    FourRays,
    Cross,
    X,
    Dash,
    Dot,
    Pixel,
}

impl MarkerType {
    /// Every marker in C++'s `marker_e` order (`end_of_markers` is their count).
    pub const ALL: [MarkerType; 18] = [
        MarkerType::Square,
        MarkerType::Diamond,
        MarkerType::Circle,
        MarkerType::CrossedCircle,
        MarkerType::SemiellipseLeft,
        MarkerType::SemiellipseRight,
        MarkerType::SemiellipseUp,
        MarkerType::SemiellipseDown,
        MarkerType::TriangleLeft,
        MarkerType::TriangleRight,
        MarkerType::TriangleUp,
        MarkerType::TriangleDown,
        MarkerType::FourRays,
        MarkerType::Cross,
        MarkerType::X,
        MarkerType::Dash,
        MarkerType::Dot,
        MarkerType::Pixel,
    ];
}

/// A coordinate or radius type the batch `markers*` calls take: C++'s
/// template `T`, converted with `int(v)` (truncation toward zero).
pub trait MarkerCoord: Copy {
    fn to_int(self) -> i32;
    /// C++'s `r == 0` in `T`'s own arithmetic.
    fn is_zero(self) -> bool;
}

impl MarkerCoord for i32 {
    fn to_int(self) -> i32 {
        self
    }
    fn is_zero(self) -> bool {
        self == 0
    }
}

impl MarkerCoord for f32 {
    fn to_int(self) -> i32 {
        self as i32
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
}

impl MarkerCoord for f64 {
    fn to_int(self) -> i32 {
        self as i32
    }
    fn is_zero(self) -> bool {
        self == 0.0
    }
}

/// C++ `renderer_markers<BaseRenderer>`.
pub struct RendererMarkers<'a, PF: PixelFormat, R: BaseRenderer<PixFmt = PF> = RendererBase<PF>> {
    prim: RendererPrimitives<'a, PF, R>,
}

impl<'a, PF: PixelFormat, R: BaseRenderer<PixFmt = PF>> Deref for RendererMarkers<'a, PF, R> {
    type Target = RendererPrimitives<'a, PF, R>;
    fn deref(&self) -> &Self::Target {
        &self.prim
    }
}

impl<PF: PixelFormat, R: BaseRenderer<PixFmt = PF>> DerefMut for RendererMarkers<'_, PF, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.prim
    }
}

impl<'a, PF: PixelFormat, R: BaseRenderer<PixFmt = PF>> RendererMarkers<'a, PF, R>
where
    PF::ColorType: Default + Clone,
{
    pub fn new(ren: &'a mut R) -> Self {
        Self {
            prim: RendererPrimitives::new(ren),
        }
    }

    /// Whether a marker of radius `r` at `x`, `y` can touch the clip box.
    ///
    /// Deliberate divergence from C++: `agg_renderer_markers.h` builds the
    /// box as `rect_i rc(x-r, y-r, x+y, y+r)`, a typo for `x+r`, so the right
    /// edge depends on `y`: a marker just left of the clip box is culled
    /// when `y < r` even though it reaches in. The box here is the marker's
    /// own, as agg-sharp's is.
    /// Pinned by agg-sharp's `RendererMarkersTests` and
    /// `rust_only_visible_uses_the_marker_box_not_x_plus_y`.
    pub fn visible(&self, x: i32, y: i32, r: i32) -> bool {
        let mut rc = RectI::new(x - r, y - r, x + r, y + r);
        rc.clip(&self.prim.ren().bounding_clip_box())
    }

    fn line(&self) -> PF::ColorType {
        self.prim.line_color().clone()
    }

    fn fill(&self) -> PF::ColorType {
        self.prim.fill_color().clone()
    }

    fn pixel_at(&mut self, x: i32, y: i32, c: &PF::ColorType) {
        self.prim.ren_mut().blend_pixel(x, y, c, COVER_FULL);
    }

    fn hline(&mut self, x1: i32, y: i32, x2: i32, c: &PF::ColorType) {
        self.prim.ren_mut().blend_hline(x1, y, x2, c, COVER_FULL);
    }

    fn vline(&mut self, x: i32, y1: i32, y2: i32, c: &PF::ColorType) {
        self.prim.ren_mut().blend_vline(x, y1, y2, c, COVER_FULL);
    }

    /// The `r == 0` case every shape shares: one pixel in the fill colour.
    fn center_pixel(&mut self, x: i32, y: i32) {
        let fc = self.fill();
        self.pixel_at(x, y, &fc);
    }

    pub fn square(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                self.prim.outlined_rectangle(x - r, y - r, x + r, y + r);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    pub fn diamond(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                let (lc, fc) = (self.line(), self.fill());
                let mut dy = -r;
                let mut dx = 0;
                loop {
                    self.pixel_at(x - dx, y + dy, &lc);
                    self.pixel_at(x + dx, y + dy, &lc);
                    self.pixel_at(x - dx, y - dy, &lc);
                    self.pixel_at(x + dx, y - dy, &lc);

                    if dx != 0 {
                        self.hline(x - dx + 1, y + dy, x + dx - 1, &fc);
                        self.hline(x - dx + 1, y - dy, x + dx - 1, &fc);
                    }
                    dy += 1;
                    dx += 1;
                    if dy > 0 {
                        break;
                    }
                }
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    pub fn circle(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                self.prim.outlined_ellipse(x, y, r, r);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    pub fn crossed_circle(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                self.prim.outlined_ellipse(x, y, r, r);
                let lc = self.line();
                let mut r6 = r + (r >> 1);
                if r <= 2 {
                    r6 += 1;
                }
                let r = r >> 1;
                self.hline(x - r6, y, x - r, &lc);
                self.hline(x + r, y, x + r6, &lc);
                self.vline(x, y - r6, y - r, &lc);
                self.vline(x, y + r, y + r6, &lc);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    /// The four semiellipses share C++'s loop; `dir` maps the loop's
    /// (dx, dy) to a pixel offset and says whether the fill runs are
    /// vertical.
    fn semiellipse(&mut self, x: i32, y: i32, r: i32, dir: Dir) {
        if !self.visible(x, y, r) {
            return;
        }
        if r == 0 {
            self.center_pixel(x, y);
            return;
        }
        let (lc, fc) = (self.line(), self.fill());
        let r8 = r * 4 / 5;
        let mut dy = -r;
        let mut dx = 0;
        let mut ei = EllipseBresenhamInterpolator::new(r * 3 / 5, r + r8);
        loop {
            dx += ei.dx();
            dy += ei.dy();
            match dir {
                Dir::Left => {
                    self.pixel_at(x + dy, y + dx, &lc);
                    self.pixel_at(x + dy, y - dx, &lc);
                    if ei.dy() != 0 && dx != 0 {
                        self.vline(x + dy, y - dx + 1, y + dx - 1, &fc);
                    }
                }
                Dir::Right => {
                    self.pixel_at(x - dy, y + dx, &lc);
                    self.pixel_at(x - dy, y - dx, &lc);
                    if ei.dy() != 0 && dx != 0 {
                        self.vline(x - dy, y - dx + 1, y + dx - 1, &fc);
                    }
                }
                Dir::Up => {
                    self.pixel_at(x + dx, y - dy, &lc);
                    self.pixel_at(x - dx, y - dy, &lc);
                    if ei.dy() != 0 && dx != 0 {
                        self.hline(x - dx + 1, y - dy, x + dx - 1, &fc);
                    }
                }
                Dir::Down => {
                    self.pixel_at(x + dx, y + dy, &lc);
                    self.pixel_at(x - dx, y + dy, &lc);
                    if ei.dy() != 0 && dx != 0 {
                        self.hline(x - dx + 1, y + dy, x + dx - 1, &fc);
                    }
                }
            }
            ei.next();
            if dy >= r8 {
                break;
            }
        }
        match dir {
            Dir::Left => self.vline(x + dy, y - dx, y + dx, &lc),
            Dir::Right => self.vline(x - dy, y - dx, y + dx, &lc),
            Dir::Up => self.hline(x - dx, y - dy - 1, x + dx, &lc),
            Dir::Down => self.hline(x - dx, y + dy + 1, x + dx, &lc),
        }
    }

    pub fn semiellipse_left(&mut self, x: i32, y: i32, r: i32) {
        self.semiellipse(x, y, r, Dir::Left);
    }

    pub fn semiellipse_right(&mut self, x: i32, y: i32, r: i32) {
        self.semiellipse(x, y, r, Dir::Right);
    }

    pub fn semiellipse_up(&mut self, x: i32, y: i32, r: i32) {
        self.semiellipse(x, y, r, Dir::Up);
    }

    pub fn semiellipse_down(&mut self, x: i32, y: i32, r: i32) {
        self.semiellipse(x, y, r, Dir::Down);
    }

    /// The four triangles share C++'s loop, oriented by `dir`.
    fn triangle(&mut self, x: i32, y: i32, r: i32, dir: Dir) {
        if !self.visible(x, y, r) {
            return;
        }
        if r == 0 {
            self.center_pixel(x, y);
            return;
        }
        let (lc, fc) = (self.line(), self.fill());
        let mut dy = -r;
        let mut dx = 0;
        let mut flip = 0;
        let r6 = r * 3 / 5;
        loop {
            match dir {
                Dir::Left => {
                    self.pixel_at(x + dy, y - dx, &lc);
                    self.pixel_at(x + dy, y + dx, &lc);
                    if dx != 0 {
                        self.vline(x + dy, y - dx + 1, y + dx - 1, &fc);
                    }
                }
                Dir::Right => {
                    self.pixel_at(x - dy, y - dx, &lc);
                    self.pixel_at(x - dy, y + dx, &lc);
                    if dx != 0 {
                        self.vline(x - dy, y - dx + 1, y + dx - 1, &fc);
                    }
                }
                Dir::Up => {
                    self.pixel_at(x - dx, y - dy, &lc);
                    self.pixel_at(x + dx, y - dy, &lc);
                    if dx != 0 {
                        self.hline(x - dx + 1, y - dy, x + dx - 1, &fc);
                    }
                }
                Dir::Down => {
                    self.pixel_at(x - dx, y + dy, &lc);
                    self.pixel_at(x + dx, y + dy, &lc);
                    if dx != 0 {
                        self.hline(x - dx + 1, y + dy, x + dx - 1, &fc);
                    }
                }
            }
            dy += 1;
            dx += flip;
            flip ^= 1;
            if dy >= r6 {
                break;
            }
        }
        match dir {
            Dir::Left => self.vline(x + dy, y - dx, y + dx, &lc),
            Dir::Right => self.vline(x - dy, y - dx, y + dx, &lc),
            Dir::Up => self.hline(x - dx, y - dy, x + dx, &lc),
            Dir::Down => self.hline(x - dx, y + dy, x + dx, &lc),
        }
    }

    pub fn triangle_left(&mut self, x: i32, y: i32, r: i32) {
        self.triangle(x, y, r, Dir::Left);
    }

    pub fn triangle_right(&mut self, x: i32, y: i32, r: i32) {
        self.triangle(x, y, r, Dir::Right);
    }

    pub fn triangle_up(&mut self, x: i32, y: i32, r: i32) {
        self.triangle(x, y, r, Dir::Up);
    }

    pub fn triangle_down(&mut self, x: i32, y: i32, r: i32) {
        self.triangle(x, y, r, Dir::Down);
    }

    pub fn four_rays(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                let (lc, fc) = (self.line(), self.fill());
                let mut dy = -r;
                let mut dx = 0;
                let mut flip = 0;
                let r3 = -(r / 3);
                loop {
                    self.pixel_at(x - dx, y + dy, &lc);
                    self.pixel_at(x + dx, y + dy, &lc);
                    self.pixel_at(x - dx, y - dy, &lc);
                    self.pixel_at(x + dx, y - dy, &lc);
                    self.pixel_at(x + dy, y - dx, &lc);
                    self.pixel_at(x + dy, y + dx, &lc);
                    self.pixel_at(x - dy, y - dx, &lc);
                    self.pixel_at(x - dy, y + dx, &lc);

                    if dx != 0 {
                        self.hline(x - dx + 1, y + dy, x + dx - 1, &fc);
                        self.hline(x - dx + 1, y - dy, x + dx - 1, &fc);
                        self.vline(x + dy, y - dx + 1, y + dx - 1, &fc);
                        self.vline(x - dy, y - dx + 1, y + dx - 1, &fc);
                    }
                    dy += 1;
                    dx += flip;
                    flip ^= 1;
                    if dy > r3 {
                        break;
                    }
                }
                self.prim
                    .solid_rectangle(x + r3 + 1, y + r3 + 1, x - r3 - 1, y - r3 - 1);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    pub fn cross(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                let lc = self.line();
                self.vline(x, y - r, y + r, &lc);
                self.hline(x - r, y, x + r, &lc);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    /// C++ `xing`: the X marker.
    pub fn xing(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                let lc = self.line();
                let mut dy = -r * 7 / 10;
                loop {
                    self.pixel_at(x + dy, y + dy, &lc);
                    self.pixel_at(x - dy, y + dy, &lc);
                    self.pixel_at(x + dy, y - dy, &lc);
                    self.pixel_at(x - dy, y - dy, &lc);
                    dy += 1;
                    if dy >= 0 {
                        break;
                    }
                }
            }
            self.center_pixel(x, y);
        }
    }

    pub fn dash(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                let lc = self.line();
                self.hline(x - r, y, x + r, &lc);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    pub fn dot(&mut self, x: i32, y: i32, r: i32) {
        if self.visible(x, y, r) {
            if r != 0 {
                self.prim.solid_ellipse(x, y, r, r);
            } else {
                self.center_pixel(x, y);
            }
        }
    }

    /// One pixel in the fill colour; the radius is ignored and nothing is culled.
    pub fn pixel(&mut self, x: i32, y: i32, _r: i32) {
        self.center_pixel(x, y);
    }

    /// Draws the marker `kind` of radius `r` at `x`, `y`.
    pub fn marker(&mut self, x: i32, y: i32, r: i32, kind: MarkerType) {
        match kind {
            MarkerType::Square => self.square(x, y, r),
            MarkerType::Diamond => self.diamond(x, y, r),
            MarkerType::Circle => self.circle(x, y, r),
            MarkerType::CrossedCircle => self.crossed_circle(x, y, r),
            MarkerType::SemiellipseLeft => self.semiellipse_left(x, y, r),
            MarkerType::SemiellipseRight => self.semiellipse_right(x, y, r),
            MarkerType::SemiellipseUp => self.semiellipse_up(x, y, r),
            MarkerType::SemiellipseDown => self.semiellipse_down(x, y, r),
            MarkerType::TriangleLeft => self.triangle_left(x, y, r),
            MarkerType::TriangleRight => self.triangle_right(x, y, r),
            MarkerType::TriangleUp => self.triangle_up(x, y, r),
            MarkerType::TriangleDown => self.triangle_down(x, y, r),
            MarkerType::FourRays => self.four_rays(x, y, r),
            MarkerType::Cross => self.cross(x, y, r),
            MarkerType::X => self.xing(x, y, r),
            MarkerType::Dash => self.dash(x, y, r),
            MarkerType::Dot => self.dot(x, y, r),
            MarkerType::Pixel => self.pixel(x, y, r),
        }
    }

    /// C++ `markers(n, x, y, r, type)`: one marker per point, all of radius
    /// `r`; a zero radius draws each point as a fill-colour pixel. Points
    /// pair up `x` and `y` up to the shorter slice (C++'s `n`).
    pub fn markers<T: MarkerCoord>(&mut self, x: &[T], y: &[T], r: T, kind: MarkerType) {
        if r.is_zero() {
            let fc = self.fill();
            for (&px, &py) in x.iter().zip(y) {
                self.pixel_at(px.to_int(), py.to_int(), &fc);
            }
            return;
        }
        for (&px, &py) in x.iter().zip(y) {
            self.marker(px.to_int(), py.to_int(), r.to_int(), kind);
        }
    }

    /// C++ `markers(n, x, y, r[], type)`: each point with its own radius.
    pub fn markers_with_radii<T: MarkerCoord>(
        &mut self,
        x: &[T],
        y: &[T],
        r: &[T],
        kind: MarkerType,
    ) {
        for ((&px, &py), &pr) in x.iter().zip(y).zip(r) {
            self.marker(px.to_int(), py.to_int(), pr.to_int(), kind);
        }
    }

    /// C++ `markers(n, x, y, r[], fc[], type)`: each point with its own radius
    /// and fill colour (the fill colour stays at the last one).
    pub fn markers_with_fill<T: MarkerCoord>(
        &mut self,
        x: &[T],
        y: &[T],
        r: &[T],
        fc: &[PF::ColorType],
        kind: MarkerType,
    ) {
        for (((&px, &py), &pr), f) in x.iter().zip(y).zip(r).zip(fc) {
            self.prim.set_fill_color(f.clone());
            self.marker(px.to_int(), py.to_int(), pr.to_int(), kind);
        }
    }

    /// C++ `markers(n, x, y, r[], fc[], lc[], type)`: each point with its own
    /// radius, fill and line colour (both stay at the last ones).
    pub fn markers_with_colors<T: MarkerCoord>(
        &mut self,
        x: &[T],
        y: &[T],
        r: &[T],
        fc: &[PF::ColorType],
        lc: &[PF::ColorType],
        kind: MarkerType,
    ) {
        for ((((&px, &py), &pr), f), l) in x.iter().zip(y).zip(r).zip(fc).zip(lc) {
            self.prim.set_fill_color(f.clone());
            self.prim.set_line_color(l.clone());
            self.marker(px.to_int(), py.to_int(), pr.to_int(), kind);
        }
    }
}

/// Which way a semiellipse or triangle marker points.
#[derive(Clone, Copy)]
enum Dir {
    Left,
    Right,
    Up,
    Down,
}

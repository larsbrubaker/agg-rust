//! Basic shape primitives renderer.
//!
//! Port of `agg_renderer_primitives.h` — draws simple shapes (rectangles,
//! ellipses, lines) directly into a renderer without anti-aliasing.

use crate::basics::{iround, COVER_FULL};
use crate::dda_line::{self, LineBresenhamInterpolator};
use crate::ellipse_bresenham::EllipseBresenhamInterpolator;
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;

// ============================================================================
// RendererPrimitives
// ============================================================================

/// Basic shape primitives renderer.
///
/// Draws rectangles, ellipses, and Bresenham lines directly into a
/// `RendererBase` without anti-aliasing. Maintains separate fill and line
/// colors and a current position for line_to operations.
///
/// Port of C++ `renderer_primitives<BaseRenderer>`.
pub struct RendererPrimitives<'a, PF: PixelFormat> {
    ren: &'a mut RendererBase<PF>,
    fill_color: PF::ColorType,
    line_color: PF::ColorType,
    curr_x: i32,
    curr_y: i32,
}

impl<'a, PF: PixelFormat> RendererPrimitives<'a, PF>
where
    PF::ColorType: Default + Clone,
{
    pub fn new(ren: &'a mut RendererBase<PF>) -> Self {
        Self {
            ren,
            fill_color: PF::ColorType::default(),
            line_color: PF::ColorType::default(),
            curr_x: 0,
            curr_y: 0,
        }
    }

    /// Convert a floating-point coordinate to subpixel Bresenham coordinate.
    pub fn coord(c: f64) -> i32 {
        iround(c * dda_line::SUBPIXEL_SCALE as f64)
    }

    pub fn set_fill_color(&mut self, c: PF::ColorType) {
        self.fill_color = c;
    }

    pub fn set_line_color(&mut self, c: PF::ColorType) {
        self.line_color = c;
    }

    pub fn fill_color(&self) -> &PF::ColorType {
        &self.fill_color
    }

    pub fn line_color(&self) -> &PF::ColorType {
        &self.line_color
    }

    /// Draw an outlined rectangle (line color only).
    pub fn rectangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.ren
            .blend_hline(x1, y1, x2 - 1, &self.line_color.clone(), COVER_FULL);
        self.ren
            .blend_vline(x2, y1, y2 - 1, &self.line_color.clone(), COVER_FULL);
        self.ren
            .blend_hline(x1 + 1, y2, x2, &self.line_color.clone(), COVER_FULL);
        self.ren
            .blend_vline(x1, y1 + 1, y2, &self.line_color.clone(), COVER_FULL);
    }

    /// Draw a solid filled rectangle (fill color only).
    pub fn solid_rectangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.ren
            .blend_bar(x1, y1, x2, y2, &self.fill_color.clone(), COVER_FULL);
    }

    /// Draw an outlined and filled rectangle.
    pub fn outlined_rectangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.rectangle(x1, y1, x2, y2);
        self.ren.blend_bar(
            x1 + 1,
            y1 + 1,
            x2 - 1,
            y2 - 1,
            &self.fill_color.clone(),
            COVER_FULL,
        );
    }

    /// Draw an outlined ellipse (line color only).
    pub fn ellipse(&mut self, x: i32, y: i32, rx: i32, ry: i32) {
        let mut ei = EllipseBresenhamInterpolator::new(rx, ry);
        let mut dx = 0i32;
        let mut dy = -ry;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();
            let lc = self.line_color.clone();
            self.ren.blend_pixel(x + dx, y + dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x + dx, y - dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x - dx, y - dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x - dx, y + dy, &lc, COVER_FULL);
            if dy >= 0 {
                break;
            }
        }
    }

    /// Draw a solid filled ellipse (fill color only).
    pub fn solid_ellipse(&mut self, x: i32, y: i32, rx: i32, ry: i32) {
        let mut ei = EllipseBresenhamInterpolator::new(rx, ry);
        let mut dx = 0i32;
        let mut dy = -ry;
        let mut dy0 = dy;
        let mut dx0 = dx;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();

            if dy != dy0 {
                let fc = self.fill_color.clone();
                self.ren
                    .blend_hline(x - dx0, y + dy0, x + dx0, &fc, COVER_FULL);
                self.ren
                    .blend_hline(x - dx0, y - dy0, x + dx0, &fc, COVER_FULL);
            }
            dx0 = dx;
            dy0 = dy;
            if dy >= 0 {
                break;
            }
        }
        let fc = self.fill_color.clone();
        self.ren
            .blend_hline(x - dx0, y + dy0, x + dx0, &fc, COVER_FULL);
    }

    /// Draw an outlined and filled ellipse.
    pub fn outlined_ellipse(&mut self, x: i32, y: i32, rx: i32, ry: i32) {
        let mut ei = EllipseBresenhamInterpolator::new(rx, ry);
        let mut dx = 0i32;
        let mut dy = -ry;

        loop {
            ei.next();
            dx += ei.dx();
            dy += ei.dy();

            let lc = self.line_color.clone();
            self.ren.blend_pixel(x + dx, y + dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x + dx, y - dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x - dx, y - dy, &lc, COVER_FULL);
            self.ren.blend_pixel(x - dx, y + dy, &lc, COVER_FULL);

            if ei.dy() != 0 && dx != 0 {
                let fc = self.fill_color.clone();
                self.ren
                    .blend_hline(x - dx + 1, y + dy, x + dx - 1, &fc, COVER_FULL);
                self.ren
                    .blend_hline(x - dx + 1, y - dy, x + dx - 1, &fc, COVER_FULL);
            }
            if dy >= 0 {
                break;
            }
        }
    }

    /// Draw a Bresenham line from (x1,y1) to (x2,y2).
    ///
    /// Coordinates are in subpixel units (use `coord()` to convert).
    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, last: bool) {
        let mut li = LineBresenhamInterpolator::new(x1, y1, x2, y2);

        let mut len = li.len();
        if len == 0 {
            if last {
                let lc = self.line_color.clone();
                self.ren.blend_pixel(
                    dda_line::line_lr(x1),
                    dda_line::line_lr(y1),
                    &lc,
                    COVER_FULL,
                );
            }
            return;
        }

        if last {
            len += 1;
        }

        if li.is_ver() {
            for _ in 0..len {
                let lc = self.line_color.clone();
                self.ren.blend_pixel(li.x2(), li.y1(), &lc, COVER_FULL);
                li.vstep();
            }
        } else {
            for _ in 0..len {
                let lc = self.line_color.clone();
                self.ren.blend_pixel(li.x1(), li.y2(), &lc, COVER_FULL);
                li.hstep();
            }
        }
    }

    /// Set the current position for line_to.
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.curr_x = x;
        self.curr_y = y;
    }

    /// Draw a line from the current position to (x, y).
    pub fn line_to(&mut self, x: i32, y: i32, last: bool) {
        self.line(self.curr_x, self.curr_y, x, y, last);
        self.curr_x = x;
        self.curr_y = y;
    }

    pub fn ren(&self) -> &RendererBase<PF> {
        self.ren
    }

    pub fn ren_mut(&mut self) -> &mut RendererBase<PF> {
        self.ren
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

    fn make_ren(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * 4) as i32;
        let buf = vec![0u8; (w * h * 4) as usize];
        let mut ra = RowAccessor::new();
        unsafe { ra.attach(buf.as_ptr() as *mut u8, w, h, stride) };
        (buf, ra)
    }

    #[test]
    fn test_rectangle() {
        let (_buf, mut ra) = make_ren(20, 20);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(255, 0, 0, 255));
        prim.rectangle(2, 2, 8, 8);
    }

    #[test]
    fn test_solid_rectangle() {
        let (_buf, mut ra) = make_ren(20, 20);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_fill_color(Rgba8::new(0, 255, 0, 255));
        prim.solid_rectangle(3, 3, 6, 6);
    }

    #[test]
    fn test_ellipse() {
        let (_buf, mut ra) = make_ren(30, 30);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(0, 0, 255, 255));
        prim.ellipse(15, 15, 5, 5);
    }

    #[test]
    fn test_solid_ellipse() {
        let (_buf, mut ra) = make_ren(30, 30);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_fill_color(Rgba8::new(255, 255, 0, 255));
        prim.solid_ellipse(15, 15, 5, 5);
    }

    #[test]
    fn test_line() {
        let (_buf, mut ra) = make_ren(20, 20);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(255, 0, 0, 255));
        let x1 = RendererPrimitives::<PixfmtRgba32>::coord(2.0);
        let y1 = RendererPrimitives::<PixfmtRgba32>::coord(5.0);
        let x2 = RendererPrimitives::<PixfmtRgba32>::coord(10.0);
        let y2 = RendererPrimitives::<PixfmtRgba32>::coord(5.0);
        prim.line(x1, y1, x2, y2, true);
    }

    #[test]
    fn test_move_to_line_to() {
        let (_buf, mut ra) = make_ren(20, 20);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(255, 255, 255, 255));
        let x1 = RendererPrimitives::<PixfmtRgba32>::coord(1.0);
        let y1 = RendererPrimitives::<PixfmtRgba32>::coord(1.0);
        prim.move_to(x1, y1);
        let x2 = RendererPrimitives::<PixfmtRgba32>::coord(10.0);
        let y2 = RendererPrimitives::<PixfmtRgba32>::coord(1.0);
        prim.line_to(x2, y2, true);
    }

    #[test]
    fn test_color_accessors() {
        let (_buf, mut ra) = make_ren(10, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_fill_color(Rgba8::new(10, 20, 30, 40));
        prim.set_line_color(Rgba8::new(50, 60, 70, 80));
        assert_eq!(prim.fill_color().r, 10);
        assert_eq!(prim.line_color().r, 50);
    }

    #[test]
    fn test_outlined_rectangle() {
        let (_buf, mut ra) = make_ren(20, 20);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut rb = RendererBase::new(pf);
        let mut prim = RendererPrimitives::new(&mut rb);
        prim.set_line_color(Rgba8::new(255, 0, 0, 255));
        prim.set_fill_color(Rgba8::new(0, 255, 0, 255));
        prim.outlined_rectangle(2, 2, 8, 8);
    }
}

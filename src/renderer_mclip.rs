//! Multi-clip renderer.
//!
//! Port of `agg_renderer_mclip.h`.
//! Wraps a `RendererBase` with multiple clipping rectangles.
//! Each rendering operation is repeated for each clip box.

use crate::basics::{CoverType, RectI};
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;

/// Renderer that supports multiple independent clipping rectangles.
///
/// Port of C++ `renderer_mclip<PixelFormat>`.
/// Each rendering call is automatically repeated for every clip box.
pub struct RendererMclip<PF: PixelFormat> {
    ren: RendererBase<PF>,
    clip_boxes: Vec<RectI>,
    curr_cb: usize,
    bounds: RectI,
}

impl<PF: PixelFormat> RendererMclip<PF> {
    pub fn new(ren: RendererBase<PF>) -> Self {
        let bounds = *ren.clip_box();
        Self {
            ren,
            clip_boxes: Vec::new(),
            curr_cb: 0,
            bounds,
        }
    }

    pub fn ren(&self) -> &RendererBase<PF> {
        &self.ren
    }

    pub fn ren_mut(&mut self) -> &mut RendererBase<PF> {
        &mut self.ren
    }

    /// Add a clip box. The box is normalized and clipped to the buffer bounds.
    pub fn add_clip_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut cb = RectI::new(x1, y1, x2, y2);
        cb.normalize();
        let buf_rect = RectI::new(
            0,
            0,
            self.ren.width() as i32 - 1,
            self.ren.height() as i32 - 1,
        );
        if cb.clip(&buf_rect) {
            if self.clip_boxes.is_empty() {
                self.bounds = cb;
            } else {
                if cb.x1 < self.bounds.x1 {
                    self.bounds.x1 = cb.x1;
                }
                if cb.y1 < self.bounds.y1 {
                    self.bounds.y1 = cb.y1;
                }
                if cb.x2 > self.bounds.x2 {
                    self.bounds.x2 = cb.x2;
                }
                if cb.y2 > self.bounds.y2 {
                    self.bounds.y2 = cb.y2;
                }
            }
            self.clip_boxes.push(cb);
        }
    }

    /// Reset clipping. If `visibility` is true, adds the full buffer as a clip box.
    pub fn reset_clipping(&mut self, visibility: bool) {
        self.clip_boxes.clear();
        if visibility {
            let w = self.ren.width() as i32;
            let h = self.ren.height() as i32;
            self.add_clip_box(0, 0, w - 1, h - 1);
        }
    }

    /// Set the current clip box to the first one. Returns true if there are any.
    fn first_clip_box(&mut self) -> bool {
        self.curr_cb = 0;
        if !self.clip_boxes.is_empty() {
            let cb = &self.clip_boxes[0];
            self.ren.clip_box_i(cb.x1, cb.y1, cb.x2, cb.y2);
            true
        } else {
            false
        }
    }

    /// Advance to the next clip box. Returns true if there's another.
    fn next_clip_box(&mut self) -> bool {
        self.curr_cb += 1;
        if self.curr_cb < self.clip_boxes.len() {
            let cb = &self.clip_boxes[self.curr_cb];
            self.ren.clip_box_i(cb.x1, cb.y1, cb.x2, cb.y2);
            true
        } else {
            false
        }
    }

    pub fn bounding_clip_box(&self) -> &RectI {
        &self.bounds
    }

    pub fn clip_box_count(&self) -> usize {
        self.clip_boxes.len()
    }

    // ========================================================================
    // Rendering operations (iterate over all clip boxes)
    // ========================================================================

    pub fn copy_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType) {
        if self.first_clip_box() {
            loop {
                self.ren.copy_pixel(x, y, c);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn blend_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType, cover: CoverType) {
        if self.first_clip_box() {
            loop {
                self.ren.blend_pixel(x, y, c, cover);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn pixel(&self, x: i32, y: i32) -> PF::ColorType
    where
        PF::ColorType: Default,
    {
        // Check each clip box — read directly from pixfmt if point is inside any box.
        for cb in &self.clip_boxes {
            if x >= cb.x1 && y >= cb.y1 && x <= cb.x2 && y <= cb.y2 {
                return self.ren.ren().pixel(x, y);
            }
        }
        PF::ColorType::default()
    }

    pub fn copy_hline(&mut self, x1: i32, y: i32, x2: i32, c: &PF::ColorType) {
        if self.first_clip_box() {
            loop {
                self.ren.copy_hline(x1, y, x2, c);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn blend_hline(
        &mut self,
        x1: i32,
        y: i32,
        x2: i32,
        c: &PF::ColorType,
        cover: CoverType,
    ) {
        if self.first_clip_box() {
            loop {
                self.ren.blend_hline(x1, y, x2, c, cover);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn blend_vline(
        &mut self,
        x: i32,
        y1: i32,
        y2: i32,
        c: &PF::ColorType,
        cover: CoverType,
    ) {
        if self.first_clip_box() {
            loop {
                self.ren.blend_vline(x, y1, y2, c, cover);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn blend_solid_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        c: &PF::ColorType,
        covers: &[CoverType],
    ) {
        if self.first_clip_box() {
            loop {
                self.ren.blend_solid_hspan(x, y, len, c, covers);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }

    pub fn blend_color_hspan(
        &mut self,
        x: i32,
        y: i32,
        len: i32,
        colors: &[PF::ColorType],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        if self.first_clip_box() {
            loop {
                self.ren.blend_color_hspan(x, y, len, colors, covers, cover);
                if !self.next_clip_box() {
                    break;
                }
            }
        }
    }
}

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
    fn test_add_clip_boxes() {
        let (_buf, mut ra) = make_buffer(200, 200);
        let pixf = PixfmtRgba32::new(&mut ra);
        let ren = RendererBase::new(pixf);
        let mut mclip = RendererMclip::new(ren);

        mclip.add_clip_box(10, 10, 50, 50);
        mclip.add_clip_box(100, 100, 150, 150);
        assert_eq!(mclip.clip_box_count(), 2);

        let b = mclip.bounding_clip_box();
        assert_eq!(b.x1, 10);
        assert_eq!(b.y1, 10);
        assert_eq!(b.x2, 150);
        assert_eq!(b.y2, 150);
    }

    #[test]
    fn test_render_to_multiple_clips() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let ren = RendererBase::new(pixf);
        let mut mclip = RendererMclip::new(ren);

        mclip.add_clip_box(0, 0, 49, 49);
        mclip.add_clip_box(50, 50, 99, 99);

        let red = Rgba8::new(255, 0, 0, 255);
        mclip.copy_pixel(25, 25, &red); // inside clip box 0
        mclip.copy_pixel(75, 75, &red); // inside clip box 1

        let p1 = mclip.pixel(25, 25);
        assert_eq!(p1.r, 255);
        let p2 = mclip.pixel(75, 75);
        assert_eq!(p2.r, 255);
    }

    #[test]
    fn test_reset_clipping() {
        let (_buf, mut ra) = make_buffer(100, 100);
        let pixf = PixfmtRgba32::new(&mut ra);
        let ren = RendererBase::new(pixf);
        let mut mclip = RendererMclip::new(ren);

        mclip.add_clip_box(10, 10, 50, 50);
        assert_eq!(mclip.clip_box_count(), 1);

        mclip.reset_clipping(false);
        assert_eq!(mclip.clip_box_count(), 0);

        mclip.reset_clipping(true);
        assert_eq!(mclip.clip_box_count(), 1);
    }
}

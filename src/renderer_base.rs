//! Base renderer with clipping.
//!
//! Port of `agg_renderer_base.h` — wraps a pixel format with a clip rectangle,
//! ensuring all rendering operations are bounded within the visible area.

use crate::basics::{CoverType, RectI};
use crate::pixfmt_rgba::PixelFormat;

// ============================================================================
// RendererBase — clip-and-delegate renderer
// ============================================================================

/// Base renderer that clips all operations to a rectangle before delegating
/// to the underlying pixel format.
///
/// Port of C++ `renderer_base<PixelFormat>`.
pub struct RendererBase<PF: PixelFormat> {
    ren: PF,
    clip_box: RectI,
}

impl<PF: PixelFormat> RendererBase<PF> {
    /// Create a new renderer wrapping the given pixel format.
    /// The clip box is initialized to the full buffer extent.
    pub fn new(ren: PF) -> Self {
        let w = ren.width() as i32;
        let h = ren.height() as i32;
        Self {
            ren,
            clip_box: RectI::new(0, 0, w - 1, h - 1),
        }
    }

    pub fn width(&self) -> u32 {
        self.ren.width()
    }
    pub fn height(&self) -> u32 {
        self.ren.height()
    }

    /// Set the clip rectangle (will be intersected with the buffer bounds).
    pub fn clip_box_i(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
        let mut cb = RectI::new(x1, y1, x2, y2);
        cb.normalize();
        if cb.clip(&RectI::new(
            0,
            0,
            self.ren.width() as i32 - 1,
            self.ren.height() as i32 - 1,
        )) {
            self.clip_box = cb;
            true
        } else {
            self.clip_box.x1 = 1;
            self.clip_box.y1 = 1;
            self.clip_box.x2 = 0;
            self.clip_box.y2 = 0;
            false
        }
    }

    /// Reset clipping to the full buffer or to nothing.
    pub fn reset_clipping(&mut self, visibility: bool) {
        if visibility {
            self.clip_box.x1 = 0;
            self.clip_box.y1 = 0;
            self.clip_box.x2 = self.ren.width() as i32 - 1;
            self.clip_box.y2 = self.ren.height() as i32 - 1;
        } else {
            self.clip_box.x1 = 1;
            self.clip_box.y1 = 1;
            self.clip_box.x2 = 0;
            self.clip_box.y2 = 0;
        }
    }

    pub fn clip_box(&self) -> &RectI {
        &self.clip_box
    }
    pub fn xmin(&self) -> i32 {
        self.clip_box.x1
    }
    pub fn ymin(&self) -> i32 {
        self.clip_box.y1
    }
    pub fn xmax(&self) -> i32 {
        self.clip_box.x2
    }
    pub fn ymax(&self) -> i32 {
        self.clip_box.y2
    }

    #[inline]
    pub fn inbox(&self, x: i32, y: i32) -> bool {
        x >= self.clip_box.x1
            && y >= self.clip_box.y1
            && x <= self.clip_box.x2
            && y <= self.clip_box.y2
    }

    /// Get a reference to the underlying pixel format.
    pub fn ren(&self) -> &PF {
        &self.ren
    }

    /// Get a mutable reference to the underlying pixel format.
    pub fn ren_mut(&mut self) -> &mut PF {
        &mut self.ren
    }

    // ========================================================================
    // Rendering operations (clip then delegate)
    // ========================================================================

    /// Clear the entire buffer to a solid color.
    pub fn clear(&mut self, c: &PF::ColorType) {
        let w = self.ren.width();
        if w > 0 {
            let h = self.ren.height();
            for y in 0..h as i32 {
                self.ren.copy_hline(0, y, w, c);
            }
        }
    }

    /// Copy a single pixel (clipped).
    pub fn copy_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType) {
        if self.inbox(x, y) {
            self.ren.copy_pixel(x, y, c);
        }
    }

    /// Blend a single pixel (clipped).
    pub fn blend_pixel(&mut self, x: i32, y: i32, c: &PF::ColorType, cover: CoverType) {
        if self.inbox(x, y) {
            self.ren.blend_pixel(x, y, c, cover);
        }
    }

    /// Get the pixel at (x, y), or default if outside clip.
    pub fn pixel(&self, x: i32, y: i32) -> PF::ColorType
    where
        PF::ColorType: Default,
    {
        if self.inbox(x, y) {
            self.ren.pixel(x, y)
        } else {
            PF::ColorType::default()
        }
    }

    /// Copy a horizontal line (clipped). x1, x2 are inclusive endpoints.
    pub fn copy_hline(&mut self, mut x1: i32, y: i32, mut x2: i32, c: &PF::ColorType) {
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y > self.ymax() || y < self.ymin() || x1 > self.xmax() || x2 < self.xmin() {
            return;
        }
        x1 = x1.max(self.xmin());
        x2 = x2.min(self.xmax());
        self.ren.copy_hline(x1, y, (x2 - x1 + 1) as u32, c);
    }

    /// Blend a horizontal line (clipped). x1, x2 are inclusive endpoints.
    pub fn blend_hline(
        &mut self,
        mut x1: i32,
        y: i32,
        mut x2: i32,
        c: &PF::ColorType,
        cover: CoverType,
    ) {
        if x1 > x2 {
            std::mem::swap(&mut x1, &mut x2);
        }
        if y > self.ymax() || y < self.ymin() || x1 > self.xmax() || x2 < self.xmin() {
            return;
        }
        x1 = x1.max(self.xmin());
        x2 = x2.min(self.xmax());
        self.ren.blend_hline(x1, y, (x2 - x1 + 1) as u32, c, cover);
    }

    /// Blend a solid horizontal span with per-pixel coverage (clipped).
    pub fn blend_solid_hspan(
        &mut self,
        mut x: i32,
        y: i32,
        mut len: i32,
        c: &PF::ColorType,
        covers: &[CoverType],
    ) {
        if y > self.ymax() || y < self.ymin() {
            return;
        }

        let mut covers_offset = 0usize;
        if x < self.xmin() {
            let d = self.xmin() - x;
            len -= d;
            if len <= 0 {
                return;
            }
            covers_offset += d as usize;
            x = self.xmin();
        }
        if x + len > self.xmax() + 1 {
            len = self.xmax() - x + 1;
            if len <= 0 {
                return;
            }
        }
        self.ren
            .blend_solid_hspan(x, y, len as u32, c, &covers[covers_offset..]);
    }

    /// Blend a horizontal span with per-pixel colors (clipped).
    ///
    /// If `covers` is non-empty, each pixel uses its corresponding coverage.
    /// If `covers` is empty, all pixels use the uniform `cover` value.
    pub fn blend_color_hspan(
        &mut self,
        mut x: i32,
        y: i32,
        mut len: i32,
        colors: &[PF::ColorType],
        covers: &[CoverType],
        cover: CoverType,
    ) {
        if y > self.ymax() || y < self.ymin() {
            return;
        }

        let mut colors_offset = 0usize;
        let mut covers_offset = 0usize;
        if x < self.xmin() {
            let d = (self.xmin() - x) as usize;
            len -= d as i32;
            if len <= 0 {
                return;
            }
            if !covers.is_empty() {
                covers_offset += d;
            }
            colors_offset += d;
            x = self.xmin();
        }
        if x + len > self.xmax() + 1 {
            len = self.xmax() - x + 1;
            if len <= 0 {
                return;
            }
        }
        self.ren.blend_color_hspan(
            x,
            y,
            len as u32,
            &colors[colors_offset..],
            if covers.is_empty() {
                &[]
            } else {
                &covers[covers_offset..]
            },
            cover,
        );
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

    const BPP: usize = 4;

    fn make_renderer(w: u32, h: u32) -> (Vec<u8>, RowAccessor) {
        let stride = (w * BPP as u32) as i32;
        let buf = vec![0u8; (h * w * BPP as u32) as usize];
        let mut ra = RowAccessor::new();
        unsafe {
            ra.attach(buf.as_ptr() as *mut u8, w, h, stride);
        }
        (buf, ra)
    }

    #[test]
    fn test_new() {
        let (_buf, mut ra) = make_renderer(100, 80);
        let pf = PixfmtRgba32::new(&mut ra);
        let ren = RendererBase::new(pf);
        assert_eq!(ren.width(), 100);
        assert_eq!(ren.height(), 80);
        assert_eq!(ren.xmin(), 0);
        assert_eq!(ren.ymin(), 0);
        assert_eq!(ren.xmax(), 99);
        assert_eq!(ren.ymax(), 79);
    }

    #[test]
    fn test_clear() {
        let (_buf, mut ra) = make_renderer(10, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let white = Rgba8::new(255, 255, 255, 255);
        ren.clear(&white);
        let p = ren.ren().pixel(5, 5);
        assert_eq!(p.r, 255);
        assert_eq!(p.a, 255);
    }

    #[test]
    fn test_copy_pixel_clipped() {
        let (_buf, mut ra) = make_renderer(10, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let red = Rgba8::new(255, 0, 0, 255);
        // Inside clip box
        ren.copy_pixel(5, 5, &red);
        assert_eq!(ren.ren().pixel(5, 5).r, 255);
        // Outside clip box — should be silently ignored
        ren.copy_pixel(-1, 5, &red);
        ren.copy_pixel(100, 5, &red);
    }

    #[test]
    fn test_blend_hline_clipped() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let green = Rgba8::new(0, 255, 0, 255);
        // Line extends beyond right edge
        ren.blend_hline(15, 5, 25, &green, 255);
        // Pixels within bounds should be drawn
        assert_eq!(ren.ren().pixel(15, 5).g, 255);
        assert_eq!(ren.ren().pixel(19, 5).g, 255);
    }

    #[test]
    fn test_clip_box() {
        let (_buf, mut ra) = make_renderer(100, 100);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        assert!(ren.clip_box_i(10, 10, 50, 50));
        assert_eq!(ren.xmin(), 10);
        assert_eq!(ren.ymin(), 10);
        assert_eq!(ren.xmax(), 50);
        assert_eq!(ren.ymax(), 50);
    }

    #[test]
    fn test_clip_box_invalid() {
        let (_buf, mut ra) = make_renderer(100, 100);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        // Clip box entirely outside buffer
        assert!(!ren.clip_box_i(200, 200, 300, 300));
    }

    #[test]
    fn test_blend_solid_hspan_clipped() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let blue = Rgba8::new(0, 0, 255, 255);
        // Span starts before clip box left edge
        let covers = vec![255u8; 10];
        ren.blend_solid_hspan(-3, 5, 10, &blue, &covers);
        // First 3 pixels should be clipped, pixels 0..6 should be drawn
        assert_eq!(ren.ren().pixel(0, 5).b, 255);
        assert_eq!(ren.ren().pixel(6, 5).b, 255);
    }

    #[test]
    fn test_blend_solid_hspan_fully_clipped() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let red = Rgba8::new(255, 0, 0, 255);
        let covers = [255u8; 5];
        // Span entirely above clip box
        ren.blend_solid_hspan(5, -1, 5, &red, &covers);
        // Nothing should be drawn
        assert_eq!(ren.ren().pixel(5, 0).r, 0);
    }

    #[test]
    fn test_inbox() {
        let (_buf, mut ra) = make_renderer(10, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let ren = RendererBase::new(pf);
        assert!(ren.inbox(0, 0));
        assert!(ren.inbox(9, 9));
        assert!(!ren.inbox(-1, 0));
        assert!(!ren.inbox(10, 0));
        assert!(!ren.inbox(0, 10));
    }

    #[test]
    fn test_reset_clipping() {
        let (_buf, mut ra) = make_renderer(100, 100);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        ren.clip_box_i(10, 10, 50, 50);
        ren.reset_clipping(true);
        assert_eq!(ren.xmin(), 0);
        assert_eq!(ren.ymin(), 0);
        assert_eq!(ren.xmax(), 99);
        assert_eq!(ren.ymax(), 99);

        ren.reset_clipping(false);
        assert!(!ren.inbox(0, 0));
    }

    #[test]
    fn test_blend_color_hspan() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let colors = [
            Rgba8::new(255, 0, 0, 255),
            Rgba8::new(0, 255, 0, 255),
            Rgba8::new(0, 0, 255, 255),
        ];
        ren.blend_color_hspan(5, 3, 3, &colors, &[], 255);
        let p0 = ren.ren().pixel(5, 3);
        assert_eq!(p0.r, 255);
        let p1 = ren.ren().pixel(6, 3);
        assert_eq!(p1.g, 255);
        let p2 = ren.ren().pixel(7, 3);
        assert_eq!(p2.b, 255);
    }

    #[test]
    fn test_blend_color_hspan_clipped_left() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let colors = [
            Rgba8::new(255, 0, 0, 255),
            Rgba8::new(0, 255, 0, 255),
            Rgba8::new(0, 0, 255, 255),
        ];
        // Starts at x=-1, so first color is clipped
        ren.blend_color_hspan(-1, 3, 3, &colors, &[], 255);
        // colors[0] (red) at x=-1 → clipped
        // colors[1] (green) at x=0
        let p0 = ren.ren().pixel(0, 3);
        assert_eq!(p0.g, 255);
        // colors[2] (blue) at x=1
        let p1 = ren.ren().pixel(1, 3);
        assert_eq!(p1.b, 255);
    }

    #[test]
    fn test_blend_color_hspan_clipped_y() {
        let (_buf, mut ra) = make_renderer(20, 10);
        let pf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pf);
        let colors = [Rgba8::new(255, 0, 0, 255)];
        // y=-1 → fully clipped
        ren.blend_color_hspan(5, -1, 1, &colors, &[], 255);
        let p = ren.ren().pixel(5, 0);
        assert_eq!(p.r, 0);
    }
}

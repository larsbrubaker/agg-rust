//! Raster text renderer for embedded bitmap fonts.
//!
//! Port of `agg_renderer_raster_text.h`.
//! Renders text using binary raster glyphs from `GlyphRasterBin`.

use crate::glyph_raster_bin::{GlyphRasterBin, GlyphRect};
use crate::pixfmt_rgba::PixelFormat;
use crate::renderer_base::RendererBase;

// ============================================================================
// render_raster_htext_solid — render horizontal text with solid color
// ============================================================================

/// Render a horizontal text string using a raster glyph generator and solid color.
///
/// Port of C++ `renderer_raster_htext_solid::render_text()`.
/// Renders each character's binary glyph as a sequence of `blend_solid_hspan` calls.
pub fn render_raster_htext_solid<PF: PixelFormat>(
    ren: &mut RendererBase<PF>,
    glyph: &mut GlyphRasterBin,
    x: f64,
    y: f64,
    text: &str,
    color: &PF::ColorType,
    flip: bool,
) {
    let mut x = x;
    let mut y = y;
    let mut r = GlyphRect::default();

    for ch in text.bytes() {
        glyph.prepare(&mut r, x, y, ch as u32, flip);
        if r.x2 >= r.x1 {
            if flip {
                for i in r.y1..=r.y2 {
                    let span = glyph.span((r.y2 - i) as u32);
                    ren.blend_solid_hspan(r.x1, i, r.x2 - r.x1 + 1, color, span);
                }
            } else {
                for i in r.y1..=r.y2 {
                    let span = glyph.span((i - r.y1) as u32);
                    ren.blend_solid_hspan(r.x1, i, r.x2 - r.x1 + 1, color, span);
                }
            }
        }
        x += r.dx;
        y += r.dy;
    }
}

/// Render a vertical text string using a raster glyph generator and solid color.
///
/// Port of C++ `renderer_raster_vtext_solid::render_text()`.
pub fn render_raster_vtext_solid<PF: PixelFormat>(
    ren: &mut RendererBase<PF>,
    glyph: &mut GlyphRasterBin,
    x: f64,
    y: f64,
    text: &str,
    color: &PF::ColorType,
    flip: bool,
) {
    let mut x = x;
    let mut y = y;
    let mut r = GlyphRect::default();

    for ch in text.bytes() {
        glyph.prepare(&mut r, x, y, ch as u32, !flip);
        if r.x2 >= r.x1 {
            if flip {
                for i in r.y1..=r.y2 {
                    let span = glyph.span((i - r.y1) as u32);
                    ren.blend_solid_vspan(i, r.x1, r.x2 - r.x1 + 1, color, span);
                }
            } else {
                for i in r.y1..=r.y2 {
                    let span = glyph.span((r.y2 - i) as u32);
                    ren.blend_solid_vspan(i, r.x1, r.x2 - r.x1 + 1, color, span);
                }
            }
        }
        x += r.dx;
        y += r.dy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;
    use crate::pixfmt_rgba::PixfmtRgba32;
    use crate::rendering_buffer::RowAccessor;

    // Minimal test font: height=2, baseline=0, start_char=65('A'), num_chars=1
    fn make_test_font() -> Vec<u8> {
        let mut font = Vec::new();
        font.push(2); // height
        font.push(0); // baseline
        font.push(65); // start_char = 'A'
        font.push(1); // num_chars = 1
        font.push(0); // glyph offset lo
        font.push(0); // glyph offset hi
        font.push(2); // glyph_width = 2
        font.push(0b1100_0000); // row 0
        font.push(0b1100_0000); // row 1
        font
    }

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
    fn test_render_htext() {
        let font_data = make_test_font();
        let mut glyph = GlyphRasterBin::new(&font_data);
        let (_buf, mut ra) = make_buffer(20, 20);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);

        let red = Rgba8::new(255, 0, 0, 255);
        render_raster_htext_solid(&mut ren, &mut glyph, 5.0, 5.0, "A", &red, false);

        // Check that some pixels were rendered.
        // With baseline=0, height=2, non-flipped: y1 = 5 - 0 + 1 = 6, y2 = 7
        let p = ren.ren().pixel(5, 6);
        assert!(p.r > 0 || p.a > 0, "Expected rendered pixel at (5,6)");
    }

    #[test]
    fn test_render_empty_string() {
        let font_data = make_test_font();
        let mut glyph = GlyphRasterBin::new(&font_data);
        let (_buf, mut ra) = make_buffer(20, 20);
        let pixf = PixfmtRgba32::new(&mut ra);
        let mut ren = RendererBase::new(pixf);

        let red = Rgba8::new(255, 0, 0, 255);
        render_raster_htext_solid(&mut ren, &mut glyph, 5.0, 5.0, "", &red, false);
        // Should not panic or crash
    }
}

//! Port of agg-sharp `RasterTextTests.cs`: glyph_raster_bin, the embedded raster fonts and the
//! raster text renderer against C++ AGG. The expected bytes are copied from
//! agg_embedded_raster_fonts.cpp's verdana12 (header 12, 3, 32, 128-32; 'A' and 'W' below).

use crate::common::Image;
use agg_rust::color::Rgba8;
use agg_rust::embedded_raster_fonts::{by_name, FONTS};
use agg_rust::glyph_raster_bin::{GlyphRasterBin, GlyphRect};
use agg_rust::pixfmt_rgba::PixfmtRgba32;
use agg_rust::renderer_base::RendererBase;
use agg_rust::renderer_raster_text::render_raster_htext_solid;

/// verdana12 'A': 8 wide, one byte per row, top row first.
const VERDANA_A: [u8; 12] = [
    0x00, 0x00, 0x00, 0x18, 0x18, 0x24, 0x24, 0x7E, 0x42, 0x42, 0x00, 0x00,
];

/// verdana12 'W': 9 wide, two bytes per row.
const VERDANA_W: [u8; 24] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, 0x00, 0x49, 0x00, 0x49, 0x00, 0x55, 0x00, 0x55, 0x00,
    0x22, 0x00, 0x22, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// All 34 C++ fonts are embedded, in the C++ order, with their C++ headers.
#[test]
fn every_cpp_font_is_embedded() {
    assert_eq!(FONTS.len(), 34);
    assert_eq!(FONTS.first().unwrap().0, "gse4x6");
    assert_eq!(FONTS.last().unwrap().0, "verdana18_bold");
    assert_eq!(by_name("verdana12").unwrap()[..4], [12, 3, 32, 128 - 32]);
    assert_eq!(by_name("gse4x6").unwrap()[..4], [6, 0, 32, 128 - 32]);
}

/// Height, base line, string width and a glyph's box and advance, as C++ glyph_raster_bin gives them.
#[test]
fn glyph_metrics_match_cpp() {
    let mut glyph = GlyphRasterBin::new(by_name("verdana12").unwrap());
    assert_eq!(glyph.height(), 12.0);
    assert_eq!(glyph.base_line(), 3.0);
    assert_eq!(glyph.width("AW"), 17.0);

    let mut r = GlyphRect::default();
    glyph.prepare(&mut r, 5.7, 20.9, 'W' as u32, false);
    assert_eq!(
        (r.x1, r.x2, r.y1, r.y2, r.dx, r.dy),
        (5, 13, 18, 29, 9.0, 0.0)
    );

    glyph.prepare(&mut r, 5.7, 20.9, 'W' as u32, true);
    assert_eq!((r.y1, r.y2), (11, 22));
}

/// "AW" through renderer_raster_htext_solid lands each font bit on its C++ pixel, y up and y
/// down, including rows that span two bytes.
#[test]
fn rendered_glyphs_match_cpp() {
    let white = Rgba8::new(255, 255, 255, 255);
    let black = Rgba8::new(0, 0, 0, 255);
    for flip in [false, true] {
        let mut image = Image::new(40, 30);
        {
            let mut ren = RendererBase::new(PixfmtRgba32::new(&mut image.ra));
            ren.clear(&white);
            let mut glyph = GlyphRasterBin::new(by_name("verdana12").unwrap());
            render_raster_htext_solid(&mut ren, &mut glyph, 2.0, 14.0, "AW", &black, flip);
        }

        // Bitmap row k (from the top) is image row y1 + 11 - k y up and y1 + k y down.
        let y1: i32 = if flip { 14 - 12 + 3 } else { 14 - 3 + 1 };
        let mut mismatches = 0;
        for y in 0..image.height as i32 {
            for x in 0..image.width as i32 {
                let k = if flip { y - y1 } else { y1 + 11 - y };
                let mut set = false;
                if (0..12).contains(&k) {
                    let k = k as usize;
                    if (2..10).contains(&x) {
                        set = (VERDANA_A[k] & (0x80 >> (x - 2))) != 0;
                    } else if (10..19).contains(&x) {
                        let bit = (x - 10) as usize;
                        set = (VERDANA_W[(k * 2) + (bit >> 3)] & (0x80 >> (bit & 7))) != 0;
                    }
                }

                let expected = if set {
                    [0, 0, 0, 255]
                } else {
                    [255, 255, 255, 255]
                };
                if image.pixel(x as u32, y as u32) != expected {
                    mismatches += 1;
                }
            }
        }

        assert_eq!(mismatches, 0, "flip {flip}");
    }
}

/// A char the font has no glyph for draws nothing and does not move the pen (C++ reads past
/// its table).
#[test]
fn char_outside_the_font_is_empty() {
    let mut glyph = GlyphRasterBin::new(by_name("gse4x6").unwrap());
    let mut r = GlyphRect::default();
    glyph.prepare(&mut r, 3.0, 3.0, 0x2014, false);
    assert!(r.x2 < r.x1);
    assert_eq!(r.dx, 0.0);
    assert_eq!(glyph.width("a\u{2014}b"), glyph.width("ab"));
}

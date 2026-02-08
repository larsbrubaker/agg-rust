//! RGBA span pattern generator.
//!
//! Port of `agg_span_pattern_rgba.h`.
//! Generates pixel spans by reading from a tiled source image with
//! coordinate offsets. Used for repeating pattern fills.

use crate::color::Rgba8;
use crate::image_accessors::ImageSource;
use crate::renderer_scanline::SpanGenerator;

/// RGBA span pattern generator — fills spans from a tiled source image.
///
/// Port of C++ `span_pattern_rgba<Source>`.
/// Reads pixels from the attached `ImageSource`, applying x/y offsets
/// for pattern positioning.
pub struct SpanPatternRgba<Src> {
    src: Src,
    offset_x: u32,
    offset_y: u32,
}

impl<Src: ImageSource> SpanPatternRgba<Src> {
    pub fn new(src: Src, offset_x: u32, offset_y: u32) -> Self {
        Self {
            src,
            offset_x,
            offset_y,
        }
    }

    pub fn source(&self) -> &Src {
        &self.src
    }

    pub fn source_mut(&mut self) -> &mut Src {
        &mut self.src
    }

    pub fn offset_x(&self) -> u32 {
        self.offset_x
    }

    pub fn set_offset_x(&mut self, v: u32) {
        self.offset_x = v;
    }

    pub fn offset_y(&self) -> u32 {
        self.offset_y
    }

    pub fn set_offset_y(&mut self, v: u32) {
        self.offset_y = v;
    }
}

impl<Src: ImageSource> SpanGenerator for SpanPatternRgba<Src> {
    type Color = Rgba8;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [Rgba8], x: i32, y: i32, len: u32) {
        let sx = x + self.offset_x as i32;
        let sy = y + self.offset_y as i32;

        let p = self.src.span(sx, sy, len);
        span[0] = Rgba8::new(p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32);

        for i in 1..len as usize {
            let p = self.src.next_x();
            span[i] = Rgba8::new(p[0] as u32, p[1] as u32, p[2] as u32, p[3] as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test image source that returns a fixed color.
    struct ConstantSource {
        pixel: [u8; 4],
    }

    impl ImageSource for ConstantSource {
        fn span(&mut self, _x: i32, _y: i32, _len: u32) -> &[u8] {
            &self.pixel
        }

        fn next_x(&mut self) -> &[u8] {
            &self.pixel
        }

        fn next_y(&mut self) -> &[u8] {
            &self.pixel
        }
    }

    #[test]
    fn test_constant_pattern() {
        let src = ConstantSource {
            pixel: [255, 0, 0, 255],
        };
        let mut pattern = SpanPatternRgba::new(src, 0, 0);
        pattern.prepare();

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 5];
        pattern.generate(&mut span, 0, 0, 5);

        for c in &span {
            assert_eq!(c.r, 255);
            assert_eq!(c.g, 0);
            assert_eq!(c.b, 0);
            assert_eq!(c.a, 255);
        }
    }

    #[test]
    fn test_offset() {
        let src = ConstantSource {
            pixel: [128, 128, 128, 255],
        };
        let mut pattern = SpanPatternRgba::new(src, 10, 20);
        assert_eq!(pattern.offset_x(), 10);
        assert_eq!(pattern.offset_y(), 20);

        pattern.set_offset_x(5);
        pattern.set_offset_y(15);
        assert_eq!(pattern.offset_x(), 5);
        assert_eq!(pattern.offset_y(), 15);
    }
}

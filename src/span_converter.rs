//! Composable span pipeline.
//!
//! Port of `agg_span_converter.h` — wraps a span generator and applies a
//! post-processing conversion to the generated span colors.

use crate::renderer_scanline::SpanGenerator;

// ============================================================================
// SpanConverterFunction trait
// ============================================================================

/// Trait for span conversion functions.
///
/// Converts a span of colors in-place. Used to post-process generated spans
/// (e.g., applying alpha masks, color adjustments, etc.).
pub trait SpanConverterFunction {
    type Color;

    fn prepare(&mut self) {}

    fn convert(&mut self, span: &mut [Self::Color], x: i32, y: i32, len: u32);
}

// ============================================================================
// SpanConverter
// ============================================================================

/// Composable span pipeline.
///
/// Combines a span generator with a span converter: first generates the span,
/// then applies the conversion function to the result.
///
/// Port of C++ `span_converter<SpanGenerator, SpanConverter>`.
pub struct SpanConverter<SG: SpanGenerator, SC: SpanConverterFunction<Color = SG::Color>> {
    span_gen: SG,
    span_cnv: SC,
}

impl<SG: SpanGenerator, SC: SpanConverterFunction<Color = SG::Color>> SpanConverter<SG, SC> {
    pub fn new(span_gen: SG, span_cnv: SC) -> Self {
        Self { span_gen, span_cnv }
    }

    pub fn generator(&self) -> &SG {
        &self.span_gen
    }

    pub fn generator_mut(&mut self) -> &mut SG {
        &mut self.span_gen
    }

    pub fn converter(&self) -> &SC {
        &self.span_cnv
    }

    pub fn converter_mut(&mut self) -> &mut SC {
        &mut self.span_cnv
    }
}

impl<SG: SpanGenerator, SC: SpanConverterFunction<Color = SG::Color>> SpanGenerator
    for SpanConverter<SG, SC>
{
    type Color = SG::Color;

    fn prepare(&mut self) {
        self.span_gen.prepare();
        self.span_cnv.prepare();
    }

    fn generate(&mut self, span: &mut [Self::Color], x: i32, y: i32, len: u32) {
        self.span_gen.generate(span, x, y, len);
        self.span_cnv.convert(span, x, y, len);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;

    struct FillRed;
    impl SpanGenerator for FillRed {
        type Color = Rgba8;
        fn prepare(&mut self) {}
        fn generate(&mut self, span: &mut [Rgba8], _x: i32, _y: i32, len: u32) {
            for pixel in span.iter_mut().take(len as usize) {
                *pixel = Rgba8::new(255, 0, 0, 255);
            }
        }
    }

    struct HalveAlpha;
    impl SpanConverterFunction for HalveAlpha {
        type Color = Rgba8;
        fn convert(&mut self, span: &mut [Rgba8], _x: i32, _y: i32, len: u32) {
            for pixel in span.iter_mut().take(len as usize) {
                pixel.a /= 2;
            }
        }
    }

    #[test]
    fn test_converter_pipeline() {
        let gen = FillRed;
        let cnv = HalveAlpha;
        let mut pipeline = SpanConverter::new(gen, cnv);

        pipeline.prepare();
        let mut span = vec![Rgba8::new(0, 0, 0, 0); 3];
        pipeline.generate(&mut span, 0, 0, 3);

        assert_eq!(span[0].r, 255);
        assert_eq!(span[0].a, 127); // 255 / 2
        assert_eq!(span[2].r, 255);
        assert_eq!(span[2].a, 127);
    }

    #[test]
    fn test_access_inner() {
        let gen = FillRed;
        let cnv = HalveAlpha;
        let pipeline = SpanConverter::new(gen, cnv);
        let _gen_ref = pipeline.generator();
        let _cnv_ref = pipeline.converter();
    }

    #[test]
    fn test_identity_converter() {
        struct IdentityConv;
        impl SpanConverterFunction for IdentityConv {
            type Color = Rgba8;
            fn convert(&mut self, _span: &mut [Rgba8], _x: i32, _y: i32, _len: u32) {}
        }

        let gen = FillRed;
        let cnv = IdentityConv;
        let mut pipeline = SpanConverter::new(gen, cnv);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 2];
        pipeline.generate(&mut span, 0, 0, 2);
        assert_eq!(span[0].r, 255);
        assert_eq!(span[0].a, 255); // unchanged
    }

    #[test]
    fn test_converter_receives_coordinates() {
        struct TrackCoords {
            last_x: i32,
            last_y: i32,
        }
        impl SpanConverterFunction for TrackCoords {
            type Color = Rgba8;
            fn convert(&mut self, _span: &mut [Rgba8], x: i32, y: i32, _len: u32) {
                self.last_x = x;
                self.last_y = y;
            }
        }

        let gen = FillRed;
        let cnv = TrackCoords {
            last_x: 0,
            last_y: 0,
        };
        let mut pipeline = SpanConverter::new(gen, cnv);

        let mut span = vec![Rgba8::new(0, 0, 0, 0); 1];
        pipeline.generate(&mut span, 42, 99, 1);
        assert_eq!(pipeline.converter().last_x, 42);
        assert_eq!(pipeline.converter().last_y, 99);
    }
}

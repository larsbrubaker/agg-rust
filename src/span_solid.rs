//! Solid color span generator.
//!
//! Port of `agg_span_solid.h` — the simplest span generator, fills every
//! pixel in the span with a single solid color.

use crate::renderer_scanline::SpanGenerator;

// ============================================================================
// SpanSolid
// ============================================================================

/// Solid color span generator.
///
/// Fills every pixel with the same color. Useful as a baseline span
/// generator and for testing the generic span rendering pipeline.
///
/// Port of C++ `span_solid<ColorT>`.
pub struct SpanSolid<C> {
    color: C,
}

impl<C: Clone + Default> SpanSolid<C> {
    pub fn new() -> Self {
        Self {
            color: C::default(),
        }
    }

    pub fn set_color(&mut self, c: C) {
        self.color = c;
    }

    pub fn color(&self) -> &C {
        &self.color
    }
}

impl<C: Clone + Default> Default for SpanSolid<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Clone + Default> SpanGenerator for SpanSolid<C> {
    type Color = C;

    fn prepare(&mut self) {}

    fn generate(&mut self, span: &mut [C], _x: i32, _y: i32, len: u32) {
        for c in span.iter_mut().take(len as usize) {
            *c = self.color.clone();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba8;

    #[test]
    fn test_new_defaults() {
        let gen = SpanSolid::<Rgba8>::new();
        let c = gen.color();
        assert_eq!(c.r, 0);
        assert_eq!(c.a, 0);
    }

    #[test]
    fn test_set_and_get_color() {
        let mut gen = SpanSolid::new();
        gen.set_color(Rgba8::new(255, 0, 0, 255));
        assert_eq!(gen.color().r, 255);
        assert_eq!(gen.color().a, 255);
    }

    #[test]
    fn test_generate_fills_span() {
        let mut gen = SpanSolid::new();
        gen.set_color(Rgba8::new(100, 150, 200, 255));
        let mut span = vec![Rgba8::default(); 5];
        gen.generate(&mut span, 10, 20, 5);
        for c in &span {
            assert_eq!(c.r, 100);
            assert_eq!(c.g, 150);
            assert_eq!(c.b, 200);
            assert_eq!(c.a, 255);
        }
    }

    #[test]
    fn test_generate_partial_len() {
        let mut gen = SpanSolid::new();
        gen.set_color(Rgba8::new(255, 0, 0, 255));
        let mut span = vec![Rgba8::default(); 5];
        // Only fill first 3
        gen.generate(&mut span, 0, 0, 3);
        assert_eq!(span[0].r, 255);
        assert_eq!(span[2].r, 255);
        // Remaining should still be default
        assert_eq!(span[3].r, 0);
    }
}

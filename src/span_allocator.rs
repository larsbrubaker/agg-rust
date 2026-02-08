//! Span color buffer allocator.
//!
//! Port of `agg_span_allocator.h` — manages a reusable buffer for span
//! generators to write per-pixel colors into during rendering.

// ============================================================================
// SpanAllocator
// ============================================================================

/// Reusable color buffer for span generators.
///
/// Allocates and reuses a buffer of color values. The buffer grows as
/// needed (aligned to 256 elements) but never shrinks, avoiding
/// repeated allocations during rendering.
///
/// Port of C++ `span_allocator<ColorT>`.
pub struct SpanAllocator<C> {
    span: Vec<C>,
}

impl<C: Default + Clone> SpanAllocator<C> {
    pub fn new() -> Self {
        Self { span: Vec::new() }
    }

    /// Allocate (or reuse) a buffer of at least `span_len` elements.
    ///
    /// Returns a mutable slice of exactly `span_len` elements.
    /// The buffer may be larger internally (aligned to 256).
    pub fn allocate(&mut self, span_len: usize) -> &mut [C] {
        if span_len > self.span.len() {
            // Align to 256 elements to reduce reallocations
            let new_size = ((span_len + 255) >> 8) << 8;
            self.span.resize(new_size, C::default());
        }
        &mut self.span[..span_len]
    }

    pub fn span(&mut self) -> &mut [C] {
        &mut self.span
    }

    pub fn max_span_len(&self) -> usize {
        self.span.len()
    }
}

impl<C: Default + Clone> Default for SpanAllocator<C> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let alloc = SpanAllocator::<u8>::new();
        assert_eq!(alloc.max_span_len(), 0);
    }

    #[test]
    fn test_allocate_grows() {
        let mut alloc = SpanAllocator::<u32>::new();
        let span = alloc.allocate(10);
        assert_eq!(span.len(), 10);
        // Internal buffer should be aligned to 256
        assert_eq!(alloc.max_span_len(), 256);
    }

    #[test]
    fn test_allocate_reuses() {
        let mut alloc = SpanAllocator::<u32>::new();
        alloc.allocate(100);
        assert_eq!(alloc.max_span_len(), 256);
        // Smaller allocation reuses existing buffer
        let span = alloc.allocate(50);
        assert_eq!(span.len(), 50);
        assert_eq!(alloc.max_span_len(), 256);
    }

    #[test]
    fn test_allocate_alignment() {
        let mut alloc = SpanAllocator::<u8>::new();
        alloc.allocate(257);
        assert_eq!(alloc.max_span_len(), 512);
    }

    #[test]
    fn test_span_accessor() {
        let mut alloc = SpanAllocator::<u32>::new();
        alloc.allocate(10);
        let span = alloc.span();
        assert!(span.len() >= 10);
    }
}

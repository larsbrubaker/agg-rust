//! Scanline boolean algebra.
//!
//! Port of `agg_scanline_boolean_algebra.h`: boolean operations (union, intersection,
//! subtraction, the three XORs) on shapes delivered as scanlines, anti-aliased or binary.
//!
//! The C++ templates map onto these pieces:
//! - the traits here stand in for C++'s template concepts: [`ScanlineGenerator`] (a
//!   rasterizer or a scanline storage), [`ScanlineSpans`] / [`ScanlineCovers`] (a scanline's
//!   span iterator) and [`ScanlineSink`] (the renderer the result goes to); `adapters.rs`
//!   implements them for agg-rust's rasterizer, scanlines and storages;
//! - [`spans`] holds the span functors and the XOR formulas;
//! - [`scanlines`] combines two scanlines of the same Y span by span;
//! - [`shapes`] walks two generators and holds the `sbool_*_shapes*` functions, including
//!   C++'s `sbool_combine_shapes_aa` / `sbool_combine_shapes_bin` dispatchers;
//! - [`legacy`]'s storage-staging functions are re-exported here under the names
//!   `sbool_combine_shapes_aa` / `sbool_combine_shapes_bin` (agg-rust's original API) and run
//!   through the same faithful code.

use crate::rasterizer_scanline_aa::Scanline;

mod adapters;
pub mod legacy;
pub mod scanlines;
pub mod shapes;
pub mod spans;
#[cfg(test)]
mod tests;

pub use legacy::{
    sbool_combine_shapes_aa, sbool_combine_shapes_bin, sbool_combine_storages_aa,
    sbool_combine_storages_bin,
};
pub use scanlines::{sbool_add_spans_and_render, sbool_intersect_scanlines, sbool_unite_scanlines};
pub use shapes::{
    sbool_intersect_shapes, sbool_intersect_shapes_aa, sbool_intersect_shapes_bin,
    sbool_subtract_shapes, sbool_subtract_shapes_aa, sbool_subtract_shapes_bin, sbool_unite_shapes,
    sbool_unite_shapes_aa, sbool_unite_shapes_bin, sbool_xor_shapes_aa,
    sbool_xor_shapes_abs_diff_aa, sbool_xor_shapes_bin, sbool_xor_shapes_saddle_aa,
};
pub use spans::{
    AddSpan, CombineSpans, SboolAddSpanAa, SboolAddSpanBin, SboolAddSpanEmpty,
    SboolCombineSpansBin, SboolCombineSpansEmpty, SboolIntersectSpansAa, SboolSubtractSpansAa,
    SboolUniteSpansAa, SboolXorFormula, SboolXorFormulaAbsDiff, SboolXorFormulaLinear,
    SboolXorFormulaSaddle, SboolXorSpansAa,
};

/// Boolean operation type (C++ `sbool_op_e`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SBoolOp {
    Or,
    And,
    Xor,
    XorSaddle,
    XorAbsDiff,
    AMinusB,
    BMinusA,
}

/// A source of scanlines: C++'s "scanline generator" concept, met by
/// `rasterizer_scanline_aa` and the scanline storages.
pub trait ScanlineGenerator {
    /// Start over at the first scanline; false when there are none.
    fn rewind_scanlines(&mut self) -> bool;
    fn min_x(&self) -> i32;
    fn min_y(&self) -> i32;
    fn max_x(&self) -> i32;
    fn max_y(&self) -> i32;
    /// Fill `sl` with the next scanline; false when all have been delivered.
    fn sweep_scanline<SL: Scanline>(&mut self, sl: &mut SL) -> bool;
}

/// A scanline whose spans can be read back: C++'s scanline `const_iterator`.
///
/// Spans are indexed `0..num_spans()`. A negative length marks a solid span of `-len`
/// pixels sharing one cover (packed scanlines only).
pub trait ScanlineSpans: Scanline {
    /// Prepare for scanlines covering `min_x..=max_x` (C++ `reset`).
    fn reset(&mut self, min_x: i32, max_x: i32);
    /// The first pixel of span `i`.
    fn span_x(&self, i: usize) -> i32;
    /// The signed length of span `i` (negative: solid).
    fn span_len(&self, i: usize) -> i32;
}

/// A scanline that also carries covers (every AA scanline; not `scanline_bin`).
pub trait ScanlineCovers: ScanlineSpans {
    /// Span `i`'s covers from its first pixel on: `len` of them for a span of per-pixel
    /// covers, the single shared cover first for a solid span.
    fn span_covers(&self, i: usize) -> &[u8];
}

/// Where combined scanlines go: C++'s renderer concept (`prepare` then `render` per line).
pub trait ScanlineSink<SL> {
    fn prepare(&mut self);
    fn render(&mut self, sl: &SL);
}

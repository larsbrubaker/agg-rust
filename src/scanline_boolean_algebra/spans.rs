//! Span functors and XOR formulas of `agg_scanline_boolean_algebra.h`.
//!
//! Each functor combines (or adds) the overlap of spans at `x..x+len` into the result
//! scanline, exactly as C++'s `operator()`s do, including the solid-span shortcuts (an
//! intersection with a full solid span adds the other span's covers unchanged). The
//! scanline walkers in `scanlines.rs` call them; `shapes.rs` picks which ones to use.

use std::marker::PhantomData;

use super::{ScanlineCovers, ScanlineSpans};
use crate::rasterizer_scanline_aa::Scanline;

const COVER_SHIFT: u32 = 8;
const COVER_SIZE: u32 = 1 << COVER_SHIFT;
const COVER_MASK: u32 = COVER_SIZE - 1;
const COVER_FULL: u32 = COVER_MASK;

/// Combines the overlap `x..x+len` of span `i1` of `sl1` and span `i2` of `sl2` into `sl`.
pub trait CombineSpans<S1, S2, SL> {
    #[allow(clippy::too_many_arguments)]
    fn combine(&self, sl1: &S1, i1: usize, sl2: &S2, i2: usize, x: i32, len: u32, sl: &mut SL);
}

/// Adds the part `x..x+len` of span `i` of `sl1` to `sl`.
pub trait AddSpan<S1, SL> {
    fn add(&self, sl1: &S1, i: usize, x: i32, len: u32, sl: &mut SL);
}

/// `cover == full*full ? full : cover >> shift`, the scaling every AA functor applies.
#[inline]
pub(super) fn scale_cover(cover: u32) -> u32 {
    if cover == COVER_FULL * COVER_FULL {
        COVER_FULL
    } else {
        cover >> COVER_SHIFT
    }
}

/// Intersection of two covers: `a * b`, scaled.
#[inline]
pub(super) fn intersect_covers(a: u32, b: u32) -> u32 {
    scale_cover(a * b)
}

/// Union of two covers: `mask*mask - (mask-a)*(mask-b)`, scaled.
#[inline]
pub(super) fn unite_covers(a: u32, b: u32) -> u32 {
    scale_cover(COVER_MASK * COVER_MASK - (COVER_MASK - a) * (COVER_MASK - b))
}

/// Subtraction of two covers before scaling: `a * (mask - b)`. The functor tests this raw
/// value for zero, as C++ does, so a tiny nonzero product still adds a cell of cover 0.
#[inline]
pub(super) fn subtract_covers_raw(a: u32, b: u32) -> u32 {
    a * (COVER_MASK - b)
}

/// Covers of span `i` from pixel `x` on (C++: `if(span->x < x) covers += x - span->x`).
#[inline]
fn covers_from<S: ScanlineCovers>(sl: &S, i: usize, x: i32) -> &[u8] {
    let covers = sl.span_covers(i);
    let sx = sl.span_x(i);
    if sx < x {
        &covers[(x - sx) as usize..]
    } else {
        covers
    }
}

/// C++ `sbool_combine_spans_bin`: the overlap becomes a full span.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolCombineSpansBin;

impl<S1, S2, SL: Scanline> CombineSpans<S1, S2, SL> for SboolCombineSpansBin {
    fn combine(&self, _: &S1, _: usize, _: &S2, _: usize, x: i32, len: u32, sl: &mut SL) {
        sl.add_span(x, len, COVER_FULL);
    }
}

/// C++ `sbool_combine_spans_empty`: the overlap is dropped (binary XOR and subtraction).
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolCombineSpansEmpty;

impl<S1, S2, SL> CombineSpans<S1, S2, SL> for SboolCombineSpansEmpty {
    fn combine(&self, _: &S1, _: usize, _: &S2, _: usize, _: i32, _: u32, _: &mut SL) {}
}

/// C++ `sbool_add_span_empty`: adds nothing (the subtrahend's own spans).
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolAddSpanEmpty;

impl<S1, SL> AddSpan<S1, SL> for SboolAddSpanEmpty {
    fn add(&self, _: &S1, _: usize, _: i32, _: u32, _: &mut SL) {}
}

/// C++ `sbool_add_span_bin`: adds a full span.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolAddSpanBin;

impl<S1, SL: Scanline> AddSpan<S1, SL> for SboolAddSpanBin {
    fn add(&self, _: &S1, _: usize, x: i32, len: u32, sl: &mut SL) {
        sl.add_span(x, len, COVER_FULL);
    }
}

/// C++ `sbool_add_span_aa`: adds the span with its anti-aliasing kept.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolAddSpanAa;

impl<S1: ScanlineCovers, SL: Scanline> AddSpan<S1, SL> for SboolAddSpanAa {
    fn add(&self, sl1: &S1, i: usize, x: i32, len: u32, sl: &mut SL) {
        let span_len = sl1.span_len(i);
        if span_len < 0 {
            sl.add_span(x, len, sl1.span_covers(i)[0] as u32);
        } else if span_len > 0 {
            sl.add_cells(x, len, covers_from(sl1, i, x));
        }
    }
}

/// The C++ operation code: 0 both AA, 1 span1 solid, 2 span2 solid, 3 both solid.
#[inline]
fn op_code<S1: ScanlineSpans, S2: ScanlineSpans>(sl1: &S1, i1: usize, sl2: &S2, i2: usize) -> u32 {
    (sl1.span_len(i1) < 0) as u32 | (((sl2.span_len(i2) < 0) as u32) << 1)
}

/// C++ `sbool_intersect_spans_aa`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolIntersectSpansAa;

impl<S1: ScanlineCovers, S2: ScanlineCovers, SL: Scanline> CombineSpans<S1, S2, SL>
    for SboolIntersectSpansAa
{
    fn combine(&self, sl1: &S1, i1: usize, sl2: &S2, i2: usize, x: i32, len: u32, sl: &mut SL) {
        let n = len as usize;
        match op_code(sl1, i1, sl2, i2) {
            0 => {
                let c1 = covers_from(sl1, i1, x);
                let c2 = covers_from(sl2, i2, x);
                for k in 0..n {
                    sl.add_cell(x + k as i32, intersect_covers(c1[k] as u32, c2[k] as u32));
                }
            }
            1 => {
                let c2 = covers_from(sl2, i2, x);
                let s1 = sl1.span_covers(i1)[0] as u32;
                if s1 == COVER_FULL {
                    sl.add_cells(x, len, c2);
                } else {
                    for (k, &c) in c2[..n].iter().enumerate() {
                        sl.add_cell(x + k as i32, intersect_covers(s1, c as u32));
                    }
                }
            }
            2 => {
                let c1 = covers_from(sl1, i1, x);
                let s2 = sl2.span_covers(i2)[0] as u32;
                if s2 == COVER_FULL {
                    sl.add_cells(x, len, c1);
                } else {
                    for (k, &c) in c1[..n].iter().enumerate() {
                        sl.add_cell(x + k as i32, intersect_covers(c as u32, s2));
                    }
                }
            }
            _ => {
                let cover =
                    intersect_covers(sl1.span_covers(i1)[0] as u32, sl2.span_covers(i2)[0] as u32);
                sl.add_span(x, len, cover);
            }
        }
    }
}

/// C++ `sbool_unite_spans_aa`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolUniteSpansAa;

impl<S1: ScanlineCovers, S2: ScanlineCovers, SL: Scanline> CombineSpans<S1, S2, SL>
    for SboolUniteSpansAa
{
    fn combine(&self, sl1: &S1, i1: usize, sl2: &S2, i2: usize, x: i32, len: u32, sl: &mut SL) {
        let n = len as usize;
        match op_code(sl1, i1, sl2, i2) {
            0 => {
                let c1 = covers_from(sl1, i1, x);
                let c2 = covers_from(sl2, i2, x);
                for k in 0..n {
                    sl.add_cell(x + k as i32, unite_covers(c1[k] as u32, c2[k] as u32));
                }
            }
            1 => {
                let c2 = covers_from(sl2, i2, x);
                let s1 = sl1.span_covers(i1)[0] as u32;
                if s1 == COVER_FULL {
                    sl.add_span(x, len, COVER_FULL);
                } else {
                    for (k, &c) in c2[..n].iter().enumerate() {
                        sl.add_cell(x + k as i32, unite_covers(s1, c as u32));
                    }
                }
            }
            2 => {
                let c1 = covers_from(sl1, i1, x);
                let s2 = sl2.span_covers(i2)[0] as u32;
                if s2 == COVER_FULL {
                    sl.add_span(x, len, COVER_FULL);
                } else {
                    for (k, &c) in c1[..n].iter().enumerate() {
                        sl.add_cell(x + k as i32, unite_covers(c as u32, s2));
                    }
                }
            }
            _ => {
                let cover =
                    unite_covers(sl1.span_covers(i1)[0] as u32, sl2.span_covers(i2)[0] as u32);
                sl.add_span(x, len, cover);
            }
        }
    }
}

/// A C++ XOR formula (`sbool_xor_formula_*::calculate`).
pub trait SboolXorFormula {
    fn calculate(a: u32, b: u32) -> u32;
}

/// C++ `sbool_xor_formula_linear`: `a + b`, folded back above full.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolXorFormulaLinear;

impl SboolXorFormulaLinear {
    pub fn calculate(a: u32, b: u32) -> u32 {
        let cover = a + b;
        if cover > COVER_MASK {
            COVER_MASK + COVER_MASK - cover
        } else {
            cover
        }
    }
}

impl SboolXorFormula for SboolXorFormulaLinear {
    fn calculate(a: u32, b: u32) -> u32 {
        SboolXorFormulaLinear::calculate(a, b)
    }
}

/// C++ `sbool_xor_formula_saddle`: `1 - ((1 - a + ab) * (1 - b + ab))` in fixed point.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolXorFormulaSaddle;

impl SboolXorFormulaSaddle {
    pub fn calculate(a: u32, b: u32) -> u32 {
        let k = a * b;
        if k == COVER_MASK * COVER_MASK {
            return 0;
        }
        let a = (COVER_MASK * COVER_MASK - (a << COVER_SHIFT) + k) >> COVER_SHIFT;
        let b = (COVER_MASK * COVER_MASK - (b << COVER_SHIFT) + k) >> COVER_SHIFT;
        COVER_MASK - ((a * b) >> COVER_SHIFT)
    }
}

impl SboolXorFormula for SboolXorFormulaSaddle {
    fn calculate(a: u32, b: u32) -> u32 {
        SboolXorFormulaSaddle::calculate(a, b)
    }
}

/// C++ `sbool_xor_formula_abs_diff`: `|a - b|`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolXorFormulaAbsDiff;

impl SboolXorFormulaAbsDiff {
    pub fn calculate(a: u32, b: u32) -> u32 {
        (a as i32 - b as i32).unsigned_abs()
    }
}

impl SboolXorFormula for SboolXorFormulaAbsDiff {
    fn calculate(a: u32, b: u32) -> u32 {
        SboolXorFormulaAbsDiff::calculate(a, b)
    }
}

/// C++ `sbool_xor_spans_aa<..., XorFormula>`; zero results are left out.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolXorSpansAa<F>(PhantomData<F>);

impl<F> SboolXorSpansAa<F> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<F: SboolXorFormula, S1: ScanlineCovers, S2: ScanlineCovers, SL: Scanline>
    CombineSpans<S1, S2, SL> for SboolXorSpansAa<F>
{
    fn combine(&self, sl1: &S1, i1: usize, sl2: &S2, i2: usize, x: i32, len: u32, sl: &mut SL) {
        let n = len as usize;
        let mut emit = |k: usize, cover: u32| {
            if cover != 0 {
                sl.add_cell(x + k as i32, cover);
            }
        };
        match op_code(sl1, i1, sl2, i2) {
            0 => {
                let c1 = covers_from(sl1, i1, x);
                let c2 = covers_from(sl2, i2, x);
                for k in 0..n {
                    emit(k, F::calculate(c1[k] as u32, c2[k] as u32));
                }
            }
            1 => {
                let s1 = sl1.span_covers(i1)[0] as u32;
                for (k, &c) in covers_from(sl2, i2, x)[..n].iter().enumerate() {
                    emit(k, F::calculate(s1, c as u32));
                }
            }
            2 => {
                let s2 = sl2.span_covers(i2)[0] as u32;
                for (k, &c) in covers_from(sl1, i1, x)[..n].iter().enumerate() {
                    emit(k, F::calculate(c as u32, s2));
                }
            }
            _ => {
                let cover =
                    F::calculate(sl1.span_covers(i1)[0] as u32, sl2.span_covers(i2)[0] as u32);
                if cover != 0 {
                    sl.add_span(x, len, cover);
                }
            }
        }
    }
}

/// C++ `sbool_subtract_spans_aa`: `a * (mask - b)`, zero results left out.
#[derive(Debug, Clone, Copy, Default)]
pub struct SboolSubtractSpansAa;

impl<S1: ScanlineCovers, S2: ScanlineCovers, SL: Scanline> CombineSpans<S1, S2, SL>
    for SboolSubtractSpansAa
{
    fn combine(&self, sl1: &S1, i1: usize, sl2: &S2, i2: usize, x: i32, len: u32, sl: &mut SL) {
        let n = len as usize;
        let mut emit = |k: usize, cover: u32| {
            if cover != 0 {
                sl.add_cell(x + k as i32, scale_cover(cover));
            }
        };
        match op_code(sl1, i1, sl2, i2) {
            0 => {
                let c1 = covers_from(sl1, i1, x);
                let c2 = covers_from(sl2, i2, x);
                for k in 0..n {
                    emit(k, subtract_covers_raw(c1[k] as u32, c2[k] as u32));
                }
            }
            1 => {
                let s1 = sl1.span_covers(i1)[0] as u32;
                for (k, &c) in covers_from(sl2, i2, x)[..n].iter().enumerate() {
                    emit(k, subtract_covers_raw(s1, c as u32));
                }
            }
            2 => {
                let s2 = sl2.span_covers(i2)[0] as u32;
                if s2 != COVER_FULL {
                    for (k, &c) in covers_from(sl1, i1, x)[..n].iter().enumerate() {
                        emit(k, subtract_covers_raw(c as u32, s2));
                    }
                }
            }
            _ => {
                let cover = subtract_covers_raw(
                    sl1.span_covers(i1)[0] as u32,
                    sl2.span_covers(i2)[0] as u32,
                );
                if cover != 0 {
                    sl.add_span(x, len, scale_cover(cover));
                }
            }
        }
    }
}

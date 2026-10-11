//! Shape-level operations of `agg_scanline_boolean_algebra.h`: walk two scanline generators
//! in Y, combine scanlines of the same Y through `scanlines.rs`, and render the result.
//! `sbool_combine_shapes_aa` / `sbool_combine_shapes_bin` here are C++'s dispatchers over
//! [`SBoolOp`]; the crate root re-exports `legacy.rs`'s storage-staging functions of the same
//! names instead.

use super::scanlines::{
    sbool_add_spans_and_render, sbool_intersect_scanlines, sbool_unite_scanlines,
};
use super::spans::*;
use super::{SBoolOp, ScanlineCovers, ScanlineGenerator, ScanlineSink, ScanlineSpans};

/// C++ `sbool_intersect_shapes`: only scanlines of the same Y in both shapes are combined,
/// only inside the intersection of the two bounding boxes.
#[allow(clippy::too_many_arguments)]
pub fn sbool_intersect_shapes<G1, G2, S1, S2, SL, R, C>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
    combine_spans: C,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
    C: CombineSpans<S1, S2, SL>,
{
    // If either generator has no scanlines there is nothing to intersect.
    if !sg1.rewind_scanlines() {
        return;
    }
    if !sg2.rewind_scanlines() {
        return;
    }

    // The intersection of the bounding boxes; return if they don't intersect.
    let ix1 = sg1.min_x().max(sg2.min_x());
    let iy1 = sg1.min_y().max(sg2.min_y());
    let ix2 = sg1.max_x().min(sg2.max_x());
    let iy2 = sg1.max_y().min(sg2.max_y());
    if ix1 > ix2 || iy1 > iy2 {
        return;
    }

    sl.reset(ix1, ix2);
    sl1.reset(sg1.min_x(), sg1.max_x());
    sl2.reset(sg2.min_x(), sg2.max_x());
    if !sg1.sweep_scanline(sl1) {
        return;
    }
    if !sg2.sweep_scanline(sl2) {
        return;
    }

    ren.prepare();

    // Synchronize the scanlines by Y; only scanlines with the same Y are combined.
    loop {
        while sl1.y() < sl2.y() {
            if !sg1.sweep_scanline(sl1) {
                return;
            }
        }
        while sl2.y() < sl1.y() {
            if !sg2.sweep_scanline(sl2) {
                return;
            }
        }
        if sl1.y() == sl2.y() {
            sbool_intersect_scanlines(sl1, sl2, sl, &combine_spans);
            if sl.num_spans() != 0 {
                sl.finalize(sl1.y());
                ren.render(sl);
            }
            if !sg1.sweep_scanline(sl1) {
                return;
            }
            if !sg2.sweep_scanline(sl2) {
                return;
            }
        }
    }
}

/// C++ `sbool_unite_shapes`: every scanline of either shape, combined where both have one.
#[allow(clippy::too_many_arguments)]
pub fn sbool_unite_shapes<G1, G2, S1, S2, SL, R, A1, A2, C>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
    add_span1: A1,
    add_span2: A2,
    combine_spans: C,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
    A1: AddSpan<S1, SL>,
    A2: AddSpan<S2, SL>,
    C: CombineSpans<S1, S2, SL>,
{
    let mut flag1 = sg1.rewind_scanlines();
    let mut flag2 = sg2.rewind_scanlines();
    if !flag1 && !flag2 {
        return;
    }

    // The union of the bounding boxes of the shapes that have scanlines.
    let (ux1, uy1, ux2, uy2) = if flag1 && flag2 {
        (
            sg1.min_x().min(sg2.min_x()),
            sg1.min_y().min(sg2.min_y()),
            sg1.max_x().max(sg2.max_x()),
            sg1.max_y().max(sg2.max_y()),
        )
    } else if flag1 {
        (sg1.min_x(), sg1.min_y(), sg1.max_x(), sg1.max_y())
    } else {
        (sg2.min_x(), sg2.min_y(), sg2.max_x(), sg2.max_y())
    };
    if ux1 > ux2 || uy1 > uy2 {
        return;
    }

    ren.prepare();
    sl.reset(ux1, ux2);
    if flag1 {
        sl1.reset(sg1.min_x(), sg1.max_x());
        flag1 = sg1.sweep_scanline(sl1);
    }
    if flag2 {
        sl2.reset(sg2.min_x(), sg2.max_x());
        flag2 = sg2.sweep_scanline(sl2);
    }

    // Merge the scanlines by Y: the same Y is combined, a lone one is added as is.
    while flag1 || flag2 {
        if flag1 && flag2 {
            if sl1.y() == sl2.y() {
                sbool_unite_scanlines(sl1, sl2, sl, &add_span1, &add_span2, &combine_spans);
                if sl.num_spans() != 0 {
                    sl.finalize(sl1.y());
                    ren.render(sl);
                }
                flag1 = sg1.sweep_scanline(sl1);
                flag2 = sg2.sweep_scanline(sl2);
            } else if sl1.y() < sl2.y() {
                sbool_add_spans_and_render(sl1, sl, ren, &add_span1);
                flag1 = sg1.sweep_scanline(sl1);
            } else {
                sbool_add_spans_and_render(sl2, sl, ren, &add_span2);
                flag2 = sg2.sweep_scanline(sl2);
            }
        } else {
            if flag1 {
                sbool_add_spans_and_render(sl1, sl, ren, &add_span1);
                flag1 = sg1.sweep_scanline(sl1);
            }
            if flag2 {
                sbool_add_spans_and_render(sl2, sl, ren, &add_span2);
                flag2 = sg2.sweep_scanline(sl2);
            }
        }
    }
}

/// C++ `sbool_subtract_shapes`: shape 1's scanlines, with shape 2 taken out where it has a
/// scanline of the same Y.
#[allow(clippy::too_many_arguments)]
pub fn sbool_subtract_shapes<G1, G2, S1, S2, SL, R, A1, C>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
    add_span1: A1,
    combine_spans: C,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
    A1: AddSpan<S1, SL>,
    C: CombineSpans<S1, S2, SL>,
{
    // Nothing to subtract from: return.
    if !sg1.rewind_scanlines() {
        return;
    }
    let mut flag2 = sg2.rewind_scanlines();

    sl.reset(sg1.min_x(), sg1.max_x());
    sl1.reset(sg1.min_x(), sg1.max_x());
    // C++ resets sl2 from sg2's bounds even when sg2 is empty, whose bounds are the
    // "no cells" sentinels (min_x = i32::MAX, max_x = i32::MIN): `max_x - min_x` overflows
    // (a panic in Rust). sl2 is never swept when sg2 is empty, so skipping the reset there
    // changes nothing else.
    if flag2 {
        sl2.reset(sg2.min_x(), sg2.max_x());
    }
    if !sg1.sweep_scanline(sl1) {
        return;
    }
    if flag2 {
        flag2 = sg2.sweep_scanline(sl2);
    }

    ren.prepare();

    // Shape 2's own spans are dropped.
    let add_span2 = SboolAddSpanEmpty;

    loop {
        // Bring shape 2 up to shape 1's Y.
        while flag2 && sl2.y() < sl1.y() {
            flag2 = sg2.sweep_scanline(sl2);
        }
        if flag2 && sl2.y() == sl1.y() {
            sbool_unite_scanlines(sl1, sl2, sl, &add_span1, &add_span2, &combine_spans);
            if sl.num_spans() != 0 {
                sl.finalize(sl1.y());
                ren.render(sl);
            }
        } else {
            sbool_add_spans_and_render(sl1, sl, ren, &add_span1);
        }
        if !sg1.sweep_scanline(sl1) {
            break;
        }
    }
}

/// C++ `sbool_intersect_shapes_aa`.
pub fn sbool_intersect_shapes_aa<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineCovers,
    S2: ScanlineCovers,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_intersect_shapes(sg1, sg2, sl1, sl2, sl, ren, SboolIntersectSpansAa);
}

/// C++ `sbool_intersect_shapes_bin`.
pub fn sbool_intersect_shapes_bin<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_intersect_shapes(sg1, sg2, sl1, sl2, sl, ren, SboolCombineSpansBin);
}

/// C++ `sbool_unite_shapes_aa`.
pub fn sbool_unite_shapes_aa<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineCovers,
    S2: ScanlineCovers,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_unite_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanAa,
        SboolAddSpanAa,
        SboolUniteSpansAa,
    );
}

/// C++ `sbool_unite_shapes_bin`.
pub fn sbool_unite_shapes_bin<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_unite_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanBin,
        SboolAddSpanBin,
        SboolCombineSpansBin,
    );
}

/// The AA XORs differ only in the formula (C++ `sbool_xor_shapes*_aa`).
fn xor_shapes_aa<F, G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    F: SboolXorFormula,
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineCovers,
    S2: ScanlineCovers,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    let combine = SboolXorSpansAa::<F>::new();
    sbool_unite_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanAa,
        SboolAddSpanAa,
        combine,
    );
}

macro_rules! xor_shapes_aa_fn {
    ($(#[$doc:meta])* $name:ident, $formula:ty) => {
        $(#[$doc])*
        pub fn $name<G1, G2, S1, S2, SL, R>(
            sg1: &mut G1,
            sg2: &mut G2,
            sl1: &mut S1,
            sl2: &mut S2,
            sl: &mut SL,
            ren: &mut R,
        ) where
            G1: ScanlineGenerator,
            G2: ScanlineGenerator,
            S1: ScanlineCovers,
            S2: ScanlineCovers,
            SL: ScanlineSpans,
            R: ScanlineSink<SL>,
        {
            xor_shapes_aa::<$formula, _, _, _, _, _, _>(sg1, sg2, sl1, sl2, sl, ren);
        }
    };
}

xor_shapes_aa_fn!(
    /// C++ `sbool_xor_shapes_aa` (the linear formula).
    sbool_xor_shapes_aa,
    SboolXorFormulaLinear
);
xor_shapes_aa_fn!(
    /// C++ `sbool_xor_shapes_saddle_aa`.
    sbool_xor_shapes_saddle_aa,
    SboolXorFormulaSaddle
);
xor_shapes_aa_fn!(
    /// C++ `sbool_xor_shapes_abs_diff_aa`.
    sbool_xor_shapes_abs_diff_aa,
    SboolXorFormulaAbsDiff
);

/// C++ `sbool_xor_shapes_bin`: the overlap is dropped.
pub fn sbool_xor_shapes_bin<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_unite_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanBin,
        SboolAddSpanBin,
        SboolCombineSpansEmpty,
    );
}

/// C++ `sbool_subtract_shapes_aa`.
pub fn sbool_subtract_shapes_aa<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineCovers,
    S2: ScanlineCovers,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_subtract_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanAa,
        SboolSubtractSpansAa,
    );
}

/// C++ `sbool_subtract_shapes_bin`.
pub fn sbool_subtract_shapes_bin<G1, G2, S1, S2, SL, R>(
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    sbool_subtract_shapes(
        sg1,
        sg2,
        sl1,
        sl2,
        sl,
        ren,
        SboolAddSpanBin,
        SboolCombineSpansEmpty,
    );
}

/// C++ `sbool_combine_shapes_bin`: `op` on two binary shapes (the three XORs are one here).
pub fn sbool_combine_shapes_bin<G1, G2, S1, S2, SL, R>(
    op: SBoolOp,
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    match op {
        SBoolOp::Or => sbool_unite_shapes_bin(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::And => sbool_intersect_shapes_bin(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::Xor | SBoolOp::XorSaddle | SBoolOp::XorAbsDiff => {
            sbool_xor_shapes_bin(sg1, sg2, sl1, sl2, sl, ren)
        }
        SBoolOp::AMinusB => sbool_subtract_shapes_bin(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::BMinusA => sbool_subtract_shapes_bin(sg2, sg1, sl2, sl1, sl, ren),
    }
}

/// C++ `sbool_combine_shapes_aa`: `op` on two anti-aliased shapes.
pub fn sbool_combine_shapes_aa<G1, G2, S1, S2, SL, R>(
    op: SBoolOp,
    sg1: &mut G1,
    sg2: &mut G2,
    sl1: &mut S1,
    sl2: &mut S2,
    sl: &mut SL,
    ren: &mut R,
) where
    G1: ScanlineGenerator,
    G2: ScanlineGenerator,
    S1: ScanlineCovers,
    S2: ScanlineCovers,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    match op {
        SBoolOp::Or => sbool_unite_shapes_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::And => sbool_intersect_shapes_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::Xor => sbool_xor_shapes_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::XorSaddle => sbool_xor_shapes_saddle_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::XorAbsDiff => sbool_xor_shapes_abs_diff_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::AMinusB => sbool_subtract_shapes_aa(sg1, sg2, sl1, sl2, sl, ren),
        SBoolOp::BMinusA => sbool_subtract_shapes_aa(sg2, sg1, sl2, sl1, sl, ren),
    }
}

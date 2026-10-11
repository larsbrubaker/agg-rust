//! Scanline-level walkers of `agg_scanline_boolean_algebra.h`: combine two scanlines of the
//! same Y span by span (`sbool_intersect_scanlines`, `sbool_unite_scanlines`), or pass one
//! scanline's spans through an add functor (`sbool_add_spans_and_render`). The span
//! functors come from `spans.rs`; `shapes.rs` drives these per Y.

use super::spans::{AddSpan, CombineSpans};
use super::{ScanlineSink, ScanlineSpans};

/// C++ `sbool_add_spans_and_render`: every span of `sl1` through `add_span` into `sl`,
/// then `sl` to the renderer.
pub fn sbool_add_spans_and_render<S1, SL, R, A>(sl1: &S1, sl: &mut SL, ren: &mut R, add_span: &A)
where
    S1: ScanlineSpans,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
    A: AddSpan<S1, SL>,
{
    sl.reset_spans();
    for i in 0..sl1.num_spans() as usize {
        add_span.add(sl1, i, sl1.span_x(i), sl1.span_len(i).unsigned_abs(), sl);
    }
    sl.finalize(sl1.y());
    ren.render(sl);
}

/// C++ `sbool_intersect_scanlines`: the overlaps of `sl1`'s and `sl2`'s spans through
/// `combine_spans` into `sl` (not finalized).
pub fn sbool_intersect_scanlines<S1, S2, SL, C>(sl1: &S1, sl2: &S2, sl: &mut SL, combine_spans: &C)
where
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    C: CombineSpans<S1, S2, SL>,
{
    sl.reset_spans();
    let mut num1 = sl1.num_spans();
    if num1 == 0 {
        return;
    }
    let mut num2 = sl2.num_spans();
    if num2 == 0 {
        return;
    }
    let mut span1 = 0usize;
    let mut span2 = 0usize;
    while num1 != 0 && num2 != 0 {
        let mut xb1 = sl1.span_x(span1);
        let xb2 = sl2.span_x(span2);
        let mut xe1 = xb1 + sl1.span_len(span1).abs() - 1;
        let xe2 = xb2 + sl2.span_len(span2).abs() - 1;

        // The span ending first advances; advance_both is the shortcut when both end
        // together.
        let advance_span1 = xe1 < xe2;
        let advance_both = xe1 == xe2;

        if xb1 < xb2 {
            xb1 = xb2;
        }
        if xe1 > xe2 {
            xe1 = xe2;
        }
        if xb1 <= xe1 {
            combine_spans.combine(sl1, span1, sl2, span2, xb1, (xe1 - xb1 + 1) as u32, sl);
        }

        if advance_both {
            num1 -= 1;
            num2 -= 1;
            if num1 != 0 {
                span1 += 1;
            }
            if num2 != 0 {
                span2 += 1;
            }
        } else if advance_span1 {
            num1 -= 1;
            if num1 != 0 {
                span1 += 1;
            }
        } else {
            num2 -= 1;
            if num2 != 0 {
                span2 += 1;
            }
        }
    }
}

/// C++'s `invalid_b` / `invalid_e`: a span position that is empty (`xb > xe`).
const INVALID_B: i32 = 0xFFF_FFFF;
const INVALID_E: i32 = INVALID_B - 1;

/// C++ `sbool_unite_scanlines`: spans only in `sl1` through `add_span1`, only in `sl2`
/// through `add_span2`, overlaps through `combine_spans`, into `sl` (not finalized).
pub fn sbool_unite_scanlines<S1, S2, SL, A1, A2, C>(
    sl1: &S1,
    sl2: &S2,
    sl: &mut SL,
    add_span1: &A1,
    add_span2: &A2,
    combine_spans: &C,
) where
    S1: ScanlineSpans,
    S2: ScanlineSpans,
    SL: ScanlineSpans,
    A1: AddSpan<S1, SL>,
    A2: AddSpan<S2, SL>,
    C: CombineSpans<S1, S2, SL>,
{
    sl.reset_spans();
    let mut num1 = sl1.num_spans();
    let mut num2 = sl2.num_spans();
    let mut span1 = 0usize;
    let mut span2 = 0usize;

    let mut xb1 = INVALID_B;
    let mut xb2 = INVALID_B;
    let mut xe1 = INVALID_E;
    let mut xe2 = INVALID_E;

    // Initialize the spans as invalid
    if num1 != 0 {
        xb1 = sl1.span_x(0);
        xe1 = xb1 + sl1.span_len(0).abs() - 1;
        num1 -= 1;
    }
    if num2 != 0 {
        xb2 = sl2.span_x(0);
        xe2 = xb2 + sl2.span_len(0).abs() - 1;
        num2 -= 1;
    }

    loop {
        // Retrieve a new span1 if it's invalid
        if num1 != 0 && xb1 > xe1 {
            num1 -= 1;
            span1 += 1;
            xb1 = sl1.span_x(span1);
            xe1 = xb1 + sl1.span_len(span1).abs() - 1;
        }
        // Retrieve a new span2 if it's invalid
        if num2 != 0 && xb2 > xe2 {
            num2 -= 1;
            span2 += 1;
            xb2 = sl2.span_x(span2);
            xe2 = xb2 + sl2.span_len(span2).abs() - 1;
        }

        if xb1 > xe1 && xb2 > xe2 {
            break;
        }

        // Calculate the intersection
        let mut xb = xb1;
        let mut xe = xe1;
        if xb < xb2 {
            xb = xb2;
        }
        if xe > xe2 {
            xe = xe2;
        }
        let len = xe - xb + 1; // The length of the intersection
        if len > 0 {
            // The spans intersect: add the beginning of a span first
            if xb1 < xb2 {
                add_span1.add(sl1, span1, xb1, (xb2 - xb1) as u32, sl);
                xb1 = xb2;
            } else if xb2 < xb1 {
                add_span2.add(sl2, span2, xb2, (xb1 - xb2) as u32, sl);
                xb2 = xb1;
            }

            // Add the combination part of the spans
            combine_spans.combine(sl1, span1, sl2, span2, xb, len as u32, sl);

            // Invalidate the fully processed span or both
            if xe1 < xe2 {
                xb1 = INVALID_B;
                xe1 = INVALID_E;
                xb2 += len;
            } else if xe2 < xe1 {
                xb2 = INVALID_B;
                xe2 = INVALID_E;
                xb1 += len;
            } else {
                xb1 = INVALID_B;
                xb2 = INVALID_B;
                xe1 = INVALID_E;
                xe2 = INVALID_E;
            }
        } else if xb1 < xb2 {
            // The spans don't intersect: add the earlier one
            if xb1 <= xe1 {
                add_span1.add(sl1, span1, xb1, (xe1 - xb1 + 1) as u32, sl);
            }
            xb1 = INVALID_B;
            xe1 = INVALID_E;
        } else {
            if xb2 <= xe2 {
                add_span2.add(sl2, span2, xb2, (xe2 - xb2 + 1) as u32, sl);
            }
            xb2 = INVALID_B;
            xe2 = INVALID_E;
        }
    }
}

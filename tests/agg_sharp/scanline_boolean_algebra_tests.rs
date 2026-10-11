//! Port of agg-sharp `ScanlineBooleanAlgebraTests.cs`: C++ `agg_scanline_boolean_algebra.h`'s
//! operations on one-pixel-high rectangles, whose covers are easy to know, plus
//! `scanline_p8`'s span merging and `add_cells`, and storages combined as generators.

use agg_rust::rasterizer_scanline_aa::{RasterizerScanlineAa, Scanline};
use agg_rust::rounded_rect::RoundedRect;
use agg_rust::scanline_bin::ScanlineBin;
use agg_rust::scanline_boolean_algebra::{
    shapes, SBoolOp, SboolXorFormulaAbsDiff, SboolXorFormulaLinear, SboolXorFormulaSaddle,
    ScanlineGenerator, ScanlineSink, ScanlineSpans,
};
use agg_rust::scanline_p::ScanlineP8;
use agg_rust::scanline_storage_aa::ScanlineStorageAa;
use agg_rust::scanline_storage_bin::ScanlineStorageBin;
use agg_rust::scanline_u::ScanlineU8;

/// Covers by x in the order they were first set, as C#'s `Dictionary<int, int>` enumerates.
type Covers = Vec<(i32, i32)>;

fn set_cover(covers: &mut Covers, x: i32, cover: i32) {
    match covers.iter_mut().find(|(k, _)| *k == x) {
        Some(entry) => entry.1 = cover,
        None => covers.push((x, cover)),
    }
}

fn cover_at(covers: &Covers, x: i32) -> i32 {
    covers
        .iter()
        .find(|(k, _)| *k == x)
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("no cover at x = {x}"))
}

#[test]
fn operations_on_full_rectangles_keep_the_right_pixels() {
    // A covers 0..9, B covers 5..14, every pixel full.
    assert_eq!(combine(SBoolOp::Or, 0.0, 10.0, 5.0, 15.0), full(&[0, 15]));
    assert_eq!(combine(SBoolOp::And, 0.0, 10.0, 5.0, 15.0), full(&[5, 10]));
    assert_eq!(
        combine(SBoolOp::Xor, 0.0, 10.0, 5.0, 15.0),
        full(&[0, 5, 10, 15])
    );
    assert_eq!(
        combine(SBoolOp::XorSaddle, 0.0, 10.0, 5.0, 15.0),
        full(&[0, 5, 10, 15])
    );
    assert_eq!(
        combine(SBoolOp::XorAbsDiff, 0.0, 10.0, 5.0, 15.0),
        full(&[0, 5, 10, 15])
    );
    assert_eq!(
        combine(SBoolOp::AMinusB, 0.0, 10.0, 5.0, 15.0),
        full(&[0, 5])
    );
    assert_eq!(
        combine(SBoolOp::BMinusA, 0.0, 10.0, 5.0, 15.0),
        full(&[10, 15])
    );
}

#[test]
fn intersecting_with_a_full_solid_span_keeps_partial_covers_as_they_are() {
    // A's first pixel is half covered; B's solid span over it is full, so C++ adds A's covers
    // untouched (add_cells) rather than scaling them by 255 / 256.
    let a = combine(SBoolOp::Or, 0.5, 10.0, 100.0, 101.0);
    let and = combine(SBoolOp::And, 0.5, 10.0, -5.0, 20.0);
    assert_eq!(cover_at(&a, 0), 128);
    assert_eq!(cover_at(&and, 0), cover_at(&a, 0));
}

#[test]
fn xor_formulas_match_cpp() {
    assert_eq!(SboolXorFormulaLinear::calculate(200, 100), 210);
    assert_eq!(SboolXorFormulaSaddle::calculate(255, 255), 0);
    assert_eq!(SboolXorFormulaSaddle::calculate(128, 0), 130);
    assert_eq!(SboolXorFormulaAbsDiff::calculate(100, 200), 100);
}

#[test]
fn adjacent_solid_spans_of_one_cover_merge() {
    // C++ scanline_p8::add_span extends the last span when it is solid, adjacent and of the
    // same cover.
    let mut scanline = ScanlineP8::new();
    scanline.reset(0, 100);
    scanline.add_span(10, 5, 200);
    scanline.add_span(15, 5, 200);
    scanline.add_span(20, 5, 100);
    assert_eq!(scanline.num_spans(), 2);
    assert_eq!(scanline.begin()[0].len, -10);
}

#[test]
fn add_cells_reads_covers_from_its_index() {
    let mut scanline = ScanlineP8::new();
    scanline.reset(0, 100);
    // C#'s `add_cells(10, 2, covers, 2)`: the covers from index 2 on.
    let covers = [1u8, 2, 30, 40];
    scanline.add_cells(10, 2, &covers[2..]);
    let span = scanline.begin()[0];
    assert_eq!(scanline.covers()[span.cover_offset], 30u8);
    assert_eq!(scanline.covers()[span.cover_offset + 1], 40u8);
}

#[test]
fn storages_combine_as_generators() {
    // C++ scanline_boolean2 stores each shape, then combines the stores into a third store.
    let mut a = ScanlineStorageBin::new();
    let mut b = ScanlineStorageBin::new();
    store(&mut rectangle(0.0, 10.0), &mut ScanlineBin::new(), &mut a);
    store(&mut rectangle(5.0, 15.0), &mut ScanlineBin::new(), &mut b);
    let mut result = ScanlineStorageBin::new();
    shapes::sbool_combine_shapes_bin(
        SBoolOp::And,
        &mut a,
        &mut b,
        &mut ScanlineBin::new(),
        &mut ScanlineBin::new(),
        &mut ScanlineBin::new(),
        &mut result,
    );

    let mut aa = ScanlineStorageAa::new();
    store(&mut rectangle(0.5, 10.0), &mut ScanlineU8::new(), &mut aa);
    let mut sink = RecordingSink::default();
    store(&mut aa, &mut ScanlineP8::new(), &mut sink);

    assert!(result.rewind_scanlines());
    assert_eq!(
        (
            result.min_x(),
            result.max_x(),
            result.min_y(),
            result.max_y()
        ),
        (5, 9, 0, 0)
    );
    assert_eq!(cover_at(&sink.covers, 0), 128);
    assert_eq!(cover_at(&sink.covers, 9), 255);
}

fn store<G, SL, R>(generator: &mut G, scanline: &mut SL, sink: &mut R)
where
    G: ScanlineGenerator,
    SL: ScanlineSpans,
    R: ScanlineSink<SL>,
{
    if generator.rewind_scanlines() {
        ScanlineSpans::reset(scanline, generator.min_x(), generator.max_x());
        sink.prepare();
        while generator.sweep_scanline(scanline) {
            sink.render(scanline);
        }
    }
}

fn full(ranges: &[i32]) -> Covers {
    let mut covers = Covers::new();
    for range in ranges.chunks(2) {
        for x in range[0]..range[1] {
            set_cover(&mut covers, x, 255);
        }
    }
    covers
}

fn combine(op: SBoolOp, a1: f64, a2: f64, b1: f64, b2: f64) -> Covers {
    let mut sink = RecordingSink::default();
    shapes::sbool_combine_shapes_aa(
        op,
        &mut rectangle(a1, a2),
        &mut rectangle(b1, b2),
        &mut ScanlineP8::new(),
        &mut ScanlineP8::new(),
        &mut ScanlineP8::new(),
        &mut sink,
    );
    sink.covers
}

fn rectangle(x1: f64, x2: f64) -> RasterizerScanlineAa {
    let mut rasterizer = RasterizerScanlineAa::new();
    rasterizer.add_path(&mut RoundedRect::new(x1, 0.0, x2, 1.0, 0.0), 0);
    rasterizer
}

/// Records every rendered pixel's cover, the way C#'s `RecordingSink` reads a packed scanline.
#[derive(Default)]
struct RecordingSink {
    covers: Covers,
}

impl ScanlineSink<ScanlineP8> for RecordingSink {
    fn prepare(&mut self) {}

    fn render(&mut self, scanline: &ScanlineP8) {
        for span in scanline.begin() {
            for x in 0..span.len.abs() {
                let index = span.cover_offset + if span.len < 0 { 0 } else { x as usize };
                set_cover(
                    &mut self.covers,
                    span.x + x,
                    scanline.covers()[index] as i32,
                );
            }
        }
    }
}

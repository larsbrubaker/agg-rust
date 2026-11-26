#![forbid(unsafe_code)]
//! Row bounds of the pixel formats and `ChannelView` (GitHub issue #3).
//!
//! Every pixel access goes through `RowAccessor`'s checked row access: a row outside
//! `[0, height)`, an unattached accessor, or a stride too short for the format's
//! `width * bytes-per-pixel` row panics (an ordinary unwinding panic) with the row check's own
//! message, before any byte of the buffer is written. C++ AGG does no such check (its pixfmts
//! trust `renderer_base` clipping); `RowAccessor::row_bytes` documents that deliberate
//! divergence. In-range reads and writes land on the bytes of the right row for both stride
//! signs.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::LazyLock;

use agg_rust::blur::ChannelView;
use agg_rust::color::{Gray8, Rgba8};
use agg_rust::comp_op::PixfmtRgba32CompOp;
use agg_rust::pixfmt_gray::PixfmtGray8;
use agg_rust::pixfmt_lcd::{LcdDistributionLut, PixfmtRgba32Lcd};
use agg_rust::pixfmt_rgb::PixfmtRgb24;
use agg_rust::pixfmt_rgb_blenders::PixfmtRgb24Pre;
use agg_rust::pixfmt_rgba::{PixelFormat, PixfmtRgba32, PixfmtRgba32Pre};
use agg_rust::rendering_buffer::RowAccessor;

const FILL: u8 = 0x11;
const W: u32 = 4;
const H: u32 = 2;

/// `region` bytes, every one `FILL`, that one test case attaches an accessor to.
struct Buffer {
    mem: Vec<u8>,
}

impl Buffer {
    fn new(region: usize) -> Self {
        Self {
            mem: vec![FILL; region],
        }
    }

    /// An accessor on the whole buffer, which must be exactly `stride.abs() * height` bytes.
    fn attach(&mut self, width: u32, height: u32, stride: i32) -> RowAccessor<'_> {
        assert_eq!(
            stride.unsigned_abs() as usize * height as usize,
            self.mem.len()
        );
        RowAccessor::new_with_buf(&mut self.mem, width, height, stride)
    }

    /// Whether every byte still holds `FILL`.
    fn untouched(&self) -> bool {
        self.mem.iter().all(|&b| b == FILL)
    }

    fn region(&self) -> &[u8] {
        &self.mem
    }
}

/// The panic `RowAccessor`'s row check raises, so a test proves that check fired rather than
/// some later one (a slice index, say).
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// `y` outside `[0, height)`: `Row(y, height)`.
    Row(i32, u32),
    /// A row of `len` bytes longer than the stride: `Stride(len, stride)`.
    Stride(usize, i32),
}

impl Expect {
    fn message(self) -> String {
        match self {
            Expect::Row(y, height) => format!("row {y} out of bounds (height={height})"),
            Expect::Stride(len, stride) => {
                format!("row of {len} bytes does not fit in stride {stride}")
            }
        }
    }
}

/// Ok when `f` panics with exactly `expect`'s message; otherwise what happened instead.
fn panics_with(expect: Expect, f: impl FnOnce()) -> Result<(), String> {
    let payload = match catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => return Err("did not panic".into()),
        Err(payload) => payload,
    };
    let got = if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".into()
    };
    if got == expect.message() {
        Ok(())
    } else {
        Err(format!(
            "panicked with {got:?}, expected {:?}",
            expect.message()
        ))
    }
}

/// `panics_with`'s result, and then every byte of `buf` must still hold `FILL`: the check
/// fired before the operation wrote anything.
fn panics_without_writing_to(buf: &Buffer, result: Result<(), String>) -> Result<(), String> {
    result?;
    if buf.untouched() {
        Ok(())
    } else {
        Err("panicked, but after writing to the buffer".into())
    }
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Pixel,
    CopyPixel,
    CopyHline,
    BlendHline,
}

const ALL_OPS: [Op; 4] = [Op::Pixel, Op::CopyPixel, Op::CopyHline, Op::BlendHline];

fn apply<P: PixelFormat>(pf: &mut P, op: Op, x: i32, y: i32, c: &P::ColorType) {
    match op {
        Op::Pixel => {
            let _ = pf.pixel(x, y);
        }
        Op::CopyPixel => pf.copy_pixel(x, y, c),
        Op::CopyHline => pf.copy_hline(x, y, 1, c),
        Op::BlendHline => pf.blend_hline(x, y, 1, c, 128),
    }
}

static LCD_LUT: LazyLock<LcdDistributionLut> =
    LazyLock::new(|| LcdDistributionLut::new(3.0 / 9.0, 2.0 / 9.0, 1.0 / 9.0));

/// The bounds cases for one pixel format.
///
/// - `bpp`: bytes per pixel in the buffer.
/// - `sub`: format x-units per buffer pixel (3 for the LCD format's subpixels, else 1).
/// - `color` / `bytes`: an opaque color and the bytes `copy_pixel` stores for it.
/// - `unattached`: the ops that reach the row on an unattached (width 0) accessor; the LCD
///   format's single-pixel ops return early for `x` past its width without touching a row.
macro_rules! row_bounds_tests {
    (
        $m:ident,bpp: $bpp:expr,sub: $sub:expr,color: $color:expr,bytes: $bytes:expr,
        unattached: $unattached:expr,| $ra:ident | $make:expr
    ) => {
        mod $m {
            use super::*;

            const BPP: usize = $bpp;
            const ROW: usize = W as usize * BPP;
            const X_LAST: i32 = (W * $sub) as i32 - 1;

            fn run(ra: &mut RowAccessor<'_>, op: Op, x: i32, y: i32) {
                let $ra = ra;
                let mut pf = $make;
                apply(&mut pf, op, x, y, &$color);
            }

            fn reads_back(ra: &mut RowAccessor<'_>, x: i32, y: i32) -> bool {
                let $ra = ra;
                let pf = $make;
                pf.pixel(x, y) == $color
            }

            /// Runs `op` on a fresh `W` x `H` buffer with `stride`; Ok when it panicked with
            /// `expect` and left every byte of the buffer alone.
            fn panics_without_writing(
                expect: Expect,
                stride: i32,
                op: Op,
                x: i32,
                y: i32,
            ) -> Result<(), String> {
                let mut g = Buffer::new(stride.unsigned_abs() as usize * H as usize);
                let mut ra = g.attach(W, H, stride);
                let result = panics_with(expect, || run(&mut ra, op, x, y));
                panics_without_writing_to(&g, result)
            }

            #[test]
            fn unattached_panics() {
                for op in $unattached {
                    let mut ra = RowAccessor::new();
                    panics_with(Expect::Row(0, 0), || run(&mut ra, op, 0, 0))
                        .unwrap_or_else(|e| panic!("{op:?}: {e}"));
                }
            }

            #[test]
            fn row_past_the_end_panics_without_writing() {
                for op in ALL_OPS {
                    for y in [H as i32, H as i32 + 3] {
                        panics_without_writing(Expect::Row(y, H), ROW as i32, op, 0, y)
                            .unwrap_or_else(|e| panic!("{op:?} y={y}: {e}"));
                    }
                }
            }

            #[test]
            fn negative_row_panics_without_writing() {
                for op in ALL_OPS {
                    panics_without_writing(Expect::Row(-1, H), ROW as i32, op, 0, -1)
                        .unwrap_or_else(|e| panic!("{op:?}: {e}"));
                }
            }

            #[test]
            fn stride_shorter_than_row_panics_without_writing() {
                let stride = (ROW / 2) as i32;
                for op in ALL_OPS {
                    for y in [0, H as i32 - 1] {
                        let expect = Expect::Stride(ROW, stride);
                        panics_without_writing(expect, stride, op, X_LAST, y)
                            .unwrap_or_else(|e| panic!("{op:?} y={y}: {e}"));
                    }
                }
            }

            #[test]
            fn in_range_rows_for_both_stride_signs() {
                let bytes: &[u8] = &$bytes;
                for stride in [ROW as i32, -(ROW as i32)] {
                    for y in 0..H as i32 {
                        for x in [0, X_LAST] {
                            let mut g = Buffer::new(ROW * H as usize);
                            let mut ra = g.attach(W, H, stride);
                            run(&mut ra, Op::CopyPixel, x, y);
                            assert!(reads_back(&mut ra, x, y), "stride={stride} x={x} y={y}");
                            // Row 0 of a bottom-up (negative stride) buffer is the last one
                            // in memory.
                            let mem_row = if stride > 0 { y } else { H as i32 - 1 - y } as usize;
                            let at = mem_row * ROW + (x / $sub) as usize * BPP;
                            let mut expected = vec![FILL; ROW * H as usize];
                            expected[at..at + BPP].copy_from_slice(bytes);
                            assert_eq!(g.region(), &expected[..], "stride={stride} x={x} y={y}");
                        }
                    }
                }
            }
        }
    };
}

fn rgba() -> Rgba8 {
    Rgba8::new(10, 20, 30, 255)
}

row_bounds_tests!(rgba32, bpp: 4, sub: 1, color: rgba(), bytes: [10, 20, 30, 255],
    unattached: ALL_OPS, |ra| PixfmtRgba32::new(ra));
row_bounds_tests!(rgba32_pre, bpp: 4, sub: 1, color: rgba(), bytes: [10, 20, 30, 255],
    unattached: ALL_OPS, |ra| PixfmtRgba32Pre::new(ra));
row_bounds_tests!(rgba32_comp_op, bpp: 4, sub: 1, color: rgba(), bytes: [10, 20, 30, 255],
    unattached: ALL_OPS, |ra| PixfmtRgba32CompOp::new(ra));
row_bounds_tests!(rgba32_lcd, bpp: 4, sub: 3, color: rgba(), bytes: [10, 20, 30, 255],
    unattached: [Op::CopyHline, Op::BlendHline], |ra| PixfmtRgba32Lcd::new(ra, &LCD_LUT));
row_bounds_tests!(rgb24, bpp: 3, sub: 1, color: rgba(), bytes: [10, 20, 30],
    unattached: ALL_OPS, |ra| PixfmtRgb24::new(ra));
row_bounds_tests!(rgb24_pre, bpp: 3, sub: 1, color: rgba(), bytes: [10, 20, 30],
    unattached: ALL_OPS, |ra| PixfmtRgb24Pre::new(ra));
row_bounds_tests!(gray8, bpp: 1, sub: 1, color: Gray8::new(77, 255), bytes: [77],
    unattached: ALL_OPS, |ra| PixfmtGray8::new(ra));

mod channel_view {
    use super::*;

    const ROW: usize = W as usize * 4;

    /// Runs `f` on an RGBA `ChannelView` of `step`-byte pixels over a fresh `W` x `H` buffer
    /// with `stride`; Ok when it panicked with `expect` and left every byte of the buffer alone.
    fn panics_without_writing(
        expect: Expect,
        stride: i32,
        step: usize,
        f: impl FnOnce(&mut ChannelView<'_>),
    ) -> Result<(), String> {
        let mut g = Buffer::new(stride.unsigned_abs() as usize * H as usize);
        let mut ra = g.attach(W, H, stride);
        let result = panics_with(expect, || f(&mut ChannelView::new(&mut ra, step, 0, 1)));
        panics_without_writing_to(&g, result)
    }

    #[test]
    fn unattached_panics() {
        let mut ra = RowAccessor::new();
        panics_with(Expect::Row(0, 0), || {
            ChannelView::rgba32(&mut ra).get(0, 0, 0);
        })
        .unwrap_or_else(|e| panic!("get: {e}"));
        panics_with(Expect::Row(0, 0), || {
            ChannelView::rgba32(&mut ra).set(0, 0, 0, 1)
        })
        .unwrap_or_else(|e| panic!("set: {e}"));
    }

    #[test]
    fn row_past_the_end_panics_without_writing() {
        for y in [H, H + 3] {
            let expect = Expect::Row(y as i32, H);
            panics_without_writing(expect, ROW as i32, 4, |v| {
                v.get(0, y, 0);
            })
            .unwrap_or_else(|e| panic!("get y={y}: {e}"));
            panics_without_writing(expect, ROW as i32, 4, |v| v.set(0, y, 0, 1))
                .unwrap_or_else(|e| panic!("set y={y}: {e}"));
        }
    }

    /// A step wider than the stride allows: `W * step` bytes do not fit in a row.
    #[test]
    fn step_past_the_stride_panics_without_writing() {
        for step in [5, 1000] {
            let expect = Expect::Stride(W as usize * step, ROW as i32);
            panics_without_writing(expect, ROW as i32, step, |v| {
                v.get(0, 0, 0);
            })
            .unwrap_or_else(|e| panic!("get step={step}: {e}"));
            panics_without_writing(expect, ROW as i32, step, |v| {
                v.set(W - 1, H - 1, 0, 1);
            })
            .unwrap_or_else(|e| panic!("set step={step}: {e}"));
        }
    }

    #[test]
    fn stride_shorter_than_row_panics_without_writing() {
        let stride = (ROW / 2) as i32;
        let expect = Expect::Stride(ROW, stride);
        panics_without_writing(expect, stride, 4, |v| {
            v.get(W - 1, H - 1, 0);
        })
        .unwrap_or_else(|e| panic!("get: {e}"));
        panics_without_writing(expect, stride, 4, |v| {
            v.set(W - 1, H - 1, 0, 1);
        })
        .unwrap_or_else(|e| panic!("set: {e}"));
    }

    #[test]
    fn in_range_rows_for_both_stride_signs() {
        for stride in [ROW as i32, -(ROW as i32)] {
            for y in 0..H {
                let mut g = Buffer::new(ROW * H as usize);
                let mut ra = g.attach(W, H, stride);
                let mut view = ChannelView::rgba32(&mut ra);
                view.set(W - 1, y, 2, 99);
                assert_eq!(view.get(W - 1, y, 2), 99);
                let mem_row = if stride > 0 { y } else { H - 1 - y } as usize;
                let mut expected = vec![FILL; ROW * H as usize];
                expected[mem_row * ROW + (W as usize - 1) * 4 + 2] = 99;
                assert_eq!(g.region(), &expected[..], "stride={stride} y={y}");
            }
        }
    }
}

/// An accessor attached to 0 rows with a negative stride rejects every row like an unattached
/// one.
#[test]
fn zero_height_negative_stride_rejects_every_row() {
    let mut empty: Vec<u8> = Vec::new();
    let mut ra = RowAccessor::new_with_buf(&mut empty, W, 0, -(W as i32 * 4));
    let pf = PixfmtRgba32::new(&mut ra);
    panics_with(Expect::Row(0, 0), || {
        let _ = pf.pixel(0, 0);
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

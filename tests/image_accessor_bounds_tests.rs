//! Bounds of the image accessors.
//!
//! `span`, `next_x` and `next_y` are safe fns, so no sequence of calls may read outside the
//! image, and each call must return exactly the accessor's meaning, which these tests compute
//! straight from the raw bytes:
//! - `ImageAccessorClone`: the pixel at (clamp(x), clamp(y)); on a stride too short for a
//!   `width * PIX_WIDTH`-byte row every access panics like every other checked row access;
//! - `ImageAccessorClip`: the pixel at (x, y), or the background colour outside the image; on
//!   a stride too short for the row a read of an image pixel panics the same way;
//! - `ImageAccessorWrap`: the pixel at the wrapped (x, y), however far it steps;
//! - `ImageAccessorNoClip`: the pixel at (x, y), and a panic for a step past the row or a row
//!   outside the image.
//!
//! The span generators step `next_x` past the `len` they pass to `span` (the resample loops
//! do, at the right edge), so the fast paths must hold that meaning however far they are
//! walked, whatever `len` is. C++'s accessors do no such checking; the Rust ones document
//! that deliberate divergence.
//!
//! Natively each image sits in a `Vec` between guard bytes, and padded rows carry padding
//! bytes, so a read past a row or past the image returns bytes no pixel holds instead of
//! reaching memory the test does not own. Under Miri the guards are empty (`GUARD` is 0) and
//! the `Vec` is exactly the attached rows (padding included, as it is part of them): the
//! accessor's pointer then has provenance over the image alone, so a read or a pointer offset
//! past it is an out-of-bounds access Miri reports as undefined behaviour. With guards it
//! would be a legal in-allocation access Miri cannot see, and only the value comparison
//! would stand guard.
//!
//! Under Miri the grids are cut down (`cfg!(miri)`) so the file runs in about a minute; each
//! reduced grid says why it still covers every class of call the full one does. Natively the
//! full grids run.

use std::panic::{catch_unwind, AssertUnwindSafe};

use agg_rust::color::Rgba8;
use agg_rust::image_accessors::{
    ImageAccessorClip, ImageAccessorClone, ImageAccessorNoClip, ImageAccessorWrap, ImageSource,
    WrapModeReflect, WrapModeReflectAutoPow2, WrapModeReflectPow2, WrapModeRepeat,
    WrapModeRepeatAutoPow2, WrapModeRepeatPow2,
};
use agg_rust::image_filters::{
    ImageFilterBessel, ImageFilterBicubic, ImageFilterBilinear, ImageFilterLut,
};
use agg_rust::renderer_scanline::SpanGenerator;
use agg_rust::rendering_buffer::RowAccessor;
use agg_rust::span_image_filter_rgba::SpanImageResampleRgbaAffine;
use agg_rust::span_interpolator_linear::SpanInterpolatorLinear;
use agg_rust::trans_affine::TransAffine;

const SENTINEL: u8 = 0xA5;
const PADDING: u8 = 0x77;
/// Guard bytes on each side of an image: none under Miri (see the module doc).
const GUARD: usize = if cfg!(miri) { 0 } else { 256 };
const W: u32 = 5;
const H: u32 = 3;
const BPP: usize = 4;
const ROW: usize = W as usize * BPP;
/// `ImageAccessorClip`'s background: no pixel's bytes (byte 0 of a pixel has a low nibble of
/// at most 9 here), nor `SENTINEL` or `PADDING`.
const BK: [u8; 4] = [0x0F, 0x1F, 0x2F, 0x3F];

/// A pixel of up to 4 bytes, zero-filled past its width, so results compare without
/// allocating (Miri runs these tests).
type Px = [u8; 4];

fn px(p: &[u8]) -> Px {
    let mut a = [0; 4];
    a[..p.len()].copy_from_slice(p);
    a
}

/// An image of `width` x `height` pixels of `bpp` bytes stored with `stride` (positive:
/// top-down; negative: bottom-up), between two `GUARD`-byte runs of `SENTINEL` (empty under
/// Miri, where `guards_intact` is then trivially true and Miri itself checks bounds).
struct Guarded {
    mem: Vec<u8>,
    bpp: usize,
    width: u32,
    height: u32,
    stride: i32,
}

impl Guarded {
    /// A `W` x `H` RGBA image.
    fn new(stride: i32) -> Self {
        Self::with(BPP, W, H, stride)
    }

    /// Pixel bytes are the first `bpp` of `expected_bytes(x, y)`: unique per pixel (byte 0
    /// alone is), never `SENTINEL`, `PADDING` or `BK`. Rows too long for the stride stay
    /// `PADDING`.
    fn with(bpp: usize, width: u32, height: u32, stride: i32) -> Self {
        let region = stride.unsigned_abs() as usize * height as usize;
        let mut mem = vec![SENTINEL; GUARD * 2 + region];
        mem[GUARD..GUARD + region].fill(PADDING);
        let mut g = Self {
            mem,
            bpp,
            width,
            height,
            stride,
        };
        let full_rows = width as usize * bpp <= stride.unsigned_abs() as usize;
        if full_rows {
            for y in 0..height as i32 {
                for x in 0..width as i32 {
                    let off = g.offset(x, y);
                    g.mem[off..off + bpp].copy_from_slice(&expected_bytes(x, y)[..bpp]);
                }
            }
        }
        g
    }

    /// Byte offset in `mem` of pixel (x, y), both inside the image.
    fn offset(&self, x: i32, y: i32) -> usize {
        let abs = self.stride.unsigned_abs() as usize;
        let row = if self.stride >= 0 {
            y as usize
        } else {
            (self.height as i32 - 1 - y) as usize
        };
        GUARD + row * abs + x as usize * self.bpp
    }

    fn attach(&mut self) -> RowAccessor {
        // SAFETY: `mem[GUARD..]` holds `height * |stride|` bytes, with the lowest address
        // first as `attach` expects for a negative stride, and outlives the accessor; nothing
        // writes `mem` while the accessor is used.
        unsafe {
            RowAccessor::new_with_buf(
                self.mem.as_mut_ptr().add(GUARD),
                self.width,
                self.height,
                self.stride,
            )
        }
    }

    /// Pixel (x, y) from the raw bytes; both must be inside the image.
    fn at(&self, x: i64, y: i64) -> Px {
        let off = self.offset(x as i32, y as i32);
        px(&self.mem[off..off + self.bpp])
    }

    /// The clamped reference: pixel (clamp(x), clamp(y)).
    fn clamped(&self, x: i64, y: i64) -> Px {
        self.at(
            x.clamp(0, self.width as i64 - 1),
            y.clamp(0, self.height as i64 - 1),
        )
    }

    /// The clipped reference: pixel (x, y), or `BK` outside the image.
    fn pixel_or_background(&self, x: i64, y: i64) -> Px {
        if (0..self.width as i64).contains(&x) && (0..self.height as i64).contains(&y) {
            self.at(x, y)
        } else {
            px(&BK[..self.bpp])
        }
    }

    fn guards_intact(&self) -> bool {
        let region = self.mem.len() - GUARD * 2;
        let (before, rest) = self.mem.split_at(GUARD);
        before.iter().chain(&rest[region..]).all(|&b| b == SENTINEL)
    }
}

fn expected_bytes(x: i32, y: i32) -> [u8; 4] {
    [(16 * x + y) as u8, 0x80 + x as u8, 0x90 + y as u8, 0x40]
}

/// The panic message of `f`, or `None` when it returned normally.
fn panic_message(f: impl FnOnce()) -> Option<String> {
    let payload = catch_unwind(AssertUnwindSafe(f)).err()?;
    Some(if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".into()
    })
}

/// How a wrap mode folds a coordinate into `[0, size)`, computed independently of the
/// `WrapMode` implementations (for the small coordinates used here they agree with C++'s
/// modes; the pow2 modes only for power-of-two sizes).
#[derive(Clone, Copy, Debug)]
enum Fold {
    Repeat,
    Reflect,
}

impl Fold {
    fn apply(self, v: i64, size: u32) -> i64 {
        let size = size as i64;
        match self {
            Fold::Repeat => v.rem_euclid(size),
            Fold::Reflect => {
                let m = v.rem_euclid(2 * size);
                if m < size {
                    m
                } else {
                    2 * size - 1 - m
                }
            }
        }
    }
}

/// An accessor under test, with the meaning its pixels must have.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Clone,
    Clip,
    /// `ImageAccessorWrap<WrapModeRepeat, WrapModeReflect>`.
    WrapRepeatReflect,
    /// `ImageAccessorWrap<WrapModeReflect, WrapModeRepeat>`.
    WrapReflectRepeat,
    /// `ImageAccessorWrap<WrapModeRepeatAutoPow2, WrapModeReflectAutoPow2>`.
    WrapAutoPow2,
    /// `ImageAccessorWrap<WrapModeReflectPow2, WrapModeRepeatPow2>`: power-of-two sizes only.
    WrapPow2,
}

impl Kind {
    fn accessor<'a, const N: usize>(self, ra: &'a RowAccessor) -> Box<dyn ImageSource + 'a> {
        match self {
            Kind::Clone => Box::new(ImageAccessorClone::<N>::new(ra)),
            Kind::Clip => Box::new(ImageAccessorClip::<N>::new(ra, &BK)),
            Kind::WrapRepeatReflect => {
                Box::new(ImageAccessorWrap::<N, WrapModeRepeat, WrapModeReflect>::new(ra))
            }
            Kind::WrapReflectRepeat => {
                Box::new(ImageAccessorWrap::<N, WrapModeReflect, WrapModeRepeat>::new(ra))
            }
            Kind::WrapAutoPow2 => Box::new(ImageAccessorWrap::<
                N,
                WrapModeRepeatAutoPow2,
                WrapModeReflectAutoPow2,
            >::new(ra)),
            Kind::WrapPow2 => Box::new(ImageAccessorWrap::<
                N,
                WrapModeReflectPow2,
                WrapModeRepeatPow2,
            >::new(ra)),
        }
    }

    /// The pixel the accessor must return at (x, y).
    fn expected(self, g: &Guarded, x: i64, y: i64) -> Px {
        let wrapped = |fx: Fold, fy: Fold| g.at(fx.apply(x, g.width), fy.apply(y, g.height));
        match self {
            Kind::Clone => g.clamped(x, y),
            Kind::Clip => g.pixel_or_background(x, y),
            Kind::WrapRepeatReflect | Kind::WrapAutoPow2 => wrapped(Fold::Repeat, Fold::Reflect),
            Kind::WrapReflectRepeat | Kind::WrapPow2 => wrapped(Fold::Reflect, Fold::Repeat),
        }
    }
}

/// The accessors that bound a fast path (the cached-pointer scheme), checked call by call.
const BOUNDED: [Kind; 2] = [Kind::Clone, Kind::Clip];

/// A span on the last row, then far more `next_x` steps than `len`: once past the row's last
/// pixel each step returns the accessor's meaning there (the edge pixel for Clone, the
/// background for Clip), never the guard bytes after the image.
#[test]
fn next_x_past_the_span_on_the_last_row_keeps_the_meaning() {
    for kind in BOUNDED {
        let mut g = Guarded::new(ROW as i32);
        let ra = g.attach();
        let y = H as i32 - 1;
        let mut got = Vec::new();
        for x in 0..W as i32 {
            for len in 1..=(W - x as u32) {
                let mut acc = kind.accessor::<4>(&ra);
                got.push(((x, len, 0), px(acc.span(x, y, len))));
                for i in 1..=2 * W as i32 {
                    got.push(((x, len, i), px(acc.next_x())));
                }
            }
        }
        for ((x, len, i), pixel) in got {
            assert_eq!(
                pixel,
                kind.expected(&g, (x + i) as i64, y as i64),
                "{kind:?}: span({x}, {y}, {len}) then {i} next_x"
            );
        }
        assert!(g.guards_intact());
    }
}

/// `span(width, y, 0)` passes C++'s `x + len <= width` test, yet pixel `width` is past the
/// row: it must return the accessor's meaning there, not the bytes after the row.
#[test]
fn span_at_width_with_zero_len_keeps_the_meaning() {
    for kind in BOUNDED {
        let mut g = Guarded::new(ROW as i32);
        let ra = g.attach();
        let mut got = Vec::new();
        for y in 0..H as i32 {
            let mut acc = kind.accessor::<4>(&ra);
            got.push((y, px(acc.span(W as i32, y, 0))));
        }
        for (y, pixel) in got {
            assert_eq!(
                pixel,
                kind.expected(&g, W as i64, y as i64),
                "{kind:?}: span({W}, {y}, 0)"
            );
        }
    }
}

/// A `len` that does not fit in `i32` (`u32::MAX` wraps to -1; `i32::MAX` and `2^31`
/// overflow `x + len`) is just a span longer than the row: the meaning holds, every step.
#[test]
fn huge_len_keeps_the_meaning() {
    for kind in BOUNDED {
        let mut g = Guarded::new(ROW as i32);
        let ra = g.attach();
        let mut got = Vec::new();
        for len in [u32::MAX, i32::MAX as u32, i32::MAX as u32 + 1] {
            for y in 0..H as i32 {
                for x in -1..=W as i32 {
                    let mut acc = kind.accessor::<4>(&ra);
                    let result = catch_unwind(AssertUnwindSafe(|| {
                        let mut pixels = vec![px(acc.span(x, y, len))];
                        for _ in 0..W + 2 {
                            pixels.push(px(acc.next_x()));
                        }
                        pixels
                    }));
                    got.push(((x, y, len), result.map_err(|_| ())));
                }
            }
        }
        for ((x, y, len), pixels) in got {
            let pixels =
                pixels.unwrap_or_else(|()| panic!("{kind:?}: span({x}, {y}, {len}) panicked"));
            for (i, pixel) in pixels.into_iter().enumerate() {
                assert_eq!(
                    pixel,
                    kind.expected(&g, x as i64 + i as i64, y as i64),
                    "{kind:?}: span({x}, {y}, {len}) then {i} next_x"
                );
            }
        }
    }
}

/// `ImageAccessorClone` on a stride shorter than `width * 4` bytes: every access,
/// fast-path coordinates and clamped ones alike, panics with the checked row access's message
/// before reading anything.
#[test]
fn clone_stride_shorter_than_the_row_panics() {
    let short = 2 * BPP as i32;
    for stride in [short, -short] {
        let mut g = Guarded::new(stride);
        let ra = g.attach();
        let expect = format!("row of {ROW} bytes does not fit in stride {stride}");
        for (x, y, len) in [
            (0, 0, 1),
            (3, H as i32 - 1, 2),
            (-1, 0, 1),
            (9, 9, 1),
            (0, -1, 3),
        ] {
            let mut acc = ImageAccessorClone::<4>::new(&ra);
            let msg = panic_message(|| {
                acc.span(x, y, len);
            });
            assert_eq!(
                msg.as_deref(),
                Some(expect.as_str()),
                "stride {stride}: span({x}, {y}, {len})"
            );
        }
        assert!(g.guards_intact());
    }
}

/// `ImageAccessorClip` on a stride shorter than `width * 4` bytes: a pixel outside the image
/// is the background, read from no row; every pixel inside it panics with the checked row
/// access's message before reading anything, whether `span` or a `next_x` stepping into the
/// image reaches it.
#[test]
fn clip_stride_shorter_than_the_row_panics_inside_the_image() {
    let short = 2 * BPP as i32;
    for stride in [short, -short] {
        let mut g = Guarded::new(stride);
        let ra = g.attach();
        let expect = format!("row of {ROW} bytes does not fit in stride {stride}");
        for (x, y, len) in [(0, 0, 1), (3, H as i32 - 1, 2), (W as i32 - 1, 1, 1)] {
            let mut acc = ImageAccessorClip::<4>::new(&ra, &BK);
            let msg = panic_message(|| {
                acc.span(x, y, len);
            });
            assert_eq!(
                msg.as_deref(),
                Some(expect.as_str()),
                "stride {stride}: span({x}, {y}, {len})"
            );
        }
        let mut acc = ImageAccessorClip::<4>::new(&ra, &BK);
        for (x, y, len) in [(-1, 0, 3), (W as i32, 0, 1), (9, 9, 1), (0, -1, 3)] {
            assert_eq!(
                acc.span(x, y, len),
                BK,
                "stride {stride}: span({x}, {y}, {len})"
            );
        }
        acc.span(-1, 1, 3);
        let msg = panic_message(|| {
            acc.next_x();
        });
        assert_eq!(
            msg.as_deref(),
            Some(expect.as_str()),
            "stride {stride}: next_x into the image"
        );
        assert!(g.guards_intact());
    }
}

/// `next_y` past the bottom row keeps the meaning there (the clamped last row for Clone, the
/// background for Clip), from the span start, and so does `next_x` along it.
#[test]
fn next_y_past_the_bottom_row_keeps_the_meaning() {
    for kind in BOUNDED {
        let mut g = Guarded::new(ROW as i32);
        let ra = g.attach();
        let y0 = H as i32 - 2;
        let mut got = Vec::new();
        for x in 0..W as i32 {
            let mut acc = kind.accessor::<4>(&ra);
            got.push(((x, 0, 0), px(acc.span(x, y0, 2))));
            for j in 1..=3 {
                got.push(((x, j, 0), px(acc.next_y())));
                got.push(((x, j, 1), px(acc.next_x())));
            }
        }
        for ((x, j, i), pixel) in got {
            assert_eq!(
                pixel,
                kind.expected(&g, (x + i) as i64, (y0 + j) as i64),
                "{kind:?}: span({x}, {y0}, 2), {j} next_y, {i} next_x"
            );
        }
    }
}

/// The coordinates a cross-check walks: every span start in `xs` x `ys` with every `len`,
/// then `rows` rows (`next_y` between them) of `cols` pixels each (`next_x` between them).
struct Grid {
    xs: Vec<i32>,
    ys: Vec<i32>,
    lens: Vec<u32>,
    rows: i32,
    cols: i32,
}

impl Grid {
    /// The grid for the bounded accessors on a `width` x `height` image.
    ///
    /// Natively: every start from 3 pixels left of the image to 3 past it, every row from 2
    /// above to 2 below, lengths around every edge and past `i32`, and walks from each start
    /// down past the bottom and right past the row.
    ///
    /// Under Miri, one start per class instead: x left of the image (-1), the first pixel
    /// (0), an interior pixel (1), the last pixel (width - 1) and one past it (width); y above
    /// the image (-1), the first row, the last row and one below it; lengths 0 (with x = width:
    /// `span(width, y, 0)`), 1 (with x = width - 1: a fast path that fits exactly at the last
    /// pixel), `width` (fits exactly from x = 0, one too long from x = 1, so the slow path),
    /// and the overflowing `u32::MAX` and `2^31`. Two rows cover `next_y` within the image
    /// (from row 0) and past the bottom (from the last row); `width + 2` pixels per row walk
    /// two steps off the row from x = 0, which falls off the fast path and then steps the
    /// slow path once more.
    ///
    /// On a narrow image some of these coincide (at width 1 the first pixel is the last, and
    /// "interior" is one past it); the duplicates are dropped, so they cost nothing.
    fn bounded(width: u32, height: u32) -> Self {
        let mut grid = Self::bounded_with_duplicates(width, height);
        for v in [&mut grid.xs, &mut grid.ys] {
            v.sort_unstable();
            v.dedup();
        }
        grid.lens.sort_unstable();
        grid.lens.dedup();
        grid
    }

    fn bounded_with_duplicates(width: u32, height: u32) -> Self {
        let (w, h) = (width as i32, height as i32);
        if cfg!(miri) {
            Grid {
                xs: vec![-1, 0, 1, w - 1, w],
                ys: vec![-1, 0, h - 1, h],
                lens: vec![0, 1, width, u32::MAX, 1 << 31],
                rows: 2,
                cols: w + 2,
            }
        } else {
            Grid {
                xs: (-3..w + 3).collect(),
                ys: (-2..h + 2).collect(),
                lens: vec![
                    0,
                    1,
                    2,
                    width - 1,
                    width,
                    width + 1,
                    u32::MAX,
                    i32::MAX as u32,
                    1 << 31,
                ],
                rows: h + 2,
                cols: w + 3,
            }
        }
    }

    /// The grid for the wrapping accessors: starts more than a period (two for reflect) to
    /// each side, and walks of more than two periods each way, so every wrap point is crossed
    /// by `next_x` and `next_y` both. `len` means nothing to them. Under Miri the starts are
    /// one per period (and 0) instead of every one, keeping the full walks, which are what
    /// cross the wrap points.
    fn wrapped(width: u32, height: u32) -> Self {
        let (w, h) = (width as i32, height as i32);
        if cfg!(miri) {
            Grid {
                xs: vec![-2 * w - 1, -1, 0, w - 1, 2 * w],
                ys: vec![-2 * h - 1, 0, h],
                lens: vec![1],
                rows: 2 * h + 2,
                cols: 2 * w + 2,
            }
        } else {
            Grid {
                xs: (-2 * w - 1..2 * w + 2).collect(),
                ys: (-2 * h - 1..2 * h + 2).collect(),
                lens: vec![1, u32::MAX],
                rows: 2 * h + 2,
                cols: 2 * w + 2,
            }
        }
    }
}

/// The round trip: for in-range and out-of-range spans of every kind of length, on the
/// given buffers, walking `next_x` past the span and `next_y` past the image, the accessor
/// returns exactly its meaning computed from the raw bytes.
fn cross_check<const N: usize>(
    kind: Kind,
    width: u32,
    images: &[(i32, u32)],
    grid: fn(u32, u32) -> Grid,
) {
    for &(stride, height) in images {
        let mut g = Guarded::with(N, width, height, stride);
        let ra = g.attach();
        let Grid {
            xs,
            ys,
            lens,
            rows,
            cols,
        } = grid(width, height);
        let mut got = Vec::new();
        for &y in &ys {
            for &x in &xs {
                for &len in &lens {
                    let mut acc = kind.accessor::<N>(&ra);
                    got.push(((x, y, len, 0, 0), px(acc.span(x, y, len))));
                    for j in 0..rows {
                        if j > 0 {
                            got.push(((x, y, len, j, 0), px(acc.next_y())));
                        }
                        for i in 1..cols {
                            got.push(((x, y, len, j, i), px(acc.next_x())));
                        }
                    }
                }
            }
        }
        for ((x, y, len, j, i), pixel) in got {
            assert_eq!(
                pixel,
                kind.expected(&g, x as i64 + i as i64, y as i64 + j as i64),
                "{kind:?}, PIX_WIDTH {N}, stride {stride}, height {height}: span({x}, {y}, \
                 {len}), {j} next_y, {i} next_x"
            );
        }
        assert!(g.guards_intact());
    }
}

/// The RGBA buffers the bounded accessors are cross-checked on. Natively: top-down and
/// bottom-up, exact and padded strides, each at one row and at `H` rows. Under Miri: a
/// top-down exact stride and a bottom-up padded one, at `H` rows (which has a first, an
/// interior and a last row): each stride sign and each of exact and padded once; the
/// one-row image is only the first and last row being the same, which `H` rows already
/// exercise separately.
fn rgba_images() -> Vec<(i32, u32)> {
    let row = ROW as i32;
    if cfg!(miri) {
        vec![(row, H), (-(row + 8), H)]
    } else {
        [row, -row, row + 8, -(row + 8)]
            .into_iter()
            .flat_map(|s| [(s, 1), (s, H)])
            .collect()
    }
}

#[test]
fn bounded_accessors_match_their_reference() {
    for kind in BOUNDED {
        cross_check::<4>(kind, W, &rgba_images(), Grid::bounded);
    }
}

/// The same round trip on an image one pixel wide, whose first pixel is also its last: a
/// fast-path span there ends the row on the first `next_x`. Natively and under Miri the same
/// four strides (each sign, exact and padded) at one row and at `H` rows: the grid is small
/// at this width.
#[test]
fn bounded_accessors_match_their_reference_at_width_1() {
    let row = BPP as i32;
    let images: Vec<(i32, u32)> = [row, -row, row + 8, -(row + 8)]
        .into_iter()
        .flat_map(|s| [(s, 1), (s, H)])
        .collect();
    for kind in BOUNDED {
        cross_check::<4>(kind, 1, &images, Grid::bounded);
    }
}

/// The same round trip for pixel widths other than 4 (odd padding, so rows do not start at
/// a multiple of the pixel width). Under Miri one buffer per width, of opposite stride signs
/// and paddings, as the RGBA ones already cover both of each.
#[test]
fn bounded_accessors_match_their_reference_for_other_pixel_widths() {
    let row3 = W as i32 * 3;
    let row1 = W as i32;
    let (images3, images1) = if cfg!(miri) {
        (vec![(-(row3 + 5), H)], vec![(row1, H)])
    } else {
        (
            vec![(row3, H), (-(row3 + 5), H)],
            vec![(row1, H), (-(row1 + 3), H)],
        )
    };
    for kind in BOUNDED {
        cross_check::<3>(kind, W, &images3, Grid::bounded);
        cross_check::<1>(kind, W, &images1, Grid::bounded);
    }
}

/// `ImageAccessorWrap` stepped any number of times stays on its wrapped meaning, for every
/// wrap mode: the general modes on a 5 x 3 image (neither a power of two), the auto modes on
/// both that and a 4 x 2 one (their two branches), the pow2 modes on the 4 x 2 one.
#[test]
fn wrap_accessor_matches_the_wrapped_reference() {
    let odd = |n: usize| vec![(5 * n as i32 + 3, 3)];
    let pow2 = |n: usize| vec![(-(4 * n as i32), 2)];
    for (kind, width, images) in [
        (Kind::WrapRepeatReflect, 5, odd(4)),
        (Kind::WrapReflectRepeat, 5, odd(4)),
        (Kind::WrapAutoPow2, 5, odd(4)),
        (Kind::WrapAutoPow2, 4, pow2(4)),
        (Kind::WrapPow2, 4, pow2(4)),
    ] {
        cross_check::<4>(kind, width, &images, Grid::wrapped);
    }
    // One other pixel width, so the wrapped offset is scaled by it.
    cross_check::<3>(Kind::WrapRepeatReflect, 5, &odd(3), Grid::wrapped);
}

/// One accessor reused across calls: whatever state the previous call left (fast path,
/// slow path, fallen off the row, or no `span` at all yet), each call returns the meaning at
/// its own coordinates.
#[test]
fn reused_accessor_keeps_the_meaning() {
    #[derive(Clone, Copy, Debug)]
    enum Op {
        Span(i32, i32, u32),
        NextX,
        NextY,
    }
    use Op::*;
    let ops = [
        // `next_x` and `next_y` before any `span`: the accessor starts at (0, 0).
        NextX,
        NextY,
        NextX,
        // Fast-path span, then a slow-path span, then fast again.
        Span(1, 1, 2),
        NextX,
        Span(-2, 1, 1),
        NextX,
        NextX,
        Span(2, 0, 3),
        // Walk off the right of row 0, then a fresh `span` on the fast path.
        NextX,
        NextX,
        NextX,
        NextX,
        Span(0, 2, 1),
        // Walk off the right of the last row, then `next_y` past the bottom.
        NextX,
        NextX,
        NextX,
        NextX,
        NextX,
        NextX,
        NextY,
        NextX,
        NextY,
        // Fast-path span after all that, and `next_y` within the image.
        Span(3, 0, 2),
        NextY,
        NextX,
        NextX,
        NextY,
        // A slow-path span left of the image whose `next_x` steps into it, then `next_y`.
        Span(-1, 1, 9),
        NextX,
        NextX,
        NextY,
        NextX,
    ];
    for kind in BOUNDED {
        let mut g = Guarded::new(ROW as i32);
        let ra = g.attach();
        let mut acc = kind.accessor::<4>(&ra);
        let (mut x0, mut x, mut y) = (0i64, 0i64, 0i64);
        let mut got = Vec::new();
        for (k, op) in ops.into_iter().enumerate() {
            let pixel = match op {
                Span(sx, sy, len) => {
                    (x0, x, y) = (sx as i64, sx as i64, sy as i64);
                    px(acc.span(sx, sy, len))
                }
                NextX => {
                    x += 1;
                    px(acc.next_x())
                }
                NextY => {
                    (x, y) = (x0, y + 1);
                    px(acc.next_y())
                }
            };
            got.push((k, op, x, y, pixel));
        }
        for (k, op, x, y, pixel) in got {
            assert_eq!(
                pixel,
                kind.expected(&g, x, y),
                "{kind:?}: op {k} ({op:?}) at ({x}, {y})"
            );
        }
    }
}

/// One call on an accessor, for the empty-image cases.
type AccessorCall = fn(&mut dyn ImageSource);

const EMPTY_IMAGE_CALLS: [(&str, AccessorCall); 4] = [
    ("span(0, 0, 1)", |a| {
        a.span(0, 0, 1);
    }),
    ("span(-1, 5, 0)", |a| {
        a.span(-1, 5, 0);
    }),
    ("next_x", |a| {
        a.next_x();
    }),
    ("next_y", |a| {
        a.next_y();
    }),
];

/// For `ImageAccessorClone` an empty image has no pixel to clamp to: every access panics,
/// with the message the struct's `# Panics` documents.
#[test]
fn clone_empty_image_panics() {
    for (width, height, stride, expect) in [
        (
            0,
            H,
            4,
            "range end index 4 out of range for slice of length 0",
        ),
        (W, 0, ROW as i32, "row 0 out of bounds (height=0)"),
        (0, 0, 4, "row 0 out of bounds (height=0)"),
    ] {
        let mut g = Guarded::with(BPP, width, height, stride);
        let ra = g.attach();
        for (name, call) in EMPTY_IMAGE_CALLS {
            let mut acc = ImageAccessorClone::<4>::new(&ra);
            assert_eq!(
                panic_message(|| call(&mut acc)).as_deref(),
                Some(expect),
                "{width}x{height}: {name}"
            );
        }
        assert!(g.guards_intact());
    }
}

/// For `ImageAccessorClip` every coordinate of an empty image is outside it: every access
/// returns the background.
#[test]
fn clip_empty_image_is_all_background() {
    for (width, height, stride) in [(0, H, 4), (W, 0, ROW as i32), (0, 0, 4)] {
        let mut g = Guarded::with(BPP, width, height, stride);
        let ra = g.attach();
        let mut acc = ImageAccessorClip::<4>::new(&ra, &BK);
        assert_eq!(acc.span(0, 0, 1), BK, "{width}x{height}");
        assert_eq!(acc.next_x(), BK, "{width}x{height}");
        assert_eq!(acc.next_y(), BK, "{width}x{height}");
        assert_eq!(acc.span(-1, 5, 0), BK, "{width}x{height}");
        for (name, call) in EMPTY_IMAGE_CALLS {
            assert_eq!(
                panic_message(|| call(&mut acc)),
                None,
                "{width}x{height}: {name}"
            );
        }
        assert!(g.guards_intact());
    }
}

/// `ImageAccessorNoClip` is checked, not unchecked: a step past the row on an exact-stride
/// image, a row outside the image (by `span` or by `next_y` past the bottom) and an `x` outside
/// the row all panic, never reading outside the image. (Within a padded stride a step
/// past the row reads the padding; the accessor promises nothing there.)
#[test]
fn no_clip_panics_outside_the_image() {
    let mut g = Guarded::new(ROW as i32);
    let ra = g.attach();
    let past_row = format!(
        "range end index {} out of range for slice of length {ROW}",
        ROW + BPP
    );
    let below = format!("row {H} out of bounds (height={H})");
    let above = format!("row {} out of bounds (height={H})", u32::MAX);
    let outside = |x: i32| format!("pixel {x} is outside the {ROW}-byte row");
    type Call = Box<dyn Fn(&mut ImageAccessorNoClip<'_, 4>)>;
    let cases: Vec<(&str, Call, String)> = vec![
        (
            "span(W - 1, last row, 1), next_x",
            Box::new(|a| {
                let last = expected_bytes(W as i32 - 1, H as i32 - 1);
                assert_eq!(px(a.span(W as i32 - 1, H as i32 - 1, 1)), last);
                a.next_x();
            }),
            past_row.clone(),
        ),
        (
            "span(0, 0, W), W next_x",
            Box::new(|a| {
                a.span(0, 0, W);
                for _ in 1..W {
                    a.next_x();
                }
                a.next_x();
            }),
            past_row.clone(),
        ),
        (
            "span(W, 0, 0)",
            Box::new(|a| {
                a.span(W as i32, 0, 0);
            }),
            outside(W as i32),
        ),
        (
            "span(0, H, 1)",
            Box::new(|a| {
                a.span(0, H as i32, 1);
            }),
            below.clone(),
        ),
        (
            "span(0, -1, 1)",
            Box::new(|a| {
                a.span(0, -1, 1);
            }),
            above,
        ),
        (
            "span(0, last row, 1), next_y",
            Box::new(|a| {
                a.span(0, H as i32 - 1, 1);
                a.next_y();
            }),
            below,
        ),
        (
            "span(-1, 0, 1)",
            Box::new(|a| {
                a.span(-1, 0, 1);
            }),
            outside(-1),
        ),
        (
            "span(i32::MIN, 0, 1)",
            Box::new(|a| {
                a.span(i32::MIN, 0, 1);
            }),
            outside(i32::MIN),
        ),
        (
            "span(i32::MAX, 0, 1)",
            Box::new(|a| {
                a.span(i32::MAX, 0, 1);
            }),
            outside(i32::MAX),
        ),
        (
            "span(2^30, 0, 1)",
            Box::new(|a| {
                a.span(1 << 30, 0, 1);
            }),
            outside(1 << 30),
        ),
    ];
    for (name, call, expect) in cases {
        let mut acc = ImageAccessorNoClip::<4>::new(&ra);
        assert_eq!(
            panic_message(|| call(&mut acc)).as_deref(),
            Some(expect.as_str()),
            "{name}"
        );
    }
    assert!(g.guards_intact());
}

/// The resample generators step `next_x` until the filter weight runs out, which can be one
/// pixel more than the `len` they pass to `span`: `rx_inv` is rounded down, so a window can
/// cover one pixel more than `len` for some scales and subpixel offsets. At the right edge
/// C++ then reads the next row's first pixel (past the buffer on the last row). Here the
/// accessor's meaning holds there: Clone's clamped edge pixel, Clip's background.
///
/// The reference is independent of the accessor under test: the image drawn into the middle
/// of a canvas large enough for every pixel the generator reads (`LEFT` and `TOP` pixels
/// before the image, `RIGHT` and `BOTTOM` after it; at these downscales the 40-pixel spans
/// reach far past the right edge), whose margins hold the meaning (the clamped pixel for
/// Clone, the background for Clip), read through `ImageAccessorNoClip` with the transform
/// shifted by (`LEFT`, `TOP`). NoClip reads raw bytes and panics on any read
/// outside the canvas, so every pixel of the reference is real bytes, and the shift is an
/// integer, so the generator does the same arithmetic. The image alone (exact stride), and
/// the image in a stride with padding columns of other bytes, must resample identically.
///
/// The cases are ones a search found to step off the row from a span that fits in it (for
/// Bessel, the only one at scales 0.75 to 4 in sixteenths and offsets in 128ths; Bessel at
/// scale 1.75, tx 0.84375 overshoots only spans already past the row). A far-off pixel read
/// there changes the Bessel and bilinear outputs; in the bicubic cases the extra pixel's
/// weight rounds to 0, so only the bounds catch it: on the exact-size image the step lands
/// past the end of the buffer, which Miri reports, and which `ImageAccessorClip`'s checked
/// row slice panicked on.
///
/// So the test cannot pass vacuously if the generators or filters change, `Overshoot` counts
/// the steps each case takes past its spans, and every case must step past the right edge
/// of the image from a span that fit in the row (the fast path's dangerous case).
///
/// Under Miri only the last row is generated (the row whose overshoot would leave the
/// buffer; the others only reach the next row) and only the Bessel case (whose output shows
/// a wrong read) and the bilinear one (the cheapest that overshoots) run.
#[test]
fn resample_past_the_right_edge_keeps_the_meaning() {
    const BG: [u8; 4] = [30, 60, 90, 120];
    const LEFT: u32 = 8;
    const TOP: u32 = 8;
    const RIGHT: u32 = 96;
    const BOTTOM: u32 = 24;
    let (w, h) = (16u32, 8u32);
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        let v = |k: u32| ((x * 37 + y * 91 + k * 53) % 200 + 20) as u8;
        [v(0), v(1), v(2), 255]
    };
    // `h` rows of `w` pixels, each row followed by `pad` pixels of `PADDING` bytes.
    let image = |pad: u32| -> Vec<u8> {
        (0..h)
            .flat_map(|y| {
                (0..w + pad).flat_map(move |x| if x < w { pixel(x, y) } else { [PADDING; 4] })
            })
            .collect()
    };
    let rows: Vec<i32> = if cfg!(miri) {
        vec![h as i32 - 1]
    } else {
        (0..h as i32).collect()
    };
    let cases: &[(&str, f64, f64)] = if cfg!(miri) {
        &[("bessel", 1.375, 0.6796875), ("bilinear", 2.5, 0.234375)]
    } else {
        &[
            ("bessel", 1.375, 0.6796875),
            ("bilinear", 2.5, 0.234375),
            ("bicubic", 2.5, 0.21875),
            ("bicubic", 2.5, 0.234375),
        ]
    };

    /// An accessor that counts the `next_x` steps the generator takes past the `len` it
    /// passed to `span`, per row of the span (`next_y` returns to its start).
    struct Overshoot<S> {
        inner: S,
        width: i64,
        x: i64,
        len: i64,
        /// `next_x` calls since `span` or `next_y`.
        steps: i64,
        /// Steps beyond pixel `len - 1` of the span.
        beyond: u32,
        /// Of those, steps to pixel `width` or later from a span inside the row
        /// (`0 <= x`, `x + len <= width`): the step off the end of the fast path's row.
        past_row: u32,
    }

    impl<S: ImageSource> ImageSource for Overshoot<S> {
        fn span(&mut self, x: i32, y: i32, len: u32) -> &[u8] {
            (self.x, self.len, self.steps) = (x as i64, len as i64, 0);
            self.inner.span(x, y, len)
        }
        fn next_x(&mut self) -> &[u8] {
            self.steps += 1;
            if self.steps >= self.len {
                self.beyond += 1;
                let in_row = self.x >= 0 && self.x + self.len <= self.width;
                if in_row && self.x + self.steps >= self.width {
                    self.past_row += 1;
                }
            }
            self.inner.next_x()
        }
        fn next_y(&mut self) -> &[u8] {
            self.steps = 0;
            self.inner.next_y()
        }
    }

    /// Resamples `rows` of 40 pixels at `scale`, shifted right by `tx` and then by `shift`
    /// in source pixels.
    fn render<S: ImageSource>(
        acc: &mut S,
        lut: &ImageFilterLut,
        (scale, tx, shift): (f64, f64, (f64, f64)),
        rows: &[i32],
    ) -> Vec<[u8; 4]> {
        let mut mtx = TransAffine::new_scaling(scale, scale);
        mtx.multiply(&TransAffine::new_translation(tx, 0.0));
        mtx.multiply(&TransAffine::new_translation(shift.0, shift.1));
        let mut interp = SpanInterpolatorLinear::new(mtx);
        let mut sg = SpanImageResampleRgbaAffine::new(acc, &mut interp, lut);
        sg.prepare();
        let mut out = Vec::new();
        for &y in rows {
            let mut span = vec![Rgba8::new(0, 0, 0, 0); 40];
            sg.generate(&mut span, 0, y, 40);
            out.extend(span.iter().map(|p| [p.r, p.g, p.b, p.a]));
        }
        out
    }
    let lut = |filter: &str| {
        let mut lut = ImageFilterLut::new();
        match filter {
            "bessel" => lut.calculate(&ImageFilterBessel, true),
            "bicubic" => lut.calculate(&ImageFilterBicubic, true),
            _ => lut.calculate(&ImageFilterBilinear, true),
        }
        lut
    };

    let (cw, ch) = (LEFT + w + RIGHT, TOP + h + BOTTOM);
    let mut exact = image(0);
    let mut padded = image(3);
    // SAFETY: each Vec holds `h` rows of the stride's bytes and outlives its accessor.
    let ra_exact = unsafe { RowAccessor::new_with_buf(exact.as_mut_ptr(), w, h, w as i32 * 4) };
    let ra_padded =
        unsafe { RowAccessor::new_with_buf(padded.as_mut_ptr(), w, h, (w + 3) as i32 * 4) };
    for kind in BOUNDED {
        // The canvas: pixel (x, y) is the meaning at image pixel (x - LEFT, y - TOP).
        let mut canvas: Vec<u8> = (0..ch as i64)
            .flat_map(|cy| {
                (0..cw as i64).flat_map(move |cx| {
                    let (x, y) = (cx - LEFT as i64, cy - TOP as i64);
                    let inside = (0..w as i64).contains(&x) && (0..h as i64).contains(&y);
                    match kind {
                        Kind::Clip if !inside => BG,
                        _ => pixel(
                            x.clamp(0, w as i64 - 1) as u32,
                            y.clamp(0, h as i64 - 1) as u32,
                        ),
                    }
                })
            })
            .collect();
        // SAFETY: the Vec holds `ch` rows of `cw * 4` bytes and outlives its accessor.
        let ra_canvas =
            unsafe { RowAccessor::new_with_buf(canvas.as_mut_ptr(), cw, ch, cw as i32 * 4) };
        /// Renders through `inner` wrapped in `Overshoot`; the pixels and the
        /// (`beyond`, `past_row`) counts.
        fn counted<S: ImageSource>(
            inner: S,
            lut: &ImageFilterLut,
            (scale, tx): (f64, f64),
            rows: &[i32],
            width: u32,
        ) -> (Vec<[u8; 4]>, (u32, u32)) {
            let mut acc = Overshoot {
                inner,
                width: width as i64,
                x: 0,
                len: 0,
                steps: 0,
                beyond: 0,
                past_row: 0,
            };
            let out = render(&mut acc, lut, (scale, tx, (0.0, 0.0)), rows);
            (out, (acc.beyond, acc.past_row))
        }
        let run = |ra: &RowAccessor, lut: &ImageFilterLut, scale: f64, tx: f64| {
            let result = catch_unwind(AssertUnwindSafe(|| match kind {
                Kind::Clip => counted(
                    ImageAccessorClip::<4>::new(ra, &BG),
                    lut,
                    (scale, tx),
                    &rows,
                    w,
                ),
                _ => counted(ImageAccessorClone::<4>::new(ra), lut, (scale, tx), &rows, w),
            }));
            result.map_err(|_| "panicked")
        };
        for &(filter, scale, tx) in cases {
            let lut = lut(filter);
            let reference = render(
                &mut ImageAccessorNoClip::<4>::new(&ra_canvas),
                &lut,
                (scale, tx, (LEFT as f64, TOP as f64)),
                &rows,
            );
            for (stride, ra) in [("exact", &ra_exact), ("padded", &ra_padded)] {
                let case = format!("{kind:?}, {filter}, scale {scale}, tx {tx}, {stride} stride");
                let (got, (beyond, past_row)) =
                    run(ra, &lut, scale, tx).unwrap_or_else(|e| panic!("{case}: {e}"));
                assert!(
                    beyond > 0 && past_row > 0,
                    "{case}: no step past the right edge ({beyond} past a span, {past_row} \
                     past the row); the case no longer tests the overshoot"
                );
                assert_eq!(got.len(), reference.len(), "{case}");
                // The first difference, rather than two spans of hundreds of pixels.
                if let Some(i) = (0..got.len()).find(|&i| got[i] != reference[i]) {
                    panic!(
                        "{case}: row {}, x {}: {:?}, reference {:?}",
                        rows[i / 40],
                        i % 40,
                        got[i],
                        reference[i]
                    );
                }
            }
        }
    }
}

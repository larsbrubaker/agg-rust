//! Recursive (IIR) Gaussian blur.
//!
//! Port of `recursive_blur` and its calculators `recursive_blur_calc_rgba`,
//! `recursive_blur_calc_rgb` and `recursive_blur_calc_gray` from `agg_blur.h` (agg-sharp
//! `RecursiveBlur.cs`), run on a [`ChannelView`]. C++'s `blur_y` blurs a `pixfmt_transposer`
//! across; here the same pass walks the columns. The RGBA32 convenience functions in `blur`
//! wrap it.

use super::channel_view::ChannelView;

/// A recursive-blur calculator: the channels of one pixel as `f64`s, C++'s
/// `recursive_blur_calc_*`. `N` is how many of the view's channels it blurs, from channel 0;
/// the rest keep their bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecursiveBlurCalc<const N: usize> {
    v: [f64; N],
}

/// C++ `recursive_blur_calc_rgba`: blurs R, G, B and A.
pub type RecursiveBlurCalcRgba = RecursiveBlurCalc<4>;
/// C++ `recursive_blur_calc_rgb`: blurs R, G and B, leaving A as it is.
pub type RecursiveBlurCalcRgb = RecursiveBlurCalc<3>;
/// C++ `recursive_blur_calc_gray`: blurs the one gray value.
pub type RecursiveBlurCalcGray = RecursiveBlurCalc<1>;

impl<const N: usize> Default for RecursiveBlurCalc<N> {
    fn default() -> Self {
        Self { v: [0.0; N] }
    }
}

impl<const N: usize> RecursiveBlurCalc<N> {
    fn from_pix(img: &ChannelView, x: u32, y: u32) -> Self {
        let mut v = [0.0; N];
        for (c, value) in v.iter_mut().enumerate() {
            *value = img.get(x, y, c) as f64;
        }
        Self { v }
    }

    #[allow(clippy::too_many_arguments)]
    fn calc(
        b1: f64,
        b2: f64,
        b3: f64,
        b4: f64,
        c1: &Self,
        c2: &Self,
        c3: &Self,
        c4: &Self,
    ) -> Self {
        let mut v = [0.0; N];
        for (c, value) in v.iter_mut().enumerate() {
            *value = b1 * c1.v[c] + b2 * c2.v[c] + b3 * c3.v[c] + b4 * c4.v[c];
        }
        Self { v }
    }

    /// The blurred channels back as bytes.
    ///
    /// Deliberate divergence from C++: `to_pix` converts with `cv_type(r)`, which truncates, and
    /// the filter's weights sum to just under 1, so a flat white area comes out 254 after one
    /// pass and 253 after both (and every level drifts down). Rounding to the nearest level and
    /// clamping to 0..=255, as agg-sharp does, keeps flat areas exact. Pinned by agg-sharp's
    /// `RecursiveBlurTests` and `rust_only_recursive_blur_rounds_where_cpp_truncates`.
    fn to_pix(self, out: &mut [u8; N]) {
        for (byte, &value) in out.iter_mut().zip(self.v.iter()) {
            *byte = if value <= 0.0 {
                0
            } else if value >= 255.0 {
                255
            } else {
                (value + 0.5) as u8
            };
        }
    }
}

/// The filter's coefficients for `radius`: each output is `b` times its input plus `b1`, `b2`
/// and `b3` times the three outputs before it (Young and van Vliet's recursive Gaussian).
pub fn recursive_blur_coefficients(radius: f64) -> (f64, f64, f64, f64) {
    let s = radius * 0.5;
    let q = if s < 2.5 {
        3.97156 - 4.14554 * (1.0 - 0.26891 * s).sqrt()
    } else {
        0.98711 * s - 0.96330
    };

    let q2 = q * q;
    let q3 = q2 * q;

    let b0 = 1.0 / (1.578250 + 2.444130 * q + 1.428100 * q2 + 0.422205 * q3);
    let b1 = 2.44413 * q + 2.85619 * q2 + 1.26661 * q3;
    let b2 = -1.42810 * q2 + -1.26661 * q3;
    let b3 = 0.422205 * q3;
    let b = 1.0 - (b1 + b2 + b3) * b0;

    (b, b1 * b0, b2 * b0, b3 * b0)
}

/// C++ `recursive_blur<ColorT, CalculatorT>`: a recursive Gaussian approximation whose cost
/// does not grow with the radius. `N` picks the calculator (see [`RecursiveBlurCalc`]).
pub struct RecursiveBlur<const N: usize> {
    sum1: Vec<RecursiveBlurCalc<N>>,
    sum2: Vec<RecursiveBlurCalc<N>>,
    buf: Vec<[u8; N]>,
}

impl<const N: usize> Default for RecursiveBlur<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> RecursiveBlur<N> {
    pub fn new() -> Self {
        Self {
            sum1: Vec::new(),
            sum2: Vec::new(),
            buf: Vec::new(),
        }
    }

    /// Blurs each row across. Radii under 0.62 and images under 3 pixels wide are left alone.
    ///
    /// # Panics
    /// When the view has fewer than `N` channels.
    pub fn blur_x(&mut self, img: &mut ChannelView, radius: f64) {
        let (w, h) = (img.width(), img.height());
        self.blur_lines(img, radius, w, h, |i, line| (i, line));
    }

    /// Blurs each column down (C++ blurs a `pixfmt_transposer` across).
    pub fn blur_y(&mut self, img: &mut ChannelView, radius: f64) {
        let (w, h) = (img.width(), img.height());
        self.blur_lines(img, radius, h, w, |i, line| (line, i));
    }

    /// Blurs across, then down.
    pub fn blur(&mut self, img: &mut ChannelView, radius: f64) {
        self.blur_x(img, radius);
        self.blur_y(img, radius);
    }

    /// C++ `blur_x` over `lines` lines of `len` pixels; `at(i, line)` is a pixel's position.
    fn blur_lines(
        &mut self,
        img: &mut ChannelView,
        radius: f64,
        len: u32,
        lines: u32,
        at: impl Fn(u32, u32) -> (u32, u32),
    ) {
        assert!(
            img.channels() >= N,
            "a {N}-channel recursive blur needs a view of at least {N} channels"
        );
        if radius < 0.62 {
            return;
        }
        if len < 3 {
            return;
        }

        let (b, b1, b2, b3) = recursive_blur_coefficients(radius);
        let w = len as usize;
        let wm = w - 1;
        self.sum1.resize(w, RecursiveBlurCalc::default());
        self.sum2.resize(w, RecursiveBlurCalc::default());
        self.buf.resize(w, [0; N]);
        let (sum1, sum2, buf) = (&mut self.sum1, &mut self.sum2, &mut self.buf);
        let pix = |img: &ChannelView, x: usize, line: u32| {
            let (px, py) = at(x as u32, line);
            RecursiveBlurCalc::<N>::from_pix(img, px, py)
        };

        for line in 0..lines {
            let c = pix(img, 0, line);
            sum1[0] = RecursiveBlurCalc::calc(b, b1, b2, b3, &c, &c, &c, &c);
            let c = pix(img, 1, line);
            sum1[1] = RecursiveBlurCalc::calc(b, b1, b2, b3, &c, &sum1[0], &sum1[0], &sum1[0]);
            let c = pix(img, 2, line);
            sum1[2] = RecursiveBlurCalc::calc(b, b1, b2, b3, &c, &sum1[1], &sum1[0], &sum1[0]);

            for x in 3..w {
                let c = pix(img, x, line);
                sum1[x] = RecursiveBlurCalc::calc(
                    b,
                    b1,
                    b2,
                    b3,
                    &c,
                    &sum1[x - 1],
                    &sum1[x - 2],
                    &sum1[x - 3],
                );
            }

            sum2[wm] =
                RecursiveBlurCalc::calc(b, b1, b2, b3, &sum1[wm], &sum1[wm], &sum1[wm], &sum1[wm]);
            sum2[wm - 1] = RecursiveBlurCalc::calc(
                b,
                b1,
                b2,
                b3,
                &sum1[wm - 1],
                &sum2[wm],
                &sum2[wm],
                &sum2[wm],
            );
            sum2[wm - 2] = RecursiveBlurCalc::calc(
                b,
                b1,
                b2,
                b3,
                &sum1[wm - 2],
                &sum2[wm - 1],
                &sum2[wm],
                &sum2[wm],
            );
            sum2[wm].to_pix(&mut buf[wm]);
            sum2[wm - 1].to_pix(&mut buf[wm - 1]);
            sum2[wm - 2].to_pix(&mut buf[wm - 2]);

            for x in (0..wm - 2).rev() {
                sum2[x] = RecursiveBlurCalc::calc(
                    b,
                    b1,
                    b2,
                    b3,
                    &sum1[x],
                    &sum2[x + 1],
                    &sum2[x + 2],
                    &sum2[x + 3],
                );
                sum2[x].to_pix(&mut buf[x]);
            }

            // C++ copy_color_hspan: only the calculator's channels are written back.
            for (x, px) in buf.iter().enumerate() {
                let (px_x, px_y) = at(x as u32, line);
                for (c, &v) in px.iter().enumerate() {
                    img.set(px_x, px_y, c, v);
                }
            }
        }
    }
}

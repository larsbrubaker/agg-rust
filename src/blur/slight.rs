//! Slight blur: a 3x3 separable Gaussian that reaches only the adjacent pixels.
//!
//! Port of `slight_blur` and `apply_slight_blur` from `agg_blur.h` (agg-sharp `SlightBlur.cs`),
//! run on a [`ChannelView`]. Sits beside the stack and recursive blurs in `blur`.

use super::channel_view::ChannelView;
use crate::basics::{uround, RectI};

/// C++ `slight_blur`: a Gaussian blur small enough to reach only the adjacent pixels, for
/// smoothing pixel-scale detail such as single-pixel lines. The curve has a standard deviation
/// of r/2 (as in the HTML/CSS spec), sampled at 0 and at 1 pixel; the radius can go to about
/// 1.33 before the cut-off matters. Every channel is blurred alike, so use it on images without
/// alpha or with premultiplied alpha.
#[derive(Clone, Copy, Debug)]
pub struct SlightBlur {
    g0: f64,
    g1: f64,
}

impl Default for SlightBlur {
    /// C++'s default radius, 1.33.
    fn default() -> Self {
        Self::new(1.33)
    }
}

impl SlightBlur {
    pub fn new(radius: f64) -> Self {
        let mut blur = Self { g0: 1.0, g1: 0.0 };
        blur.radius(radius);
        blur
    }

    /// Sets the radius in pixels. 0 or less leaves the image as it is.
    pub fn radius(&mut self, r: f64) {
        if r > 0.0 {
            // Sample the gaussian curve at 0 and r/2 standard deviations. At 3 standard
            // deviations, the response is < 0.005. C++'s truncated pi is kept so the weights
            // match to the last bit.
            #[allow(clippy::approx_constant)]
            let pi: f64 = 3.14159;
            let n = 2.0 / r;
            self.g0 = 1.0 / (2.0 * pi).sqrt();
            self.g1 = self.g0 * (-n * n).exp();

            // Normalize.
            let sum = self.g0 + 2.0 * self.g1;
            self.g0 /= sum;
            self.g1 /= sum;
        } else {
            self.g0 = 1.0;
            self.g1 = 0.0;
        }
    }

    /// The center pixel's weight.
    pub fn center_weight(&self) -> f64 {
        self.g0
    }

    /// Each neighbour's weight.
    pub fn neighbor_weight(&self) -> f64 {
        self.g1
    }

    /// Blurs the inclusive pixel box `bounds` (clipped to the image) in place: each row across,
    /// then the rows down, the box's edge pixels standing in for those beyond it. A box under 3
    /// pixels wide or high is left alone.
    pub fn blur(&self, img: &mut ChannelView, bounds: RectI) {
        // Make sure we stay within the image area.
        let mut bounds = bounds;
        bounds.clip(&RectI::new(
            0,
            0,
            img.width() as i32 - 1,
            img.height() as i32 - 1,
        ));

        let w = bounds.x2 - bounds.x1 + 1;
        let h = bounds.y2 - bounds.y1 + 1;
        if w < 3 || h < 3 {
            return;
        }
        let (x1, y1) = (bounds.x1 as u32, bounds.y1 as u32);
        let (w, h) = (w as usize, h as usize);
        let channels = img.channels();
        let len = w * channels;

        // The rows above, at and below the one being written, each already blurred across; the
        // top row stands in for the one above it.
        let mut r0 = vec![0u8; len];
        let mut r1 = vec![0u8; len];
        let mut r2 = vec![0u8; len];
        self.calc_row(img, x1, y1, w, &mut r0);
        r1.copy_from_slice(&r0);

        for y in 0..h {
            if y + 1 < h {
                self.calc_row(img, x1, y1 + y as u32 + 1, w, &mut r2);
            } else {
                // duplicate bottom row
                r2.copy_from_slice(&r1);
            }

            // Combine blurred rows into destination.
            for x in 0..w {
                for c in 0..channels {
                    let i = x * channels + c;
                    let v = self.calc_value(r0[i], r1[i], r2[i]);
                    img.set(x1 + x as u32, y1 + y as u32, c, v);
                }
            }

            // The row below becomes the current one, the current one the row above.
            std::mem::swap(&mut r0, &mut r1);
            std::mem::swap(&mut r1, &mut r2);
        }
    }

    /// Row `y` from `x` blurred across into `row`, the end pixels standing in for their missing
    /// neighbours.
    fn calc_row(&self, img: &ChannelView, x: u32, y: u32, w: usize, row: &mut [u8]) {
        let channels = img.channels();
        for i in 0..w {
            let center = x + i as u32;
            let left = if i > 0 { center - 1 } else { center };
            let right = if i + 1 < w { center + 1 } else { center };
            for c in 0..channels {
                row[i * channels + c] = self.calc_value(
                    img.get(left, y, c),
                    img.get(center, y, c),
                    img.get(right, y, c),
                );
            }
        }
    }

    /// C++ `calc_value`, in the same order of operations so the sums match to the bit.
    ///
    /// Deliberate divergence from C++: `agg_blur.h` converts with `value_type(...)`, which
    /// truncates, so on 8-bit channels every blurred pixel loses up to a level (a lone white
    /// pixel at radius 1 ends at 236 rather than 237; C++'s own line_thickness example uses a
    /// float pixel format, where it does not show). Rounding, as agg-sharp does, keeps the
    /// result nearest the true weighted sum. Pinned by agg-sharp's `SlightBlurTests` and
    /// `rust_only_slight_blur_rounds_where_cpp_truncates`.
    #[inline]
    fn calc_value(&self, v1: u8, v2: u8, v3: u8) -> u8 {
        uround(self.g1 * v1 as f64 + self.g0 * v2 as f64 + self.g1 * v3 as f64).min(255) as u8
    }
}

/// C++ `apply_slight_blur(img, bounds, r)`: blurs `bounds` when `r` is above 0.
pub fn apply_slight_blur(img: &mut ChannelView, bounds: RectI, r: f64) {
    if r > 0.0 {
        SlightBlur::new(r).blur(img, bounds);
    }
}

/// C++ `apply_slight_blur(img, r)`: blurs the whole image when `r` is above 0.
pub fn apply_slight_blur_all(img: &mut ChannelView, r: f64) {
    let bounds = RectI::new(0, 0, img.width() as i32 - 1, img.height() as i32 - 1);
    apply_slight_blur(img, bounds, r);
}

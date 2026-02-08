//! Bicubic spline interpolation.
//!
//! Port of `agg_bspline.h` / `agg_bspline.cpp` — interpolates a set of
//! (x, y) data points using bicubic spline with linear extrapolation
//! outside the data range.

use std::cell::Cell;

/// Bicubic spline interpolator.
///
/// Use `init()` or the point-by-point API (`init_num`, `add_point`, `prepare`)
/// to set up data, then `get()` or `get_stateful()` to evaluate.
///
/// Port of C++ `agg::bspline`.
pub struct Bspline {
    num: usize,
    x: Vec<f64>,
    y: Vec<f64>,
    am: Vec<f64>,
    last_idx: Cell<i32>,
}

impl Bspline {
    /// Create an empty spline.
    pub fn new() -> Self {
        Self {
            num: 0,
            x: Vec::new(),
            y: Vec::new(),
            am: Vec::new(),
            last_idx: Cell::new(-1),
        }
    }

    /// Create and initialize from arrays.
    pub fn new_with_points(x: &[f64], y: &[f64]) -> Self {
        let mut s = Self::new();
        s.init(x, y);
        s
    }

    /// Initialize the spline storage for `max` points.
    pub fn init_num(&mut self, max: usize) {
        if max > 2 {
            self.x.resize(max, 0.0);
            self.y.resize(max, 0.0);
            self.am.resize(max, 0.0);
        }
        self.num = 0;
        self.last_idx.set(-1);
    }

    /// Add a data point (call after `init_num`, before `prepare`).
    pub fn add_point(&mut self, x: f64, y: f64) {
        if self.num < self.x.len() {
            self.x[self.num] = x;
            self.y[self.num] = y;
            self.num += 1;
        }
    }

    /// Compute the spline coefficients (call after adding all points).
    pub fn prepare(&mut self) {
        if self.num > 2 {
            let n1 = self.num;

            for k in 0..n1 {
                self.am[k] = 0.0;
            }

            let mut al = vec![0.0; 3 * n1];
            let n1 = self.num - 1;
            let mut d = self.x[1] - self.x[0];
            let mut e = (self.y[1] - self.y[0]) / d;

            for k in 1..n1 {
                let h = d;
                d = self.x[k + 1] - self.x[k];
                let f = e;
                e = (self.y[k + 1] - self.y[k]) / d;
                al[k] = d / (d + h);
                al[self.num + k] = 1.0 - al[k]; // r[k]
                al[self.num * 2 + k] = 6.0 * (e - f) / (h + d); // s[k]
            }

            for k in 1..n1 {
                let p = 1.0 / (al[self.num + k] * al[k - 1] + 2.0);
                al[k] *= -p;
                al[self.num * 2 + k] =
                    (al[self.num * 2 + k] - al[self.num + k] * al[self.num * 2 + k - 1]) * p;
            }

            self.am[n1] = 0.0;
            al[n1 - 1] = al[self.num * 2 + n1 - 1];
            self.am[n1 - 1] = al[n1 - 1];

            let mut k = n1 as i32 - 2;
            for _i in 0..self.num - 2 {
                let ku = k as usize;
                al[ku] = al[ku] * al[ku + 1] + al[self.num * 2 + ku];
                self.am[ku] = al[ku];
                k -= 1;
            }
        }
        self.last_idx.set(-1);
    }

    /// Initialize from x and y arrays.
    pub fn init(&mut self, x: &[f64], y: &[f64]) {
        let num = x.len().min(y.len());
        if num > 2 {
            self.init_num(num);
            for i in 0..num {
                self.add_point(x[i], y[i]);
            }
            self.prepare();
        }
        self.last_idx.set(-1);
    }

    /// Evaluate the spline at `x` (stateless binary search each call).
    pub fn get(&self, x: f64) -> f64 {
        if self.num > 2 {
            if x < self.x[0] {
                return self.extrapolation_left(x);
            }
            if x >= self.x[self.num - 1] {
                return self.extrapolation_right(x);
            }
            let i = self.bsearch(x);
            return self.interpolation(x, i);
        }
        0.0
    }

    /// Evaluate the spline at `x` with cached index (faster for sequential access).
    pub fn get_stateful(&self, x: f64) -> f64 {
        if self.num > 2 {
            if x < self.x[0] {
                return self.extrapolation_left(x);
            }
            if x >= self.x[self.num - 1] {
                return self.extrapolation_right(x);
            }

            let last = self.last_idx.get();
            if last >= 0 {
                let li = last as usize;
                if x < self.x[li] || x > self.x[li + 1] {
                    if li < self.num - 2 && x >= self.x[li + 1] && x <= self.x[li + 2] {
                        self.last_idx.set(last + 1);
                    } else if li > 0 && x >= self.x[li - 1] && x <= self.x[li] {
                        self.last_idx.set(last - 1);
                    } else {
                        let i = self.bsearch(x);
                        self.last_idx.set(i as i32);
                    }
                }
                return self.interpolation(x, self.last_idx.get() as usize);
            } else {
                let i = self.bsearch(x);
                self.last_idx.set(i as i32);
                return self.interpolation(x, i);
            }
        }
        0.0
    }

    /// Binary search for the interval containing `x`.
    fn bsearch(&self, x0: f64) -> usize {
        let mut lo = 0usize;
        let mut hi = self.num - 1;
        while hi - lo > 1 {
            let mid = (lo + hi) >> 1;
            if x0 < self.x[mid] {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }

    /// Cubic interpolation in interval `[x[i], x[i+1]]`.
    fn interpolation(&self, x: f64, i: usize) -> f64 {
        let j = i + 1;
        let d = self.x[i] - self.x[j];
        let h = x - self.x[j];
        let r = self.x[i] - x;
        let p = d * d / 6.0;
        (self.am[j] * r * r * r + self.am[i] * h * h * h) / 6.0 / d
            + ((self.y[j] - self.am[j] * p) * r + (self.y[i] - self.am[i] * p) * h) / d
    }

    /// Linear extrapolation beyond the left endpoint.
    fn extrapolation_left(&self, x: f64) -> f64 {
        let d = self.x[1] - self.x[0];
        (-d * self.am[1] / 6.0 + (self.y[1] - self.y[0]) / d) * (x - self.x[0]) + self.y[0]
    }

    /// Linear extrapolation beyond the right endpoint.
    fn extrapolation_right(&self, x: f64) -> f64 {
        let d = self.x[self.num - 1] - self.x[self.num - 2];
        (d * self.am[self.num - 2] / 6.0 + (self.y[self.num - 1] - self.y[self.num - 2]) / d)
            * (x - self.x[self.num - 1])
            + self.y[self.num - 1]
    }
}

impl Default for Bspline {
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
    fn test_linear_interpolation() {
        // y = 2x, spline through linear data should reproduce it
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [0.0, 2.0, 4.0, 6.0, 8.0];
        let s = Bspline::new_with_points(&x, &y);

        assert!((s.get(0.0) - 0.0).abs() < 1e-6);
        assert!((s.get(2.0) - 4.0).abs() < 1e-6);
        assert!((s.get(4.0) - 8.0).abs() < 1e-6);
        assert!((s.get(0.5) - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_data_points_exact() {
        // Spline should pass through data points
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [0.0, 1.0, 0.0, -1.0, 0.0, 1.0];
        let s = Bspline::new_with_points(&x, &y);

        for i in 0..x.len() {
            assert!(
                (s.get(x[i]) - y[i]).abs() < 1e-6,
                "at x={}, expected {}, got {}",
                x[i],
                y[i],
                s.get(x[i])
            );
        }
    }

    #[test]
    fn test_extrapolation_left() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [0.0, 1.0, 4.0, 9.0];
        let s = Bspline::new_with_points(&x, &y);

        // Should extrapolate linearly to the left
        let v1 = s.get(-1.0);
        let v2 = s.get(-2.0);
        // Linear extrapolation: v2 - v1 should equal v1 - s.get(0.0) approximately
        let slope = v1 - s.get(0.0);
        assert!((v2 - v1 - slope).abs() < 1e-6);
    }

    #[test]
    fn test_extrapolation_right() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [0.0, 1.0, 4.0, 9.0];
        let s = Bspline::new_with_points(&x, &y);

        let v1 = s.get(4.0);
        let v2 = s.get(5.0);
        let slope = v1 - s.get(3.0);
        assert!((v2 - v1 - slope).abs() < 1e-6);
    }

    #[test]
    fn test_get_stateful_matches_get() {
        let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [0.0, 0.5, 1.0, 1.5, 1.0, 0.0];
        let s = Bspline::new_with_points(&x, &y);

        // Sequential access should give same results
        for i in 0..50 {
            let xi = i as f64 * 0.1;
            let v1 = s.get(xi);
            let v2 = s.get_stateful(xi);
            assert!(
                (v1 - v2).abs() < 1e-10,
                "at x={}, get={}, get_stateful={}",
                xi,
                v1,
                v2
            );
        }
    }

    #[test]
    fn test_empty_spline() {
        let s = Bspline::new();
        assert_eq!(s.get(1.0), 0.0);
        assert_eq!(s.get_stateful(1.0), 0.0);
    }

    #[test]
    fn test_point_by_point_api() {
        let mut s = Bspline::new();
        s.init_num(4);
        s.add_point(0.0, 0.0);
        s.add_point(1.0, 1.0);
        s.add_point(2.0, 0.0);
        s.add_point(3.0, 1.0);
        s.prepare();

        assert!((s.get(0.0) - 0.0).abs() < 1e-6);
        assert!((s.get(1.0) - 1.0).abs() < 1e-6);
        assert!((s.get(2.0) - 0.0).abs() < 1e-6);
        assert!((s.get(3.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_monotonic_in_monotonic_region() {
        // For strictly increasing data, spline should be roughly increasing
        let x = [0.0, 1.0, 2.0, 3.0, 4.0];
        let y = [0.0, 1.0, 2.0, 3.0, 4.0];
        let s = Bspline::new_with_points(&x, &y);

        let mut prev = s.get(0.0);
        for i in 1..40 {
            let xi = i as f64 * 0.1;
            let v = s.get(xi);
            assert!(v >= prev - 1e-6, "not monotonic at x={}", xi);
            prev = v;
        }
    }
}

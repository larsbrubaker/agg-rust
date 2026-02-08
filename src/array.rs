//! Container utilities and vertex sequence types.
//!
//! Port of `agg_array.h` and `agg_vertex_sequence.h`.
//!
//! Most of AGG's C++ containers (`pod_vector`, `pod_array`, `pod_bvector`) map
//! directly to Rust's `Vec<T>`. This module provides the algorithms and
//! specialized types that don't have direct Rust equivalents.

use crate::math::{calc_distance, VERTEX_DIST_EPSILON};

// ============================================================================
// Sorting and searching utilities
// ============================================================================

/// Threshold below which quicksort switches to insertion sort.
/// Matches C++ `quick_sort_threshold`.
pub const QUICK_SORT_THRESHOLD: usize = 9;

/// Sort a mutable slice using AGG's quicksort algorithm.
/// Uses insertion sort for small partitions.
///
/// This is a direct port of C++ `quick_sort` from `agg_array.h`.
/// For most Rust code you'd use `slice::sort_by`, but this is provided
/// for exact behavioral matching where sort order matters.
pub fn quick_sort<T, F>(arr: &mut [T], less: &F)
where
    T: Copy,
    F: Fn(&T, &T) -> bool,
{
    if arr.len() < 2 {
        return;
    }

    let mut stack = [0i32; 80];
    let mut top: usize = 0;
    let mut limit = arr.len() as i32;
    let mut base = 0i32;

    loop {
        let len = limit - base;

        if len > QUICK_SORT_THRESHOLD as i32 {
            let pivot = base + len / 2;
            arr.swap(base as usize, pivot as usize);

            let mut i = base + 1;
            let mut j = limit - 1;

            // Ensure arr[i] <= arr[base] <= arr[j]
            if less(&arr[j as usize], &arr[i as usize]) {
                arr.swap(j as usize, i as usize);
            }
            if less(&arr[base as usize], &arr[i as usize]) {
                arr.swap(base as usize, i as usize);
            }
            if less(&arr[j as usize], &arr[base as usize]) {
                arr.swap(j as usize, base as usize);
            }

            loop {
                loop {
                    i += 1;
                    if !less(&arr[i as usize], &arr[base as usize]) {
                        break;
                    }
                }
                loop {
                    j -= 1;
                    if !less(&arr[base as usize], &arr[j as usize]) {
                        break;
                    }
                }
                if i > j {
                    break;
                }
                arr.swap(i as usize, j as usize);
            }

            arr.swap(base as usize, j as usize);

            // Push the larger sub-array
            if j - base > limit - i {
                stack[top] = base;
                stack[top + 1] = j;
                base = i;
            } else {
                stack[top] = i;
                stack[top + 1] = limit;
                limit = j;
            }
            top += 2;
        } else {
            // Insertion sort for small partitions
            let mut j = base;
            let mut i = j + 1;
            while i < limit {
                while j >= base && less(&arr[(j + 1) as usize], &arr[j as usize]) {
                    arr.swap((j + 1) as usize, j as usize);
                    if j == base {
                        break;
                    }
                    j -= 1;
                }
                i += 1;
                j = i - 1;
            }

            if top > 0 {
                top -= 2;
                base = stack[top];
                limit = stack[top + 1];
            } else {
                break;
            }
        }
    }
}

/// Remove duplicates from a sorted slice. Returns the number of remaining elements.
/// The slice is modified in place (duplicates are overwritten, tail is unchanged).
///
/// Port of C++ `remove_duplicates`.
pub fn remove_duplicates<T, F>(arr: &mut [T], equal: &F) -> usize
where
    T: Copy,
    F: Fn(&T, &T) -> bool,
{
    if arr.len() < 2 {
        return arr.len();
    }
    let mut j = 1usize;
    for i in 1..arr.len() {
        if !equal(&arr[i], &arr[i - 1]) {
            arr[j] = arr[i];
            j += 1;
        }
    }
    j
}

/// Reverse the elements of a slice in place.
/// Port of C++ `invert_container`.
pub fn invert_container<T>(arr: &mut [T]) {
    arr.reverse();
}

/// Binary search for the insertion position of `val` in a sorted slice.
/// Returns the index where `val` would be inserted to maintain sort order.
///
/// Port of C++ `binary_search_pos`.
pub fn binary_search_pos<T, V, F>(arr: &[T], val: &V, less: &F) -> usize
where
    F: Fn(&V, &T) -> bool,
{
    if arr.is_empty() {
        return 0;
    }

    let mut beg = 0usize;
    let mut end = arr.len() - 1;

    if less(val, &arr[0]) {
        return 0;
    }
    // Need a reverse less for this check
    // In C++: if(less(arr[end], val)) return end + 1;
    // We check: if val > arr[end]
    if !less(val, &arr[end]) {
        return end + 1;
    }

    while end - beg > 1 {
        let mid = (end + beg) >> 1;
        if less(val, &arr[mid]) {
            end = mid;
        } else {
            beg = mid;
        }
    }
    end
}

// ============================================================================
// Vertex dist types
// ============================================================================

/// A vertex with coordinates and the distance to the next vertex.
/// Port of C++ `vertex_dist`.
///
/// The `calc_dist` method computes the distance to another vertex_dist
/// and returns `true` if the distance exceeds `VERTEX_DIST_EPSILON`
/// (i.e., the vertices are not coincident).
#[derive(Debug, Clone, Copy)]
pub struct VertexDist {
    pub x: f64,
    pub y: f64,
    pub dist: f64,
}

impl VertexDist {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, dist: 0.0 }
    }

    /// Calculate distance to `val` and store it. Returns `true` if the
    /// points are not coincident (distance > VERTEX_DIST_EPSILON).
    /// If coincident, sets dist to `1.0 / VERTEX_DIST_EPSILON`.
    pub fn calc_dist(&mut self, val: &VertexDist) -> bool {
        self.dist = calc_distance(self.x, self.y, val.x, val.y);
        let ret = self.dist > VERTEX_DIST_EPSILON;
        if !ret {
            self.dist = 1.0 / VERTEX_DIST_EPSILON;
        }
        ret
    }
}

/// Like `VertexDist` but with an additional path command.
/// Port of C++ `vertex_dist_cmd`.
#[derive(Debug, Clone, Copy)]
pub struct VertexDistCmd {
    pub x: f64,
    pub y: f64,
    pub dist: f64,
    pub cmd: u32,
}

impl VertexDistCmd {
    pub fn new(x: f64, y: f64, cmd: u32) -> Self {
        Self {
            x,
            y,
            dist: 0.0,
            cmd,
        }
    }

    /// Calculate distance to `val`. Returns `true` if non-coincident.
    pub fn calc_dist(&mut self, val: &VertexDistCmd) -> bool {
        self.dist = calc_distance(self.x, self.y, val.x, val.y);
        let ret = self.dist > VERTEX_DIST_EPSILON;
        if !ret {
            self.dist = 1.0 / VERTEX_DIST_EPSILON;
        }
        ret
    }
}

// ============================================================================
// Vertex sequence
// ============================================================================

/// A sequence of vertices that automatically filters coincident points.
///
/// Port of C++ `vertex_sequence<T>` which inherits from `pod_bvector`.
/// When a new vertex is added, it calculates the distance from the previous
/// vertex. If the previous vertex is coincident (distance <= epsilon),
/// it is removed.
///
/// This is the Rust equivalent using `Vec<VertexDist>` as backing storage.
pub struct VertexSequence {
    vertices: Vec<VertexDist>,
}

impl VertexSequence {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    pub fn size(&self) -> usize {
        self.vertices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Add a vertex to the sequence, removing the previous vertex if it's
    /// coincident with the one before it.
    ///
    /// The C++ version checks if `vertices[size-2]` is coincident with
    /// `vertices[size-1]`, and if so removes `vertices[size-1]`. This is a
    /// "lazy" check — coincident pairs are cleaned up when the NEXT vertex
    /// is added.
    pub fn add(&mut self, val: VertexDist) {
        if self.vertices.len() > 1 {
            let len = self.vertices.len();
            let last = self.vertices[len - 1];
            let keep = self.vertices[len - 2].calc_dist(&last);
            if !keep {
                self.vertices.pop();
            }
        }
        self.vertices.push(val);
    }

    /// Modify the last vertex.
    pub fn modify_last(&mut self, val: VertexDist) {
        self.vertices.pop();
        self.add(val);
    }

    /// Close the sequence, removing trailing coincident vertices.
    /// If `closed` is true, also removes the last vertex if it's coincident
    /// with the first.
    pub fn close(&mut self, closed: bool) {
        while self.vertices.len() > 1 {
            let len = self.vertices.len();
            let keep = {
                let mut prev = self.vertices[len - 2];
                let last = self.vertices[len - 1];
                prev.calc_dist(&last)
            };
            if keep {
                break;
            }
            let t = self.vertices[self.vertices.len() - 1];
            self.vertices.pop();
            self.modify_last(t);
        }

        if closed {
            while self.vertices.len() > 1 {
                let len = self.vertices.len();
                let keep = {
                    let mut last = self.vertices[len - 1];
                    let first = self.vertices[0];
                    last.calc_dist(&first)
                };
                if keep {
                    break;
                }
                self.vertices.pop();
            }
        }
    }

    pub fn remove_all(&mut self) {
        self.vertices.clear();
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Get a reference to the underlying vertex slice.
    pub fn as_slice(&self) -> &[VertexDist] {
        &self.vertices
    }

    /// Get a mutable reference to the underlying vertex slice.
    pub fn as_mut_slice(&mut self) -> &mut [VertexDist] {
        &mut self.vertices
    }
}

impl Default for VertexSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl core::ops::Index<usize> for VertexSequence {
    type Output = VertexDist;

    fn index(&self, i: usize) -> &VertexDist {
        &self.vertices[i]
    }
}

impl core::ops::IndexMut<usize> for VertexSequence {
    fn index_mut(&mut self, i: usize) -> &mut VertexDist {
        &mut self.vertices[i]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_sort_basic() {
        let mut arr = [5, 3, 8, 1, 9, 2, 7, 4, 6, 0];
        quick_sort(&mut arr, &|a: &i32, b: &i32| *a < *b);
        assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_quick_sort_already_sorted() {
        let mut arr = [0, 1, 2, 3, 4];
        quick_sort(&mut arr, &|a: &i32, b: &i32| *a < *b);
        assert_eq!(arr, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_quick_sort_reverse() {
        let mut arr = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        quick_sort(&mut arr, &|a: &i32, b: &i32| *a < *b);
        assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_quick_sort_empty_and_single() {
        let mut empty: [i32; 0] = [];
        quick_sort(&mut empty, &|a: &i32, b: &i32| *a < *b);

        let mut single = [42];
        quick_sort(&mut single, &|a: &i32, b: &i32| *a < *b);
        assert_eq!(single, [42]);
    }

    #[test]
    fn test_quick_sort_small_partition() {
        // Test with array size <= QUICK_SORT_THRESHOLD to exercise insertion sort
        let mut arr = [5, 3, 1, 4, 2];
        quick_sort(&mut arr, &|a: &i32, b: &i32| *a < *b);
        assert_eq!(arr, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_quick_sort_large() {
        let mut arr: Vec<i32> = (0..100).rev().collect();
        quick_sort(&mut arr, &|a: &i32, b: &i32| *a < *b);
        let expected: Vec<i32> = (0..100).collect();
        assert_eq!(arr, expected);
    }

    #[test]
    fn test_remove_duplicates() {
        let mut arr = [1, 1, 2, 3, 3, 3, 4, 5, 5];
        let n = remove_duplicates(&mut arr, &|a: &i32, b: &i32| *a == *b);
        assert_eq!(n, 5);
        assert_eq!(&arr[..n], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_remove_duplicates_no_dups() {
        let mut arr = [1, 2, 3, 4, 5];
        let n = remove_duplicates(&mut arr, &|a: &i32, b: &i32| *a == *b);
        assert_eq!(n, 5);
    }

    #[test]
    fn test_remove_duplicates_all_same() {
        let mut arr = [1, 1, 1, 1];
        let n = remove_duplicates(&mut arr, &|a: &i32, b: &i32| *a == *b);
        assert_eq!(n, 1);
    }

    #[test]
    fn test_invert_container() {
        let mut arr = [1, 2, 3, 4, 5];
        invert_container(&mut arr);
        assert_eq!(arr, [5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_binary_search_pos() {
        let arr = [10, 20, 30, 40, 50];
        assert_eq!(binary_search_pos(&arr, &0, &|a: &i32, b: &i32| *a < *b), 0);
        assert_eq!(binary_search_pos(&arr, &25, &|a: &i32, b: &i32| *a < *b), 2);
        assert_eq!(binary_search_pos(&arr, &60, &|a: &i32, b: &i32| *a < *b), 5);
    }

    #[test]
    fn test_vertex_dist_coincident() {
        let mut v1 = VertexDist::new(1.0, 2.0);
        let v2 = VertexDist::new(1.0, 2.0);
        assert!(!v1.calc_dist(&v2));
        assert_eq!(v1.dist, 1.0 / VERTEX_DIST_EPSILON);
    }

    #[test]
    fn test_vertex_dist_non_coincident() {
        let mut v1 = VertexDist::new(0.0, 0.0);
        let v2 = VertexDist::new(3.0, 4.0);
        assert!(v1.calc_dist(&v2));
        assert!((v1.dist - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_vertex_sequence_basic() {
        let mut seq = VertexSequence::new();
        seq.add(VertexDist::new(0.0, 0.0));
        seq.add(VertexDist::new(1.0, 0.0));
        seq.add(VertexDist::new(2.0, 0.0));
        assert_eq!(seq.size(), 3);
    }

    #[test]
    fn test_vertex_sequence_removes_coincident() {
        let mut seq = VertexSequence::new();
        seq.add(VertexDist::new(0.0, 0.0));
        seq.add(VertexDist::new(1.0, 0.0));
        // Add a point coincident with the previous
        seq.add(VertexDist::new(1.0, 0.0));
        // Lazy check: coincident pair isn't removed until next add
        assert_eq!(seq.size(), 3);
        // Adding another point triggers removal of the coincident vertex
        seq.add(VertexDist::new(2.0, 0.0));
        assert_eq!(seq.size(), 3); // (0,0), (1,0), (2,0) — one of the duplicate (1,0) removed
    }

    #[test]
    fn test_vertex_sequence_close() {
        let mut seq = VertexSequence::new();
        seq.add(VertexDist::new(0.0, 0.0));
        seq.add(VertexDist::new(1.0, 0.0));
        seq.add(VertexDist::new(0.0, 0.0)); // Same as first
                                            // Close with closed=true should remove vertex coincident with first
        seq.close(true);
        assert_eq!(seq.size(), 2);
    }
}

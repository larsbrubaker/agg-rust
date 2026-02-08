//! Solving simultaneous equations via Gaussian elimination.
//!
//! Port of `agg_simul_eq.h` — solves systems of linear equations using
//! Gaussian elimination with partial pivoting.

// ============================================================================
// Simultaneous equation solver
// ============================================================================

/// Solve the system `left * X = right` for `X`.
///
/// Uses Gaussian elimination with partial pivoting.
/// Returns `true` if successful, `false` if the matrix is singular.
///
/// Port of C++ `simul_eq<Size, RightCols>::solve`.
#[allow(clippy::needless_range_loop)]
pub fn simul_eq_solve<const SIZE: usize, const RIGHT_COLS: usize>(
    left: &[[f64; SIZE]; SIZE],
    right: &[[f64; RIGHT_COLS]; SIZE],
    result: &mut [[f64; RIGHT_COLS]; SIZE],
) -> bool {
    // Build augmented matrix [left | right] using Vec (const generic arithmetic
    // not supported in stable Rust)
    let cols = SIZE + RIGHT_COLS;
    let mut tmp = vec![vec![0.0_f64; cols]; SIZE];

    for i in 0..SIZE {
        for j in 0..SIZE {
            tmp[i][j] = left[i][j];
        }
        for j in 0..RIGHT_COLS {
            tmp[i][SIZE + j] = right[i][j];
        }
    }

    // Forward elimination with partial pivoting
    for k in 0..SIZE {
        let mut pivot_row = k;
        let mut max_val = -1.0_f64;
        for i in k..SIZE {
            let tmp_val = tmp[i][k].abs();
            if tmp_val > max_val && tmp_val != 0.0 {
                max_val = tmp_val;
                pivot_row = i;
            }
        }
        if tmp[pivot_row][k] == 0.0 {
            return false; // Singular
        }
        if pivot_row != k {
            tmp.swap(pivot_row, k);
        }

        let a1 = tmp[k][k];
        for j in k..cols {
            tmp[k][j] /= a1;
        }

        for i in (k + 1)..SIZE {
            let a1 = tmp[i][k];
            for j in k..cols {
                tmp[i][j] -= a1 * tmp[k][j];
            }
        }
    }

    // Back substitution
    for k in 0..RIGHT_COLS {
        for m in (0..SIZE).rev() {
            result[m][k] = tmp[m][SIZE + k];
            for j in (m + 1)..SIZE {
                result[m][k] -= tmp[m][j] * result[j][k];
            }
        }
    }

    true
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_system() {
        // I * x = b => x = b
        let left = [[1.0, 0.0], [0.0, 1.0]];
        let right = [[3.0], [7.0]];
        let mut result = [[0.0]; 2];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 3.0).abs() < 1e-10);
        assert!((result[1][0] - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_2x2_system() {
        // 2x + y = 5
        // x + 3y = 10
        // Solution: x = 1, y = 3
        let left = [[2.0, 1.0], [1.0, 3.0]];
        let right = [[5.0], [10.0]];
        let mut result = [[0.0]; 2];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 1.0).abs() < 1e-10);
        assert!((result[1][0] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_3x3_system() {
        // x + y + z = 6
        // 2x + y - z = 1
        // x - y + z = 2
        // Solution: x = 1, y = 2, z = 3
        let left = [[1.0, 1.0, 1.0], [2.0, 1.0, -1.0], [1.0, -1.0, 1.0]];
        let right = [[6.0], [1.0], [2.0]];
        let mut result = [[0.0]; 3];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 1.0).abs() < 1e-10);
        assert!((result[1][0] - 2.0).abs() < 1e-10);
        assert!((result[2][0] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_singular_matrix() {
        // Singular: rows are linearly dependent
        let left = [[1.0, 2.0], [2.0, 4.0]];
        let right = [[3.0], [6.0]];
        let mut result = [[0.0]; 2];
        assert!(!simul_eq_solve(&left, &right, &mut result));
    }

    #[test]
    fn test_multiple_right_columns() {
        // Solve for two RHS vectors simultaneously
        let left = [[1.0, 0.0], [0.0, 1.0]];
        let right = [[1.0, 2.0], [3.0, 4.0]];
        let mut result = [[0.0; 2]; 2];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 1.0).abs() < 1e-10);
        assert!((result[0][1] - 2.0).abs() < 1e-10);
        assert!((result[1][0] - 3.0).abs() < 1e-10);
        assert!((result[1][1] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_4x4_system() {
        // 4x4 system (used by parl_to_parl in trans_affine)
        let left = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let right = [[5.0, 6.0], [7.0, 8.0], [9.0, 10.0], [11.0, 12.0]];
        let mut result = [[0.0; 2]; 4];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 5.0).abs() < 1e-10);
        assert!((result[3][1] - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_needs_pivoting() {
        // First row has zero in pivot position, requires row swap
        let left = [[0.0, 1.0], [1.0, 0.0]];
        let right = [[3.0], [5.0]];
        let mut result = [[0.0]; 2];
        assert!(simul_eq_solve(&left, &right, &mut result));
        assert!((result[0][0] - 5.0).abs() < 1e-10);
        assert!((result[1][0] - 3.0).abs() < 1e-10);
    }
}

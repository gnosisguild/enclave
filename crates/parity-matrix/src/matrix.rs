// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::errors::{ConstraintError, MatrixError, ParityMatrixError, ParityMatrixResult};
use crate::math::mod_inverse;
use crate::math::mod_pow;
use num_bigint::BigUint;
use num_traits::{One, Zero};

/// Configuration for parity matrix generation
#[derive(Debug, Clone)]
pub struct ParityMatrixConfig {
    /// Modulus q
    pub q: BigUint,
    /// Degree t of the polynomial
    pub t: usize,
    /// Number of points n
    pub n: usize,
}

/// Build the generator matrix G of size (t+1) × (n+1)
/// G[i][j] = j^i mod q
/// Each row corresponds to evaluations of x^i at points 0, 1, ..., n
/// For polynomials of degree t, we have t+1 coefficients (a_0, ..., a_t)
pub fn build_generator_matrix(config: ParityMatrixConfig) -> ParityMatrixResult<Vec<Vec<BigUint>>> {
    // Check constraint: t ≤ (n-1)/2
    let max_t = config.n.saturating_sub(1) / 2;
    if config.t > max_t {
        return Err(ParityMatrixError::from(ConstraintError::DegreeConstraint {
            t: config.t,
            n: config.n,
            max_t,
        }));
    }

    let num_coeffs = config.t + 1; // degree t polynomial has t+1 coefficients
    let mut g = vec![vec![BigUint::zero(); config.n + 1]; num_coeffs];

    for (i, row) in g.iter_mut().enumerate().take(num_coeffs) {
        for (j, cell) in row.iter_mut().enumerate().take(config.n + 1) {
            *cell = mod_pow(&BigUint::from(j), i, &config.q);
        }
    }

    Ok(g)
}

/// Compute the null space of a matrix over Z_q using Gaussian elimination
/// Returns a basis for the null space (each vector is a row)
pub fn null_space(matrix: &[Vec<BigUint>], q: &BigUint) -> ParityMatrixResult<Vec<Vec<BigUint>>> {
    if matrix.is_empty() {
        return Ok(vec![]);
    }

    let rows = matrix.len();
    let cols = matrix[0].len();

    // Create augmented matrix [A | I] to track column operations
    // We'll do row reduction on A^T to find the null space
    let mut aug: Vec<Vec<BigUint>> = vec![vec![BigUint::zero(); rows + cols]; cols];

    // Initialize: transpose of original matrix in left part, identity in right part
    for (i, aug_row) in aug.iter_mut().enumerate().take(cols) {
        for (j, aug_cell) in aug_row.iter_mut().enumerate().take(rows) {
            *aug_cell = matrix[j][i].clone();
        }
        aug_row[rows + i] = BigUint::one();
    }

    // Gaussian elimination with partial pivoting
    let mut pivot_row = 0;

    for col in 0..rows {
        // Find pivot in current column
        let mut found = false;
        for row in pivot_row..cols {
            if !aug[row][col].is_zero() {
                // Swap rows
                aug.swap(pivot_row, row);
                found = true;
                break;
            }
        }

        if !found {
            continue;
        }

        // Make pivot = 1
        let inv = mod_inverse(&aug[pivot_row][col], q)?;
        for j in 0..aug[pivot_row].len() {
            aug[pivot_row][j] = (&aug[pivot_row][j] * &inv) % q;
        }

        // Eliminate other entries in this column
        for row in 0..cols {
            if row != pivot_row && !aug[row][col].is_zero() {
                let factor = aug[row][col].clone();
                for j in 0..aug[row].len() {
                    let subtract = (&factor * &aug[pivot_row][j]) % q;
                    aug[row][j] = (q + &aug[row][j] - subtract) % q;
                }
            }
        }

        pivot_row += 1;
        if pivot_row >= cols {
            break;
        }
    }

    // The null space basis vectors come from rows that are zero in the left part
    let mut null_basis = Vec::new();

    for row in aug.iter().take(cols) {
        let mut is_zero_left = true;
        for col_val in row.iter().take(rows) {
            if !col_val.is_zero() {
                is_zero_left = false;
                break;
            }
        }

        if is_zero_left {
            // Extract the right part (identity portion) as a null space vector
            let null_vec: Vec<BigUint> = row[rows..].to_vec();
            // Check it's not all zeros
            if null_vec.iter().any(|x| !x.is_zero()) {
                null_basis.push(null_vec);
            }
        }
    }

    Ok(null_basis)
}

/// Verify that H * G^T = 0 (mod q)
pub fn verify_parity_matrix(
    matrix: &[Vec<BigUint>],
    h: &[Vec<BigUint>],
    q: &BigUint,
) -> ParityMatrixResult<bool> {
    if h.is_empty() || matrix.is_empty() {
        return Ok(true);
    }

    // Validate dimensions
    if matrix.is_empty() {
        return Err(ParityMatrixError::from(MatrixError::EmptyMatrix));
    }

    let h_rows = h.len();
    let g_rows = matrix.len();
    let n_plus_1 = h[0].len();

    // Compute H * G^T
    for (i, h_row) in h.iter().enumerate().take(h_rows) {
        for (j, matrix_row) in matrix.iter().enumerate().take(g_rows) {
            let mut sum = BigUint::zero();
            for k in 0..n_plus_1 {
                sum = (sum + &h_row[k] * &matrix_row[k]) % q;
            }
            if !sum.is_zero() {
                return Err(ParityMatrixError::verification(format!(
                    "H · G^T ≠ 0 (mod q): entry at position ({}, {}) is {}",
                    i, j, sum
                )));
            }
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::math::mod_pow;
    use crate::matrix::{
        build_generator_matrix, null_space, verify_parity_matrix, ParityMatrixConfig,
    };
    use num_bigint::BigUint;
    use num_traits::One;
    use num_traits::Zero;
    use std::str::FromStr;

    /// Helper: evaluate a polynomial with given coefficients at point x mod q
    fn eval_poly(coeffs: &[BigUint], x: &BigUint, q: &BigUint) -> BigUint {
        let mut val = BigUint::zero();
        for (i, coeff) in coeffs.iter().enumerate() {
            val = (val + coeff * mod_pow(x, i, q)) % q;
        }
        val
    }

    /// Helper: compute H * v mod q
    fn matrix_vector_mult(h: &[Vec<BigUint>], v: &[BigUint], q: &BigUint) -> Vec<BigUint> {
        h.iter()
            .map(|row| {
                row.iter()
                    .zip(v.iter())
                    .fold(BigUint::zero(), |acc, (h_val, v_val)| {
                        (acc + h_val * v_val) % q
                    })
            })
            .collect()
    }

    /// Test that H * G^T = 0 and that polynomial evaluations are in the null space of H
    /// t is the degree of polynomial F, so we have t+1 coefficients
    fn test_parity_matrix_for_params(q: BigUint, n: usize, t: usize) {
        let max_t = (n.saturating_sub(1)) / 2;
        assert!(
            t <= max_t,
            "t ({}) must be <= (n-1)/2 = {} for n = {}",
            t,
            max_t,
            n
        );

        let num_coeffs = t + 1; // degree t polynomial has t+1 coefficients
        let g = build_generator_matrix(ParityMatrixConfig { q: q.clone(), n, t }).unwrap();
        let h = null_space(&g, &q).unwrap();

        // Check dimensions
        assert_eq!(g.len(), num_coeffs, "G should have t+1 rows");
        assert_eq!(g[0].len(), n + 1, "G should have n+1 columns");

        // Note: When n+1 > q, evaluation points repeat mod q, so the rank of G
        // may be less than t+1. We only check exact dimensions when n+1 <= q.
        let n_plus_1 = BigUint::from(n + 1);
        if num_coeffs < n + 1 && n_plus_1 <= q {
            assert_eq!(
                h.len(),
                n + 1 - num_coeffs,
                "H should have n-t rows for q={}, n={}, t={}",
                q,
                n,
                t
            );
            assert_eq!(h[0].len(), n + 1, "H should have n+1 columns");
        } else if !h.is_empty() {
            assert_eq!(h[0].len(), n + 1, "H should have n+1 columns");
        }

        // Verify H * G^T = 0
        assert!(
            verify_parity_matrix(&g, &h, &q).unwrap(),
            "H * G^T should be zero for q={}, n={}, t={}",
            q,
            n,
            t
        );

        // Test with multiple polynomial evaluations (degree t, so t+1 coefficients)
        let test_polys: Vec<Vec<BigUint>> = vec![
            (0..num_coeffs).map(|i| BigUint::from(i + 1) % &q).collect(), // [1, 2, 3, ...]
            (0..num_coeffs).map(|i| BigUint::from(i * 2) % &q).collect(), // [0, 2, 4, ...]
            (0..num_coeffs).map(|_| BigUint::one()).collect(),            // [1, 1, 1, ...]
            std::iter::once(BigUint::one())
                .chain(std::iter::repeat(BigUint::zero()).take(t))
                .collect(), // [1, 0, 0, ...]
        ];

        for coeffs in test_polys.iter().filter(|c| c.len() == num_coeffs) {
            // Evaluate polynomial at all points
            let eval_vec: Vec<BigUint> = (0..=n)
                .map(|j| eval_poly(coeffs, &BigUint::from(j), &q))
                .collect();

            // Check H * eval_vec = 0
            if !h.is_empty() {
                let result = matrix_vector_mult(&h, &eval_vec, &q);
                assert!(
                    result.iter().all(|x| x.is_zero()),
                    "H * v should be 0 for polynomial {:?} with q={}, n={}, t={}",
                    coeffs,
                    q,
                    n,
                    t
                );
            }
        }
    }

    // ==================== Tests with constraint t ≤ (n-1)/2 ====================
    // For t to be valid: t ≤ (n-1)/2, i.e., 2t+1 ≤ n
    // n=3: t ≤ 1, n=5: t ≤ 2, n=7: t ≤ 3, n=9: t ≤ 4, n=11: t ≤ 5, etc.

    // ==================== Small prime q = 2 ====================

    #[test]
    fn test_q2_n3_t0() {
        // n=3, max t = (3-1)/2 = 1, testing t=0 (constants)
        test_parity_matrix_for_params(BigUint::from(2u32), 3, 0);
    }

    #[test]
    fn test_q2_n3_t1() {
        // n=3, max t = 1 (linear polynomials)
        test_parity_matrix_for_params(BigUint::from(2u32), 3, 1);
    }

    #[test]
    fn test_q2_n5_t2() {
        // n=5, max t = 2 (quadratic)
        test_parity_matrix_for_params(BigUint::from(2u32), 5, 2);
    }

    // ==================== Small prime q = 3 ====================

    #[test]
    fn test_q3_n3_t1() {
        test_parity_matrix_for_params(BigUint::from(3u32), 3, 1);
    }

    #[test]
    fn test_q3_n5_t2() {
        test_parity_matrix_for_params(BigUint::from(3u32), 5, 2);
    }

    #[test]
    fn test_q3_n7_t3() {
        test_parity_matrix_for_params(BigUint::from(3u32), 7, 3);
    }

    // ==================== Prime q = 5 ====================

    #[test]
    fn test_q5_n5_t2() {
        // n=5, max t = 2
        test_parity_matrix_for_params(BigUint::from(5u32), 5, 2);
    }

    #[test]
    fn test_q5_n7_t3() {
        // n=7, max t = 3
        test_parity_matrix_for_params(BigUint::from(5u32), 7, 3);
    }

    #[test]
    fn test_q5_n9_t4() {
        // n=9, max t = 4
        test_parity_matrix_for_params(BigUint::from(5u32), 9, 4);
    }

    // ==================== Prime q = 7 (default) ====================

    #[test]
    fn test_q7_n5_t2_default() {
        // Default parameters: n=5, t=2
        test_parity_matrix_for_params(BigUint::from(7u32), 5, 2);
    }

    #[test]
    fn test_q7_n7_t3() {
        test_parity_matrix_for_params(BigUint::from(7u32), 7, 3);
    }

    #[test]
    fn test_q7_n9_t4() {
        test_parity_matrix_for_params(BigUint::from(7u32), 9, 4);
    }

    #[test]
    fn test_q7_n11_t5() {
        test_parity_matrix_for_params(BigUint::from(7u32), 11, 5);
    }

    // ==================== Prime q = 11 ====================

    #[test]
    fn test_q11_n5_t2() {
        test_parity_matrix_for_params(BigUint::from(11u32), 5, 2);
    }

    #[test]
    fn test_q11_n9_t4() {
        test_parity_matrix_for_params(BigUint::from(11u32), 9, 4);
    }

    #[test]
    fn test_q11_n11_t5() {
        test_parity_matrix_for_params(BigUint::from(11u32), 11, 5);
    }

    #[test]
    fn test_q11_n15_t7() {
        test_parity_matrix_for_params(BigUint::from(11u32), 15, 7);
    }

    // ==================== Prime q = 13 ====================

    #[test]
    fn test_q13_n7_t3() {
        test_parity_matrix_for_params(BigUint::from(13u32), 7, 3);
    }

    #[test]
    fn test_q13_n11_t5() {
        test_parity_matrix_for_params(BigUint::from(13u32), 11, 5);
    }

    #[test]
    fn test_q13_n13_t6() {
        test_parity_matrix_for_params(BigUint::from(13u32), 13, 6);
    }

    // ==================== Larger prime q = 17 ====================

    #[test]
    fn test_q17_n9_t4() {
        test_parity_matrix_for_params(BigUint::from(17u32), 9, 4);
    }

    #[test]
    fn test_q17_n13_t6() {
        test_parity_matrix_for_params(BigUint::from(17u32), 13, 6);
    }

    #[test]
    fn test_q17_n17_t8() {
        test_parity_matrix_for_params(BigUint::from(17u32), 17, 8);
    }

    // ==================== Larger prime q = 23 ====================

    #[test]
    fn test_q23_n11_t5() {
        test_parity_matrix_for_params(BigUint::from(23u32), 11, 5);
    }

    #[test]
    fn test_q23_n17_t8() {
        test_parity_matrix_for_params(BigUint::from(23u32), 17, 8);
    }

    #[test]
    fn test_q23_n21_t10() {
        test_parity_matrix_for_params(BigUint::from(23u32), 21, 10);
    }

    // ==================== Large prime q = 101 ====================

    #[test]
    fn test_q101_n11_t5() {
        test_parity_matrix_for_params(BigUint::from(101u32), 11, 5);
    }

    #[test]
    fn test_q101_n21_t10() {
        test_parity_matrix_for_params(BigUint::from(101u32), 21, 10);
    }

    #[test]
    fn test_q101_n51_t25() {
        // n=51, max t = 25
        test_parity_matrix_for_params(BigUint::from(101u32), 51, 25);
    }

    // ==================== Very large primes ====================

    #[test]
    fn test_large_prime_64bit() {
        // A 64-bit prime, n=7, t=3
        let q = BigUint::from(18446744073709551557u64); // 2^64 - 59 (prime)
        test_parity_matrix_for_params(q, 7, 3);
    }

    #[test]
    fn test_large_prime_128bit() {
        // A 128-bit prime: 2^127 - 1 (Mersenne prime), n=9, t=4
        let q = BigUint::from_str("170141183460469231731687303715884105727").unwrap();
        test_parity_matrix_for_params(q, 9, 4);
    }

    #[test]
    fn test_large_prime_256bit() {
        // A 256-bit prime (BN254 prime), n=11, t=5
        let q = BigUint::from_str(
            "21888242871839275222246405745257275088548364400416034343698204186575808495617",
        )
        .unwrap();
        test_parity_matrix_for_params(q, 11, 5);
    }

    // ==================== Edge cases ====================

    #[test]
    fn test_t_equals_0() {
        // Degree 0 polynomials (constants), n=3
        test_parity_matrix_for_params(BigUint::from(7u32), 3, 0);
    }

    #[test]
    fn test_t_max_for_n5() {
        // n=5, max t = (5-1)/2 = 2
        test_parity_matrix_for_params(BigUint::from(11u32), 5, 2);
    }

    #[test]
    fn test_t_max_for_n7() {
        // n=7, max t = (7-1)/2 = 3
        test_parity_matrix_for_params(BigUint::from(11u32), 7, 3);
    }

    #[test]
    fn test_small_n3_t1() {
        // Minimal meaningful case: n=3, t=1 (linear)
        test_parity_matrix_for_params(BigUint::from(5u32), 3, 1);
    }

    // ==================== Stress test with various parameter combinations ====================

    #[test]
    fn test_many_combinations() {
        let primes: Vec<BigUint> = vec![2u32, 3, 5, 7, 11, 13]
            .into_iter()
            .map(BigUint::from)
            .collect();

        for q in &primes {
            // n must be at least 3 for t=1 (since t ≤ (n-1)/2 means n ≥ 2t+1)
            for n in 3..=15 {
                let max_t = (n - 1) / 2;
                for t in 0..=max_t {
                    test_parity_matrix_for_params(q.clone(), n, t);
                }
            }
        }
    }

    // ==================== Tests for higher degree polynomials (should NOT be in null space) ====================

    /// Test that polynomials of degree > t are NOT in the null space of H
    fn test_higher_degree_not_in_nullspace(q: BigUint, n: usize, t: usize, higher_degree: usize) {
        assert!(higher_degree > t, "higher_degree must be > t");
        assert!(
            higher_degree <= n,
            "higher_degree must be <= n for meaningful test"
        );

        let g = build_generator_matrix(ParityMatrixConfig { q: q.clone(), n, t }).unwrap();
        let h = null_space(&g, &q).unwrap();

        if h.is_empty() {
            return; // No parity checks, skip
        }

        // Create a polynomial of degree `higher_degree` (has higher_degree+1 coefficients)
        // Make sure the leading coefficient is non-zero
        let num_coeffs = higher_degree + 1;
        let coeffs: Vec<BigUint> = (1..=num_coeffs).map(|x| BigUint::from(x) % &q).collect();

        // Ensure leading coefficient is non-zero (it should be (higher_degree+1) % q)
        assert!(
            !coeffs[higher_degree].is_zero(),
            "Leading coefficient must be non-zero"
        );

        // Evaluate polynomial at all points
        let eval_vec: Vec<BigUint> = (0..=n)
            .map(|j| eval_poly(&coeffs, &BigUint::from(j), &q))
            .collect();

        // Check H * eval_vec ≠ 0
        let result = matrix_vector_mult(&h, &eval_vec, &q);
        assert!(
            !result.iter().all(|x| x.is_zero()),
            "H * v should NOT be 0 for degree-{} polynomial with q={}, n={}, t={}\ncoeffs={:?}\nresult={:?}",
            higher_degree,
            q,
            n,
            t,
            coeffs,
            result
        );
    }

    #[test]
    fn test_degree_t_plus_1_not_in_nullspace() {
        // For various (q, n, t), test that degree t+1 polynomial is not in null space
        let q = BigUint::from(101u32);
        let n = 11;
        let t = 5; // max degree allowed

        // Test with degree t+1
        test_higher_degree_not_in_nullspace(q.clone(), n, t, t + 1);
    }

    #[test]
    fn test_degree_t_plus_2_not_in_nullspace() {
        let q = BigUint::from(101u32);
        let n = 11;
        let t = 4;

        // Test with degree t+2
        test_higher_degree_not_in_nullspace(q.clone(), n, t, t + 2);
    }

    #[test]
    fn test_degree_n_not_in_nullspace() {
        // Test polynomial of degree n (maximum possible given n+1 points)
        let q = BigUint::from(101u32);
        let n = 11;
        let t = 3;

        test_higher_degree_not_in_nullspace(q.clone(), n, t, n);
    }

    #[test]
    fn test_higher_degree_various_params() {
        // Test multiple parameter combinations
        let test_cases = vec![
            (BigUint::from(7u32), 7, 2, 3),   // q=7, n=7, t=2, test degree 3
            (BigUint::from(7u32), 7, 2, 4),   // q=7, n=7, t=2, test degree 4
            (BigUint::from(7u32), 7, 3, 4),   // q=7, n=7, t=3, test degree 4
            (BigUint::from(11u32), 9, 3, 4),  // q=11, n=9, t=3, test degree 4
            (BigUint::from(11u32), 9, 3, 5),  // q=11, n=9, t=3, test degree 5
            (BigUint::from(11u32), 9, 4, 5),  // q=11, n=9, t=4, test degree 5
            (BigUint::from(13u32), 11, 4, 5), // q=13, n=11, t=4, test degree 5
            (BigUint::from(13u32), 11, 4, 6), // q=13, n=11, t=4, test degree 6
            (BigUint::from(13u32), 11, 5, 6), // q=13, n=11, t=5, test degree 6
            (BigUint::from(17u32), 13, 5, 6), // q=17, n=13, t=5, test degree 6
            (BigUint::from(17u32), 13, 5, 7), // q=17, n=13, t=5, test degree 7
            (BigUint::from(17u32), 13, 6, 7), // q=17, n=13, t=6, test degree 7
        ];

        for (q, n, t, higher_deg) in test_cases {
            test_higher_degree_not_in_nullspace(q, n, t, higher_deg);
        }
    }

    #[test]
    fn test_higher_degree_with_random_coeffs() {
        // Test with various non-sequential coefficient patterns
        let q = BigUint::from(101u32);
        let n = 11;
        let t = 4;
        let higher_degree = 6;

        let g = build_generator_matrix(ParityMatrixConfig { q: q.clone(), n, t }).unwrap();
        let h = null_space(&g, &q).unwrap();

        // Test with different coefficient patterns for degree-6 polynomial
        let test_coeffs: Vec<Vec<BigUint>> = vec![
            // Pattern 1: [0, 0, 0, 0, 0, 0, 1] - pure x^6
            {
                let mut c = vec![BigUint::zero(); higher_degree];
                c.push(BigUint::one());
                c
            },
            // Pattern 2: [1, 0, 0, 0, 0, 0, 1] - 1 + x^6
            {
                let mut c = vec![BigUint::zero(); higher_degree + 1];
                c[0] = BigUint::one();
                c[higher_degree] = BigUint::one();
                c
            },
            // Pattern 3: [5, 3, 7, 2, 9, 4, 8] - random-ish
            vec![5u32, 3, 7, 2, 9, 4, 8]
                .into_iter()
                .map(BigUint::from)
                .collect(),
            // Pattern 4: [99, 50, 25, 12, 6, 3, 1] - decreasing
            vec![99u32, 50, 25, 12, 6, 3, 1]
                .into_iter()
                .map(BigUint::from)
                .collect(),
        ];

        for coeffs in test_coeffs {
            assert_eq!(coeffs.len(), higher_degree + 1);
            assert!(
                !coeffs[higher_degree].is_zero(),
                "Leading coefficient must be non-zero"
            );

            let eval_vec: Vec<BigUint> = (0..=n)
                .map(|j| eval_poly(&coeffs, &BigUint::from(j), &q))
                .collect();

            let result = matrix_vector_mult(&h, &eval_vec, &q);
            assert!(
                !result.iter().all(|x| x.is_zero()),
                "H * v should NOT be 0 for degree-{} polynomial with coeffs {:?}",
                higher_degree,
                coeffs
            );
        }
    }

    #[test]
    fn test_just_above_threshold() {
        // For each valid t, test that degree exactly t+1 is detected
        let q = BigUint::from(101u32);

        for n in [7, 9, 11, 13, 15] {
            let max_t = (n - 1) / 2;
            for t in 0..max_t {
                // Test degree t+1 (just above threshold)
                test_higher_degree_not_in_nullspace(q.clone(), n, t, t + 1);
            }
        }
    }
}

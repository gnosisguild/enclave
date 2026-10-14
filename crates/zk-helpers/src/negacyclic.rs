// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Negacyclic folding with descending coefficients.

use e3_polynomial::Polynomial;
#[cfg(test)]
use num_bigint::BigInt;
#[cfg(test)]
use num_traits::Zero;

/// Reduces a polynomial of degree `2N-2` modulo `x^N + 1`, in O(N).
///
/// `x^N = -1` in the ring, so a coefficient at degree `e + N` folds onto degree `e` with its sign
/// flipped. Coefficients are stored **descending**, so the coefficient of `x^(N-1-j)` sits at index
/// `j`: its low part is `poly[N-1+j]` and the high part that wraps onto it is `poly[j-1]`. Index
/// `j = 0` is degree `N-1`, whose partner would be degree `2N-1` and so does not exist.
///
/// The tests compare this linear fold with independent polynomial long division.
///
/// # Panics
///
/// Panics when `poly` does not have exactly `2N-1` coefficients.
pub(crate) fn fold_negacyclic(poly: &Polynomial, n: usize) -> Polynomial {
    let c = poly.coefficients();
    assert_eq!(c.len(), 2 * n - 1, "fold_negacyclic expects degree 2(N-1)");

    let mut out = Vec::with_capacity(n);
    out.push(c[n - 1].clone());
    for j in 1..n {
        out.push(&c[n - 1 + j] - &c[j - 1]);
    }
    Polynomial::new(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "fold_negacyclic expects degree 2(N-1)")]
    fn fold_rejects_extra_coefficients() {
        fold_negacyclic(&Polynomial::new(vec![BigInt::zero(); 8]), 4);
    }

    /// Verify odd/even sizes and signed high coefficients against long division.
    #[test]
    fn fold_matches_generic_reduction() {
        for n in [2usize, 3, 4, 8, 17] {
            // Deterministic but sign-varying coefficients across the whole degree-2(N-1) range.
            let coefficients: Vec<BigInt> = (0..2 * n - 1)
                .map(|k| {
                    let magnitude = BigInt::from((k * 37 + 11) as i64);
                    if k % 3 == 0 {
                        -magnitude
                    } else {
                        magnitude
                    }
                })
                .collect();
            let poly = Polynomial::new(coefficients);

            let fast = fold_negacyclic(&poly, n);
            let generic = reduce_reference(&poly, n);
            assert_eq!(
                fast.coefficients(),
                generic.coefficients(),
                "fold disagrees with long division at N = {n}"
            );
        }

        let n = 4;
        // Degree 2(N-1) = 6, with everything above degree N-1 = 3 set to zero. Descending order
        // puts the low half last, so the leading N-1 entries are the high half.
        let poly = Polynomial::new(vec![
            BigInt::from(0),
            BigInt::from(0),
            BigInt::from(0),
            BigInt::from(7),
            BigInt::from(-5),
            BigInt::from(3),
            BigInt::from(-1),
        ]);
        let folded = fold_negacyclic(&poly, n);
        assert_eq!(
            folded.coefficients(),
            &[
                BigInt::from(7),
                BigInt::from(-5),
                BigInt::from(3),
                BigInt::from(-1)
            ]
        );
        assert_eq!(folded, reduce_reference(&poly, n));
    }

    /// Independent long division by the monic polynomial X^N + 1.
    fn reduce_reference(poly: &Polynomial, n: usize) -> Polynomial {
        let mut divisor = vec![BigInt::zero(); n + 1];
        divisor[0] = BigInt::from(1);
        divisor[n] = BigInt::from(1);
        let mut remainder = poly.coefficients().to_vec();
        for i in 0..remainder.len().saturating_sub(n) {
            let leading = remainder[i].clone();
            for (j, coefficient) in divisor.iter().enumerate() {
                remainder[i + j] -= &leading * coefficient;
            }
        }
        let mut coefficients = vec![BigInt::zero(); n];
        let tail_len = remainder.len().min(n);
        coefficients[n - tail_len..].clone_from_slice(&remainder[remainder.len() - tail_len..]);
        Polynomial::new(coefficients)
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! CRT (Chinese Remainder Theorem) polynomial representation.

use crate::polynomial::Polynomial;
use crate::reduce_and_center_coefficients_mut;
use std::sync::Arc;
use thiserror::Error;

use fhe_math::rq::{Poly, Representation};
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Errors that can occur during CRT polynomial operations.
#[derive(Debug, Error)]
pub enum CrtPolynomialError {
    /// Moduli list must not be empty.
    #[error("moduli must be non-empty")]
    EmptyModuli,

    /// Ring degree `n` must be non-zero.
    #[error("n must be > 0")]
    InvalidN,

    /// Number of limbs must match number of moduli.
    #[error("limbs.len() ({actual}) must match ctx.moduli.len() ({expected})")]
    LimbCountMismatch { expected: usize, actual: usize },

    /// Each limb must have exactly `n` coefficients.
    #[error("limb {limb_index} length ({actual}) must match ctx.n ({expected})")]
    LimbLengthMismatch {
        limb_index: usize,
        expected: usize,
        actual: usize,
    },

    /// fhe-math Poly must be in PowerBasis representation for conversion.
    #[error("fhe Poly must be in PowerBasis representation")]
    UnsupportedRepresentation,
}

/// CRT context for a family of polynomials sharing the same ring degree `n` and CRT moduli.
///
/// Notes:
/// - `n` is the ring degree (also the number of coefficients / NTT size). Each limb has exactly `n` coefficients.
/// - The maximum exponent is `n - 1`.
/// - `moduli` are the CRT factors q_0..q_{L-1} (u64 primes).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CrtContext {
    pub n: usize,
    pub moduli: Vec<u64>,
}

impl CrtContext {
    /// Creates a new CRT context.
    ///
    /// # Errors
    /// Returns `CrtPolynomialError::InvalidN` if `n` is zero.
    /// Returns `CrtPolynomialError::EmptyModuli` if `moduli` is empty.
    pub fn new(n: usize, moduli: Vec<u64>) -> Result<Self, CrtPolynomialError> {
        if n == 0 {
            return Err(CrtPolynomialError::InvalidN);
        }
        if moduli.is_empty() {
            return Err(CrtPolynomialError::EmptyModuli);
        }
        Ok(Self { n, moduli })
    }

    pub fn num_moduli(&self) -> usize {
        self.moduli.len()
    }
}

/// A polynomial in CRT form: one limb polynomial per modulus.
///
/// Each limb is a `Polynomial` whose coefficients are expected to be reduced/centered
/// modulo the corresponding `ctx.moduli[i]` as required by the caller.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CrtPolynomial {
    pub ctx: Arc<CrtContext>,
    pub limbs: Vec<Polynomial>,
}

impl CrtPolynomial {
    /// Creates a new CRT polynomial and validates limb dimensions.
    ///
    /// # Errors
    /// Returns `CrtPolynomialError::LimbCountMismatch` if the number of limbs does not match `ctx.moduli.len()`.
    /// Returns `CrtPolynomialError::LimbLengthMismatch` if any limb has a coefficient vector length different from `ctx.n`.
    pub fn new(ctx: Arc<CrtContext>, limbs: Vec<Polynomial>) -> Result<Self, CrtPolynomialError> {
        let expected_limbs = ctx.moduli.len();

        if limbs.len() != expected_limbs {
            return Err(CrtPolynomialError::LimbCountMismatch {
                expected: expected_limbs,
                actual: limbs.len(),
            });
        }

        let expected_len = ctx.n;

        for (i, limb) in limbs.iter().enumerate() {
            let actual_len = limb.coefficients().len();
            if actual_len != expected_len {
                return Err(CrtPolynomialError::LimbLengthMismatch {
                    limb_index: i,
                    expected: expected_len,
                    actual: actual_len,
                });
            }
        }

        Ok(Self { ctx, limbs })
    }

    /// Ring degree (number of coefficients).
    pub fn n(&self) -> usize {
        self.ctx.n
    }

    /// Maximum exponent (at most `n - 1`).
    pub fn max_exponent(&self) -> usize {
        self.ctx.n.saturating_sub(1)
    }

    pub fn modulus(&self, i: usize) -> u64 {
        self.ctx.moduli[i]
    }

    pub fn limb(&self, i: usize) -> &Polynomial {
        &self.limbs[i]
    }

    /// Builds a `CrtPolynomial` from an fhe-math `Poly` in PowerBasis representation.
    ///
    /// Main use: preparing inputs for ZK circuits by converting from FHE BFV ciphertext
    /// polynomials to a CRT limb format compatible with the circuits.
    ///
    /// Coefficient layout: fhe-math rows are ascending degree (c_0 + c_1·x + …).
    /// We convert to descending order so that evaluation in the circuit matches
    /// Horner's method in a single forward pass: P(x) = ((...((a_n * x + a_{n-1}) * x + ...) * x + a_0).
    /// The circuit can then iterate `result = result * x + coefficients[i]` from i = 0 without
    /// reversing or reindexing, keeping the constraint system simple and efficient.
    ///
    /// # Errors
    /// Returns `CrtPolynomialError::UnsupportedRepresentation` if the poly is not in PowerBasis.
    pub fn from_fhe_poly(p: &Poly) -> Result<Self, CrtPolynomialError> {
        let mut p = p.clone();

        if *p.representation() == Representation::Ntt {
            p.change_representation(Representation::PowerBasis);
        }

        let ctx = p.ctx.as_ref();
        let n = ctx.degree;
        let moduli = ctx.moduli().to_vec();
        let crt_ctx = Arc::new(CrtContext::new(n, moduli)?);

        let coeffs = p.coefficients();

        let limbs: Vec<Polynomial> = coeffs
            .outer_iter()
            .zip(crt_ctx.moduli.iter())
            .map(|(row, qi)| {
                let mut coeffs: Vec<BigInt> = row.iter().rev().map(|&c| BigInt::from(c)).collect();
                let qi_bigint = BigInt::from(*qi);

                reduce_and_center_coefficients_mut(&mut coeffs, &qi_bigint);

                Polynomial::new(coeffs)
            })
            .collect();

        CrtPolynomial::new(crt_ctx, limbs)
    }
}

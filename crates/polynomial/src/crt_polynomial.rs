// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! CRT (Chinese Remainder Theorem) polynomial representation.

use crate::polynomial::Polynomial;
use fhe_math::rq::{Poly, Representation};
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during CRT polynomial operations.
#[derive(Debug, Error)]
pub enum CrtPolynomialError {
    /// Moduli slice length does not match number of limbs.
    #[error("moduli length ({moduli_len}) must match limbs length ({limbs_len})")]
    ModuliLengthMismatch { limbs_len: usize, moduli_len: usize },
}

/// A polynomial in CRT form: one limb polynomial per modulus.
///
/// Each limb is a `Polynomial` whose coefficients are expected to be reduced/centered
/// modulo the corresponding `ctx.moduli[i]` as required by the caller.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CrtPolynomial {
    pub limbs: Vec<Polynomial>,
}

impl CrtPolynomial {
    /// Builds a `CrtPolynomial` from a vector of polynomials.
    ///
    /// # Arguments
    ///
    /// * `limbs` - Vector of polynomials.
    pub fn new(limbs: Vec<Polynomial>) -> Self {
        Self { limbs }
    }

    /// Builds a `CrtPolynomial` from coefficient vectors (one `Vec<BigInt>` per modulus).
    ///
    /// # Arguments
    ///
    /// * `limbs` - Vector of coefficient vectors.
    pub fn from_bigint_vectors(limbs: Vec<Vec<BigInt>>) -> Self {
        let limbs = limbs.into_iter().map(Polynomial::new).collect::<Vec<_>>();

        Self { limbs }
    }

    /// Builds a `CrtPolynomial` from an fhe-math `Poly` in PowerBasis representation.
    ///
    /// Used to prepare inputs for ZK circuits by converting FHE BFV ciphertext polynomials
    /// into CRT limb format. If `p` is in NTT form, it is converted to PowerBasis first.
    ///
    /// # Arguments
    ///
    /// * `p` - An fhe-math polynomial (PowerBasis or Ntt).
    ///
    /// # Coefficient order
    ///

    pub fn from_fhe_polynomial(p: &Poly) -> Self {
        let mut p = p.clone();

        if *p.representation() == Representation::Ntt {
            p.change_representation(Representation::PowerBasis);
        }

        let limbs = p
            .coefficients()
            .outer_iter()
            .map(|row| Polynomial::from_u64_vector(row.to_vec()))
            .collect();

        Self { limbs }
    }

    /// Reverses the coefficient order of every limb in-place.
    ///
    /// For each limb, converts between descending degree (a_n, …, a_0) and ascending
    /// degree (a_0, …, a_n). Calling this twice restores the original order.
    pub fn reverse(&mut self) {
        for limb in &mut self.limbs {
            limb.reverse();
        }
    }

    /// Reduces and centers each limb's coefficients modulo the corresponding modulus in-place.
    ///
    /// Each limb `self.limbs[i]` is reduced modulo `moduli[i]`, with coefficients centered
    /// in the symmetric range `(-q/2, q/2]`.
    ///
    /// # Arguments
    ///
    /// * `moduli` - One modulus per limb; `moduli[i]` is used for `self.limbs[i]`.
    ///
    /// # Errors
    ///
    /// Returns [`CrtPolynomialError::ModuliLengthMismatch`] if `moduli.len() != self.limbs.len()`.
    pub fn reduce_and_center(&mut self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        if self.limbs.len() != moduli.len() {
            return Err(CrtPolynomialError::ModuliLengthMismatch {
                limbs_len: self.limbs.len(),
                moduli_len: moduli.len(),
            });
        }

        for (limb, qi) in self.limbs.iter_mut().zip(moduli.iter()) {
            limb.reduce_and_center(&BigInt::from(*qi));
        }

        Ok(())
    }

    /// Returns a reference to the limb polynomial at the given index.
    ///
    /// # Arguments
    ///
    /// * `i` - Limb index; must be in range `0..self.limbs.len()`.
    ///
    /// # Returns
    ///
    /// A reference to the polynomial for modulus `i`. Coefficients are expected to be
    /// reduced/centered modulo the corresponding modulus as required by the caller.
    ///
    /// # Panics
    ///
    /// Panics if `i >= self.limbs.len()`.
    pub fn limb(&self, i: usize) -> &Polynomial {
        &self.limbs[i]
    }

    /// Returns limb coefficient vectors (one `Vec<BigInt>` per modulus).
    ///
    /// Use when you need a raw CRT representation for serialization, hashing,
    /// or APIs that expect `&[Vec<BigInt>]`. The inverse of [`from_limb_coefficients`](Self::from_limb_coefficients).
    pub fn to_limb_coefficients(&self) -> Vec<Vec<BigInt>> {
        self.limbs
            .iter()
            .map(|l| l.coefficients().to_vec())
            .collect()
    }
}

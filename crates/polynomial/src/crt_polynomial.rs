// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! CRT (Chinese Remainder Theorem) polynomial representation.

use crate::polynomial::Polynomial;
use crate::utils::reduce;
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

    /// Builds a CRT polynomial from a polynomial mod Q (Q>>128) and moduli.
    ///
    /// # Arguments
    ///
    /// * `coeffs` - Polynomial coefficients mod Q (Q>>128).
    /// * `moduli` - One modulus per limb.
    pub fn from_mod_q_polynomial(poly: &Vec<BigInt>, moduli: &[u64]) -> Self {
        let limbs: Vec<Vec<BigInt>> = moduli
            .iter()
            .map(|&qi| {
                let qi_big = BigInt::from(qi);
                poly.iter().map(|c| reduce(c, &qi_big)).collect()
            })
            .collect();
        Self::from_bigint_vectors(limbs)
    }

    /// Builds a `CrtPolynomial` from an fhe-math `Poly` in PowerBasis representation.
    ///
    /// Used to prepare inputs for ZK circuits by converting FHE BFV ciphertext polynomials
    /// into CRT limb format. If `p` is in NTT form, it is converted to PowerBasis first.
    ///
    /// # Arguments
    ///
    /// * `p` - An fhe-math polynomial (PowerBasis or Ntt).
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

    /// Centers each limb's coefficients (already in [0, q_i)) into (-q_i/2, q_i/2] in-place.
    ///
    /// # Arguments
    ///
    /// * `moduli` - One modulus per limb; `moduli[i]` is used for `self.limbs[i]`.
    ///
    /// # Errors
    ///
    /// Returns [`CrtPolynomialError::ModuliLengthMismatch`] if `moduli.len() != self.limbs.len()`.
    pub fn center(&mut self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        if self.limbs.len() != moduli.len() {
            return Err(CrtPolynomialError::ModuliLengthMismatch {
                limbs_len: self.limbs.len(),
                moduli_len: moduli.len(),
            });
        }

        for (limb, qi) in self.limbs.iter_mut().zip(moduli.iter()) {
            limb.center(&BigInt::from(*qi));
        }

        Ok(())
    }

    /// Reduces each limb's coefficients modulo the corresponding modulus in-place (range [0, qi)).
    ///
    /// # Arguments
    ///
    /// * `moduli` - One modulus per limb; `moduli[i]` is used for `self.limbs[i]`.
    ///
    /// # Errors
    ///
    /// Returns [`CrtPolynomialError::ModuliLengthMismatch`] if `moduli.len() != self.limbs.len()`.
    pub fn reduce(&mut self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        if self.limbs.len() != moduli.len() {
            return Err(CrtPolynomialError::ModuliLengthMismatch {
                limbs_len: self.limbs.len(),
                moduli_len: moduli.len(),
            });
        }

        for (limb, qi) in self.limbs.iter_mut().zip(moduli.iter()) {
            limb.reduce(&BigInt::from(*qi));
        }

        Ok(())
    }

    /// Reduces each limb's coefficients modulo the same modulus in-place.
    ///
    /// Every limb uses the same `modulus`; coefficients are reduced into the range `[0, modulus)`.
    /// Use this when all limbs should be reduced by one common modulus (e.g. a single prime)
    /// instead of per-limb moduli as in [`reduce`](Self::reduce).
    ///
    /// # Arguments
    ///
    /// * `modulus` - The modulus applied to every limb.
    pub fn reduce_uniform(&mut self, modulus: &BigInt) {
        for limb in &mut self.limbs {
            limb.reduce(&modulus);
        }
    }

    /// Multiplies each limb's coefficients by a scalar.
    ///
    /// # Arguments
    ///
    /// * `scalar` - The scalar to multiply each coefficient by.
    pub fn scalar_mul(&mut self, scalar: &BigInt) {
        for limb in &mut self.limbs {
            *limb = limb.scalar_mul(scalar);
        }
    }

    /// Adds a limb to the CRT polynomial.
    ///
    /// # Arguments
    ///
    /// * `limb` - The limb to add.
    pub fn add_limb(&mut self, limb: Polynomial) {
        self.limbs.push(limb);
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
}

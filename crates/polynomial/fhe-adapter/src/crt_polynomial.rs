// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! FHE conversions for [`CrtPolynomial`]: the limb bridge between core polynomials
//! and fhe-math RNS polynomials.

use std::sync::Arc;

use crate::fhe_poly::ToPowerBasisPoly;
use e3_polynomial::{CrtPolynomial, Polynomial};
use fhe_math::rq::traits::TryConvertFrom;
use fhe_math::rq::{Context, Poly, PowerBasis};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};

/// Builds core polynomials from fhe-math RNS polynomials.
///
/// The adapter owns these constructors because the core crate does not depend on
/// `fhe-math`. Import this trait to call the methods on [`CrtPolynomial`] and
/// [`Polynomial`].
pub trait FromFhePolynomial: Sized {
    /// Builds a limb-per-modulus CRT polynomial from an fhe-math `Poly` in any
    /// representation.
    ///
    /// Used to prepare inputs for ZK circuits by converting FHE BFV ciphertext
    /// polynomials into CRT limb format. If `p` is not in PowerBasis form (e.g. NTT
    /// or NttShoup), it is converted first.
    ///
    /// # Arguments
    ///
    /// * `p` - An fhe-math polynomial (any representation).
    fn from_fhe_polynomial(p: &impl ToPowerBasisPoly) -> Self;
}

impl FromFhePolynomial for CrtPolynomial {
    fn from_fhe_polynomial(p: &impl ToPowerBasisPoly) -> Self {
        let p = p.to_power_basis_poly();

        let limbs = p
            .coefficients()
            .outer_iter()
            .map(|row| Polynomial::from_u64_vector(row.to_vec()))
            .collect();

        CrtPolynomial::new(limbs)
    }
}

/// Converts core polynomials back into fhe-math RNS polynomials.
pub trait ToFhePolynomial {
    /// Builds an fhe-math `Poly<PowerBasis>` (inverse of
    /// [`FromFhePolynomial::from_fhe_polynomial`]).
    ///
    /// Coefficients are reduced to `[0, q_i)` per limb using `moduli[i]` before packing
    /// the RNS buffer. `ctx` must match the TRBFV/BFV context: `ctx.q.len()` equals limb
    /// count, and each limb has `ctx.degree` coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`fhe_math::Error`] if limb counts or coefficient lengths disagree with
    /// `ctx`, a coefficient does not fit `u64`, or `Poly::try_convert_from` fails.
    fn to_fhe_polynomial(
        &self,
        ctx: &Arc<Context>,
        moduli: &[u64],
    ) -> Result<Poly<PowerBasis>, fhe_math::Error>;
}

impl ToFhePolynomial for CrtPolynomial {
    fn to_fhe_polynomial(
        &self,
        ctx: &Arc<Context>,
        moduli: &[u64],
    ) -> Result<Poly<PowerBasis>, fhe_math::Error> {
        let degree = ctx.degree;
        let l = ctx.q.len();
        if self.limbs.len() != l {
            return Err(fhe_math::Error::InvalidCoefficientShape {
                actual_rows: self.limbs.len(),
                actual_columns: degree,
                expected_rows: l,
                expected_columns: degree,
            });
        }
        if moduli.len() != l {
            return Err(fhe_math::Error::InvalidCoefficientShape {
                actual_rows: moduli.len(),
                actual_columns: degree,
                expected_rows: l,
                expected_columns: degree,
            });
        }
        let mut data = Vec::with_capacity(l * degree);
        for (i, &qi) in moduli.iter().enumerate().take(l) {
            let coeffs = self.limb(i).coefficients();
            if coeffs.len() != degree {
                return Err(fhe_math::Error::InvalidCoefficientShape {
                    actual_rows: self.limbs.len(),
                    actual_columns: coeffs.len(),
                    expected_rows: l,
                    expected_columns: degree,
                });
            }
            for coeff in coeffs.iter().take(degree) {
                let u = bigint_to_u64_mod(coeff, qi)?;
                data.push(u);
            }
        }
        Poly::<PowerBasis>::try_convert_from(data, ctx, false)
    }
}

fn bigint_to_u64_mod(c: &BigInt, m: u64) -> Result<u64, fhe_math::Error> {
    if m < 2 {
        return Err(fhe_math::Error::InvalidModulus(m));
    }
    let bm = BigInt::from(m);
    let mut r = c % &bm;
    if r < BigInt::zero() {
        r += bm;
    }
    r.to_u64().ok_or(fhe_math::Error::InvalidModulus(m))
}

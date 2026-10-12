// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Read fhe-math polynomials as integer coefficients in native ascending order.
//! Conversion does not center coefficients, reverse them, or encode witnesses.

use e3_polynomial::{CrtPolynomial, Polynomial};
use fhe_math::rq::{Ntt, NttShoup, Poly, PowerBasis};
use num_bigint::{BigInt, BigUint};

/// Obtain coefficient form from a supported FHE representation.
pub trait ToPowerBasisPoly {
    fn to_power_basis_poly(&self) -> Poly<PowerBasis>;
}

impl ToPowerBasisPoly for Poly<PowerBasis> {
    fn to_power_basis_poly(&self) -> Poly<PowerBasis> {
        self.clone()
    }
}

impl ToPowerBasisPoly for Poly<Ntt> {
    fn to_power_basis_poly(&self) -> Poly<PowerBasis> {
        self.to_power_basis()
    }
}

impl ToPowerBasisPoly for Poly<NttShoup> {
    fn to_power_basis_poly(&self) -> Poly<PowerBasis> {
        self.to_power_basis()
    }
}

impl<T: ToPowerBasisPoly + ?Sized> ToPowerBasisPoly for &T {
    fn to_power_basis_poly(&self) -> Poly<PowerBasis> {
        (*self).to_power_basis_poly()
    }
}

/// Read native ascending coefficients as separate CRT limbs or reconstructed integers.
pub trait FromFhePolynomial: Sized {
    fn from_fhe_polynomial(poly: &impl ToPowerBasisPoly) -> Self;
}

impl FromFhePolynomial for CrtPolynomial {
    fn from_fhe_polynomial(poly: &impl ToPowerBasisPoly) -> Self {
        let poly = poly.to_power_basis_poly();
        Self::new(
            poly.coefficients()
                .outer_iter()
                .map(|row| Polynomial::new(row.iter().copied().map(BigInt::from).collect()))
                .collect(),
        )
    }
}

impl FromFhePolynomial for Polynomial {
    fn from_fhe_polynomial(poly: &impl ToPowerBasisPoly) -> Self {
        let poly = poly.to_power_basis_poly();
        let coefficients = Vec::<BigUint>::from(&poly);
        Self::new(coefficients.into_iter().map(BigInt::from).collect())
    }
}

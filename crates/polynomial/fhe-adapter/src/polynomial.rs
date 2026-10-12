// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! FHE conversions for [`Polynomial`].

use crate::fhe_poly::ToPowerBasisPoly;
use crate::FromFhePolynomial;
use e3_polynomial::Polynomial;
use num_bigint::{BigInt, BigUint, ToBigInt};

impl FromFhePolynomial for Polynomial {
    /// Creates a new polynomial from an fhe-math `Poly` (CRT-reconstructed mod Q).
    fn from_fhe_polynomial(p: &impl ToPowerBasisPoly) -> Self {
        let p = p.to_power_basis_poly();

        let coefficients: Vec<BigUint> = Vec::<BigUint>::from(&p);
        let bigints: Vec<BigInt> = coefficients
            .iter()
            .map(|c| c.to_bigint().unwrap())
            .collect();

        Polynomial::new(bigints)
    }
}

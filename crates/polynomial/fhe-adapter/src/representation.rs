// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Reversed, centered CRT conversion of fhe-math polynomials.

use crate::fhe_poly::ToPowerBasisPoly;
use crate::FromFhePolynomial;
use e3_polynomial::{CrtPolynomial, CrtPolynomialError};

/// Convert an FHE polynomial to reversed, centered CRT coefficients.
pub fn fhe_poly_to_crt_centered(
    poly: &impl ToPowerBasisPoly,
    moduli: &[u64],
) -> Result<CrtPolynomial, CrtPolynomialError> {
    let mut crt = CrtPolynomial::from_fhe_polynomial(poly);
    crt.reverse();
    crt.center(moduli)?;
    Ok(crt)
}

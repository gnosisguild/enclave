// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! CRT (Chinese Remainder) operations for ZK circuit computations.
//!
//! Used by share_encryption, user_data_encryption, pk, and other circuits that work with
//! polynomials in CRT form or need to convert FHE types to CRT limbs.

use crate::CircuitsErrors;
use e3_polynomial::{CrtPolynomial, CrtPolynomialError};
use fhe_math::rq::Poly;
use fhe_math::zq::Modulus;

/// Computes k0_i = (-t)^{-1} mod q_i for each modulus (used in Configs and bounds).
///
/// Same logic as share_encryption and user_data_encryption when building k0is.
pub fn compute_k0is(moduli: &[u64], plaintext_modulus: u64) -> Result<Vec<u64>, CircuitsErrors> {
    let mut k0is = Vec::with_capacity(moduli.len());
    for &qi in moduli {
        let m = Modulus::new(qi).map_err(|e| {
            CircuitsErrors::Sample(format!("Failed to create modulus for k0is: {:?}", e))
        })?;
        let k0qi = m.inv(m.neg(plaintext_modulus)).ok_or_else(|| {
            CircuitsErrors::Fhe(fhe::Error::MathError(fhe_math::Error::Default(
                "Failed to calculate modulus inverse for k0qi".into(),
            )))
        })?;
        k0is.push(k0qi);
    }
    Ok(k0is)
}

/// Converts an FHE polynomial to CRT form with reverse + center (no ZKP reduce).
///
/// Same pattern used by share_encryption, user_data_encryption, and pk circuits
/// when building circuit input from FHE types.
pub fn fhe_poly_to_crt_centered(
    poly: &Poly,
    moduli: &[u64],
) -> Result<CrtPolynomial, CrtPolynomialError> {
    let mut crt = CrtPolynomial::from_fhe_polynomial(poly);
    crt.reverse();
    crt.center(moduli)?;
    Ok(crt)
}

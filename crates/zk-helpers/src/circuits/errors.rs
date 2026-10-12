// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Error types for circuit and codegen operations.

use crate::utils::ZkHelpersUtilsError;
use e3_fhe_params::derived::DerivedError;
use e3_polynomial::CrtPolynomialError;
use thiserror::Error;

/// Errors that can occur during circuit codegen or artifact I/O.
#[derive(Error, Debug)]
pub enum CircuitsErrors {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("BFV error: {0}")]
    Fhe(#[from] fhe::Error),
    #[error("CRT polynomial error: {0}")]
    CrtPolynomial(#[from] CrtPolynomialError),
    #[error("ZK helper error: {0}")]
    ZkHelpers(#[from] ZkHelpersUtilsError),
    #[error("Sample error: {0}")]
    Sample(String),
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Unexpected error: {0}")]
    Other(String),
}

impl From<DerivedError> for CircuitsErrors {
    fn from(error: DerivedError) -> Self {
        match error {
            DerivedError::Fhe(error) => Self::Fhe(error),
            DerivedError::Sample(message) => Self::Sample(message),
            DerivedError::Other(message) => Self::Other(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::derived::{compute_k0is, compute_q_inverse_mod_t};
    use num_bigint::BigUint;

    #[test]
    fn derived_failures_keep_circuit_variants_and_context() {
        let cases = [
            (
                compute_q_inverse_mod_t(&BigUint::from(21u64), 7).unwrap_err(),
                "Other",
                "Unexpected error: Q and t are not coprime, gcd = 7",
            ),
            (
                compute_k0is(&[17], 17).unwrap_err(),
                "Other",
                "Unexpected error: plaintext modulus 17 must be smaller than CRT modulus 17",
            ),
            (
                compute_k0is(&[1], 0).unwrap_err(),
                "Sample",
                "Sample error: Failed to create modulus for k0is: InvalidModulus(1)",
            ),
            (
                compute_k0is(&[15], 5).unwrap_err(),
                "Fhe",
                "BFV error: Math library error: Value 10 is not invertible modulo 15.",
            ),
        ];

        for (error, expected_variant, expected_message) in cases {
            let error = CircuitsErrors::from(error);
            assert_eq!(error.to_string(), expected_message);
            let variant = match error {
                CircuitsErrors::Fhe(_) => "Fhe",
                CircuitsErrors::Sample(_) => "Sample",
                CircuitsErrors::Other(_) => "Other",
                _ => panic!("unexpected circuit error: {error:?}"),
            };
            assert_eq!(variant, expected_variant);
        }
    }
}

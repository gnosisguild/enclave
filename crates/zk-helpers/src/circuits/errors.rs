// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Error types for circuit and codegen operations.

use crate::utils::ZkHelpersUtilsError;
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

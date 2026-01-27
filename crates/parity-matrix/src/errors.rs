// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Error types for parity matrix operations
//!
//! This module defines specific error types using `thiserror` for better error handling,
//! debugging, and user experience in parity matrix generation and validation.

use thiserror::Error;

/// Main error type for parity matrix operations
///
/// This enum covers all the different types of errors that can occur
/// during parity matrix generation, validation, and computation.
#[derive(Error, Debug)]
pub enum ParityMatrixError {
    /// Constraint violation errors
    #[error("Constraint violation: {message}")]
    Constraint { message: String },

    /// Mathematical computation errors
    #[error("Mathematical error: {message}")]
    Math { message: String },

    /// Matrix operation errors
    #[error("Matrix operation error: {message}")]
    Matrix { message: String },

    /// Verification errors
    #[error("Verification failed: {message}")]
    Verification { message: String },

    /// Generic error with context
    #[error("Error: {message}")]
    Generic { message: String },
}

/// Result type alias for parity matrix operations
pub type ParityMatrixResult<T> = Result<T, ParityMatrixError>;

/// Constraint error type for constraint violations
#[derive(Error, Debug)]
pub enum ConstraintError {
    /// Degree constraint violation: t > (n-1)/2
    #[error("Degree constraint violated: t ({t}) must be ≤ (n-1)/2 = {max_t} for n = {n}")]
    DegreeConstraint { t: usize, n: usize, max_t: usize },
}

/// Mathematical error type for computation failures
#[derive(Error, Debug)]
pub enum MathError {
    /// Modular inverse doesn't exist
    #[error("Modular inverse does not exist for {a} mod {modulus} (gcd != 1)")]
    NoModularInverse { a: String, modulus: String },

    /// Invalid modulus for operation
    #[error("Invalid modulus: {modulus} - {reason}")]
    InvalidModulus { modulus: String, reason: String },
}

// Conversion implementations for better error handling
impl From<ConstraintError> for ParityMatrixError {
    fn from(err: ConstraintError) -> Self {
        ParityMatrixError::Constraint {
            message: err.to_string(),
        }
    }
}

impl From<MathError> for ParityMatrixError {
    fn from(err: MathError) -> Self {
        ParityMatrixError::Math {
            message: err.to_string(),
        }
    }
}

impl From<String> for ParityMatrixError {
    fn from(message: String) -> Self {
        ParityMatrixError::Generic { message }
    }
}

impl From<&str> for ParityMatrixError {
    fn from(message: &str) -> Self {
        ParityMatrixError::Generic {
            message: message.to_string(),
        }
    }
}

// Helper functions for creating errors with context
impl ParityMatrixError {
    /// Create a verification error with a message
    pub fn verification(message: impl Into<String>) -> Self {
        ParityMatrixError::Verification {
            message: message.into(),
        }
    }

    /// Create a dimension mismatch error
    pub fn dimension_mismatch(expected: usize, actual: usize, context: impl Into<String>) -> Self {
        ParityMatrixError::Matrix {
            message: format!(
                "Dimension mismatch in {}: expected {}, got {}",
                context.into(),
                expected,
                actual
            ),
        }
    }
}

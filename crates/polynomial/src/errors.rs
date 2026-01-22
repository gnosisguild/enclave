//! Error types for polynomial operations.

use thiserror::Error;

/// Errors that can occur during polynomial operations.
#[derive(Debug, Error)]
pub enum PolynomialError {
    /// Division by zero polynomial
    #[error("Division by zero polynomial")]
    DivisionByZero,

    /// Invalid polynomial (e.g., empty coefficients or zero leading coefficient)
    #[error("Invalid polynomial: {message}")]
    InvalidPolynomial { message: String },

    /// Modulus operation error
    #[error("Modulus error: {message}")]
    ModulusError { message: String },

    /// Cyclotomic polynomial error
    #[error("Cyclotomic polynomial error: {message}")]
    CyclotomicError { message: String },

    /// Range check failure
    #[error("Range check error: {message}")]
    RangeCheckError { message: String },

    /// Arithmetic overflow or underflow
    #[error("Arithmetic error: {message}")]
    ArithmeticError { message: String },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Parse error for BigInt
    #[error("Parse error: {0}")]
    ParseError(#[from] num_bigint::ParseBigIntError),
}

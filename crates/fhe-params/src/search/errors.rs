//! Error types for BFV parameter search
//!
//! This module defines specific error types using `thiserror` for better error handling,
//! debugging, and user experience in BFV parameter search operations.

use thiserror::Error;

/// Main error type for BFV parameter search
///
/// This enum covers all the different types of errors that can occur
/// during BFV parameter search and validation.
#[derive(Error, Debug)]
pub enum BfvParamsError {
    /// Validation errors for BFV parameters
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Parameter search errors
    #[error("Parameter search error: {message}")]
    Search { message: String },

    /// Mathematical computation errors
    #[error("Mathematical error: {message}")]
    Math { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Prime selection errors
    #[error("Prime selection error: {message}")]
    PrimeSelection { message: String },

    /// Generic error with context
    #[error("Error: {message}")]
    Generic { message: String },
}

/// Result type alias for BFV parameter operations
pub type BfvParamsResult<T> = Result<T, BfvParamsError>;

/// Validation error type for specific parameter validation failures
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Invalid number of votes/plaintext additions
    #[error("Invalid number of votes: {z} - {reason}")]
    InvalidVotes { z: u128, reason: String },

    /// Invalid number of parties
    #[error("Invalid number of parties: {n} - {reason}")]
    InvalidParties { n: u128, reason: String },

    /// Invalid security parameter
    #[error("Invalid security parameter: {lambda} - {reason}")]
    InvalidSecurity { lambda: u32, reason: String },

    /// Invalid error bound
    #[error("Invalid error bound: {b} - {reason}")]
    InvalidErrorBound { b: u128, reason: String },

    /// General validation error
    #[error("Validation failed: {message}")]
    General { message: String },
}

/// Search error type for parameter search failures
#[derive(Error, Debug)]
pub enum SearchError {
    /// No feasible parameters found
    #[error("No feasible BFV parameters found for the given constraints")]
    NoFeasibleParameters,

    /// Equation validation failed
    #[error("Equation validation failed: {equation} - {reason}")]
    EquationValidation { equation: String, reason: String },

    /// Prime selection failed
    #[error("Failed to select suitable primes: {reason}")]
    PrimeSelection { reason: String },

    /// General search error
    #[error("Search failed: {message}")]
    General { message: String },
}

// Conversion implementations for better error handling
impl From<ValidationError> for BfvParamsError {
    fn from(err: ValidationError) -> Self {
        BfvParamsError::Validation {
            message: err.to_string(),
        }
    }
}

impl From<SearchError> for BfvParamsError {
    fn from(err: SearchError) -> Self {
        BfvParamsError::Search {
            message: err.to_string(),
        }
    }
}

impl From<String> for BfvParamsError {
    fn from(message: String) -> Self {
        BfvParamsError::Generic { message }
    }
}

impl From<&str> for BfvParamsError {
    fn from(message: &str) -> Self {
        BfvParamsError::Generic {
            message: message.to_string(),
        }
    }
}

// Helper functions for creating errors with context
impl BfvParamsError {
    /// Create a validation error with a message
    pub fn validation(message: impl Into<String>) -> Self {
        BfvParamsError::Validation {
            message: message.into(),
        }
    }

    /// Create a search error with a message
    pub fn search(message: impl Into<String>) -> Self {
        BfvParamsError::Search {
            message: message.into(),
        }
    }

    /// Create a mathematical error with a message
    pub fn math(message: impl Into<String>) -> Self {
        BfvParamsError::Math {
            message: message.into(),
        }
    }

    /// Create a configuration error with a message
    pub fn config(message: impl Into<String>) -> Self {
        BfvParamsError::Config {
            message: message.into(),
        }
    }

    /// Create a prime selection error with a message
    pub fn prime_selection(message: impl Into<String>) -> Self {
        BfvParamsError::PrimeSelection {
            message: message.into(),
        }
    }
}

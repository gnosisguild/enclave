// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation traits and artifact types.
//!
//! [`Computation`] is a generic trait for computing values from parameters and input.
//! [`CircuitComputation`] extends it for circuits that produce witness/bounds/bits.
//! [`Toml`] and [`Configs`] are the string types used for Prover.toml and configs.nr.

/// Variant for input types for DKG.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DkgInputType {
    /// The input type that generates shares of a secret key using secret sharing.
    SecretKey,
    /// The input type that generates shares of smudging noise instead of secret key shares.
    SmudgingNoise,
}

/// Generic computation from parameters and input to a result.
pub trait Computation: Sized {
    type BfvThresholdParametersPreset;
    type Input;
    type Error;

    /// Computes the result from parameters and input.
    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self, Self::Error>;
}

/// Circuit-specific computation: parameters and input produce bounds, bits, witness, etc.
pub trait CircuitComputation: crate::registry::Circuit {
    type BfvThresholdParametersPreset;
    type Input;
    type Output;
    type Error;

    /// Computes circuit-specific data (bounds, bits, witness) from parameters and input.
    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error>;
}

/// Converts a value to a JSON [`serde_json::Value`] for serialization.
pub trait ConvertToJson {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value>;
}

/// Any `Serialize` type can be converted to JSON for round-trip tests and artifact generation.
impl<T: serde::Serialize> ConvertToJson for T {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

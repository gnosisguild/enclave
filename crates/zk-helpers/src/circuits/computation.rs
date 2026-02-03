// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// TOML file for the circuit.
pub type Toml = String;
/// Configs file for the circuit.
pub type Configs = String;

/// Trait for computation.
pub trait Computation: Sized {
    type Params;
    type Input;
    type Error;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error>;
}

/// Trait for circuit computation.
pub trait CircuitComputation: crate::registry::Circuit {
    type Params;
    type Input;
    type Output;
    type Error;

    /// Compute circuit-specific data.
    fn compute(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error>;
}

/// Trait for converting to JSON.
pub trait ConvertToJson {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value>;
}

/// Trait for reducing to ZKP modulus.
pub trait ReduceToZkpModulus: Sized {
    fn reduce_to_zkp_modulus(&self) -> Self;
}

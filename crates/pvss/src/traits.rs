// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Common computation behavior across circuits.

use crate::types::DkgInputType;
use e3_fhe_params::ParameterType;
use serde_json::Value;

pub trait Computation: Sized {
    type Params;
    type Input;
    type Error;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error>;
}

pub trait ConvertToJson {
    fn convert_to_json(&self) -> serde_json::Result<Value>;
}

pub trait ReduceToZkpModulus: Sized {
    fn reduce_to_zkp_modulus(&self) -> Self;
}

pub trait Circuit: Send + Sync {
    const NAME: &'static str;
    const PREFIX: &'static str;
    const SUPPORTED_PARAMETER: ParameterType;
    const DKG_INPUT_TYPE: Option<DkgInputType>;
    const N_PROOFS: usize;
    const N_PUBLIC_INPUTS: usize;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn prefix(&self) -> &'static str {
        Self::PREFIX
    }

    fn supported_parameter(&self) -> ParameterType {
        Self::SUPPORTED_PARAMETER
    }

    fn dkg_input_type(&self) -> Option<DkgInputType> {
        Self::DKG_INPUT_TYPE
    }

    fn n_recursive_proofs(&self) -> usize {
        Self::N_PROOFS
    }

    fn n_public_inputs(&self) -> usize {
        Self::N_PUBLIC_INPUTS
    }
}

pub trait CircuitMetadata: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_parameter(&self) -> ParameterType;
    fn dkg_input_type(&self) -> Option<DkgInputType>;
    fn n_recursive_proofs(&self) -> usize;
    fn n_public_inputs(&self) -> usize;
}

impl<T: Circuit> CircuitMetadata for T {
    fn name(&self) -> &'static str {
        T::NAME
    }

    fn supported_parameter(&self) -> ParameterType {
        T::SUPPORTED_PARAMETER
    }

    fn dkg_input_type(&self) -> Option<DkgInputType> {
        T::DKG_INPUT_TYPE
    }

    fn n_recursive_proofs(&self) -> usize {
        T::N_PROOFS
    }

    fn n_public_inputs(&self) -> usize {
        T::N_PUBLIC_INPUTS
    }
}

pub trait CircuitCodegen: Circuit {
    type Input;
    type Error;

    /// Generate artifacts for a circuit.
    fn codegen(&self, input: Self::Input) -> Result<crate::types::Artifacts, Self::Error>;
}

pub trait CircuitComputation: Circuit {
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

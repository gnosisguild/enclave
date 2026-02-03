// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::circuits::pk_bfv::codegen;
use crate::circuits::pk_bfv::computation::{Bits, Bounds, Witness};
use crate::codegen::Artifacts;
use crate::codegen::CircuitCodegen;
use crate::computation::CircuitComputation;
use crate::computation::Computation;
use crate::errors::CircuitsErrors;
use crate::registry::Circuit;
use crate::registry::DkgInputType;
use e3_fhe_params::{BfvPreset, ParameterType};
use fhe::bfv::{BfvParameters, PublicKey};

#[derive(Debug)]
pub struct PkBfvCircuit;

#[derive(Debug)]
pub struct PkBfvComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

#[derive(Debug, Clone)]
pub struct PkBfvCodegenInput {
    pub preset: BfvPreset,
    pub public_key: PublicKey,
}

impl Circuit for PkBfvCircuit {
    const NAME: &'static str = "pk-bfv";
    const PREFIX: &'static str = "PK_BFV";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
    const N_PROOFS: usize = 1;
    const N_PUBLIC_INPUTS: usize = 1;
}

impl CircuitCodegen for PkBfvCircuit {
    type Input = PkBfvCodegenInput;
    type Error = CircuitsErrors;

    fn codegen(&self, input: Self::Input) -> Result<Artifacts, Self::Error> {
        codegen::codegen(input.preset, input.public_key)
    }
}

impl CircuitComputation for PkBfvCircuit {
    type Params = BfvParameters;
    type Input = PublicKey;
    type Output = PkBfvComputationOutput;
    type Error = CircuitsErrors;

    fn compute(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(params, &())?;
        let bits = Bits::compute(params, &bounds)?;
        let witness = Witness::compute(params, input)?;

        Ok(PkBfvComputationOutput {
            bounds,
            bits,
            witness,
        })
    }
}

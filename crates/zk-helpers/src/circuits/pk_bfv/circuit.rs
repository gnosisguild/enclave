// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Public-key BFV circuit type and implementations of [`Circuit`], [`CircuitCodegen`], [`CircuitComputation`].

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

/// Public-key BFV commitment circuit (name: `pk-bfv`).
/// Proves knowledge of a BFV public key; used for DKG parameter type.
#[derive(Debug)]
pub struct PkBfvCircuit;

/// Output of [`CircuitComputation::compute`] for [`PkBfvCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct PkBfvComputationOutput {
    /// Coefficient bounds for public key polynomials.
    pub bounds: Bounds,
    /// Bit widths for the prover (e.g. pk_bit).
    pub bits: Bits,
    /// Witness data (pk0is, pk1is) for the Noir prover.
    pub witness: Witness,
}

/// Input for [`CircuitCodegen::codegen`] for [`PkBfvCircuit`]: BFV preset and public key.
#[derive(Debug, Clone)]
pub struct PkBfvCodegenInput {
    /// BFV parameter preset (e.g. default).
    pub preset: BfvPreset,
    /// BFV public key to commit to.
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

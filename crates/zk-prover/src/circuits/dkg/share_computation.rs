// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{
    ChunkInputs, Inputs, ShareComputationBaseCircuit, ShareComputationChunkCircuit,
    ShareComputationChunkCircuitData, ShareComputationCircuitData,
};

impl Provable for ShareComputationBaseCircuit {
    type Params = BfvPreset;
    type Input = ShareComputationCircuitData;
    type Inputs = Inputs;

    fn resolve_circuit_name(&self, _params: &Self::Params, input: &Self::Input) -> CircuitName {
        match input.dkg_input_type {
            DkgInputType::SecretKey => CircuitName::SkShareComputationBase,
            DkgInputType::SmudgingNoise => CircuitName::ESmShareComputationBase,
        }
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![
            CircuitName::SkShareComputationBase,
            CircuitName::ESmShareComputationBase,
        ]
    }

    fn circuit(&self) -> CircuitName {
        CircuitName::SkShareComputationBase
    }
}

impl Provable for ShareComputationChunkCircuit {
    type Params = BfvPreset;
    type Input = ShareComputationChunkCircuitData;
    type Inputs = ChunkInputs;

    fn circuit(&self) -> CircuitName {
        CircuitName::ShareComputationChunk
    }
}

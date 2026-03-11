// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::circuits::recursive_aggregation::generate_share_computation_proof;
use crate::prover::ZkProver;
use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{
    ChunkInputs, Configs, Inputs, ShareComputationBaseCircuit, ShareComputationChunkCircuit,
    ShareComputationChunkCircuitData, ShareComputationCircuit, ShareComputationCircuitData,
};
use e3_zk_helpers::Computation;

/// Full share-computation proof (base + chunks + aggregation + wrapper).
/// Used by local e2e tests; the enclave uses the same pipeline via multithread handler.
impl Provable for ShareComputationCircuit {
    type Params = BfvPreset;
    type Input = ShareComputationCircuitData;
    type Inputs = Inputs;

    fn circuit(&self) -> CircuitName {
        CircuitName::ShareComputation
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![CircuitName::ShareComputation]
    }

    fn prove(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<e3_events::Proof, crate::error::ZkError> {
        let base_circuit = ShareComputationBaseCircuit;
        let base_proof = base_circuit.prove(prover, params, input, &format!("{e3_id}_base"))?;

        let n_chunks = Configs::compute(params.clone(), input)
            .map_err(|e| crate::error::ZkError::InputsGenerationFailed(e.to_string()))?
            .n_chunks;

        let chunk_circuit = ShareComputationChunkCircuit;
        let mut chunk_proofs = Vec::with_capacity(n_chunks);
        for chunk_idx in 0..n_chunks {
            let chunk_data = ShareComputationChunkCircuitData {
                share_data: input.clone(),
                chunk_idx,
            };
            let chunk_proof = chunk_circuit.prove(
                prover,
                params,
                &chunk_data,
                &format!("{e3_id}_chunk_{chunk_idx}"),
            )?;
            chunk_proofs.push(chunk_proof);
        }

        let c2_proof = generate_share_computation_proof(prover, &base_proof, &chunk_proofs, e3_id)?;
        // Return the inner share_computation proof (no wrapper). Tests verify with Recursive variant.
        Ok(c2_proof)
    }

    /// Inner share_computation proof is verified with Recursive variant.
    fn verify(
        &self,
        prover: &ZkProver,
        proof: &e3_events::Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, crate::error::ZkError> {
        self.verify_with_variant(
            prover,
            proof,
            e3_id,
            party_id,
            e3_events::CircuitVariant::Recursive,
        )
    }
}

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

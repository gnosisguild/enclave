// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::circuits::utils::{
    bytes_to_field_strings, prove_recursive_circuit, prove_recursive_circuit_non_zk,
};
use crate::circuits::vk;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::traits::Provable;
use e3_events::{CircuitName, CircuitVariant, Proof};
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{
    ChunkInputs, Configs, Inputs, ShareComputationBaseCircuit, ShareComputationChunkCircuit,
    ShareComputationChunkCircuitData, ShareComputationCircuit, ShareComputationCircuitData,
};
use e3_zk_helpers::Computation;

//////////////////////////////////////////////////////////////////////////////
// Two-level wrapper proof generation
//////////////////////////////////////////////////////////////////////////////

/// Level 1: Input for the chunk_batch circuit (base + CHUNKS_PER_BATCH chunks).
/// Field names match Noir parameter names exactly for witness generation.
#[derive(serde::Serialize)]
struct ChunkBatchInput {
    base_verification_key: Vec<String>,
    base_proof: Vec<String>,
    base_public_inputs: Vec<String>,
    base_key_hash: String,
    chunk_verification_key: Vec<String>,
    chunk_proofs: Vec<Vec<String>>,
    chunk_public_inputs: Vec<Vec<String>>,
    chunk_key_hash: String,
    batch_idx: String,
}

/// Level 1: Proves the share_computation_chunk_batch circuit that binds
/// 1 base proof + CHUNKS_PER_BATCH chunk proofs for a single batch.
pub fn generate_chunk_batch_proof(
    prover: &ZkProver,
    base_proof: &Proof,
    chunk_proofs: &[Proof],
    batch_idx: u32,
    e3_id: &str,
) -> Result<Proof, ZkError> {
    let recursive_dir = prover.circuits_dir(CircuitVariant::Recursive);

    let base_vk = vk::load_vk_artifacts(&recursive_dir, base_proof.circuit)?;
    let chunk_vk = vk::load_vk_artifacts(&recursive_dir, CircuitName::ShareComputationChunk)?;

    let mut chunk_proof_fields = Vec::with_capacity(chunk_proofs.len());
    let mut chunk_public_inputs = Vec::with_capacity(chunk_proofs.len());
    for cp in chunk_proofs {
        chunk_proof_fields.push(bytes_to_field_strings(&cp.data)?);
        chunk_public_inputs.push(bytes_to_field_strings(&cp.public_signals)?);
    }

    let input = ChunkBatchInput {
        base_verification_key: base_vk.verification_key,
        base_proof: bytes_to_field_strings(&base_proof.data)?,
        base_public_inputs: bytes_to_field_strings(&base_proof.public_signals)?,
        base_key_hash: base_vk.key_hash,
        chunk_verification_key: chunk_vk.verification_key,
        chunk_proofs: chunk_proof_fields,
        chunk_public_inputs,
        chunk_key_hash: chunk_vk.key_hash,
        batch_idx: batch_idx.to_string(),
    };

    // Non-ZK: chunk_batch proofs are intermediate, consumed by the final
    // share_computation circuit which verifies with verify_honk_proof_non_zk.
    prove_recursive_circuit_non_zk(
        prover,
        CircuitName::ShareComputationChunkBatch,
        &input,
        e3_id,
    )
}

/// Level 2: Input for the final share_computation wrapper (N_BATCHES batch proofs).
/// Field names match Noir parameter names exactly for witness generation.
#[derive(serde::Serialize)]
struct ShareComputationFinalInput {
    batch_verification_key: Vec<String>,
    batch_proofs: Vec<Vec<String>>,
    batch_public_inputs: Vec<Vec<String>>,
    batch_key_hash: String,
}

/// Level 2: Proves the final share_computation circuit that aggregates N_BATCHES
/// batch wrapper proofs into a single C2 proof.
pub fn generate_share_computation_final_proof(
    prover: &ZkProver,
    batch_proofs: &[Proof],
    e3_id: &str,
) -> Result<Proof, ZkError> {
    let recursive_dir = prover.circuits_dir(CircuitVariant::Recursive);

    let batch_vk = vk::load_vk_artifacts(&recursive_dir, CircuitName::ShareComputationChunkBatch)?;

    let mut batch_proof_fields = Vec::with_capacity(batch_proofs.len());
    let mut batch_public_inputs = Vec::with_capacity(batch_proofs.len());
    for bp in batch_proofs {
        batch_proof_fields.push(bytes_to_field_strings(&bp.data)?);
        batch_public_inputs.push(bytes_to_field_strings(&bp.public_signals)?);
    }

    let input = ShareComputationFinalInput {
        batch_verification_key: batch_vk.verification_key,
        batch_proofs: batch_proof_fields,
        batch_public_inputs,
        batch_key_hash: batch_vk.key_hash,
    };

    prove_recursive_circuit(prover, CircuitName::ShareComputation, &input, e3_id)
}

/// Proves a single chunk circuit from pre-computed [`ChunkInputs`].
/// Avoids redundant `Inputs::compute` when proving multiple chunks in a loop.
pub fn generate_chunk_proof(
    prover: &ZkProver,
    chunk_inputs: &ChunkInputs,
    e3_id: &str,
) -> Result<Proof, ZkError> {
    use crate::circuits::utils::inputs_json_to_input_map;
    use crate::witness::{CompiledCircuit, WitnessGenerator};

    let circuit_name = CircuitName::ShareComputationChunk;
    let recursive_dir = prover.circuits_dir(CircuitVariant::Recursive);
    let circuit_path = recursive_dir
        .join(circuit_name.dir_path())
        .join(format!("{}.json", circuit_name.as_str()));
    let compiled = CompiledCircuit::from_file(&circuit_path)?;

    let json = chunk_inputs
        .to_json()
        .map_err(|e| ZkError::SerializationError(e.to_string()))?;
    let input_map = inputs_json_to_input_map(&json)?;

    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&compiled, input_map)?;

    prover.generate_proof_with_variant(circuit_name, &witness, e3_id, CircuitVariant::Recursive)
}

//////////////////////////////////////////////////////////////////////////////
// Provable impls
//////////////////////////////////////////////////////////////////////////////

/// Full share-computation proof (base + chunks + two-level wrapper).
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
    ) -> Result<Proof, ZkError> {
        let base_circuit = ShareComputationBaseCircuit;
        let base_proof = base_circuit.prove(prover, params, input, &format!("{e3_id}_base"))?;

        let configs = Configs::compute(params.clone(), input)
            .map_err(|e| ZkError::InputsGenerationFailed(e.to_string()))?;
        let base_inputs = Inputs::compute(params.clone(), input)
            .map_err(|e| ZkError::InputsGenerationFailed(e.to_string()))?;

        let mut chunk_proofs = Vec::with_capacity(configs.n_chunks);
        for chunk_idx in 0..configs.n_chunks {
            let chunk_inputs = ChunkInputs::from_inputs(&base_inputs, &configs, chunk_idx)
                .map_err(|e| ZkError::InputsGenerationFailed(e.to_string()))?;
            let chunk_proof =
                generate_chunk_proof(prover, &chunk_inputs, &format!("{e3_id}_chunk_{chunk_idx}"))?;
            chunk_proofs.push(chunk_proof);
        }

        // Level 1: group chunks into batches and prove each batch
        let mut batch_proofs = Vec::with_capacity(configs.n_batches);
        for batch_idx in 0..configs.n_batches {
            let start = batch_idx * configs.chunks_per_batch;
            let end = usize::min(start + configs.chunks_per_batch, chunk_proofs.len());
            let batch_chunks = chunk_proofs.get(start..end).ok_or_else(|| {
                ZkError::ProveFailed(format!(
                    "chunk_proofs slice out of bounds: batch_idx={batch_idx}, start={start}, end={end}, len={}",
                    chunk_proofs.len()
                ))
            })?;
            let batch_proof = generate_chunk_batch_proof(
                prover,
                &base_proof,
                batch_chunks,
                batch_idx as u32,
                &format!("{e3_id}_batch_{batch_idx}"),
            )?;
            batch_proofs.push(batch_proof);
        }

        // Level 2: aggregate batch proofs into final C2 proof
        generate_share_computation_final_proof(prover, &batch_proofs, e3_id)
    }

    /// Inner share_computation proof is verified with Recursive variant.
    fn verify(
        &self,
        prover: &ZkProver,
        proof: &Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        self.verify_with_variant(prover, proof, e3_id, party_id, CircuitVariant::Recursive)
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

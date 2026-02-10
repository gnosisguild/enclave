// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::fmt::Display;

use crate::circuits::utils::witness_json_to_input_map;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, Proof};
use e3_zk_helpers::Computation;
use noirc_abi::InputMap;

/// Trait for types that can generate ZK proofs.
///
/// Implementors specify the circuit, params, input, and witness types.
/// `build_witness`, `prove`, and `verify` use default implementations
/// that compute the witness via [`Computation::compute`] and serialize via
/// [`Computation::to_json`].
pub trait Provable: Send + Sync {
    type Params: Send + Sync + Clone;
    type Input: Send + Sync;
    type Witness;

    fn circuit(&self) -> CircuitName;

    fn build_witness(&self, params: &Self::Params, input: &Self::Input) -> Result<InputMap, ZkError>
    where
        Self::Witness: Computation<Preset = Self::Params, Input = Self::Input> + serde::Serialize,
        <Self::Witness as Computation>::Error: Display,
    {
        let witness = Self::Witness::compute(params.clone(), input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;
        let json = witness
            .to_json()
            .map_err(|e| ZkError::SerializationError(e.to_string()))?;
        witness_json_to_input_map(&json)
    }

    fn prove(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<Proof, ZkError>
    where
        Self::Witness: Computation<Preset = Self::Params, Input = Self::Input> + serde::Serialize,
        <Self::Witness as Computation>::Error: Display,
    {
        let inputs = self.build_witness(params, input)?;

        let circuit_name = self.circuit().as_str();
        let circuit_path = prover
            .circuits_dir()
            .join(self.circuit().dir_path())
            .join(format!("{}.json", circuit_name));

        let circuit = CompiledCircuit::from_file(&circuit_path)?;

        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs)?;

        prover.generate_proof(self.circuit(), &witness, e3_id)
    }

    fn verify(
        &self,
        prover: &ZkProver,
        proof: &Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        if proof.circuit != self.circuit() {
            return Err(ZkError::VerifyFailed(format!(
                "circuit mismatch: expected {}, got {}",
                self.circuit(),
                proof.circuit
            )));
        }
        prover.verify(proof, e3_id, party_id)
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::fmt::Display;

use crate::circuits::utils::inputs_json_to_input_map;
use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, Proof};
use e3_zk_helpers::Computation;
use noirc_abi::InputMap;

/// Trait for types that can generate ZK proofs.
///
/// Implementors specify the circuit, params, and input types.
/// `build_inputs`, `prove`, and `verify` use default implementations
/// that compute the inputs via [`Computation::compute`] and serialize via
/// [`Computation::to_json`].
pub trait Provable: Send + Sync {
    type Params: Send + Sync + Clone;
    type Input: Send + Sync;
    type Inputs;

    fn circuit(&self) -> CircuitName;

    /// Override this to select a circuit variant based on input.
    /// Default just returns the standard circuit name.
    fn resolve_circuit_name(&self, _input: &Self::Input) -> CircuitName {
        self.circuit()
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![self.circuit()]
    }

    fn build_inputs(&self, params: &Self::Params, input: &Self::Input) -> Result<InputMap, ZkError>
    where
        Self::Inputs: Computation<Preset = Self::Params, Data = Self::Input> + serde::Serialize,
        <Self::Inputs as Computation>::Error: Display,
    {
        let inputs = Self::Inputs::compute(params.clone(), input)
            .map_err(|e| ZkError::InputsGenerationFailed(e.to_string()))?;

        let json = inputs
            .to_json()
            .map_err(|e| ZkError::SerializationError(e.to_string()))?;
        inputs_json_to_input_map(&json)
    }

    fn prove(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<Proof, ZkError>
    where
        Self::Inputs: Computation<Preset = Self::Params, Data = Self::Input> + serde::Serialize,
        <Self::Inputs as Computation>::Error: Display,
    {
        let inputs = self.build_inputs(params, input)?;

        let resolved_name = self.resolve_circuit_name(input);
        let circuit_path = prover
            .circuits_dir()
            .join(resolved_name.dir_path())
            .join(format!("{}.json", resolved_name.as_str()));

        let circuit = CompiledCircuit::from_file(&circuit_path)?;

        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs)?;

        prover.generate_proof(resolved_name, &witness, e3_id)
    }

    fn verify(
        &self,
        prover: &ZkProver,
        proof: &Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        if !self.valid_circuits().contains(&proof.circuit) {
            return Err(ZkError::VerifyFailed(format!(
                "circuit mismatch: expected one of {:?}, got {}",
                self.valid_circuits(),
                proof.circuit
            )));
        }

        println!(
            "Verifying proof for circuit {} with e3_id {} and party_id {}",
            proof.circuit, e3_id, party_id
        );
        prover.verify(proof, e3_id, party_id)
    }
}

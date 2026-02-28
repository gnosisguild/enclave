// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::fmt::Display;

use crate::circuits::recursive_aggregation::{generate_fold_proof, generate_wrapper_proof};
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

    /// Override this to select a circuit variant based on params and input.
    /// Default just returns the standard circuit name.
    fn resolve_circuit_name(&self, _params: &Self::Params, _input: &Self::Input) -> CircuitName {
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

        let resolved_name = self.resolve_circuit_name(params, input);
        let circuit_path = prover
            .circuits_dir()
            .join(resolved_name.dir_path())
            .join(format!("{}.json", resolved_name.as_str()));

        let circuit = CompiledCircuit::from_file(&circuit_path)?;

        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs)?;

        prover.generate_proof(resolved_name, &witness, e3_id)
    }

    /// Proves for recursive aggregation (poseidon2). Accepts 1 or 2 inputs of the same circuit,
    /// generates recursive proof(s), wraps them with the wrapper circuit.
    /// When `fold_proof` is provided: if it is a wrapper proof, does initial fold (two wrappers → fold);
    /// if it is a fold proof, folds wrapper with it. When `None`, returns the wrapper proof.
    fn aggregate_proof(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        inputs: &[Self::Input],
        aggregated_proof: Option<&Proof>,
        e3_id: &str,
    ) -> Result<Proof, ZkError>
    where
        Self::Inputs: Computation<Preset = Self::Params, Data = Self::Input> + serde::Serialize,
        <Self::Inputs as Computation>::Error: Display,
    {
        if !matches!(inputs.len(), 1 | 2) {
            return Err(ZkError::InvalidInput(
                "aggregate_proof requires 1 or 2 inputs".into(),
            ));
        }

        let mut recursive_proofs = Vec::with_capacity(inputs.len());
        let mut resolved_names = Vec::with_capacity(inputs.len());
        let witness_gen = WitnessGenerator::new();

        for (i, input) in inputs.iter().enumerate() {
            let input_map = self.build_inputs(params, input)?;
            let resolved_name = self.resolve_circuit_name(params, input);
            resolved_names.push(resolved_name);
            let circuit_path = prover
                .circuits_dir()
                .join(resolved_names[i].dir_path())
                .join(format!("{}.json", resolved_names[i].as_str()));
            let circuit = CompiledCircuit::from_file(&circuit_path)?;
            let witness = witness_gen.generate_witness(&circuit, input_map)?;
            let inner_e3_id = format!("{}_inner_{}", e3_id, i);
            let proof =
                prover.generate_recursive_proof(resolved_names[i], &witness, &inner_e3_id)?;
            recursive_proofs.push(proof);
        }

        if recursive_proofs.len() == 2 && resolved_names[0] != resolved_names[1] {
            return Err(ZkError::InvalidInput(
                "aggregate_proof requires both inputs to use the same circuit".into(),
            ));
        }

        let wrapper_proof = generate_wrapper_proof(prover, &recursive_proofs, e3_id)?;

        match aggregated_proof {
            Some(ap) => generate_fold_proof(prover, &wrapper_proof, ap, e3_id),
            None => Ok(wrapper_proof),
        }
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
        prover.verify_proof(proof, e3_id, party_id)
    }
}

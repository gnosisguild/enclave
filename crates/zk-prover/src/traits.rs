// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::ZkError;
use crate::prover::ZkProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use e3_events::{CircuitName, Proof};
use noirc_abi::InputMap;

/// Trait for types that can generate ZK proofs.
///
/// Implementors define how to build witness data from their inputs.
/// The prove/verify methods are provided with default implementations.
pub trait Provable: Send + Sync {
    type Params: Send + Sync;
    type Input: Send + Sync;

    fn circuit(&self) -> CircuitName;

    fn build_witness(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError>;

    fn prove(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
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

    fn verify(&self, prover: &ZkProver, proof: &Proof, e3_id: &str) -> Result<bool, ZkError> {
        if proof.circuit != self.circuit() {
            return Err(ZkError::VerifyFailed(format!(
                "circuit mismatch: expected {}, got {}",
                self.circuit(),
                proof.circuit
            )));
        }
        prover.verify(proof, e3_id)
    }
}

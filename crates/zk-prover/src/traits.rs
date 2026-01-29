// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::ZkError;
use crate::prover::{Proof, ZkProver};
use crate::witness::{CompiledCircuit, WitnessGenerator};
use async_trait::async_trait;
use noirc_abi::InputMap;

/// Trait for types that can generate ZK proofs.
///
/// Implementors define how to build witness data from their inputs
/// and how to parse the public output. The prove/verify methods
/// are provided with default implementations.
#[async_trait]
pub trait Provable: Send + Sync {
    type Params: Send + Sync;
    type Input: Send + Sync;

    fn circuit_name(&self) -> &'static str;

    fn build_witness(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError>;

    async fn prove(
        &self,
        prover: &ZkProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        let inputs = self.build_witness(params, input)?;

        let circuit_path = prover
            .circuits_dir()
            .join(format!("{}.json", self.circuit_name()));
        let circuit = CompiledCircuit::from_file(&circuit_path).await?;

        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs).await?;

        prover
            .generate_proof(self.circuit_name(), &witness, e3_id)
            .await
    }

    async fn verify(&self, prover: &ZkProver, proof: &Proof, e3_id: &str) -> Result<bool, ZkError> {
        if proof.circuit != self.circuit_name() {
            return Err(ZkError::VerifyFailed(format!(
                "circuit mismatch: expected {}, got {}",
                self.circuit_name(),
                proof.circuit
            )));
        }
        prover.verify(proof, e3_id).await
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::NoirProverError;
use crate::prover::NoirProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use async_trait::async_trait;
use noirc_abi::InputMap;

#[derive(Debug, Clone)]
pub struct ProofResult<O> {
    pub proof: Vec<u8>,
    pub output: O,
}

#[async_trait]
pub trait CircuitProver: Send + Sync {
    type Params: Send + Sync;
    type Input: Send + Sync;
    type Output: Send + Sync + AsRef<[u8]>;

    fn circuit_name(&self) -> &'static str;

    fn build_witness(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, NoirProverError>;

    fn parse_output(&self, bytes: &[u8]) -> Result<Self::Output, NoirProverError>;

    async fn prove(
        &self,
        prover: &NoirProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<ProofResult<Self::Output>, NoirProverError> {
        let inputs = self.build_witness(params, input)?;

        let circuit_path = prover
            .circuits_dir()
            .join(format!("{}.json", self.circuit_name()));
        let circuit = CompiledCircuit::from_file(&circuit_path).await?;

        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs).await?;

        let proof = prover
            .generate_proof(self.circuit_name(), &witness, e3_id)
            .await?;

        let output_path = prover
            .work_dir()
            .join(e3_id)
            .join("out")
            .join("public_inputs");
        let output_bytes = tokio::fs::read(&output_path).await?;
        let output = self.parse_output(&output_bytes)?;

        Ok(ProofResult { proof, output })
    }

    async fn verify(
        &self,
        prover: &NoirProver,
        proof: &[u8],
        output: &Self::Output,
        e3_id: &str,
    ) -> Result<bool, NoirProverError> {
        let job_dir = prover.work_dir().join(e3_id);
        tokio::fs::create_dir_all(&job_dir).await?;

        let out_dir = job_dir.join("out");
        tokio::fs::create_dir_all(&out_dir).await?;

        let output_path = out_dir.join("public_inputs");
        tokio::fs::write(&output_path, output.as_ref()).await?;

        prover.verify_proof(self.circuit_name(), proof, e3_id).await
    }
}

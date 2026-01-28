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

/// Result of proof generation
#[derive(Debug, Clone)]
pub struct ProofResult<O> {
    /// The generated proof bytes
    pub proof: Vec<u8>,
    /// The public output from the circuit
    pub output: O,
}

/// Trait for circuits that can generate Noir proofs.
///
/// This extends `CircuitComputation` by adding the ability to:
/// 1. Convert computation output to Noir `InputMap`
/// 2. Parse public outputs from proof generation
///
/// The actual prove/verify logic is provided by the blanket `CircuitProverExt` impl.
pub trait CircuitProver: e3_pvss::traits::CircuitComputation {
    /// The public output type extracted from the proof
    type ProofOutput;

    /// Error type for prover operations (can differ from computation error)
    type ProverError: From<NoirProverError> + From<Self::Error> + From<std::io::Error>;

    /// Convert the computation output to a Noir InputMap.
    ///
    /// This is where you map your witness data to the circuit's expected inputs.
    fn build_inputs(output: &Self::Output) -> Result<InputMap, Self::ProverError>;

    /// Parse the public output from raw bytes (from bb's public_inputs file).
    fn parse_proof_output(bytes: &[u8]) -> Result<Self::ProofOutput, Self::ProverError>;

    /// Get the circuit filename (defaults to circuit NAME with underscores).
    ///
    /// Override if your compiled circuit has a different filename.
    fn circuit_filename() -> String {
        Self::NAME.replace('-', "_")
    }
}

/// Extension trait providing prove/verify methods for any CircuitProver.
///
/// This is automatically implemented for all types implementing `CircuitProver`.
#[async_trait]
pub trait CircuitProverExt: CircuitProver {
    /// Generate a proof for this circuit.
    ///
    /// # Arguments
    /// * `prover` - The NoirProver instance
    /// * `params` - Circuit parameters (e.g., BfvParameters)
    /// * `input` - Circuit input (e.g., PublicKey)
    /// * `e3_id` - Unique job identifier for temp files
    async fn prove(
        &self,
        prover: &NoirProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<ProofResult<Self::ProofOutput>, Self::ProverError>;

    /// Verify a proof for this circuit.
    ///
    /// # Arguments
    /// * `prover` - The NoirProver instance
    /// * `proof` - The proof bytes
    /// * `output` - The public output to verify against
    /// * `e3_id` - Unique job identifier for temp files
    async fn verify(
        &self,
        prover: &NoirProver,
        proof: &[u8],
        output: &Self::ProofOutput,
        e3_id: &str,
    ) -> Result<bool, Self::ProverError>
    where
        Self::ProofOutput: AsRef<[u8]>;
}

#[async_trait]
impl<T> CircuitProverExt for T
where
    T: CircuitProver + Sync,
    T::Params: Sync,
    T::Input: Sync,
    T::Output: Send,
    T::ProofOutput: Send + Sync,
    T::ProverError: Send,
{
    async fn prove(
        &self,
        prover: &NoirProver,
        params: &Self::Params,
        input: &Self::Input,
        e3_id: &str,
    ) -> Result<ProofResult<Self::ProofOutput>, Self::ProverError> {
        // 1. Compute circuit data (bounds, bits, witness)
        let computation_output = self.compute(params, input)?;

        // 2. Build Noir inputs from computation output
        let inputs = Self::build_inputs(&computation_output)?;

        // 3. Load compiled circuit
        let circuit_path = prover
            .circuits_dir()
            .join(format!("{}.json", Self::circuit_filename()));
        let circuit = CompiledCircuit::from_file(&circuit_path)?;

        // 4. Generate witness
        let witness_gen = WitnessGenerator::new();
        let witness = witness_gen.generate_witness(&circuit, inputs)?;

        // 5. Generate proof
        let proof = prover
            .generate_proof(&Self::circuit_filename(), &witness, e3_id)
            .await?;

        // 6. Read and parse public output
        let output_path = prover
            .work_dir()
            .join(e3_id)
            .join("out")
            .join("public_inputs");
        let output_bytes = tokio::fs::read(&output_path).await?;
        let output = Self::parse_proof_output(&output_bytes)?;

        Ok(ProofResult { proof, output })
    }

    async fn verify(
        &self,
        prover: &NoirProver,
        proof: &[u8],
        output: &Self::ProofOutput,
        e3_id: &str,
    ) -> Result<bool, Self::ProverError>
    where
        Self::ProofOutput: AsRef<[u8]>,
    {
        // Write public inputs to expected location
        let job_dir = prover.work_dir().join(e3_id);
        tokio::fs::create_dir_all(&job_dir).await?;

        let out_dir = job_dir.join("out");
        tokio::fs::create_dir_all(&out_dir).await?;

        let output_path = out_dir.join("public_inputs");
        tokio::fs::write(&output_path, output.as_ref()).await?;

        Ok(prover
            .verify_proof(&Self::circuit_filename(), proof, e3_id)
            .await?)
    }
}

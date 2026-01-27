// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::NoirProverError;
use crate::setup::NoirSetup;
use serde::Serialize;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Circuit {
    PkBfv,
    DecShareTrbfv,
    VerifyShares,
}

// TODO: update circuit list
impl Circuit {
    pub fn filename(&self) -> &'static str {
        match self {
            Circuit::PkBfv => "pk_bfv.json",
            Circuit::DecShareTrbfv => "dec_share_trbfv.json",
            Circuit::VerifyShares => "verify_shares.json",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Circuit::PkBfv => "BFV Public Key Generation",
            Circuit::DecShareTrbfv => "Decryption Share",
            Circuit::VerifyShares => "Share Verification",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Proof {
    pub bytes: Vec<u8>,
    pub circuit: Circuit,
}

impl Proof {
    pub fn new(bytes: Vec<u8>, circuit: Circuit) -> Self {
        Self { bytes, circuit }
    }

    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

#[derive(Debug, Clone)]
pub struct NoirProver {
    setup: NoirSetup,
}

impl NoirProver {
    pub fn new(setup: NoirSetup) -> Self {
        Self { setup }
    }

    pub fn with_default_setup() -> Result<Self, NoirProverError> {
        let setup = NoirSetup::with_default_dir()?;
        Ok(Self::new(setup))
    }

    pub async fn is_ready(&self) -> bool {
        self.setup.bb_binary.exists() && self.setup.circuits_dir.exists()
    }

    pub async fn ensure_ready(&self) -> Result<(), NoirProverError> {
        self.setup.ensure_installed().await
    }

    fn circuit_path(&self, circuit: Circuit) -> PathBuf {
        self.setup.circuits_dir.join(circuit.filename())
    }

    pub async fn generate_proof<T: Serialize>(
        &self,
        circuit: Circuit,
        inputs: &T,
        e3_id: &str,
    ) -> Result<Proof, NoirProverError> {
        // Verify bb exists
        if !self.setup.bb_binary.exists() {
            return Err(NoirProverError::BbNotInstalled);
        }

        // Verify circuit exists
        let circuit_path = self.circuit_path(circuit);
        if !circuit_path.exists() {
            return Err(NoirProverError::CircuitNotFound(
                circuit.filename().to_string(),
            ));
        }

        let work_dir = self.setup.work_dir_for(e3_id);
        fs::create_dir_all(&work_dir).await?;

        let inputs_path = work_dir.join("Prover.toml");
        let inputs_toml = toml::to_string(inputs)
            .map_err(|e| NoirProverError::SerializationError(e.to_string()))?;
        fs::write(&inputs_path, &inputs_toml).await?;

        debug!("Wrote inputs to {}", inputs_path.display());

        let witness_path = work_dir.join("witness.gz");
        let proof_path = work_dir.join("proof");

        info!("Generating {} proof for e3_id={}", circuit.name(), e3_id);

        // Run bb prove
        let output = tokio::process::Command::new(&self.setup.bb_binary)
            .args([
                "prove",
                "--scheme",
                "ultra_honk",
                "-b",
                circuit_path.to_str().unwrap(),
                "-w",
                witness_path.to_str().unwrap(),
                "--write_vk",
                "-o",
                work_dir.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoirProverError::ProveFailed(stderr.to_string()));
        }

        let proof_bytes = fs::read(&proof_path).await.map_err(|e| {
            NoirProverError::OutputReadError(format!(
                "Failed to read proof from {}: {}",
                proof_path.display(),
                e
            ))
        })?;

        info!(
            "✓ Generated proof ({} bytes) for e3_id={}",
            proof_bytes.len(),
            e3_id
        );

        Ok(Proof::new(proof_bytes, circuit))
    }

    pub async fn verify_proof(
        &self,
        circuit: Circuit,
        proof: &Proof,
    ) -> Result<(), NoirProverError> {
        if !self.setup.bb_binary.exists() {
            return Err(NoirProverError::BbNotInstalled);
        }

        let temp_dir = tempfile::tempdir()?;
        let proof_path = temp_dir.path().join("proof");
        let vk_path = self
            .setup
            .circuits_dir
            .join("vk")
            .join(circuit.filename().replace(".json", ".vk"));

        fs::write(&proof_path, &proof.bytes).await?;

        let output = tokio::process::Command::new(&self.setup.bb_binary)
            .args([
                "verify",
                "--scheme",
                "ultra_honk",
                "-p",
                proof_path.to_str().unwrap(),
                "-k",
                vk_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoirProverError::VerifyFailed(stderr.to_string()));
        }

        Ok(())
    }

    pub async fn cleanup(&self, e3_id: &str) -> Result<(), NoirProverError> {
        self.setup.cleanup_work_dir(e3_id).await
    }

    pub fn setup(&self) -> &NoirSetup {
        &self.setup
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PkBfvInputs {
    pub pk0: Vec<String>,
    pub pk1: Vec<String>,
    pub sk_commitment: String,
    pub randomness: Vec<String>,
}

impl PkBfvInputs {
    pub fn dummy() -> Self {
        Self {
            pk0: vec!["0".to_string(); 1024],
            pk1: vec!["0".to_string(); 1024],
            sk_commitment: "0x0".to_string(),
            randomness: vec!["0".to_string(); 32],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_filenames() {
        assert_eq!(Circuit::PkBfv.filename(), "pk_bfv.json");
        assert_eq!(Circuit::DecShareTrbfv.filename(), "dec_share_trbfv.json");
    }

    #[test]
    fn test_proof_size() {
        let proof = Proof::new(vec![0u8; 2048], Circuit::PkBfv);
        assert_eq!(proof.size(), 2048);
    }

    #[test]
    fn test_pk_bfv_inputs_serialization() {
        let inputs = PkBfvInputs::dummy();
        let toml = toml::to_string(&inputs).unwrap();
        assert!(toml.contains("pk0"));
        assert!(toml.contains("sk_commitment"));
    }
}

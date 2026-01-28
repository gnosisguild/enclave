// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Noir prover using native witness generation + bb CLI

use crate::error::NoirProverError;
use crate::setup::NoirSetup;
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info};

pub struct NoirProver {
    bb_binary: PathBuf,
    circuits_dir: PathBuf,
    work_dir: PathBuf,
}

impl NoirProver {
    pub fn new(setup: &NoirSetup) -> Self {
        Self {
            bb_binary: setup.bb_binary.clone(),
            circuits_dir: setup.circuits_dir.clone(),
            work_dir: setup.work_dir.clone(),
        }
    }

    pub fn circuits_dir(&self) -> &PathBuf {
        &self.circuits_dir
    }

    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    pub async fn generate_proof(
        &self,
        circuit_name: &str,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Vec<u8>, NoirProverError> {
        if !self.bb_binary.exists() {
            return Err(NoirProverError::BbNotInstalled);
        }

        let circuit_path = self.circuits_dir.join(format!("{}.json", circuit_name));
        if !circuit_path.exists() {
            return Err(NoirProverError::CircuitNotFound(circuit_name.to_string()));
        }

        let vk_path = self
            .circuits_dir
            .join("vk")
            .join(format!("{}.vk", circuit_name));
        if !vk_path.exists() {
            return Err(NoirProverError::CircuitNotFound(format!(
                "VK not found: {}",
                vk_path.display()
            )));
        }

        let job_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&job_dir).await?;

        let witness_path = job_dir.join("witness.gz");
        let output_dir = job_dir.join("out");
        let proof_path = output_dir.join("proof");

        fs::write(&witness_path, witness_data).await?;

        debug!("generating proof for circuit: {}", circuit_name);

        let output = Command::new(&self.bb_binary)
            .args([
                "prove",
                "--scheme",
                "ultra_honk",
                "-b",
                circuit_path.to_str().unwrap(),
                "-w",
                witness_path.to_str().unwrap(),
                "-k",
                vk_path.to_str().unwrap(),
                "-o",
                output_dir.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoirProverError::ProveFailed(stderr.to_string()));
        }

        let proof = fs::read(&proof_path).await?;
        info!("generated proof ({} bytes) for {}", proof.len(), e3_id);

        Ok(proof)
    }

    pub async fn verify_proof(
        &self,
        circuit_name: &str,
        proof: &[u8],
        e3_id: &str,
    ) -> Result<bool, NoirProverError> {
        if !self.bb_binary.exists() {
            return Err(NoirProverError::BbNotInstalled);
        }

        let vk_path = self
            .circuits_dir
            .join("vk")
            .join(format!("{}.vk", circuit_name));
        if !vk_path.exists() {
            return Err(NoirProverError::CircuitNotFound(format!(
                "{}.vk",
                circuit_name
            )));
        }

        let job_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&job_dir).await?;

        let proof_path = job_dir.join("proof_to_verify");
        fs::write(&proof_path, proof).await?;

        let public_inputs_path = job_dir.join("out").join("public_inputs");
        if !public_inputs_path.exists() {
            return Err(NoirProverError::ProveFailed(
                "public_inputs not found".to_string(),
            ));
        }

        debug!("verifying proof for circuit: {}", circuit_name);

        let output = Command::new(&self.bb_binary)
            .args([
                "verify",
                "--scheme",
                "ultra_honk",
                "-i",
                public_inputs_path.to_str().unwrap(),
                "-p",
                proof_path.to_str().unwrap(),
                "-k",
                vk_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        Ok(output.status.success())
    }

    pub async fn cleanup(&self, e3_id: &str) -> Result<(), NoirProverError> {
        let job_dir = self.work_dir.join(e3_id);
        if job_dir.exists() {
            fs::remove_dir_all(&job_dir).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_prover_requires_bb() {
        let temp = tempdir().unwrap();
        let prover = NoirProver {
            bb_binary: temp.path().join("nonexistent"),
            circuits_dir: temp.path().join("circuits"),
            work_dir: temp.path().join("work"),
        };

        let result = prover.generate_proof("test", b"witness", "e3-1").await;
        assert!(matches!(result, Err(NoirProverError::BbNotInstalled)));
    }
}

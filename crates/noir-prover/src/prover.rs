// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

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

        let work_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&work_dir).await?;

        let witness_path = work_dir.join("witness.gz");
        let proof_path = work_dir.join("proof");

        fs::write(&witness_path, witness_data).await?;

        debug!("Generating proof for circuit: {}", circuit_name);

        let output = Command::new(&self.bb_binary)
            .args([
                "prove",
                "--scheme",
                "ultra_honk",
                "-b",
                circuit_path.to_str().unwrap(),
                "-w",
                witness_path.to_str().unwrap(),
                "-o",
                proof_path.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(NoirProverError::ProveFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let proof = fs::read(&proof_path).await?;
        info!("Generated proof ({} bytes) for {}", proof.len(), e3_id);
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

        let work_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&work_dir).await?;

        let proof_path = work_dir.join("proof");
        fs::write(&proof_path, proof).await?;

        debug!("Verifying proof for circuit: {}", circuit_name);

        let output = Command::new(&self.bb_binary)
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

        Ok(output.status.success())
    }

    pub async fn cleanup(&self, e3_id: &str) -> Result<(), NoirProverError> {
        let work_dir = self.work_dir.join(e3_id);
        if work_dir.exists() {
            fs::remove_dir_all(&work_dir).await?;
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

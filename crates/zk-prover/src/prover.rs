// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backend::ZkBackend;
use crate::error::ZkError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use]
pub struct Proof {
    /// Circuit name (e.g., "pk_bfv", "pk_trbfv").
    pub circuit: String,
    /// The proof bytes.
    pub data: Vec<u8>,
    /// Public signals (inputs and outputs) from the circuit.
    pub public_signals: Vec<u8>,
}

impl Proof {
    pub fn new(circuit: impl Into<String>, data: Vec<u8>, public_signals: Vec<u8>) -> Self {
        Self {
            circuit: circuit.into(),
            data,
            public_signals,
        }
    }
}

pub struct ZkProver {
    bb_binary: PathBuf,
    circuits_dir: PathBuf,
    work_dir: PathBuf,
}

impl ZkProver {
    pub fn new(backend: &ZkBackend) -> Self {
        Self {
            bb_binary: backend.bb_binary.clone(),
            circuits_dir: backend.circuits_dir.clone(),
            work_dir: backend.work_dir.clone(),
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
    ) -> Result<Proof, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let circuit_path = self.circuits_dir.join(format!("{}.json", circuit_name));
        if !circuit_path.exists() {
            return Err(ZkError::CircuitNotFound(circuit_name.to_string()));
        }

        let vk_path = self
            .circuits_dir
            .join("vk")
            .join(format!("{}.vk", circuit_name));
        if !vk_path.exists() {
            return Err(ZkError::CircuitNotFound(format!(
                "VK not found: {}",
                vk_path.display()
            )));
        }

        let job_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&job_dir).await?;

        let witness_path = job_dir.join("witness.gz");
        let output_dir = job_dir.join("out");
        let proof_path = output_dir.join("proof");
        let public_inputs_path = output_dir.join("public_inputs");

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
            return Err(ZkError::ProveFailed(stderr.to_string()));
        }

        let proof_data = fs::read(&proof_path).await?;
        let public_signals = fs::read(&public_inputs_path).await?;

        info!(
            "generated proof ({} bytes) for {} / {}",
            proof_data.len(),
            circuit_name,
            e3_id
        );

        Ok(Proof::new(circuit_name, proof_data, public_signals))
    }

    pub async fn verify(&self, proof: &Proof, e3_id: &str) -> Result<bool, ZkError> {
        self.verify_proof(&proof.circuit, &proof.data, &proof.public_signals, e3_id)
            .await
    }

    pub async fn verify_proof(
        &self,
        circuit_name: &str,
        proof_data: &[u8],
        public_signals: &[u8],
        e3_id: &str,
    ) -> Result<bool, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let vk_path = self
            .circuits_dir
            .join("vk")
            .join(format!("{}.vk", circuit_name));
        if !vk_path.exists() {
            return Err(ZkError::CircuitNotFound(format!("{}.vk", circuit_name)));
        }

        let job_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&job_dir).await?;

        let out_dir = job_dir.join("out");
        fs::create_dir_all(&out_dir).await?;

        let proof_path = job_dir.join("proof_to_verify");
        let public_inputs_path = out_dir.join("public_inputs");

        fs::write(&proof_path, proof_data).await?;
        fs::write(&public_inputs_path, public_signals).await?;

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

    pub async fn cleanup(&self, e3_id: &str) -> Result<(), ZkError> {
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
    use crate::config::ZkConfig;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_prover_requires_bb() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());
        let prover = ZkProver::new(&backend);

        let result = prover.generate_proof("test", b"witness", "e3-1").await;
        assert!(matches!(result, Err(ZkError::BbNotInstalled)));
    }
}

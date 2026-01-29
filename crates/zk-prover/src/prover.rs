// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::backend::ZkBackend;
use crate::error::ZkError;
use e3_events::{CircuitName, Proof};
use e3_utils::utility_types::ArcBytes;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tracing::{debug, info};

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

    pub fn generate_proof(
        &self,
        circuit: CircuitName,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let circuit_name = circuit.as_str();
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
        fs::create_dir_all(&job_dir)?;

        let witness_path = job_dir.join("witness.gz");
        let output_dir = job_dir.join("out");
        let proof_path = output_dir.join("proof");
        let public_inputs_path = output_dir.join("public_inputs");

        fs::write(&witness_path, witness_data)?;

        debug!("generating proof for circuit: {}", circuit_name);

        let output = StdCommand::new(&self.bb_binary)
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
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ZkError::ProveFailed(stderr.to_string()));
        }

        let proof_data = fs::read(&proof_path)?;
        let public_signals = fs::read(&public_inputs_path)?;

        info!(
            "generated proof ({} bytes) for {} / {}",
            proof_data.len(),
            circuit_name,
            e3_id
        );

        Ok(Proof::new(
            circuit,
            ArcBytes::from_bytes(&proof_data),
            ArcBytes::from_bytes(&public_signals),
        ))
    }

    pub fn verify(&self, proof: &Proof, e3_id: &str) -> Result<bool, ZkError> {
        self.verify_proof(proof.circuit, &proof.data, &proof.public_signals, e3_id)
    }

    pub fn verify_proof(
        &self,
        circuit: CircuitName,
        proof_data: &[u8],
        public_signals: &[u8],
        e3_id: &str,
    ) -> Result<bool, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let circuit_name = circuit.as_str();
        let vk_path = self
            .circuits_dir
            .join("vk")
            .join(format!("{}.vk", circuit_name));
        if !vk_path.exists() {
            return Err(ZkError::CircuitNotFound(format!("{}.vk", circuit_name)));
        }

        let job_dir = self.work_dir.join(e3_id);
        fs::create_dir_all(&job_dir)?;

        let out_dir = job_dir.join("out");
        fs::create_dir_all(&out_dir)?;

        let proof_path = job_dir.join("proof_to_verify");
        let public_inputs_path = out_dir.join("public_inputs");

        fs::write(&proof_path, proof_data)?;
        fs::write(&public_inputs_path, public_signals)?;

        debug!("verifying proof for circuit: {}", circuit_name);

        let output = StdCommand::new(&self.bb_binary)
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
            .output()?;

        Ok(output.status.success())
    }

    pub fn cleanup(&self, e3_id: &str) -> Result<(), ZkError> {
        let job_dir = self.work_dir.join(e3_id);
        if job_dir.exists() {
            fs::remove_dir_all(&job_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ZkConfig;
    use tempfile::tempdir;

    #[test]
    fn test_prover_requires_bb() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());
        let prover = ZkProver::new(&backend);

        let result = prover.generate_proof(CircuitName::PkBfv, b"witness", "e3-1");
        assert!(matches!(result, Err(ZkError::BbNotInstalled)));
    }
}

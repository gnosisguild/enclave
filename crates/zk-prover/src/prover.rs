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
use tracing::{debug, info, warn};

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

        // Circuits are organized as: circuits/{group}/{name}/{name}.json
        let circuit_dir = self.circuits_dir.join(circuit.dir_path());
        let circuit_path = circuit_dir.join(format!("{}.json", circuit.as_str()));
        let vk_path = circuit_dir.join(format!("{}.vk", circuit.as_str()));

        if !circuit_path.exists() {
            return Err(ZkError::CircuitNotFound(format!(
                "Circuit not found: {} (expected at {})",
                circuit.as_str(),
                circuit_path.display()
            )));
        }
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

        fs::write(&witness_path, witness_data)?;

        debug!(
            "generating proof for circuit {} using circuit: {}, vk: {}",
            circuit.as_str(),
            circuit_path.display(),
            vk_path.display()
        );

        let output = StdCommand::new(&self.bb_binary)
            .args([
                "prove",
                "--scheme",
                "ultra_honk",
                "--oracle_hash",
                "keccak",
                "-b",
                &circuit_path.to_string_lossy(),
                "-w",
                &witness_path.to_string_lossy(),
                "-k",
                &vk_path.to_string_lossy(),
                "-o",
                &output_dir.to_string_lossy(),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(ZkError::ProveFailed(format!(
                "bb prove failed:\nstderr: {}\nstdout: {}",
                stderr, stdout
            )));
        }

        let proof_data = fs::read(output_dir.join("proof"))?;
        let public_signals = fs::read(output_dir.join("public_inputs"))?;

        info!(
            "generated proof ({} bytes) for {} / {}",
            proof_data.len(),
            circuit.as_str(),
            e3_id
        );

        Ok(Proof::new(
            circuit,
            ArcBytes::from_bytes(&proof_data),
            ArcBytes::from_bytes(&public_signals),
        ))
    }

    pub fn verify(&self, proof: &Proof, e3_id: &str, party_id: u64) -> Result<bool, ZkError> {
        self.verify_proof(
            proof.circuit,
            &proof.data,
            &proof.public_signals,
            e3_id,
            party_id,
        )
    }

    pub fn verify_proof(
        &self,
        circuit: CircuitName,
        proof_data: &[u8],
        public_signals: &[u8],
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let vk_path = self
            .circuits_dir
            .join(circuit.dir_path())
            .join(format!("{}.vk", circuit.as_str()));
        if !vk_path.exists() {
            return Err(ZkError::CircuitNotFound(format!(
                "VK not found: {}",
                vk_path.display()
            )));
        }

        let verification_subdir = format!("verify_party_{}", party_id);

        debug!(
            "verifying proof for circuit {} (party {}) using VK: {}",
            circuit.as_str(),
            party_id,
            vk_path.display()
        );

        let job_dir = self.work_dir.join(e3_id).join(&verification_subdir);
        let out_dir = job_dir.join("out");
        fs::create_dir_all(&out_dir)?;

        let proof_path = job_dir.join("proof");
        let public_inputs_path = out_dir.join("public_inputs");

        fs::write(&proof_path, proof_data)?;
        fs::write(&public_inputs_path, public_signals)?;

        let output = StdCommand::new(&self.bb_binary)
            .args([
                "verify",
                "--scheme",
                "ultra_honk",
                "--oracle_hash",
                "keccak",
                "-i",
                &public_inputs_path.to_string_lossy(),
                "-p",
                &proof_path.to_string_lossy(),
                "-k",
                &vk_path.to_string_lossy(),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            warn!(
                "bb verification failed for {}:\nVK: {}\nstderr: {}\nstdout: {}",
                circuit.as_str(),
                vk_path.display(),
                stderr,
                stdout
            );
        }

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
    use crate::test_utils::get_tempdir;
    use e3_config::BBPath;

    #[test]
    fn test_prover_requires_bb() {
        let temp = get_tempdir().unwrap();
        let temp_path = temp.path();
        let noir_dir = temp_path.join("noir");
        let bb_binary = noir_dir.join("bin").join("bb");
        let circuits_dir = noir_dir.join("circuits");
        let work_dir = noir_dir.join("work").join("test_node");
        let backend = ZkBackend::new(BBPath::Default(bb_binary), circuits_dir, work_dir);
        let prover = ZkProver::new(&backend);

        let result = prover.generate_proof(CircuitName::PkBfv, b"witness", "e3-1");
        assert!(matches!(result, Err(ZkError::BbNotInstalled)));
    }
}

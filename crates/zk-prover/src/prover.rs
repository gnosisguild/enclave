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

    pub fn bb_binary(&self) -> &PathBuf {
        &self.bb_binary
    }

    pub fn generate_proof(
        &self,
        circuit: CircuitName,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        self.generate_proof_impl(circuit, witness_data, e3_id, &circuit.dir_path(), None)
    }

    /// Generates a proof for recursive aggregation (poseidon2, noir-recursive-no-zk).
    /// Uses inner circuit dir and `.vk_recursive`.
    pub fn generate_recursive_proof(
        &self,
        circuit: CircuitName,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        self.generate_proof_impl(
            circuit,
            witness_data,
            e3_id,
            &circuit.dir_path(),
            Some("noir-recursive-no-zk"),
        )
    }

    /// Generates a proof of the wrapper circuit (for aggregation output).
    /// Uses wrapper dir; verifier_target determines proof format and VK suffix.
    pub fn generate_wrapper_proof(
        &self,
        circuit: CircuitName,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        self.generate_proof_impl(
            circuit,
            witness_data,
            e3_id,
            &circuit.wrapper_dir_path(),
            Some("noir-recursive-no-zk"),
        )
    }

    /// Generates a proof of the fold circuit (for aggregation output).
    /// The fold circuit is independent; uses fixed path `recursive_aggregation/fold`.
    /// Verifier target: `noir-recursive-no-zk`.
    pub fn generate_fold_proof(&self, witness_data: &[u8], e3_id: &str) -> Result<Proof, ZkError> {
        let dir = CircuitName::Fold.dir_path();
        self.generate_proof_impl(
            CircuitName::Fold,
            witness_data,
            e3_id,
            &dir,
            Some("noir-recursive-no-zk"),
        )
    }

    /// Generates the final fold proof for on-chain verification (evm target).
    pub fn generate_final_fold_proof(
        &self,
        witness_data: &[u8],
        e3_id: &str,
    ) -> Result<Proof, ZkError> {
        let dir = CircuitName::Fold.dir_path();
        self.generate_proof_impl(CircuitName::Fold, witness_data, e3_id, &dir, Some("evm"))
    }

    fn generate_proof_impl(
        &self,
        circuit: CircuitName,
        witness_data: &[u8],
        e3_id: &str,
        dir_path: &str,
        verifier_target: Option<&str>,
    ) -> Result<Proof, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let vk_suffix = match verifier_target {
            Some("noir-recursive") | Some("noir-recursive-no-zk") => "_recursive",
            _ => "",
        };

        let circuit_dir = self.circuits_dir.join(dir_path);
        let circuit_path = circuit_dir.join(format!("{}.json", circuit.as_str()));
        let vk_path = circuit_dir.join(format!("{}.vk{vk_suffix}", circuit.as_str()));

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

        let circuit_path_s = circuit_path.to_string_lossy();
        let witness_path_s = witness_path.to_string_lossy();
        let vk_path_s = vk_path.to_string_lossy();
        let output_dir_s = output_dir.to_string_lossy();

        let mut args = vec![
            "prove",
            "--scheme",
            "ultra_honk",
            "-b",
            circuit_path_s.as_ref(),
            "-w",
            witness_path_s.as_ref(),
            "-k",
            vk_path_s.as_ref(),
            "-o",
            output_dir_s.as_ref(),
        ];
        if let Some(t) = verifier_target {
            args.extend(["-t", t]);
        } else {
            args.extend(["--oracle_hash", "keccak"]);
        }

        let output = StdCommand::new(&self.bb_binary).args(&args).output()?;

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

    pub fn verify_proof(&self, proof: &Proof, e3_id: &str, party_id: u64) -> Result<bool, ZkError> {
        self.verify_proof_impl(
            proof.circuit,
            &proof.data,
            &proof.public_signals,
            proof.circuit.dir_path(),
            e3_id,
            party_id,
            None,
        )
    }

    /// Verifies a wrapper/aggregation proof using the wrapper circuit's recursive VK.
    pub fn verify_wrapper_proof(
        &self,
        proof: &Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        self.verify_proof_impl(
            proof.circuit,
            &proof.data,
            &proof.public_signals,
            proof.circuit.wrapper_dir_path(),
            e3_id,
            party_id,
            Some("noir-recursive-no-zk"),
        )
    }

    /// Verifies a fold proof using the fold circuit's recursive VK.
    pub fn verify_fold_proof(
        &self,
        proof: &Proof,
        e3_id: &str,
        party_id: u64,
    ) -> Result<bool, ZkError> {
        use e3_events::CircuitName;
        if proof.circuit != CircuitName::Fold {
            return Err(ZkError::InvalidInput(format!(
                "expected Fold proof, got {}",
                proof.circuit
            )));
        }
        self.verify_proof_impl(
            proof.circuit,
            &proof.data,
            &proof.public_signals,
            proof.circuit.dir_path(),
            e3_id,
            party_id,
            Some("noir-recursive-no-zk"),
        )
    }

    fn verify_proof_impl(
        &self,
        circuit: CircuitName,
        proof_data: &[u8],
        public_signals: &[u8],
        dir_path: String,
        e3_id: &str,
        party_id: u64,
        verifier_target: Option<&str>,
    ) -> Result<bool, ZkError> {
        if !self.bb_binary.exists() {
            return Err(ZkError::BbNotInstalled);
        }

        let vk_suffix = match verifier_target {
            Some("noir-recursive") | Some("noir-recursive-no-zk") => "_recursive",
            _ => "",
        };
        let vk_path = self
            .circuits_dir
            .join(&dir_path)
            .join(format!("{}.vk{vk_suffix}", circuit.as_str()));
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

        let public_inputs_s = public_inputs_path.to_string_lossy();
        let proof_s = proof_path.to_string_lossy();
        let vk_s = vk_path.to_string_lossy();

        let mut args = vec![
            "verify",
            "--scheme",
            "ultra_honk",
            "-i",
            public_inputs_s.as_ref(),
            "-p",
            proof_s.as_ref(),
            "-k",
            vk_s.as_ref(),
        ];
        if let Some(t) = verifier_target {
            args.extend(["-t", t]);
        } else {
            args.extend(["--oracle_hash", "keccak"]);
        }

        let output = StdCommand::new(&self.bb_binary).args(&args).output()?;

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

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_fhe_params::{build_bfv_params_from_set_arc, BfvPreset};
use e3_pvss::sample::generate_sample;
use e3_pvss::traits::{CircuitComputation, ReduceToZkpModulus};
use e3_zk_helpers::commitments::compute_pk_bfv_commitment;
use e3_zk_prover::{
    input_map, CompiledCircuit, PkBfvCircuit, Provable, SetupStatus, WitnessGenerator, ZkBackend,
    ZkConfig, ZkProver,
};
use num_bigint::BigInt;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::{fs, process::Command};

#[tokio::test]
async fn test_check_status_on_empty_dir() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    let status = backend.check_status().await;
    assert!(matches!(status, SetupStatus::FullSetupNeeded));
}

#[tokio::test]
async fn test_placeholder_circuits_creation() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    fs::create_dir_all(&backend.circuits_dir).await.unwrap();
    backend.download_circuits().await.unwrap();

    let circuit_path = backend.circuits_dir.join("pk_bfv.json");
    assert!(circuit_path.exists());

    let content = fs::read_to_string(&circuit_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_work_dir_creation_and_cleanup() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    let e3_id = "test-e3-123";
    let work_dir = backend.work_dir_for(e3_id);

    fs::create_dir_all(&work_dir).await.unwrap();
    assert!(work_dir.exists());

    fs::write(work_dir.join("test.txt"), "hello").await.unwrap();

    backend.cleanup_work_dir(e3_id).await.unwrap();
    assert!(!work_dir.exists());
}

#[tokio::test]
async fn test_version_info_persistence() {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());
    fs::create_dir_all(&backend.base_dir).await.unwrap();

    let info = backend.load_version_info().await;
    assert!(info.bb_version.is_none());

    let mut info = info;
    info.bb_version = Some("0.87.0".to_string());
    info.circuits_version = Some("0.1.0".to_string());
    info.save(&backend.base_dir.join("version.json"))
        .await
        .unwrap();

    let reloaded = backend.load_version_info().await;
    assert_eq!(reloaded.bb_version, Some("0.87.0".to_string()));
    assert_eq!(reloaded.circuits_version, Some("0.1.0".to_string()));
}

async fn find_bb() -> Option<PathBuf> {
    if let Ok(output) = Command::new("which").arg("bb").output().await {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        for path in [
            format!("{}/.bb/bb", home),
            format!("{}/.nargo/bin/bb", home),
            format!("{}/.enclave/noir/bin/bb", home),
        ] {
            if std::path::Path::new(&path).exists() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

async fn setup_test_prover(bb: &PathBuf) -> (ZkBackend, tempfile::TempDir) {
    let temp = tempdir().unwrap();
    let backend = ZkBackend::new(temp.path(), ZkConfig::default());

    fs::create_dir_all(&backend.circuits_dir).await.unwrap();
    fs::create_dir_all(backend.circuits_dir.join("vk"))
        .await
        .unwrap();
    fs::create_dir_all(&backend.work_dir).await.unwrap();
    fs::create_dir_all(backend.base_dir.join("bin"))
        .await
        .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(bb, &backend.bb_binary).unwrap();

    (backend, temp)
}

#[tokio::test]
async fn test_dummy_circuit() {
    let bb = match find_bb().await {
        Some(p) => p,
        None => {
            println!("skipping: bb not found");
            return;
        }
    };

    let (backend, _temp) = setup_test_prover(&bb).await;
    let fixtures = fixtures_dir();

    fs::copy(
        fixtures.join("dummy.json"),
        backend.circuits_dir.join("dummy.json"),
    )
    .await
    .unwrap();
    fs::copy(
        fixtures.join("dummy.vk"),
        backend.circuits_dir.join("vk").join("dummy.vk"),
    )
    .await
    .unwrap();

    let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json"))
        .await
        .unwrap();
    let witness_gen = WitnessGenerator::new();
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]);
    let witness = witness_gen
        .generate_witness(&circuit, inputs)
        .await
        .unwrap();

    let prover = ZkProver::new(&backend);
    let e3_id = "test-e3-001";

    let proof = prover
        .generate_proof("dummy", &witness, e3_id)
        .await
        .unwrap();
    let valid = prover.verify(&proof, e3_id).await.unwrap();

    assert!(valid);
    prover.cleanup(e3_id).await.unwrap();
}

#[tokio::test]
async fn test_pk_bfv_proof() {
    let bb = match find_bb().await {
        Some(p) => p,
        None => {
            println!("skipping: bb not found");
            return;
        }
    };

    let (backend, _temp) = setup_test_prover(&bb).await;
    let fixtures = fixtures_dir();

    fs::copy(
        fixtures.join("pk_bfv.json"),
        backend.circuits_dir.join("pk_bfv.json"),
    )
    .await
    .unwrap();
    fs::copy(
        fixtures.join("pk_bfv.vk"),
        backend.circuits_dir.join("vk").join("pk_bfv.vk"),
    )
    .await
    .unwrap();

    let preset = BfvPreset::InsecureDkg512;
    let params = build_bfv_params_from_set_arc(preset.into());
    let sample = generate_sample(&params);

    let prover = ZkProver::new(&backend);
    let circuit = PkBfvCircuit;
    let e3_id = "1";

    let proof = circuit
        .prove(&prover, &params, &sample.public_key, e3_id)
        .await
        .unwrap();

    let computation_output = circuit.compute(&params, &sample.public_key).unwrap();
    let reduced_witness = computation_output.witness.reduce_to_zkp_modulus();
    let commitment_calculated = compute_pk_bfv_commitment(
        &reduced_witness.pk0is,
        &reduced_witness.pk1is,
        computation_output.bits.pk_bit,
    );
    let commitment_from_proof =
        BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof.public_signals);

    assert_eq!(commitment_calculated, commitment_from_proof);

    let valid = circuit.verify(&prover, &proof, e3_id).await.unwrap();

    assert!(valid);
    prover.cleanup(e3_id).await.unwrap();
}

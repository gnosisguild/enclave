// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_fhe_params::{build_bfv_params_from_set_arc, BfvPreset};
use e3_noir_prover::{
    input_map, CircuitProverExt, CompiledCircuit, NoirConfig, NoirProver, NoirSetup, SetupStatus,
    WitnessGenerator,
};
use e3_pvss::circuits::pk_bfv::circuit::PkBfvCircuit;
use e3_pvss::sample::generate_sample;
use e3_pvss::traits::{CircuitComputation, ReduceToZkpModulus};
use e3_zk_helpers::commitments::compute_pk_bfv_commitment;
use num_bigint::BigInt;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::{fs, process::Command};

#[tokio::test]
async fn test_check_status_on_empty_dir() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    let status = setup.check_status().await;
    assert!(matches!(status, SetupStatus::FullSetupNeeded));
}

#[tokio::test]
async fn test_placeholder_circuits_creation() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    fs::create_dir_all(&setup.circuits_dir).await.unwrap();

    setup.download_circuits().await.unwrap();

    let circuit_path = setup.circuits_dir.join("pk_bfv.json");
    assert!(circuit_path.exists());

    let content = fs::read_to_string(&circuit_path).await.unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[tokio::test]
async fn test_work_dir_creation_and_cleanup() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    let e3_id = "test-e3-123";
    let work_dir = setup.work_dir_for(e3_id);

    fs::create_dir_all(&work_dir).await.unwrap();
    assert!(work_dir.exists());

    fs::write(work_dir.join("test.txt"), "hello").await.unwrap();

    setup.cleanup_work_dir(e3_id).await.unwrap();
    assert!(!work_dir.exists());
}

#[tokio::test]
async fn test_version_info_persistence() {
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());
    fs::create_dir_all(&setup.noir_dir).await.unwrap();

    let info = setup.load_version_info().await;
    assert!(info.bb_version.is_none());

    let mut info = info;
    info.bb_version = Some("0.87.0".to_string());
    info.circuits_version = Some("0.1.0".to_string());
    info.save(&setup.noir_dir.join("version.json"))
        .await
        .unwrap();

    let reloaded = setup.load_version_info().await;
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

#[tokio::test]
async fn test_dummy_circuit() {
    const CIRCUIT_NAME: &str = "dummy";

    // 1. Find bb
    let bb = match find_bb().await {
        Some(p) => p,
        None => {
            println!("⚠ Skipping: bb not found");
            return;
        }
    };

    // 2. Create NoirSetup
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    // 3. Init directories
    fs::create_dir_all(&setup.circuits_dir).await.unwrap();
    fs::create_dir_all(setup.circuits_dir.join("vk"))
        .await
        .unwrap();
    fs::create_dir_all(&setup.work_dir).await.unwrap();
    fs::create_dir_all(setup.noir_dir.join("bin"))
        .await
        .unwrap();

    // 4. Symlink bb
    #[cfg(unix)]
    std::os::unix::fs::symlink(&bb, &setup.bb_binary).unwrap();

    // 5. Copy circuit and VK from fixtures
    let fixtures = fixtures_dir();
    let circuit_src = fixtures.join(format!("{}.json", CIRCUIT_NAME));
    let vk_src = fixtures.join(format!("{}.vk", CIRCUIT_NAME));

    let circuit_dst = setup.circuits_dir.join(format!("{}.json", CIRCUIT_NAME));
    let vk_dst = setup
        .circuits_dir
        .join("vk")
        .join(format!("{}.vk", CIRCUIT_NAME));

    fs::copy(&circuit_src, &circuit_dst).await.unwrap();
    fs::copy(&vk_src, &vk_dst).await.unwrap();

    // 6. Load circuit
    let circuit = CompiledCircuit::from_file(&circuit_src).unwrap();

    // 7. Generate witness (NATIVE!)
    let witness_gen = WitnessGenerator::new();
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]);
    let witness = witness_gen.generate_witness(&circuit, inputs).unwrap();

    // 8. Create prover and generate proof
    let prover = NoirProver::new(&setup);
    let e3_id = "test-e3-001";
    let proof = prover
        .generate_proof(CIRCUIT_NAME, &witness, e3_id)
        .await
        .unwrap();

    // 9. Verify
    let valid = prover
        .verify_proof(CIRCUIT_NAME, &proof, e3_id)
        .await
        .unwrap();

    assert!(valid);

    // 10. Cleanup
    prover.cleanup(e3_id).await.unwrap();
}

#[tokio::test]
async fn test_pk_bfv_proof() {
    // 1. Find bb
    let bb = match find_bb().await {
        Some(p) => p,
        None => {
            println!("⚠ Skipping: bb not found");
            return;
        }
    };

    // 2. Create NoirSetup
    let temp = tempdir().unwrap();
    let setup = NoirSetup::new(temp.path(), NoirConfig::default());

    // 3. Init directories
    fs::create_dir_all(&setup.circuits_dir).await.unwrap();
    fs::create_dir_all(setup.circuits_dir.join("vk"))
        .await
        .unwrap();
    fs::create_dir_all(&setup.work_dir).await.unwrap();
    fs::create_dir_all(setup.noir_dir.join("bin"))
        .await
        .unwrap();

    // 4. Symlink bb
    #[cfg(unix)]
    std::os::unix::fs::symlink(&bb, &setup.bb_binary).unwrap();

    // 5. Copy circuit and VK from fixtures
    let fixtures = fixtures_dir();
    let circuit_src = fixtures.join("pk_bfv.json");
    let vk_src = fixtures.join("pk_bfv.vk");

    let circuit_dst = setup.circuits_dir.join("pk_bfv.json");
    let vk_dst = setup.circuits_dir.join("vk").join("pk_bfv.vk");

    fs::copy(&circuit_src, &circuit_dst).await.unwrap();
    fs::copy(&vk_src, &vk_dst).await.unwrap();

    // 6. Setup BFV params and generate test data
    let preset = BfvPreset::InsecureDkg512;
    let param_set = preset.into();
    let params = build_bfv_params_from_set_arc(param_set);

    let sample = generate_sample(&params);

    // 7. Create prover and circuit instance
    let prover = NoirProver::new(&setup);
    let circuit = PkBfvCircuit;

    // 8. Generate proof
    let e3_id = "1";
    let proof_result = circuit
        .prove(&prover, &params, &sample.public_key, e3_id)
        .await
        .unwrap();

    // 9. Confirm that the commitment matches
    let computation_output = circuit.compute(&params, &sample.public_key).unwrap();
    let reduced_witness = computation_output.witness.reduce_to_zkp_modulus();
    let commitment_calculated = compute_pk_bfv_commitment(
        &reduced_witness.pk0is,
        &reduced_witness.pk1is,
        computation_output.bits.pk_bit,
    );
    let commitment_from_proof =
        BigInt::from_bytes_be(num_bigint::Sign::Plus, &proof_result.output.as_ref());

    assert_eq!(
        commitment_calculated, commitment_from_proof,
        "Commitment should match!"
    );

    // 10. Verify proof using the trait-based API
    let valid = circuit
        .verify(&prover, &proof_result.proof, &proof_result.output, e3_id)
        .await
        .unwrap();

    assert!(valid, "Proof should be valid!");

    // 11. Cleanup
    prover.cleanup(e3_id).await.unwrap();
}

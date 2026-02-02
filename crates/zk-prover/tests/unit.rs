// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod common;

use common::fixtures_dir;
use e3_zk_prover::{input_map, CompiledCircuit, WitnessGenerator, ZkBackend, ZkConfig, ZkProver};
use tempfile::tempdir;
use tokio::fs;

mod unit {
    use super::*;

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

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_witness_generation_from_fixture() {
        let fixtures = fixtures_dir();
        let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json")).unwrap();

        let witness_gen = WitnessGenerator::new();
        let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]);
        let witness = witness_gen.generate_witness(&circuit, inputs).unwrap();

        assert!(witness.len() > 2);
        assert_eq!(witness[0], 0x1f);
        assert_eq!(witness[1], 0x8b);
    }

    #[test]
    fn test_witness_generation_wrong_sum_fails() {
        let fixtures = fixtures_dir();
        let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json")).unwrap();

        let witness_gen = WitnessGenerator::new();
        let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "10")]);
        let result = witness_gen.generate_witness(&circuit, inputs);

        assert!(result.is_err());
    }

    #[test]
    fn test_pk_bfv_witness_generation() {
        let fixtures = fixtures_dir();
        let circuit = CompiledCircuit::from_file(&fixtures.join("pk_bfv.json")).unwrap();

        assert!(!circuit.abi.parameters.is_empty());
    }

    #[tokio::test]
    async fn test_prover_without_bb_returns_error() {
        let temp = tempdir().unwrap();
        let backend = ZkBackend::new(temp.path(), ZkConfig::default());
        let prover = ZkProver::new(&backend);

        let result =
            prover.generate_proof(e3_events::CircuitName::PkBfv, b"fake witness", "test-e3");

        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), e3_zk_prover::ZkError::BbNotInstalled),
            "expected BbNotInstalled error"
        );

        let temp_path = temp.path().to_path_buf();
        drop(temp);
        assert!(!temp_path.exists());
    }
}

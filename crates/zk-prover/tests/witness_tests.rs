// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod common;

use common::fixtures_dir;
use e3_zk_prover::{input_map, CompiledCircuit, WitnessGenerator};

#[test]
fn test_witness_generation_from_fixture() {
    let fixtures = fixtures_dir();
    let circuit = CompiledCircuit::from_file(&fixtures.join("dummy.json")).unwrap();

    let witness_gen = WitnessGenerator::new();
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]).unwrap();
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
    let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "10")]).unwrap();
    let result = witness_gen.generate_witness(&circuit, inputs);

    assert!(result.is_err());
}

#[test]
fn test_compiled_circuit_from_fixture() {
    let fixtures = fixtures_dir();
    let circuit = CompiledCircuit::from_file(&fixtures.join("pk_bfv.json")).unwrap();

    assert!(
        !circuit.abi.parameters.is_empty(),
        "PkBfv circuit should have parameters"
    );
    assert!(circuit.abi.parameters.len() > 0);
}

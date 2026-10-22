// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_committee::CiphernodesCommitteeSize;
use e3_fhe_params::BfvPreset;
use e3_polynomial::{CrtPolynomial, Polynomial};
use e3_zk_config::threshold::{decrypted_shares_aggregation as c7, share_decryption as c6};
use e3_zk_prover::{inputs_json_to_input_map, CompiledCircuit, WitnessGenerator};
use e3_zk_witness::threshold::{decrypted_shares_aggregation as w7, share_decryption as w6};
use fhe::bfv::Ciphertext;
use fhe_math::rq::{Poly, PowerBasis};
use num_bigint::BigInt;

#[test]
#[ignore = "requires freshly compiled insecure-64/minimum C2/C3/C4 artifacts"]
fn dkg_proofs_bind_packed_prf_key_bits() {
    use e3_zk_config::{dkg, DkgInputType};
    use e3_zk_witness::dkg as witness;
    let preset = BfvPreset::InsecureThreshold64;
    let committee = CiphernodesCommitteeSize::Minimum;
    let config2 = dkg::share_computation::Configs::for_preset(preset, committee).unwrap();
    let config3 = dkg::share_encryption::Configs::for_preset(preset).unwrap();
    let config4 = dkg::share_decryption::Configs::for_preset(preset).unwrap();
    let data2 =
        e3_zk_samples::dkg::share_computation::generate(&config2, preset, DkgInputType::SecretKey)
            .unwrap();
    let data3 = e3_zk_samples::dkg::share_encryption::generate(
        &config2.params,
        &config3.params,
        preset,
        committee.values(),
        DkgInputType::SecretKey,
        preset.search_defaults().unwrap().z,
    )
    .unwrap();
    let data4 = e3_zk_samples::dkg::share_decryption::generate(
        &config4,
        preset,
        committee.values(),
        DkgInputType::SecretKey,
    )
    .unwrap();
    let cases = [
        (
            "sk_share_computation",
            witness::share_computation::Inputs::compute(&config2, &data2)
                .unwrap()
                .to_json(),
        ),
        (
            "share_encryption",
            witness::share_encryption::Inputs::compute(&config3, &data3)
                .unwrap()
                .to_json(),
        ),
        (
            "share_decryption",
            witness::share_decryption::Inputs::compute(&config4, &data4)
                .unwrap()
                .to_json(),
        ),
    ];
    let root = e3_zk_prover::test_utils::workspace_root().join("circuits/bin/dkg/target");
    let generator = WitnessGenerator::new();
    for (name, json) in cases {
        let compiled = CompiledCircuit::from_file(&root.join(format!("{name}.json"))).unwrap();
        generator
            .generate_witness(&compiled, inputs_json_to_input_map(&json).unwrap())
            .unwrap();
        let mut altered = json;
        if name == "share_encryption" {
            altered["prf_key"][0] = serde_json::json!(0);
        } else {
            altered["prf_keys"][0][0] = serde_json::json!(0);
        }
        assert!(
            generator
                .generate_witness(&compiled, inputs_json_to_input_map(&altered).unwrap())
                .is_err(),
            "{name} accepted a different transported key"
        );
    }
}

#[test]
#[ignore = "requires freshly compiled insecure-64/minimum C7 artifact"]
fn synchronized_c7_sums_weighted_shares_and_adds_c0_once() {
    let preset = BfvPreset::InsecureThreshold64;
    let config7 = c7::Configs::for_preset(preset, CiphernodesCommitteeSize::Minimum).unwrap();
    let config6 = c6::Configs::new(&config7.params).unwrap();
    let (data, parties) =
        e3_zk_samples::threshold::decrypted_shares_aggregation::generate_correlated(
            &config7, preset,
        )
        .unwrap();
    let inputs = w7::Inputs::compute(&config7, &data).unwrap();
    for (i, party) in parties.iter().enumerate() {
        let partial = w6::Inputs::compute(&config6, party).unwrap();
        assert_eq!(partial.c0_commitment, inputs.c0_commitment);
        assert_eq!(
            e3_zk_witness::compute_threshold_decryption_share_commitment(
                &partial.d_native_trunc,
                config6.bits.d_native_bit,
                config6.max_msg_non_zero_coeffs
            ),
            inputs.expected_d_commitments[i]
        );
    }
    let path = e3_zk_prover::test_utils::workspace_root()
        .join("circuits/bin/threshold/target/decrypted_shares_aggregation.json");
    let compiled = CompiledCircuit::from_file(&path).unwrap();
    let generator = WitnessGenerator::new();
    let json = inputs.to_json();
    generator
        .generate_witness(&compiled, inputs_json_to_input_map(&json).unwrap())
        .unwrap();
    let mut forged = json.clone();
    forged["c0"][0]["coefficients"][0] = serde_json::json!(0);
    assert!(generator
        .generate_witness(&compiled, inputs_json_to_input_map(&forged).unwrap())
        .is_err());
    let mut forged = json;
    forged["message"]["coefficients"][0] = serde_json::json!(0);
    assert!(generator
        .generate_witness(&compiled, inputs_json_to_input_map(&forged).unwrap())
        .is_err());
}

#[test]
#[ignore = "requires freshly compiled insecure-64/minimum C6 artifact"]
fn synchronized_c6_accepts_released_fhe_shares_and_rejects_tampering() {
    let preset = BfvPreset::InsecureThreshold64;
    let committee = CiphernodesCommitteeSize::Minimum;
    let config = c6::Configs::for_preset(preset).unwrap();
    let mut data = e3_zk_samples::threshold::share_decryption::generate(
        &config.params,
        preset,
        committee.values(),
    )
    .unwrap();
    // Nonconsecutive IDs exercise a large centered modular Lagrange weight, not just small
    // integer binomial coefficients from the first T+1 parties.
    data.decryptors = vec![1, 3];
    data.d_share = e3_zk_samples::threshold::share_decryption::synchronized_decryption_share(
        &data,
        &config.params,
        preset,
    )
    .unwrap();
    let prepared = w6::Inputs::compute(&config, &data).unwrap();
    let path = e3_zk_prover::test_utils::workspace_root()
        .join("circuits/bin/threshold/target/share_decryption.json");
    let compiled = CompiledCircuit::from_file(&path).unwrap();
    let json = prepared.to_json();
    let generator = WitnessGenerator::new();
    generator
        .generate_witness(&compiled, inputs_json_to_input_map(&json).unwrap())
        .unwrap();
    for field in ["r", "d", "e_fresh", "e_quot"] {
        let mut forged = json.clone();
        forged[field][0]["coefficients"][0] = serde_json::json!("9999999999999999999999999");
        assert!(
            generator
                .generate_witness(&compiled, inputs_json_to_input_map(&forged).unwrap())
                .is_err(),
            "accepted altered {field}"
        );
    }
    let mut forged = json.clone();
    forged["outgoing_keys"][2] = serde_json::json!("0");
    assert!(generator
        .generate_witness(&compiled, inputs_json_to_input_map(&forged).unwrap())
        .is_err());
    let mut forged = json;
    forged["decryptors"][0] = serde_json::json!(0);
    assert!(generator
        .generate_witness(&compiled, inputs_json_to_input_map(&forged).unwrap())
        .is_err());
}

#[test]
#[ignore = "requires prepared insecure-64/minimum C6/C7 artifacts"]
fn c6_c7_zero_products_execute_and_nondefault_widths_reject_fixed_abi() {
    let root = e3_zk_prover::test_utils::workspace_root().join("circuits/bin");
    let active: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".active-preset.json")).unwrap())
            .unwrap();
    assert_eq!(active["preset"], "insecure-64");
    assert_eq!(active["committee"], "minimum");
    let mut config7 = c7::Configs::for_preset(
        BfvPreset::InsecureThreshold64,
        CiphernodesCommitteeSize::Minimum,
    )
    .unwrap();
    let mut config6 = c6::Configs::new(&config7.params).unwrap();
    let poly = Poly::<PowerBasis>::zero(config7.params.context_at_level(0).unwrap());
    let crt = CrtPolynomial::new(vec![
        Polynomial::new(vec![
            BigInt::from(0);
            config6.params.degree()
        ]);
        config6.params.moduli().len()
    ]);
    let data6 = w6::ShareDecryptionCircuitData {
        ciphertext: Ciphertext::new(
            vec![poly.clone().into_ntt(), poly.clone().into_ntt()],
            &config6.params,
        )
        .unwrap(),
        public_key: fhe::bfv::PublicKey::new(
            &fhe::bfv::SecretKey::random(&config6.params, &mut rand::rng()),
            &mut rand::rng(),
        ),
        s: crt.clone(),
        e: crt.clone(),
        e_signed: vec![BigInt::from(0); config6.params.degree()],
        d_share: crt,
        domain_hi: 0,
        domain_lo: 0,
        committee: CiphernodesCommitteeSize::Minimum.values(),
        party_idx: 0,
        decryptors: (1..=config7.committee.values().threshold as u32 + 1).collect(),
        outgoing_prf_keys: vec![vec![1; 32]; config7.committee.values().n],
        incoming_prf_keys: vec![vec![1; 32]; config7.committee.values().n],
    };
    let count = config7.committee.values().threshold + 1;
    let data7 = w7::DecryptedSharesAggregationCircuitData {
        ct0: poly.clone(),
        d_share_polys: vec![poly; count],
        reconstructing_parties: (1..=count).collect(),
        message_vec: vec![0],
    };
    let compiled6 =
        CompiledCircuit::from_file(&root.join("threshold/target/share_decryption.json")).unwrap();
    let compiled7 = CompiledCircuit::from_file(
        &root.join("threshold/target/decrypted_shares_aggregation.json"),
    )
    .unwrap();
    let generator = WitnessGenerator::new();
    let json6 = w6::Inputs::compute(&config6, &data6).unwrap().to_json();
    let json7 = w7::Inputs::compute(&config7, &data7).unwrap().to_json();
    generator
        .generate_witness(&compiled6, inputs_json_to_input_map(&json6).unwrap())
        .unwrap();
    generator
        .generate_witness(&compiled7, inputs_json_to_input_map(&json7).unwrap())
        .unwrap();
    let mut wrong_quotient = json6;
    wrong_quotient["r"][0]["coefficients"][0] = serde_json::json!(1);
    assert!(generator
        .generate_witness(
            &compiled6,
            inputs_json_to_input_map(&wrong_quotient).unwrap()
        )
        .is_err());
    let mut wrong_message = json7;
    wrong_message["message"]["coefficients"][0] = serde_json::json!(1);
    assert!(generator
        .generate_witness(
            &compiled7,
            inputs_json_to_input_map(&wrong_message).unwrap()
        )
        .is_err());
    config6.max_msg_non_zero_coeffs = 7;
    config7.max_msg_non_zero_coeffs = 7;
    for (compiled, json) in [
        (
            &compiled6,
            w6::Inputs::compute(&config6, &data6).unwrap().to_json(),
        ),
        (
            &compiled7,
            w7::Inputs::compute(&config7, &data7).unwrap().to_json(),
        ),
    ] {
        let error = generator
            .generate_witness(compiled, inputs_json_to_input_map(&json).unwrap())
            .unwrap_err();
        assert!(error.to_string().contains("ABI encode"), "{error}");
    }
}

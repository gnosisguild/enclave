// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Serialization module for CRISP ZK inputs data.
//!
//! This module handles the serialization of inputs data to JSON format.

use crate::ciphertext_addition::CiphertextAdditionInputs;
use eyre::{Context, Result};
use greco::bounds::GrecoBounds;
use greco::bounds::GrecoCryptographicParameters;
use greco::vectors::GrecoVectors;
use num_bigint::BigInt;
use serde::Serialize;

#[derive(Serialize)]
pub struct ZKInputs {
    ct_add: serde_json::Value,
    params: serde_json::Value,
    ct0is: Vec<serde_json::Value>,
    ct1is: Vec<serde_json::Value>,
    pk0is: Vec<serde_json::Value>,
    pk1is: Vec<serde_json::Value>,
    r1is: Vec<serde_json::Value>,
    r2is: Vec<serde_json::Value>,
    p1is: Vec<serde_json::Value>,
    p2is: Vec<serde_json::Value>,
    u: serde_json::Value,
    e0: serde_json::Value,
    e1: serde_json::Value,
    k1: serde_json::Value,
}

/// Converts a 1D vector of BigInt to a vector of strings.
fn to_string_1d_vec(vec: &[BigInt]) -> Vec<String> {
    vec.iter().map(|x| x.to_string()).collect()
}

/// Constructs a ZKInputs from GRECO bounds and vectors.
///
/// # Arguments
/// * `crypto_params` - Cryptographic parameters from GRECO
/// * `bounds` - Bounds from GRECO
/// * `vectors_standard` - Standard form vectors from GRECO
/// * `ciphertext_addition_inputs_standard` - Standard form ciphertext addition inputs
///
/// # Returns
/// A ZKInputs struct ready for JSON serialization
pub fn construct_inputs(
    crypto_params: &GrecoCryptographicParameters,
    bounds: &GrecoBounds,
    vectors_standard: &GrecoVectors,
    ciphertext_addition_inputs_standard: &CiphertextAdditionInputs,
) -> ZKInputs {
    let mut params_json = serde_json::Map::new();

    // Add crypto params.
    let crypto_json = serde_json::json!({
        "q_mod_t": crypto_params.q_mod_t.to_string(),
        "qis": crypto_params.moduli.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "k0is": crypto_params.k0is.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
    });
    params_json.insert("crypto".to_string(), crypto_json);

    // Add bounds.
    let bounds_json = serde_json::json!({
        "e_bound": bounds.e_bound.to_string(),
        "u_bound": bounds.u_bound.to_string(),
        "k1_low_bound": bounds.k1_low_bound.to_string(),
        "k1_up_bound": bounds.k1_up_bound.to_string(),
        "p1_bounds": bounds.p1_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "p2_bounds": bounds.p2_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "pk_bounds": bounds.pk_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "r1_low_bounds": bounds.r1_low_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "r1_up_bounds": bounds.r1_up_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "r2_bounds": bounds.r2_bounds.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
    });
    params_json.insert("bounds".to_string(), bounds_json);

    let mut ciphertext_addition_params_json = serde_json::Map::new();
    ciphertext_addition_params_json.insert(
        "prev_ct0is".to_string(),
        ciphertext_addition_inputs_standard
            .prev_ct0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "prev_ct1is".to_string(),
        ciphertext_addition_inputs_standard
            .prev_ct1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "sum_ct0is".to_string(),
        ciphertext_addition_inputs_standard
            .sum_ct0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "sum_ct1is".to_string(),
        ciphertext_addition_inputs_standard
            .sum_ct1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "r0is".to_string(),
        ciphertext_addition_inputs_standard
            .r0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "r1is".to_string(),
        ciphertext_addition_inputs_standard
            .r1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "r_bound".to_string(),
        serde_json::json!(ciphertext_addition_inputs_standard.r_bound),
    );

    ZKInputs {
        ct_add: serde_json::Value::Object(ciphertext_addition_params_json),
        params: serde_json::Value::Object(params_json),
        ct0is: vectors_standard
            .ct0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        ct1is: vectors_standard
            .ct1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        pk0is: vectors_standard
            .pk0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        pk1is: vectors_standard
            .pk1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        r1is: vectors_standard
            .r1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        r2is: vectors_standard
            .r2is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        p1is: vectors_standard
            .p1is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        p2is: vectors_standard
            .p2is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
        u: serde_json::json!({
            "coefficients": to_string_1d_vec(&vectors_standard.u)
        }),
        e0: serde_json::json!({
            "coefficients": to_string_1d_vec(&vectors_standard.e0)
        }),
        e1: serde_json::json!({
            "coefficients": to_string_1d_vec(&vectors_standard.e1)
        }),
        k1: serde_json::json!({
            "coefficients": to_string_1d_vec(&vectors_standard.k1)
        }),
    }
}

/// Serializes a ZKInputs to JSON string.
///
/// # Arguments
/// * `inputs` - The ZKInputs to serialize
///
/// # Returns
/// JSON string representation of the inputs
pub fn serialize_inputs_to_json(inputs: &ZKInputs) -> Result<String> {
    serde_json::to_string(inputs).with_context(|| "Failed to serialize inputs to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use serde_json::Value;

    fn create_mock_crypto_params() -> GrecoCryptographicParameters {
        GrecoCryptographicParameters {
            q_mod_t: BigInt::from(12345),
            moduli: vec![1000007u64, 1000009u64],
            k0is: vec![1u64, 2u64],
        }
    }

    fn create_mock_bounds() -> GrecoBounds {
        GrecoBounds {
            e_bound: 100u64,
            u_bound: 200u64,
            k1_low_bound: 10i64,
            k1_up_bound: 20u64,
            p1_bounds: vec![30u64, 40u64],
            p2_bounds: vec![50u64, 60u64],
            pk_bounds: vec![70u64, 80u64],
            r1_low_bounds: vec![90i64, 100i64],
            r1_up_bounds: vec![110u64, 120u64],
            r2_bounds: vec![130u64, 140u64],
        }
    }

    fn create_mock_vectors() -> GrecoVectors {
        GrecoVectors {
            ct0is: vec![
                vec![BigInt::from(1), BigInt::from(2)],
                vec![BigInt::from(3), BigInt::from(4)],
            ],
            ct1is: vec![
                vec![BigInt::from(5), BigInt::from(6)],
                vec![BigInt::from(7), BigInt::from(8)],
            ],
            pk0is: vec![
                vec![BigInt::from(9), BigInt::from(10)],
                vec![BigInt::from(11), BigInt::from(12)],
            ],
            pk1is: vec![
                vec![BigInt::from(13), BigInt::from(14)],
                vec![BigInt::from(15), BigInt::from(16)],
            ],
            k0is: vec![BigInt::from(17), BigInt::from(18)],
            r1is: vec![
                vec![BigInt::from(21), BigInt::from(22)],
                vec![BigInt::from(23), BigInt::from(24)],
            ],
            r2is: vec![
                vec![BigInt::from(25), BigInt::from(26)],
                vec![BigInt::from(27), BigInt::from(28)],
            ],
            p1is: vec![
                vec![BigInt::from(29), BigInt::from(30)],
                vec![BigInt::from(31), BigInt::from(32)],
            ],
            p2is: vec![
                vec![BigInt::from(33), BigInt::from(34)],
                vec![BigInt::from(35), BigInt::from(36)],
            ],
            u: vec![BigInt::from(37), BigInt::from(38)],
            e0: vec![BigInt::from(39), BigInt::from(40)],
            e1: vec![BigInt::from(41), BigInt::from(42)],
            k1: vec![BigInt::from(43), BigInt::from(44)],
        }
    }

    fn create_mock_ciphertext_addition_inputs() -> CiphertextAdditionInputs {
        CiphertextAdditionInputs {
            prev_ct0is: vec![vec![BigInt::from(1), BigInt::from(2)]],
            prev_ct1is: vec![vec![BigInt::from(3), BigInt::from(4)]],
            sum_ct0is: vec![vec![BigInt::from(5), BigInt::from(6)]],
            sum_ct1is: vec![vec![BigInt::from(7), BigInt::from(8)]],
            r0is: vec![vec![BigInt::from(9), BigInt::from(10)]],
            r1is: vec![vec![BigInt::from(11), BigInt::from(12)]],
            r_bound: 1,
        }
    }

    #[test]
    fn test_construct_inputs_basic() {
        let crypto_params = create_mock_crypto_params();
        let bounds = create_mock_bounds();
        let vectors = create_mock_vectors();
        let ciphertext_addition_inputs = create_mock_ciphertext_addition_inputs();

        let inputs = construct_inputs(
            &crypto_params,
            &bounds,
            &vectors,
            &ciphertext_addition_inputs,
        );

        // Verify basic structure.
        assert!(inputs.params.is_object());
        assert!(inputs.ct_add.is_object());
        assert_eq!(inputs.ct0is.len(), 2);
        assert_eq!(inputs.ct1is.len(), 2);
        assert_eq!(inputs.pk0is.len(), 2);
        assert_eq!(inputs.pk1is.len(), 2);
        assert!(inputs.u.is_object());
        assert!(inputs.e0.is_object());
        assert!(inputs.e1.is_object());
        assert!(inputs.k1.is_object());
    }

    #[test]
    fn test_serialize_inputs_to_json() {
        let crypto_params = create_mock_crypto_params();
        let bounds = create_mock_bounds();
        let vectors = create_mock_vectors();
        let ciphertext_addition_inputs = create_mock_ciphertext_addition_inputs();

        let inputs = construct_inputs(
            &crypto_params,
            &bounds,
            &vectors,
            &ciphertext_addition_inputs,
        );

        let json_result = serialize_inputs_to_json(&inputs);
        assert!(json_result.is_ok());

        let json_string = json_result.unwrap();
        assert!(!json_string.is_empty());

        // Verify it's valid JSON.
        let parsed: Value = serde_json::from_str(&json_string).unwrap();
        assert!(parsed.is_object());

        // Verify required fields exist.
        assert!(parsed.get("params").is_some());
        assert!(parsed.get("ct_add").is_some());
        assert!(parsed.get("ct0is").is_some());
        assert!(parsed.get("ct1is").is_some());
        assert!(parsed.get("pk0is").is_some());
        assert!(parsed.get("pk1is").is_some());
    }
}

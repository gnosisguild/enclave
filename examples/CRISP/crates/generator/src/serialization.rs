// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Serialization module for crisp inputs data
//!
//! This module handles the serialization of inputs data to JSON format.

use crate::ciphertext_addition::CiphertextAdditionParams;
use greco::bounds::GrecoBounds;
use greco::bounds::GrecoCryptographicParameters;
use greco::vectors::GrecoVectors;
use num_bigint::BigInt;
use serde::Serialize;

#[derive(Serialize)]
pub struct CrispZKInputs {
    ct_add_params: serde_json::Value,
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

/// Convert a 1D vector of BigInt to a vector of strings
fn to_string_1d_vec(vec: &[BigInt]) -> Vec<String> {
    vec.iter().map(|x| x.to_string()).collect()
}

/// Constructs a CrispZKInputs from GRECO bounds and vectors
pub fn construct_inputs(
    crypto_params: &GrecoCryptographicParameters,
    bounds: &GrecoBounds,
    vectors_standard: &GrecoVectors,
    ciphertext_addition_vectors_standard: &CiphertextAdditionParams,
) -> CrispZKInputs {
    let mut params_json = serde_json::Map::new();

    // Add crypto params
    let crypto_json = serde_json::json!({
        "q_mod_t": crypto_params.q_mod_t.to_string(),
        "qis": crypto_params.moduli.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
        "k0is": crypto_params.k0is.iter().map(|b| b.to_string()).collect::<Vec<_>>(),
    });
    params_json.insert("crypto".to_string(), crypto_json);

    // Add bounds
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
        "old_ct0is".to_string(),
        ciphertext_addition_vectors_standard
            .old_ct0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "old_ct1is".to_string(),
        ciphertext_addition_vectors_standard
            .old_ct1is
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
        ciphertext_addition_vectors_standard
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
        ciphertext_addition_vectors_standard
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
        "sum_r0is".to_string(),
        ciphertext_addition_vectors_standard
            .sum_r0is
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v)
                })
            })
            .collect(),
    );
    ciphertext_addition_params_json.insert(
        "sum_r1is".to_string(),
        ciphertext_addition_vectors_standard
            .sum_r1is
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
        serde_json::json!(ciphertext_addition_vectors_standard.r_bound),
    );

    CrispZKInputs {
        ct_add_params: serde_json::Value::Object(ciphertext_addition_params_json),
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

/// Serializes a CrispZKInputs to JSON string
pub fn serialize_inputs_to_json(inputs: &CrispZKInputs) -> Result<String, String> {
    serde_json::to_string(inputs).map_err(|e| format!("Failed to serialize inputs: {}", e))
}

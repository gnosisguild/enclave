// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Serialization module for CRISP ZK inputs data.
//!
//! This module handles the serialization of inputs data to JSON format.

use crate::ciphertext_addition::CiphertextAdditionWitness;
use e3_zk_helpers::{threshold::UserDataEncryptionComputationOutput, to_string_1d_vec};
use eyre::{Context, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct ZKInputs {
    prev_ct0is: Vec<serde_json::Value>,
    prev_ct1is: Vec<serde_json::Value>,
    prev_ct_commitment: String,
    sum_ct0is: Vec<serde_json::Value>,
    sum_ct1is: Vec<serde_json::Value>,
    sum_r0is: Vec<serde_json::Value>,
    sum_r1is: Vec<serde_json::Value>,
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
    e0is: Vec<serde_json::Value>,
    e0_quotients: Vec<serde_json::Value>,
    e1: serde_json::Value,
    k1: serde_json::Value,
    pk_commitment: String,
}

/// Constructs a ZKInputs from user data encryption computation output and ciphertext addition witness.
///
/// # Arguments
/// * `user_data_encryption_computation_output` - User data encryption computation output
/// * `ciphertext_addition_witness` - Ciphertext addition witness
///
/// # Returns
/// A ZKInputs struct ready for JSON serialization
pub fn construct_inputs(
    user_data_encryption_computation_output: &UserDataEncryptionComputationOutput,
    ciphertext_addition_witness: &CiphertextAdditionWitness,
) -> ZKInputs {
    let mut params_json = serde_json::Map::new();

    let bounds = &user_data_encryption_computation_output.bounds;
    let witness = &user_data_encryption_computation_output.witness;

    // Add bounds.
    let bounds_json = serde_json::json!({
        "e0_bound": bounds.e0_bound.to_string(),
        "e1_bound": bounds.e1_bound.to_string(),
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

    ZKInputs {
        prev_ct0is: ciphertext_addition_witness
            .prev_ct0is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        prev_ct1is: ciphertext_addition_witness
            .prev_ct1is
            .limbs
            .iter()
            .map(|limb| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(limb.coefficients())
                })
            })
            .collect(),
        prev_ct_commitment: ciphertext_addition_witness.prev_ct_commitment.to_string(),
        sum_ct0is: ciphertext_addition_witness
            .sum_ct0is
            .limbs
            .iter()
            .map(|limb| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(limb.coefficients())
                })
            })
            .collect(),
        sum_ct1is: ciphertext_addition_witness
            .sum_ct1is
            .limbs
            .iter()
            .map(|limb| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(limb.coefficients())
                })
            })
            .collect(),
        sum_r0is: ciphertext_addition_witness
            .r0is
            .limbs
            .iter()
            .map(|limb| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(limb.coefficients())
                })
            })
            .collect(),
        sum_r1is: ciphertext_addition_witness
            .r1is
            .limbs
            .iter()
            .map(|limb| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(limb.coefficients())
                })
            })
            .collect(),
        params: serde_json::Value::Object(params_json),
        ct0is: witness
            .ct0is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        ct1is: witness
            .ct1is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        pk0is: witness
            .pk0is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        pk1is: witness
            .pk1is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        r1is: witness
            .r1is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        r2is: witness
            .r2is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        p1is: witness
            .p1is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        p2is: witness
            .p2is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        e0is: witness
            .e0is
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        u: serde_json::json!({
            "coefficients": to_string_1d_vec(witness.u.coefficients())
        }),
        e0: serde_json::json!({
            "coefficients": to_string_1d_vec(witness.e0.coefficients())
        }),
        e0_quotients: witness
            .e0_quotients
            .limbs
            .iter()
            .map(|v| {
                serde_json::json!({
                    "coefficients": to_string_1d_vec(v.coefficients())
                })
            })
            .collect(),
        e1: serde_json::json!({
            "coefficients": to_string_1d_vec(witness.e1.coefficients())
        }),
        k1: serde_json::json!({
            "coefficients": to_string_1d_vec(witness.k1.coefficients())
        }),
        pk_commitment: witness.pk_commitment.to_string(),
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
    use e3_polynomial::{CrtPolynomial, Polynomial};
    use e3_zk_helpers::threshold::{Bounds, UserDataEncryptionComputationOutput, Witness};
    use num_bigint::BigInt;
    use num_bigint::BigUint;
    use serde_json::Value;

    fn limb(n: i32) -> CrtPolynomial {
        let p = Polynomial::new(vec![BigInt::from(n), BigInt::from(n + 1)]);
        CrtPolynomial::new(vec![p.clone(), p])
    }

    fn create_mock_user_data_encryption_computation_output() -> UserDataEncryptionComputationOutput
    {
        let bounds = Bounds {
            pk_bounds: vec![BigUint::from(70u64), BigUint::from(80u64)],
            u_bound: BigUint::from(200u64),
            e0_bound: BigUint::from(100u64),
            e1_bound: BigUint::from(100u64),
            k1_low_bound: BigUint::from(10u64),
            k1_up_bound: BigUint::from(20u64),
            r1_low_bounds: vec![BigUint::from(90u64), BigUint::from(100u64)],
            r1_up_bounds: vec![BigUint::from(110u64), BigUint::from(120u64)],
            r2_bounds: vec![BigUint::from(130u64), BigUint::from(140u64)],
            p1_bounds: vec![BigUint::from(30u64), BigUint::from(40u64)],
            p2_bounds: vec![BigUint::from(50u64), BigUint::from(60u64)],
        };
        let bits = e3_zk_helpers::threshold::Bits {
            pk_bit: 8,
            ct_bit: 8,
            u_bit: 8,
            e0_bit: 8,
            e1_bit: 8,
            k_bit: 8,
            r1_bit: 8,
            r2_bit: 8,
            p1_bit: 8,
            p2_bit: 8,
        };
        let witness = Witness {
            pk0is: limb(9),
            pk1is: limb(13),
            ct0is: limb(1),
            ct1is: limb(5),
            r1is: limb(21),
            r2is: limb(25),
            p1is: limb(29),
            p2is: limb(33),
            e0is: limb(43),
            e0_quotients: limb(47),
            e0: Polynomial::new(vec![BigInt::from(39), BigInt::from(40)]),
            e1: Polynomial::new(vec![BigInt::from(41), BigInt::from(42)]),
            u: Polynomial::new(vec![BigInt::from(37), BigInt::from(38)]),
            k1: Polynomial::new(vec![BigInt::from(43), BigInt::from(44)]),
            pk_commitment: BigInt::from(45),
            ct_commitment: BigInt::from(0),
            ciphertext: vec![],
        };
        UserDataEncryptionComputationOutput {
            bounds,
            bits,
            witness,
        }
    }

    fn create_mock_ciphertext_addition_inputs() -> CiphertextAdditionWitness {
        CiphertextAdditionWitness {
            prev_ct0is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(1), BigInt::from(2)]),
                Polynomial::new(vec![BigInt::from(1), BigInt::from(2)]),
            ]),
            prev_ct1is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(3), BigInt::from(4)]),
                Polynomial::new(vec![BigInt::from(3), BigInt::from(4)]),
            ]),
            prev_ct_commitment: BigInt::from(0),
            sum_ct0is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(5), BigInt::from(6)]),
                Polynomial::new(vec![BigInt::from(5), BigInt::from(6)]),
            ]),
            sum_ct1is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(7), BigInt::from(8)]),
                Polynomial::new(vec![BigInt::from(7), BigInt::from(8)]),
            ]),
            r0is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(9), BigInt::from(10)]),
                Polynomial::new(vec![BigInt::from(9), BigInt::from(10)]),
            ]),
            r1is: CrtPolynomial::new(vec![
                Polynomial::new(vec![BigInt::from(11), BigInt::from(12)]),
                Polynomial::new(vec![BigInt::from(11), BigInt::from(12)]),
            ]),
        }
    }

    #[test]
    fn test_construct_inputs_basic() {
        let user_data_encryption_output = create_mock_user_data_encryption_computation_output();
        let ciphertext_addition_inputs = create_mock_ciphertext_addition_inputs();

        let inputs = construct_inputs(&user_data_encryption_output, &ciphertext_addition_inputs);

        assert!(inputs.params.is_object());
        assert_eq!(inputs.prev_ct0is.len(), 2);
        assert_eq!(inputs.prev_ct1is.len(), 2);
        assert_eq!(inputs.sum_ct0is.len(), 2);
        assert_eq!(inputs.sum_ct1is.len(), 2);
        assert_eq!(inputs.sum_r0is.len(), 2);
        assert_eq!(inputs.sum_r1is.len(), 2);
        assert_eq!(inputs.ct0is.len(), 2);
        assert_eq!(inputs.ct1is.len(), 2);
        assert_eq!(inputs.pk0is.len(), 2);
        assert_eq!(inputs.pk1is.len(), 2);
        assert!(inputs.u.is_object());
        assert!(inputs.e0.is_object());
        assert!(inputs.e1.is_object());
        assert!(inputs.k1.is_object());
        assert!(!inputs.pk_commitment.is_empty());
    }

    #[test]
    fn test_serialize_inputs_to_json() {
        let user_data_encryption_output = create_mock_user_data_encryption_computation_output();
        let ciphertext_addition_inputs = create_mock_ciphertext_addition_inputs();

        let inputs = construct_inputs(&user_data_encryption_output, &ciphertext_addition_inputs);

        let json_result = serialize_inputs_to_json(&inputs);
        assert!(json_result.is_ok());

        let json_string = json_result.unwrap();
        assert!(!json_string.is_empty());

        let parsed: Value = serde_json::from_str(&json_string).unwrap();
        assert!(parsed.is_object());

        assert!(parsed.get("params").is_some());
        assert!(parsed.get("prev_ct0is").is_some());
        assert!(parsed.get("prev_ct1is").is_some());
        assert!(parsed.get("sum_ct0is").is_some());
        assert!(parsed.get("sum_ct1is").is_some());
        assert!(parsed.get("sum_r0is").is_some());
        assert!(parsed.get("sum_r1is").is_some());
        assert!(parsed.get("ct0is").is_some());
        assert!(parsed.get("ct1is").is_some());
        assert!(parsed.get("pk0is").is_some());
        assert!(parsed.get("pk1is").is_some());
        assert!(parsed.get("pk_commitment").is_some());
    }
}

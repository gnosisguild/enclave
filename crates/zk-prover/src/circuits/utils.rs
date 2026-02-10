// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use std::collections::BTreeMap;

use crate::error::ZkError;
use acir::FieldElement;
use noirc_abi::{input_parser::InputValue, InputMap};
use acvm::AcirField;
use e3_polynomial::{CrtPolynomial, Polynomial};
use num_bigint::BigInt;

/// Converts inputs JSON (from `Inputs::to_json()`) to `InputMap` for Noir ABI.
/// Expects the same structure: CRT fields as arrays of `{coefficients: [...]}`,
/// polynomial fields as `{coefficients: [...]}`.
pub fn inputs_json_to_input_map(json: &serde_json::Value) -> Result<InputMap, ZkError> {
    let obj = json
        .as_object()
        .ok_or_else(|| ZkError::SerializationError("inputs json must be an object".into()))?;

    let mut inputs = InputMap::new();
    for (key, value) in obj {
        let input_value = json_value_to_input_value(value)?;
        inputs.insert(key.clone(), input_value);
    }
    Ok(inputs)
}

fn json_value_to_input_value(v: &serde_json::Value) -> Result<InputValue, ZkError> {
    if let Some(arr) = v.as_array() {
        let items = arr
            .iter()
            .map(json_value_to_input_value)
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(InputValue::Vec(items));
    }
    if let Some(obj) = v.as_object() {
        if let Some(coeffs) = obj.get("coefficients") {
            let coeff_arr = coeffs
                .as_array()
                .ok_or_else(|| ZkError::SerializationError("coefficients must be array".into()))?;
            let fields = coeff_arr
                .iter()
                .map(|c| {
                    let s = c.as_str().ok_or_else(|| {
                        ZkError::SerializationError("coefficient must be string".into())
                    })?;
                    FieldElement::try_from_str(s)
                        .map(InputValue::Field)
                        .ok_or_else(|| {
                            ZkError::SerializationError(format!("invalid field element: {}", s))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut map = BTreeMap::new();
            map.insert("coefficients".into(), InputValue::Vec(fields));
            return Ok(InputValue::Struct(map));
        }
    }
    Err(ZkError::SerializationError(
        "unexpected json structure".into(),
    ))
}

pub fn bigint_to_field_value(b: &BigInt) -> InputValue {
    InputValue::Field(acvm::FieldElement::from_hex(&b.to_str_radix(16)).unwrap())
}

pub fn vec3d_to_input_value(v: &Vec<Vec<Vec<BigInt>>>) -> InputValue {
    InputValue::Vec(
        v.iter()
            .map(|v2| {
                InputValue::Vec(
                    v2.iter()
                        .map(|v3| {
                            InputValue::Vec(
                                v3.iter().map(bigint_to_field_value).collect(),
                            )
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

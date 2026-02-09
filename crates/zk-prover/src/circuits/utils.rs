// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use std::collections::BTreeMap;

use crate::error::ZkError;
use acir::FieldElement;
use e3_polynomial::{CrtPolynomial, Polynomial};
use noirc_abi::input_parser::InputValue;

pub fn crt_polynomial_to_array(crt_poly: &CrtPolynomial) -> Result<InputValue, ZkError> {
    let mut polynomials = Vec::with_capacity(crt_poly.limbs.len());

    for limb in &crt_poly.limbs {
        let coeffs = limb.coefficients();
        let mut field_coeffs = Vec::with_capacity(coeffs.len());

        for b in coeffs {
            let s = b.to_string();
            let field = FieldElement::try_from_str(&s).ok_or_else(|| {
                ZkError::SerializationError(format!("invalid field element: {}", s))
            })?;
            field_coeffs.push(InputValue::Field(field));
        }

        let mut fields = BTreeMap::new();
        fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));
        polynomials.push(InputValue::Struct(fields));
    }

    Ok(InputValue::Vec(polynomials))
}

pub fn polynomial_to_input_value(poly: &Polynomial) -> Result<InputValue, ZkError> {
    let coeffs = poly.coefficients();
    let mut field_coeffs = Vec::with_capacity(coeffs.len());

    for b in coeffs {
        let s = b.to_string();
        let field = FieldElement::try_from_str(&s)
            .ok_or_else(|| ZkError::SerializationError(format!("invalid field element: {}", s)))?;
        field_coeffs.push(InputValue::Field(field));
    }

    let mut fields = BTreeMap::new();
    fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));
    Ok(InputValue::Struct(fields))
}

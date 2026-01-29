// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::error::ZkError;
use crate::traits::Provable;
use acir::FieldElement;
use async_trait::async_trait;
use e3_pvss::circuits::pk_bfv::circuit::PkBfvCircuit;
use e3_pvss::traits::{CircuitComputation, ReduceToZkpModulus};
use fhe::bfv::{BfvParameters, PublicKey};
use noirc_abi::input_parser::InputValue;
use noirc_abi::InputMap;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use std::sync::Arc;

#[async_trait]
impl Provable for PkBfvCircuit {
    type Params = Arc<BfvParameters>;
    type Input = PublicKey;

    fn circuit_name(&self) -> &'static str {
        "pk_bfv"
    }

    fn build_witness(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError> {
        let output = self
            .compute(params, input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;

        let reduced = output.witness.reduce_to_zkp_modulus();

        let mut inputs = InputMap::new();
        inputs.insert("pk0is".to_string(), to_polynomial_array(&reduced.pk0is)?);
        inputs.insert("pk1is".to_string(), to_polynomial_array(&reduced.pk1is)?);

        Ok(inputs)
    }
}

fn to_polynomial_array(vecs: &[Vec<BigInt>]) -> Result<InputValue, ZkError> {
    let mut polynomials = Vec::with_capacity(vecs.len());

    for coeffs in vecs {
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

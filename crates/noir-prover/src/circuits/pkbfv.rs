// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::NoirProverError;
use crate::traits::CircuitProver;
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

#[derive(Debug, Clone)]
pub struct PkBfvCommitment(pub Vec<u8>);

impl AsRef<[u8]> for PkBfvCommitment {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[async_trait]
impl CircuitProver for PkBfvCircuit {
    type Params = Arc<BfvParameters>;
    type Input = PublicKey;
    type Output = PkBfvCommitment;

    fn circuit_name(&self) -> &'static str {
        "pk_bfv"
    }

    fn build_witness(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, NoirProverError> {
        let output = self
            .compute(params, input)
            .map_err(|e| NoirProverError::WitnessGenerationFailed(e.to_string()))?;

        let reduced = output.witness.reduce_to_zkp_modulus();

        let mut inputs = InputMap::new();
        inputs.insert("pk0is".to_string(), to_polynomial_array(&reduced.pk0is)?);
        inputs.insert("pk1is".to_string(), to_polynomial_array(&reduced.pk1is)?);

        Ok(inputs)
    }

    fn parse_output(&self, bytes: &[u8]) -> Result<Self::Output, NoirProverError> {
        Ok(PkBfvCommitment(bytes.to_vec()))
    }
}

fn to_polynomial_array(vecs: &[Vec<BigInt>]) -> Result<InputValue, NoirProverError> {
    let mut polynomials = Vec::with_capacity(vecs.len());

    for coeffs in vecs {
        let mut field_coeffs = Vec::with_capacity(coeffs.len());

        for b in coeffs {
            let s = b.to_string();
            let field = FieldElement::try_from_str(&s).ok_or_else(|| {
                NoirProverError::SerializationError(format!("invalid field element: {}", s))
            })?;
            field_coeffs.push(InputValue::Field(field));
        }

        let mut fields = BTreeMap::new();
        fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));
        polynomials.push(InputValue::Struct(fields));
    }

    Ok(InputValue::Vec(polynomials))
}

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
    let polynomials: Vec<InputValue> = vecs
        .iter()
        .map(|coeffs| {
            let field_coeffs: Vec<InputValue> = coeffs
                .iter()
                .map(|b| {
                    let field = FieldElement::try_from_str(&b.to_string()).unwrap_or_default();
                    InputValue::Field(field)
                })
                .collect();

            let mut fields = BTreeMap::new();
            fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));
            InputValue::Struct(fields)
        })
        .collect();

    Ok(InputValue::Vec(polynomials))
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::error::NoirProverError;
use crate::traits::CircuitProver;
use acir::FieldElement;
use e3_pvss::circuits::pk_bfv::circuit::{PkBfvCircuit, PkBfvComputationOutput};
use e3_pvss::traits::ReduceToZkpModulus;
use noirc_abi::input_parser::InputValue;
use noirc_abi::InputMap;
use num_bigint::BigInt;
use std::collections::BTreeMap;
use thiserror::Error;

/// Public output (commitment) from pk_bfv circuit
#[derive(Debug, Clone)]
pub struct PkBfvCommitment(pub Vec<u8>);

impl AsRef<[u8]> for PkBfvCommitment {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Errors specific to pk_bfv proving
#[derive(Error, Debug)]
pub enum PkBfvProverError {
    #[error("Computation error: {0}")]
    Computation(#[from] e3_pvss::errors::CodegenError),

    #[error("Noir prover error: {0}")]
    NoirProver(#[from] NoirProverError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Field conversion error: {0}")]
    FieldConversion(String),
}

impl CircuitProver for PkBfvCircuit {
    type ProofOutput = PkBfvCommitment;
    type ProverError = PkBfvProverError;

    fn build_inputs(output: &PkBfvComputationOutput) -> Result<InputMap, Self::ProverError> {
        // Reduce witness to ZKP modulus
        let reduced_witness = output.witness.reduce_to_zkp_modulus();

        // Build InputMap
        let mut inputs = InputMap::new();

        inputs.insert(
            "pk0is".to_string(),
            bigint_2d_to_polynomial_array(&reduced_witness.pk0is)?,
        );
        inputs.insert(
            "pk1is".to_string(),
            bigint_2d_to_polynomial_array(&reduced_witness.pk1is)?,
        );

        Ok(inputs)
    }

    fn parse_proof_output(bytes: &[u8]) -> Result<Self::ProofOutput, Self::ProverError> {
        Ok(PkBfvCommitment(bytes.to_vec()))
    }

    fn circuit_filename() -> String {
        // Circuit file is "pk_bfv.json", not "pk-bfv.json"
        "pk_bfv".to_string()
    }
}

/// Convert 2D BigInt vector to array of Polynomial structs for Noir
fn bigint_2d_to_polynomial_array(vecs: &[Vec<BigInt>]) -> Result<InputValue, PkBfvProverError> {
    let polynomials: Result<Vec<InputValue>, PkBfvProverError> = vecs
        .iter()
        .map(|coeffs| {
            let field_coeffs: Result<Vec<InputValue>, PkBfvProverError> = coeffs
                .iter()
                .map(|b| {
                    let s = b.to_string();
                    let field = FieldElement::try_from_str(&s).ok_or_else(|| {
                        PkBfvProverError::FieldConversion(format!(
                            "Failed to convert {} to FieldElement",
                            s
                        ))
                    })?;
                    Ok(InputValue::Field(field))
                })
                .collect();

            let field_coeffs = field_coeffs?;

            let mut struct_fields = BTreeMap::new();
            struct_fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));

            Ok(InputValue::Struct(struct_fields))
        })
        .collect();

    Ok(InputValue::Vec(polynomials?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::{BfvParamSet, BfvPreset};
    use e3_pvss::sample::generate_sample;
    use e3_pvss::traits::CircuitComputation;

    #[test]
    fn test_build_inputs_from_computation() {
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let sample = generate_sample(&params);

        let circuit = PkBfvCircuit;
        let output = circuit.compute(&params, &sample.public_key).unwrap();

        let inputs = PkBfvCircuit::build_inputs(&output).unwrap();

        assert!(inputs.contains_key("pk0is"));
        assert!(inputs.contains_key("pk1is"));
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::error::ZkError;
use crate::traits::Provable;
use acir::FieldElement;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_polynomial::CrtPolynomial;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitInput};
use e3_zk_helpers::circuits::dkg::pk::computation::Witness;
use e3_zk_helpers::Computation;
use fhe::bfv::PublicKey;
use noirc_abi::{input_parser::InputValue, InputMap};
use std::collections::BTreeMap;

impl Provable for PkCircuit {
    type Params = BfvPreset;
    type Input = PublicKey;

    fn circuit(&self) -> CircuitName {
        CircuitName::PkBfv
    }

    fn build_witness(
        &self,
        preset: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError> {
        // Use the existing Witness::compute implementation from zk-helpers
        // to ensure consistency between proof generation and verification
        let circuit_input = PkCircuitInput {
            public_key: input.clone(),
        };
        
        let witness = Witness::compute(*preset, &circuit_input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;

        // Convert the witness to InputMap format for Noir
        let mut inputs = InputMap::new();
        inputs.insert("pk0is".to_string(), crt_polynomial_to_array(&witness.pk0is)?);
        inputs.insert("pk1is".to_string(), crt_polynomial_to_array(&witness.pk1is)?);

        Ok(inputs)
    }
}

fn crt_polynomial_to_array(crt_poly: &CrtPolynomial) -> Result<InputValue, ZkError> {
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

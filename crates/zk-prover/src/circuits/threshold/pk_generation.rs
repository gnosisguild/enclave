// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::circuits::utils::{crt_polynomial_to_array, polynomial_to_input_value};
use crate::error::ZkError;
use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::threshold::pk_generation::circuit::{
    PkGenerationCircuit, PkGenerationCircuitInput,
};
use e3_zk_helpers::circuits::threshold::pk_generation::computation::Witness;
use e3_zk_helpers::Computation;
use noirc_abi::InputMap;
use std::collections::BTreeMap;

impl Provable for PkGenerationCircuit {
    type Params = BfvPreset;
    type Input = PkGenerationCircuitInput;

    fn circuit(&self) -> CircuitName {
        CircuitName::PkGeneration
    }

    fn build_witness(
        &self,
        preset: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError> {
        let witness = Witness::compute(preset.clone(), &input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;

        let mut inputs = BTreeMap::new();
        inputs.insert("a".into(), crt_polynomial_to_array(&witness.a)?);
        inputs.insert("eek".into(), polynomial_to_input_value(&witness.eek)?);
        inputs.insert("sk".into(), polynomial_to_input_value(&witness.sk)?);
        inputs.insert("e_sm".into(), crt_polynomial_to_array(&witness.e_sm)?);
        inputs.insert("r1is".into(), crt_polynomial_to_array(&witness.r1is)?);
        inputs.insert("r2is".into(), crt_polynomial_to_array(&witness.r2is)?);
        inputs.insert("pk0is".into(), crt_polynomial_to_array(&witness.pk0is)?);
        inputs.insert("pk1is".into(), crt_polynomial_to_array(&witness.pk1is)?);

        Ok(inputs)
    }
}

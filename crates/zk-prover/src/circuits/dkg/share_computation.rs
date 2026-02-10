// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::circuits::utils::{bigint_to_field_value, crt_polynomial_to_array, vec3d_to_input_value};
use crate::error::ZkError;
use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::Computation;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{ShareComputationCircuit, ShareComputationCircuitData, Inputs};
use noirc_abi::InputMap;

impl Provable for ShareComputationCircuit {
    type Params = BfvPreset;
    type Input = ShareComputationCircuitData;

    fn circuit(&self) -> CircuitName {
        CircuitName::ShareComputation
    }

    fn build_witness(
        &self,
        preset: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError> {
        let witness = Witness::compute(preset.clone(), input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;

        let secret_key_name = match input.dkg_input_type {
            DkgInputType::SecretKey => "sk_secret",
            DkgInputType::SmudgingNoise => "e_sm_secret",
        };
    
        let mut inputs = InputMap::new();
        inputs.insert(
            secret_key_name.to_string(),
            crt_polynomial_to_array(&witness.secret_crt)?,
        );
        inputs.insert(
            "y".to_string(),
            vec3d_to_input_value(&witness.y),
        );
        inputs.insert(
            "expected_secret_commitment".to_string(),
            bigint_to_field_value(&witness.expected_secret_commitment),
        );

        Ok(inputs)
    }
}

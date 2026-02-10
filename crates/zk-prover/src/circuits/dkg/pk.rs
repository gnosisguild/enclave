// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::circuits::utils::crt_polynomial_to_array;
use crate::error::ZkError;
use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitInput};
use e3_zk_helpers::circuits::dkg::pk::computation::Witness;
use e3_zk_helpers::Computation;
use noirc_abi::InputMap;

impl Provable for PkCircuit {
    type Params = BfvPreset;
    type Input = PkCircuitInput;

    fn circuit(&self) -> CircuitName {
        CircuitName::PkBfv
    }

    fn build_witness(
        &self,
        preset: &Self::Params,
        input: &Self::Input,
    ) -> Result<InputMap, ZkError> {
        let witness = Witness::compute(preset.clone(), input)
            .map_err(|e| ZkError::WitnessGenerationFailed(e.to_string()))?;

        let mut inputs = InputMap::new();
        inputs.insert(
            "pk0is".to_string(),
            crt_polynomial_to_array(&witness.pk0is)?,
        );
        inputs.insert(
            "pk1is".to_string(),
            crt_polynomial_to_array(&witness.pk1is)?,
        );

        Ok(inputs)
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_decryption::{
    Inputs, ShareDecryptionCircuit, ShareDecryptionCircuitData,
};

impl Provable for ShareDecryptionCircuit {
    type Params = BfvPreset;
    type Input = ShareDecryptionCircuitData;
    type Inputs = Inputs;

    fn resolve_circuit_name(&self, input: &Self::Input) -> CircuitName {
        match input.dkg_input_type {
            DkgInputType::SecretKey => CircuitName::DkgSkShareDecryption,
            DkgInputType::SmudgingNoise => CircuitName::DkgESmShareDecryption,
        }
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![
            CircuitName::DkgSkShareDecryption,
            CircuitName::DkgESmShareDecryption,
        ]
    }

    fn circuit(&self) -> CircuitName {
        CircuitName::DkgSkShareDecryption
    }
}

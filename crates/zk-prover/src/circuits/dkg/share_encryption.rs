// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_encryption::{ShareEncryptionCircuit, ShareEncryptionCircuitData, Inputs};

impl Provable for ShareEncryptionCircuit {
    type Params = BfvPreset;
    type Input = ShareEncryptionCircuitData;
    type Inputs = Inputs;

    fn resolve_circuit_name(&self, _input: &Self::Input) -> CircuitName {
        match _input.dkg_input_type {
            DkgInputType::SecretKey => CircuitName::SkShareEncryption,
            DkgInputType::SmudgingNoise => CircuitName::ESmShareEncryption,
        }
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![
            CircuitName::SkShareEncryption,
            CircuitName::ESmShareEncryption,
        ]
    }

    fn circuit(&self) -> CircuitName {
        CircuitName::SkShareEncryption
    }
}

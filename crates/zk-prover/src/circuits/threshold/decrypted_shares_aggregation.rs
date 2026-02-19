// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::threshold::decrypted_shares_aggregation::circuit::{
    DecryptedSharesAggregationCircuit, DecryptedSharesAggregationCircuitData,
};
use e3_zk_helpers::circuits::threshold::decrypted_shares_aggregation::computation::Inputs;

impl Provable for DecryptedSharesAggregationCircuit {
    type Params = BfvPreset;
    type Input = DecryptedSharesAggregationCircuitData;
    type Inputs = Inputs;

    fn circuit(&self) -> CircuitName {
        CircuitName::DecryptedSharesAggregationMod
    }

    fn resolve_circuit_name(&self, params: &Self::Params, _input: &Self::Input) -> CircuitName {
        match params {
            BfvPreset::SecureThreshold8192 | BfvPreset::SecureDkg8192 => {
                CircuitName::DecryptedSharesAggregationBn
            }
            _ => CircuitName::DecryptedSharesAggregationMod,
        }
    }

    fn valid_circuits(&self) -> Vec<CircuitName> {
        vec![
            CircuitName::DecryptedSharesAggregationBn,
            CircuitName::DecryptedSharesAggregationMod,
        ]
    }
}

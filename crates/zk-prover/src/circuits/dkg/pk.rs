// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitInput};
use e3_zk_helpers::circuits::dkg::pk::computation::Witness;

impl Provable for PkCircuit {
    type Params = BfvPreset;
    type Input = PkCircuitInput;
    type Witness = Witness;

    fn circuit(&self) -> CircuitName {
        CircuitName::PkBfv
    }
}

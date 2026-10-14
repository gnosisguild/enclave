// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE

use crate::traits::Provable;
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::threshold::pk_generation::circuit::{
    PkGenerationCircuit, PkGenerationCircuitData,
};
use e3_zk_helpers::circuits::threshold::pk_generation::computation::Inputs;

impl Provable for PkGenerationCircuit {
    type Params = BfvPreset;
    type Input = PkGenerationCircuitData;
    type Inputs = Inputs;

    /// C1 exists on the trBFV path only; the l-BFV path proves no C1.
    fn circuit(&self) -> CircuitName {
        CircuitName::PkGeneration
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Common computation behavior across circuits.

use serde_json::Value;

pub trait Computation: Sized {
    type Params;
    type Input;
    type Error;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error>;
}

pub trait ConvertToJson {
    fn convert_to_json(&self) -> serde_json::Result<Value>;
}

pub trait ReduceToZkpModulus: Sized {
    fn reduce_to_zkp_modulus(&self) -> Self;
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! DKG share-computation circuit (SK or smudging noise).

pub mod circuit;
pub mod codegen;
pub mod computation;

pub use circuit::{ShareComputationCircuit, ShareComputationCircuitInput};
pub use computation::{Bits, Bounds, Configs, ShareComputationOutput, Witness};

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// Circuit metadata
pub const PK_BFV_CIRCUIT_NAME: &str = "pk-bfv";
pub const PK_BFV_N_PROOFS: u32 = 1;
pub const PK_BFV_N_PUBLIC_INPUTS: u32 = 1;

pub mod codegen;
pub mod computation;

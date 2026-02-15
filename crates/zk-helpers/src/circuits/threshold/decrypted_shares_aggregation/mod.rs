// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Decrypted shares aggregation circuit for threshold BFV.
//!
//! Proves correct aggregation of T+1 decryption shares (Lagrange interpolation at 0 per modulus,
//! CRT reconstruction to u_global, and CRT quotients). Input: decryption share polynomials,
//! 1-based party IDs, and the decoded message. Output: input (decryption_shares, party_ids,
//! message, u_global, crt_quotients) in standard form for the Noir circuit.

pub mod circuit;
pub mod codegen;
pub mod computation;
pub mod sample;
pub mod utils;
pub use circuit::*;
pub use codegen::*;
pub use computation::*;
pub use utils::*;

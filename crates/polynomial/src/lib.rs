// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Exact signed-integer polynomial arithmetic and CRT coefficient containers.
//!
//! Coefficient arithmetic preserves the integer values needed for reduction quotients.
//! It does not perform FHE ring arithmetic or encode circuit witnesses. FHE conversions
//! belong to `e3-polynomial-fhe`; circuit encoding belongs to the ZK layer.

pub mod crt_polynomial;
pub mod polynomial;

pub use crt_polynomial::{CrtPolynomial, CrtPolynomialError};
pub use polynomial::{Polynomial, PolynomialError};

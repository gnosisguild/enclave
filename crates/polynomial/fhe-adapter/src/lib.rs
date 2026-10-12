// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! # Polynomial FHE Adapter
//!
//! Conversions between the core [`e3_polynomial`] types and the `fhe-math` RNS
//! polynomial types (`Poly<PowerBasis>`, `Poly<Ntt>`, `Poly<NttShoup>`).
//!
//! The core polynomial crate does not depend on `fhe-math`; this adapter owns every
//! bridge in that direction. Callers that move values between FHE ciphertext
//! polynomials and circuit-facing CRT limbs depend on both crates and import the
//! traits here.

pub mod crt_polynomial;
pub mod fhe_poly;
pub mod polynomial;
pub mod representation;

pub use crt_polynomial::{FromFhePolynomial, ToFhePolynomial};
pub use fhe_poly::ToPowerBasisPoly;
pub use representation::fhe_poly_to_crt_centered;

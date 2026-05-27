// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Medium committee size: N=10, T=4, H=8. First committee where H is less than N.
//!
//! Has been validated end-to-end (DKG + decryption + EVM verifier) when
//! `parity_insecure.nr` is populated. Switching `committee::active` to `medium`
//! requires regenerating that file (and `parity_secure.nr` for the secure-8192
//! preset) from the matrix generator.

pub global N_PARTIES: u32 = 10;
pub global T: u32 = 4;
pub global H: u32 = 8;

pub mod parity_insecure;
pub mod parity_secure;

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Small committee size: N=5, T=2, H=5.
//!
//! NOTE: not currently buildable end-to-end: `parity_insecure.nr` /
//! `parity_secure.nr` have not been generated for this committee. Switching
//! `committee::active` to `small` will fail to compile until those files exist.

pub global N_PARTIES: u32 = 5;
pub global T: u32 = 2;
pub global H: u32 = 5;

pub mod parity_insecure;
pub mod parity_secure;

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Small committee size: N=5, T=2, H=5 (all parties expected honest during DKG).
//! Parity matrices are auto-generated; switch with `pnpm build:circuits --committee small`.

pub global N_PARTIES: u32 = 5;
pub global T: u32 = 2;
pub global H: u32 = 5;

pub mod parity_insecure;
pub mod parity_secure;

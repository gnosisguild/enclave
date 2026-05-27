// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Micro committee size: N=3 parties, T=1 reconstruction threshold, H=3 honest.
//! Used as the default in CI / local dev for fast circuit-compile times.

/// Number of registered parties (matches on-chain `topNodes` length).
pub global N_PARTIES: u32 = 3;
/// Secret-sharing reconstruction threshold.
pub global T: u32 = 1;
/// Honest-party count expected during the DKG (`H <= N_PARTIES`).
pub global H: u32 = 3;

pub mod parity_insecure;
pub mod parity_secure;

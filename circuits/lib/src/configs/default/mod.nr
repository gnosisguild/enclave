// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
//
// Only place where we change param-set: re-export insecure or production.
// (in future any other param-set). All circuits use lib::configs::default::*.

pub use super::committee::micro::{H, N_PARTIES, T};
pub use super::insecure::dkg;
pub use super::insecure::threshold;

/// Max number of non-zero coefficients in the message polynomial.
/// This is a conservative estimate that should be okay for most use cases.
pub global MAX_MSG_NON_ZERO_COEFFS: u32 = 100;

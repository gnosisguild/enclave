// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// Maximum number of options for a vote.
/// We need this value because we need a compile-time bound value
/// for looping.
pub global MAX_OPTIONS: u32 = 10;

// This constant defines the size of the coefficient region allocated for encoding vote amounts
/// in binary format within each half of the polynomial. Votes are encoded as binary numbers.
pub global MAX_VOTE_BITS: u32 = 50; 

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_client::decode_bytes_to_vec_u64;
use eyre::Result;
use num_bigint::BigUint;

/// Maximum number of bits that can fit each vote option.
pub const MAX_VOTE_BITS: usize = 50;

/// Represents decoded vote counts from a tally
#[derive(Debug, Clone)]
pub struct VoteCounts {
    pub yes: BigUint,
    pub no: BigUint,
}

/// Decode an FHE-encrypted tally result into vote counts for each choice.
///
/// # Encoding scheme
///
/// The BFV plaintext polynomial has `degree` coefficients (e.g., 512).
/// When encoding a vote with `n` choices, the polynomial is divided into
/// `n` equal segments of `floor(degree / n)` coefficients each.
///
/// Each segment represents one choice's vote count in binary (big-endian):
///
///   |---- segment 0 ----|---- segment 1 ----|---- segment 2 ----|-- padding --|
///   |  choice 0 (Yes)   |  choice 1 (No)    |  choice 2 (Abst)  |   zeros    |
///   |  binary, MSB first|  binary, MSB first |  binary, MSB first|            |
///
/// Within each segment, the binary representation is right-aligned:
///   [0, 0, 0, ..., 0, 1, 0, 1, 1]  ← represents decimal 11
///    ^-- leading zeros    ^-- MSB     ^-- LSB
///
/// Because FHE addition is performed coefficient-wise on the polynomial,
/// summing N encrypted votes produces the total count per coefficient.
/// The binary reconstruction then recovers the final tally per choice.
///
/// # MAX_VOTE_BITS
///
/// To prevent overflow during FHE computation, only the last `MAX_VOTE_BITS`
/// coefficients of each segment are used (MAX_VOTE_BITS = 50). This caps the
/// maximum representable vote count at `2^50 - 1` (~1.1 quadrillion).
///
/// We read from the right side of each segment (the significant bits)
/// and ignore leading zeros on the left.
///
/// # Arguments
///
/// * `tally_bytes` - Raw bytes from the FHE decryption, encoding u64 values
///                   in little-endian format (8 bytes per coefficient).
/// * `num_choices` - Number of voting options (must match what was used to encode).
///
/// # Returns
///
/// A `Vec<BigUint>` of length `num_choices`, where `results[i]` is the
/// total vote weight for choice `i`.
///
/// # Example
///
/// Given degree=512, MAX_VOTE_BITS=50, num_choices=2:
///   segment_size   = 512 / 2 = 256 coefficients per choice
///   effective_size = min(256, 50) = 50
///
///   Choice 0 reads coefficients [206..256)   (last 50 of segment 0)
///   Choice 1 reads coefficients [462..512)   (last 50 of segment 1)
///
/// Given degree=512, MAX_VOTE_BITS=50, num_choices=4:
///   segment_size   = 512 / 4 = 128 coefficients per choice
///   effective_size = min(128, 50) = 50
///   remainder      = 512 - (128 * 4) = 0
///
///   Choice 0 reads coefficients [ 78..128)   (last 50 of segment 0)
///   Choice 1 reads coefficients [206..256)   (last 50 of segment 1)
///   Choice 2 reads coefficients [334..384)   (last 50 of segment 2)
///   Choice 3 reads coefficients [462..512)   (last 50 of segment 3)
///
/// Given degree=512, MAX_VOTE_BITS=50, num_choices=3:
///   segment_size   = 512 / 3 = 170 coefficients per choice
///   effective_size = min(170, 50) = 50
///   remainder      = 512 - (170 * 3) = 2 coefficients (trailing zeros, ignored)
///
///   Choice 0 reads coefficients [120..170)
///   Choice 1 reads coefficients [290..340)
///   Choice 2 reads coefficients [460..510)
///
pub fn decode_tally(tally_bytes: &[u8], num_choices: usize) -> Result<Vec<BigUint>> {
    if num_choices == 0 {
        return Err(eyre::eyre!("Number of choices must be positive"));
    }

    // Each u64 coefficient is stored as 8 little-endian bytes.
    // This gives us the full polynomial coefficient array.
    let values = decode_bytes_to_vec_u64(tally_bytes)?;

    // Divide the polynomial evenly into num_choices segments.
    // Any leftover coefficients (degree % num_choices) are trailing
    // zeros and are ignored.
    let segment_size = values.len() / num_choices;

    // Only read the rightmost MAX_VOTE_BITS (50) coefficients from each
    // segment to avoid overflow. If the segment is smaller than
    // MAX_VOTE_BITS (unlikely with degree=512), use the full segment.
    let effective_size = segment_size.min(MAX_VOTE_BITS);

    let mut results = Vec::with_capacity(num_choices);

    for choice_idx in 0..num_choices {
        // Find where this choice's segment starts in the array
        let segment_start = choice_idx * segment_size;

        // Right-align: skip leading zeros, read only the significant bits
        // at the end of the segment
        let read_start = segment_start + segment_size - effective_size;
        let segment = &values[read_start..read_start + effective_size];

        // Reconstruct the vote count from binary (big-endian within segment):
        //   value = segment[0] * 2^(n-1) + segment[1] * 2^(n-2) + ... + segment[n-1] * 2^0
        let mut value = BigUint::from(0u64);
        for (i, &v) in segment.iter().enumerate() {
            let weight = BigUint::from(2u64).pow((segment.len() - 1 - i) as u32);
            value += BigUint::from(v) * weight;
        }

        results.push(value);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_tally_fhe_output() {
        // Expected: yes = 10000000000, no = 30000000000
        let tally_hex = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let bytes = hex::decode(tally_hex.strip_prefix("0x").unwrap_or(tally_hex)).unwrap();
        let result = decode_tally(&bytes, 2).unwrap();

        assert_eq!(result[0], BigUint::from(10000000000u64));
        assert_eq!(result[1], BigUint::from(30000000000u64));
    }

    #[test]
    fn test_decode_tally_with_wrong_num_options() {
        // Expected: yes = 10000000000, no = 30000000000
        let tally_hex = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

        let bytes = hex::decode(tally_hex.strip_prefix("0x").unwrap_or(tally_hex)).unwrap();
        let result = decode_tally(&bytes, 3).unwrap();

        assert!(result[0] != BigUint::from(10000000000u64));
        assert!(result[1] != BigUint::from(30000000000u64));
    }
}

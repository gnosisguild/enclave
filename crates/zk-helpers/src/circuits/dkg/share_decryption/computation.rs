// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the share-decryption circuit: configs, bounds, bit widths, and witness.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) honest ciphertexts and secret key. Witness values are normalized for the ZKP
//! field so the Noir circuit's range checks and commitment checks succeed.

use crate::circuits::commitments::compute_share_encryption_commitment_from_message;
use crate::dkg::share_decryption::ShareDecryptionCircuit;
use crate::dkg::share_decryption::ShareDecryptionCircuitInput;
use crate::CircuitsErrors;
use crate::{bigint_2d_to_json_values, calculate_bit_width, poly_coefficients_to_toml_json};
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::Polynomial;
use fhe_traits::FheDecrypter;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Output of [`CircuitComputation::compute`] for [`ShareDecryptionCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct ShareDecryptionOutput {
    /// Coefficient bounds used to derive bit widths.
    pub bounds: Bounds,
    /// Bit widths used by the Noir prover for packing.
    pub bits: Bits,
    /// Witness data for the share-decryption circuit.
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`ShareDecryptionCircuit`].
impl CircuitComputation for ShareDecryptionCircuit {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Output = ShareDecryptionOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, input)?;
        let bits = Bits::compute(preset, &bounds)?;
        let witness = Witness::compute(preset, input)?;

        Ok(ShareDecryptionOutput {
            bounds,
            bits,
            witness,
        })
    }
}

/// Global configs for the share-decryption circuit: degree, number of moduli, honest parties, bits, and bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    /// Polynomial degree (N).
    pub n: usize,
    /// Number of CRT moduli (L).
    pub l: usize,
    /// Number of honest parties (H).
    pub h: usize,
    pub bits: Bits,
    pub bounds: Bounds,
}

/// Bit widths used by the Noir prover (e.g. for packing message coefficients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    /// Bit width for plaintext/message coefficients (in [0, t)).
    pub msg_bit: u32,
}

/// Coefficient bounds for the share-decryption circuit (currently empty; bounds are derived from plaintext modulus).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {}

/// Witness data for the share-decryption circuit: expected commitments and decrypted shares.
///
/// Coefficients are reduced to the ZKP field modulus for serialization. The circuit verifies
/// that decrypted shares match the expected commitments from the share-encryption circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Expected message commitments from share-encryption (CIRCUIT 3) for H honest parties: [party_idx][mod_idx].
    pub expected_commitments: Vec<Vec<BigInt>>, // [H][L]
    /// Decrypted share coefficients per party and modulus: [party_idx][mod_idx][coeff_idx].
    pub decrypted_shares: Vec<Vec<Vec<BigInt>>>, // [H][L][N]
}

impl Computation for Configs {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, CircuitsErrors> {
        let (_, dkg_params) = build_pair_for_preset(preset)
            .map_err(|e| crate::utils::ZkHelpersUtilsError::ParseBound(e.to_string()))?;

        let n = dkg_params.degree() as usize;
        let l = dkg_params.moduli().len();
        let h = input.honest_ciphertexts.len();

        let bounds = Bounds::compute(preset, &input)?;
        let bits = Bits::compute(preset, &bounds)?;

        Ok(Configs {
            n,
            l,
            h,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type Preset = BfvPreset;
    type Input = Bounds;
    type Error = crate::utils::ZkHelpersUtilsError;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (_, dkg_params) = build_pair_for_preset(preset)
            .map_err(|e| crate::utils::ZkHelpersUtilsError::ParseBound(e.to_string()))?;

        Ok(Bits {
            msg_bit: calculate_bit_width(BigInt::from(dkg_params.plaintext())),
        })
    }
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(_: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        Ok(Bounds {})
    }
}

impl Computation for Witness {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, dkg_params) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
        let threshold_l = threshold_params.moduli().len();

        let mut expected_commitments: Vec<Vec<BigInt>> = Vec::new();
        let mut decrypted_shares: Vec<Vec<Vec<BigInt>>> = Vec::new();

        let msg_bit = calculate_bit_width(BigInt::from(dkg_params.plaintext()));

        // Decrypt each ciphertext and compute its commitment
        for party_cts in input.honest_ciphertexts.iter() {
            if party_cts.len() < threshold_l {
                return Err(CircuitsErrors::Other(format!(
                    "honest_ciphertexts party has {} ciphertexts but threshold_l is {}; \
                     each party must have at least threshold_l ciphertexts",
                    party_cts.len(),
                    threshold_l
                )));
            }
            let mut party_commitments = Vec::with_capacity(threshold_l);
            let mut party_shares = Vec::with_capacity(threshold_l);
            for mod_idx in 0..threshold_l {
                // Decrypt the ciphertext to get the plaintext share
                let decrypted_pt = input.secret_key.try_decrypt(&party_cts[mod_idx]).unwrap();
                let share_coeffs = decrypted_pt.value.deref().to_vec();
                party_commitments.push(compute_share_encryption_commitment_from_message(
                    &Polynomial::from_u64_vector(share_coeffs.clone()),
                    msg_bit,
                ));
                party_shares.push(
                    share_coeffs
                        .iter()
                        .map(|c| BigInt::from(*c))
                        .collect::<Vec<_>>(),
                );
            }
            expected_commitments.push(party_commitments);
            decrypted_shares.push(party_shares);
        }

        Ok(Witness {
            expected_commitments,
            decrypted_shares,
        })
    }

    // Used as witness for Nargo execution.
    /// Serializes witness so that `decrypted_shares` matches Noir's `[[Polynomial<N>; L]; H]`:
    /// each polynomial is `{ "coefficients": [string, ...] }`.
    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        let expected_commitments = bigint_2d_to_json_values(&self.expected_commitments);
        let decrypted_shares: Vec<Vec<serde_json::Value>> = self
            .decrypted_shares
            .iter()
            .map(|party_shares| {
                party_shares
                    .iter()
                    .map(|share| poly_coefficients_to_toml_json(share))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let json = serde_json::json!({
            "expected_commitments": expected_commitments,
            "decrypted_shares": decrypted_shares,
        });

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::computation::DkgInputType;
    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
        )
        .unwrap();
        let bounds = Bounds::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();
        let bits = Bits::compute(BfvPreset::InsecureThreshold512, &bounds).unwrap();

        let (_, dkg_params) = build_pair_for_preset(BfvPreset::InsecureThreshold512).unwrap();
        let expected_msg_bit = calculate_bit_width(BigInt::from(dkg_params.plaintext()));
        assert_eq!(bits.msg_bit, expected_msg_bit);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
        )
        .unwrap();
        let constants = Configs::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();

        let json = constants.to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.h, constants.h);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }

    #[test]
    fn test_witness_decryption_consistency() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
        )
        .unwrap();
        let witness = Witness::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();

        // Witness should have one row per honest party
        assert_eq!(
            witness.expected_commitments.len(),
            sample.honest_ciphertexts.len()
        );
        assert_eq!(
            witness.decrypted_shares.len(),
            sample.honest_ciphertexts.len()
        );
    }
}

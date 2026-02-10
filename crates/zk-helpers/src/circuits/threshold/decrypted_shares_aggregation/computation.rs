// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Bounds, configs, bits, and witness computation for the Decryption Share Aggregation TRBFV circuit.
//!
//! Uses [`crate::threshold::decrypted_shares_aggregation::utils`] for Q/delta, modular inverses,
//! Lagrange-at-zero recovery, and scalar CRT reconstruction. Witness coefficients are normalized
//! with [`e3_polynomial::reduce`] in [`Witness::standard_form`], consistent with other circuits.

use crate::calculate_bit_width;
use crate::get_zkp_modulus;
use crate::threshold::decrypted_shares_aggregation::circuit::DecryptedSharesAggregationCircuit;
use crate::threshold::decrypted_shares_aggregation::circuit::DecryptedSharesAggregationCircuitInput;
use crate::threshold::decrypted_shares_aggregation::utils;
use crate::CircuitsErrors;
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::reduce;
use fhe_math::rq::Representation;
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`DecryptedSharesAggregationCircuit`].
#[derive(Debug)]
pub struct DecryptedSharesAggregationComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

impl CircuitComputation for DecryptedSharesAggregationCircuit {
    type Preset = BfvPreset;
    type Input = DecryptedSharesAggregationCircuitInput;
    type Output = DecryptedSharesAggregationComputationOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &bounds)?;
        let witness = Witness::compute(preset, input)?;

        Ok(DecryptedSharesAggregationComputationOutput {
            bounds,
            bits,
            witness,
        })
    }
}

/// Bounds for noise and scaling: delta = floor(Q/t), delta_half = floor(delta/2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub delta: BigUint,
    pub delta_half: BigUint,
}

/// Bit widths used by the circuit (e.g. noise bit for range checks).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub noise_bit: u32,
}

/// Circuit config: moduli count, plaintext modulus, q_inverse_mod_t, bits, bounds, and message polynomial length.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub l: usize,
    pub threshold: usize,
    pub moduli: Vec<u64>,
    pub plaintext_modulus: u64,
    pub q_inverse_mod_t: u64,
    pub bits: Bits,
    pub bounds: Bounds,
    /// Max number of non-zero coefficients in the message polynomial (matches Noir's MAX_MSG_NON_ZERO_COEFFS).
    pub max_msg_non_zero_coeffs: usize,
}

/// Witness for decrypted shares aggregation (same shape as old DecSharesAggTrBfvVectors).
/// All coefficients reduced to [0, zkp_modulus) in standard_form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// [party][modulus][coeff]
    pub decryption_shares: Vec<Vec<Vec<BigInt>>>,
    /// Party IDs (1-based: 1, 2, ..., T+1)
    pub party_ids: Vec<BigInt>,
    /// Message polynomial coefficients
    pub message: Vec<BigInt>,
    /// u_global polynomial (CRT reconstruction)
    pub u_global: Vec<BigInt>,
    /// [modulus][coeff]
    pub crt_quotients: Vec<Vec<BigInt>>,
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;
        let moduli = threshold_params.moduli();
        let t = threshold_params.plaintext();
        let q = utils::compute_q_product(moduli);
        let delta = utils::compute_delta(&q, t);
        let delta_half = utils::compute_delta_half(&delta);
        Ok(Bounds { delta, delta_half })
    }
}

impl Computation for Bits {
    type Preset = BfvPreset;
    type Input = Bounds;
    type Error = CircuitsErrors;

    fn compute(_: Self::Preset, bounds: &Self::Input) -> Result<Self, Self::Error> {
        let noise_bit = calculate_bit_width(BigInt::from(bounds.delta_half.clone()));
        Ok(Bits { noise_bit })
    }
}

impl Computation for Configs {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;
        let moduli = threshold_params.moduli().to_vec();
        let t = threshold_params.plaintext();
        let q = utils::compute_q_product(&moduli);
        let q_inverse_mod_t = utils::compute_q_inverse_mod_t(&q, t)?;
        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &bounds)?;
        Ok(Configs {
            threshold: 0, // Not derived from preset; set by caller if needed.
            l: moduli.len(),
            moduli,
            plaintext_modulus: t,
            q_inverse_mod_t,
            bits,
            bounds,
            // TODO: make this configurable based on the application (e.g., CRISP = 80,
            //       since there's just CRISP for now we can hardcode it).
            max_msg_non_zero_coeffs: 80, // Default; matches Noir's MAX_MSG_NON_ZERO_COEFFS.
        })
    }
}

impl Computation for Witness {
    type Preset = BfvPreset;
    type Input = DecryptedSharesAggregationCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        let configs = Configs::compute(preset, &())?;
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;
        let ctx = threshold_params
            .ctx_at_level(0)
            .map_err(|e| CircuitsErrors::Other(format!("ctx_at_level: {:?}", e)))?;
        let num_moduli = ctx.moduli().len();
        let degree = ctx.degree;
        let threshold = input.committee.threshold;
        let max_msg_non_zero_coeffs = configs.max_msg_non_zero_coeffs;

        // Copy to PowerBasis for coefficient extraction
        let d_share_polys: Vec<_> = input
            .d_share_polys
            .iter()
            .map(|p| {
                let mut copy = p.clone();
                copy.change_representation(Representation::PowerBasis);
                copy
            })
            .collect();

        if d_share_polys.len() < threshold + 1 {
            return Err(CircuitsErrors::Other(format!(
                "d_share_polys.len() {} < threshold + 1 ({}); need at least {} polynomials",
                d_share_polys.len(),
                threshold + 1,
                threshold + 1
            )));
        }

        // 1. Extract decryption shares per modulus per party [party][modulus][coeff]
        let mut decryption_shares = Vec::with_capacity(d_share_polys.len());
        for d_share in &d_share_polys {
            let coeffs = d_share.coefficients();
            let mut party_shares = Vec::with_capacity(num_moduli);
            for m in 0..num_moduli {
                let modulus_row = coeffs.row(m);
                let qi_bigint = BigInt::from(ctx.moduli()[m]);
                let coeff_vec: Vec<BigInt> = modulus_row
                    .iter()
                    .map(|&x| {
                        let mut coeff = BigInt::from(x);
                        coeff %= &qi_bigint;
                        if coeff < BigInt::zero() {
                            coeff += &qi_bigint;
                        }
                        coeff
                    })
                    .collect();
                party_shares.push(coeff_vec);
            }
            decryption_shares.push(party_shares);
        }

        // 2. Party IDs (1-based)
        let party_ids: Vec<BigInt> = input
            .reconstructing_parties
            .iter()
            .map(|&x| BigInt::from(x))
            .collect();

        // 3. Message (pad to degree for computation, then truncate to MAX_MSG_NON_ZERO_COEFFS for witness)
        let mut message: Vec<BigInt> = input.message_vec.iter().map(|&x| BigInt::from(x)).collect();
        message.resize(degree, BigInt::zero());

        // 4. u^{(l)} via Lagrange per modulus
        let reconstructing_parties = &input.reconstructing_parties;
        let mut u_per_modulus: Vec<Vec<u64>> = Vec::new();
        for m in 0..num_moduli {
            let modulus = ctx.moduli()[m];
            let mut u_modulus_coeffs = Vec::with_capacity(degree);
            for coeff_idx in 0..degree {
                let shares: Vec<BigInt> = (0..=threshold)
                    .map(|party_idx| {
                        let coeffs = d_share_polys[party_idx].coefficients();
                        let row = coeffs.row(m);
                        BigInt::from(row[coeff_idx])
                    })
                    .collect();
                let u_coeff_u64 =
                    utils::lagrange_recover_at_zero(reconstructing_parties, &shares, modulus)?;
                u_modulus_coeffs.push(u_coeff_u64);
            }
            u_per_modulus.push(u_modulus_coeffs);
        }

        // 5. u_global via CRT reconstruction
        let mut u_global: Vec<BigInt> = Vec::with_capacity(degree);
        for coeff_idx in 0..degree {
            let rests: Vec<u64> = (0..num_moduli)
                .map(|m| u_per_modulus[m][coeff_idx])
                .collect();
            let u_global_coeff = utils::crt_reconstruct(&rests, ctx.moduli())?;
            u_global.push(BigInt::from(u_global_coeff));
        }

        // 6. CRT quotients: r^{(m)} = (u_global - u^{(m)}) / q_m
        let mut crt_quotients: Vec<Vec<BigInt>> = Vec::new();
        for (m, u_modulus) in u_per_modulus.iter().enumerate().take(num_moduli) {
            let q_m = ctx.moduli()[m];
            let q_m_bigint = BigInt::from(q_m);
            let mut r_m_coeffs = Vec::with_capacity(degree);
            for (coeff_idx, u_global_val) in u_global.iter().enumerate().take(degree) {
                let u_m = BigInt::from(u_modulus[coeff_idx]);
                let diff = u_global_val - &u_m;
                let remainder = &diff % &q_m_bigint;
                if !remainder.is_zero() {
                    return Err(CircuitsErrors::Other(format!(
                        "CRT quotient not exact at m={} coeff={}",
                        m, coeff_idx
                    )));
                }
                r_m_coeffs.push(&diff / &q_m_bigint);
            }
            crt_quotients.push(r_m_coeffs);
        }

        // Truncate to max_msg_non_zero_coeffs. Do NOT reverse: match old impl (dec_shares_agg_trbfv
        // vectors.rs) and circuit—index 0 = constant term (ascending order).
        let truncate = |v: &[BigInt]| -> Vec<BigInt> {
            v.iter().take(max_msg_non_zero_coeffs).cloned().collect()
        };
        let decryption_shares: Vec<Vec<Vec<BigInt>>> = decryption_shares
            .into_iter()
            .map(|party| party.into_iter().map(|row| truncate(&row)).collect())
            .collect();
        let message = truncate(&message);
        let u_global = truncate(&u_global);
        let crt_quotients: Vec<Vec<BigInt>> = crt_quotients
            .into_iter()
            .map(|row| truncate(&row))
            .collect();

        let witness = Witness {
            decryption_shares,
            party_ids,
            message,
            u_global,
            crt_quotients,
        };
        Ok(witness.standard_form())
    }
}

impl Witness {
    /// Reduce all coefficients to [0, zkp_modulus). Uses `e3_polynomial::reduce` like other circuits.
    pub fn standard_form(&self) -> Self {
        let zkp_modulus = get_zkp_modulus();
        Witness {
            decryption_shares: self
                .decryption_shares
                .iter()
                .map(|party| {
                    party
                        .iter()
                        .map(|row| row.iter().map(|c| reduce(c, &zkp_modulus)).collect())
                        .collect()
                })
                .collect(),
            party_ids: self
                .party_ids
                .iter()
                .map(|c| reduce(c, &zkp_modulus))
                .collect(),
            message: self
                .message
                .iter()
                .map(|c| reduce(c, &zkp_modulus))
                .collect(),
            u_global: self
                .u_global
                .iter()
                .map(|c| reduce(c, &zkp_modulus))
                .collect(),
            crt_quotients: self
                .crt_quotients
                .iter()
                .map(|row| row.iter().map(|c| reduce(c, &zkp_modulus)).collect())
                .collect(),
        }
    }

    /// Serializes the witness to JSON for Prover.toml. Each polynomial is emitted as
    /// `{ "coefficients": [string, ...] }` to match Noir's `Polynomial` struct.
    pub fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        use crate::bigint_1d_to_json_values;
        use crate::poly_coefficients_to_toml_json;

        let decryption_shares_json: Vec<Vec<serde_json::Value>> = self
            .decryption_shares
            .iter()
            .map(|party| {
                party
                    .iter()
                    .map(|modulus_row| poly_coefficients_to_toml_json(modulus_row))
                    .collect()
            })
            .collect();
        let party_ids_json = bigint_1d_to_json_values(&self.party_ids);
        let message_json = poly_coefficients_to_toml_json(&self.message);
        let u_global_json = poly_coefficients_to_toml_json(&self.u_global);
        let crt_quotients_json: Vec<serde_json::Value> = self
            .crt_quotients
            .iter()
            .map(|row| poly_coefficients_to_toml_json(row))
            .collect();

        let json = serde_json::json!({
            "decryption_shares": decryption_shares_json,
            "party_ids": party_ids_json,
            "message": message_json,
            "u_global": u_global_json,
            "crt_quotients": crt_quotients_json,
        });

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threshold::decrypted_shares_aggregation::DecryptedSharesAggregationCircuitInput;
    use crate::CiphernodesCommitteeSize;

    #[test]
    fn test_bounds_and_bits_consistency() {
        let preset = BfvPreset::InsecureThreshold512;
        let bounds = Bounds::compute(preset, &()).unwrap();
        let bits = Bits::compute(preset, &bounds).unwrap();

        assert!(!bounds.delta.is_zero());
        assert!(!bounds.delta_half.is_zero());
        assert!(bounds.delta_half < bounds.delta);
        assert!(bits.noise_bit > 0);
    }

    #[test]
    fn test_configs_compute() {
        let preset = BfvPreset::InsecureThreshold512;
        let configs = Configs::compute(preset, &()).unwrap();

        assert_eq!(configs.moduli.len(), configs.l);
        assert!(configs.q_inverse_mod_t > 0);
    }

    #[test]
    fn test_full_computation_with_sample() {
        let preset = BfvPreset::InsecureThreshold512;
        let committee = CiphernodesCommitteeSize::Small.values();
        let input =
            DecryptedSharesAggregationCircuitInput::generate_sample(preset, committee.clone());

        let out = DecryptedSharesAggregationCircuit::compute(preset, &input).unwrap();

        let configs = Configs::compute(preset, &()).unwrap();
        assert_eq!(out.witness.decryption_shares.len(), committee.threshold + 1);
        assert_eq!(out.witness.party_ids.len(), committee.threshold + 1);
        assert_eq!(out.witness.message.len(), configs.max_msg_non_zero_coeffs);
        assert_eq!(out.witness.u_global.len(), configs.max_msg_non_zero_coeffs);
        assert!(out.bits.noise_bit > 0);
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the share-computation circuit: constants, bounds, bit widths, and witness.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) secret plus shares. Witness values are normalized to [0, q_j) per modulus
//! and then to the ZKP field modulus so the Noir circuit's range check and parity check succeed.

use crate::calculate_bit_width;
use crate::circuits::commitments::{
    compute_share_computation_e_sm_commitment, compute_share_computation_sk_commitment,
};
use crate::computation::DkgInputType;
use crate::dkg::share_computation::ShareComputationCircuit;
use crate::dkg::share_computation::ShareComputationCircuitInput;
use crate::get_zkp_modulus;
use crate::CircuitsErrors;
use crate::ConvertToJson;
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::{reduce, CrtPolynomial};
use fhe::bfv::SecretKey;
use fhe::trbfv::{SmudgingBoundCalculator, SmudgingBoundCalculatorConfig};
use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`ShareComputationCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct ShareComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`ShareComputationCircuit`].
impl CircuitComputation for ShareComputationCircuit {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = ShareComputationCircuitInput;
    type Output = ShareComputationOutput;
    type Error = CircuitsErrors;

    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, input)?;
        let bits = Bits::compute(preset, &bounds)?;
        let witness = Witness::compute(preset, input)?;

        Ok(ShareComputationOutput {
            bounds,
            bits,
            witness,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub n: usize,
    pub l: usize,
    pub moduli: Vec<u64>,
    pub bits: Bits,
    pub bounds: Bounds,
}

/// Bit widths used by the Noir prover (e.g. for packing coefficients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub bit_sk_secret: u32,
    pub bit_e_sm_secret: u32,
    pub bit_share: u32,
}

/// Coefficient bounds for public key polynomials (used to derive bit widths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub sk_bound: BigUint,
    pub e_sm_bound: BigUint,
}

/// Witness data for the share-computation circuit: secret in CRT form, y (secret + shares per coeff/modulus), and commitment.
///
/// All coefficients are reduced to the ZKP field modulus for serialization. Before that,
/// secret_crt and y are normalized so that per modulus j: secret and shares are in [0, q_j),
/// ensuring the circuit's secret consistency (y[i][j][0] == e_sm_secret[j][i]), range check, and parity check pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Secret polynomial in CRT form (SK or smudging noise). Coefficients in [0, zkp_modulus) for serialization.
    pub secret_crt: CrtPolynomial,
    /// y[coeff_idx][mod_idx][0] = secret at (mod_idx, coeff_idx); y[coeff_idx][mod_idx][1 + party] = share for party. Values in [0, zkp_modulus).
    pub y: Vec<Vec<Vec<BigInt>>>,
    /// Expected secret commitment (matches C1's compute_secret_commitment).
    pub expected_secret_commitment: BigInt,
}

impl Computation for Configs {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = ShareComputationCircuitInput;
    type Error = CircuitsErrors;

    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self, CircuitsErrors> {
        let (_, dkg_params) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let moduli = dkg_params.moduli().to_vec();
        let bounds = Bounds::compute(preset, input)?;
        let bits = Bits::compute(preset, &bounds)?;

        Ok(Configs {
            n: dkg_params.degree(),
            l: moduli.len(),
            moduli,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = Bounds;
    type Error = crate::utils::ZkHelpersUtilsError;

    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self, Self::Error> {
        let (threshold_params, _) = build_pair_for_preset(preset)
            .map_err(|e| crate::utils::ZkHelpersUtilsError::ParseBound(e.to_string()))?;

        let mut bit_share = 0;
        for &qi in threshold_params.moduli() {
            let share_bound = BigUint::from(qi - 1);
            let bit_width = calculate_bit_width(&share_bound.to_string())?;
            bit_share = bit_share.max(bit_width);
        }

        Ok(Bits {
            bit_sk_secret: calculate_bit_width(&input.sk_bound.to_string())?,
            bit_e_sm_secret: calculate_bit_width(&input.e_sm_bound.to_string())?,
            bit_share,
        })
    }
}

impl Computation for Bounds {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = ShareComputationCircuitInput;
    type Error = CircuitsErrors;

    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
        let num_ciphertexts = preset.search_defaults().unwrap().z;
        let lambda = preset.search_defaults().unwrap().lambda;

        let e_sm_config = SmudgingBoundCalculatorConfig::new(
            threshold_params,
            input.n_parties as usize,
            num_ciphertexts as usize,
            lambda as usize,
        );

        let e_sm_calculator = SmudgingBoundCalculator::new(e_sm_config);

        let e_sm_bound = e_sm_calculator.calculate_sm_bound()?;

        Ok(Bounds {
            sk_bound: BigUint::from(SecretKey::sk_bound() as u128),
            e_sm_bound,
        })
    }
}

impl Computation for Witness {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = ShareComputationCircuitInput;
    type Error = CircuitsErrors;

    fn compute(
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
        let moduli = threshold_params.moduli();
        let degree = threshold_params.degree();
        let num_moduli = moduli.len();
        let n_parties = input.n_parties as usize;

        let mut secret_crt = input.secret.clone();
        let sss = &input.secret_sss;

        if input.dkg_input_type == DkgInputType::SmudgingNoise {
            // Normalize secret_crt to [0, q_j) per limb so it matches what we put in y and what the circuit expects (e_sm_secret[j][i] == y[i][j][0]).
            secret_crt
                .reduce(moduli)
                .map_err(|e| CircuitsErrors::Sample(format!("secret_crt reduce: {:?}", e)))?;
        }

        // y[coeff_idx][mod_idx][0] = secret_crt[mod_idx][coeff_idx] (already in [0, q_j)); y[coeff_idx][mod_idx][1+party] = share in [0, q_j).
        let mut y: Vec<Vec<Vec<BigInt>>> = Vec::with_capacity(degree);
        for coeff_idx in 0..degree {
            let mut y_coeff: Vec<Vec<BigInt>> = Vec::with_capacity(num_moduli);
            for mod_idx in 0..num_moduli {
                let q_j = BigInt::from(moduli[mod_idx]);
                let mut y_mod: Vec<BigInt> = Vec::with_capacity(1 + n_parties);
                y_mod.push(secret_crt.limb(mod_idx).coefficients()[coeff_idx].clone());
                for party_idx in 0..n_parties {
                    let share_value = &sss[mod_idx][[party_idx, coeff_idx]];
                    y_mod.push(reduce(share_value, &q_j));
                }
                y_coeff.push(y_mod);
            }
            y.push(y_coeff);
        }

        let bounds = Bounds::compute(preset, input)?;
        let bits = Bits::compute(preset, &bounds)?;
        let expected_secret_commitment = match input.dkg_input_type {
            DkgInputType::SecretKey => {
                compute_share_computation_sk_commitment(secret_crt.limb(0), bits.bit_sk_secret)
            }
            DkgInputType::SmudgingNoise => {
                compute_share_computation_e_sm_commitment(&secret_crt, bits.bit_e_sm_secret)
            }
        };

        let zkp_modulus = &get_zkp_modulus();

        secret_crt.reduce_uniform(zkp_modulus);
        for coeff in &mut y {
            for mod_row in coeff.iter_mut() {
                for value in mod_row.iter_mut() {
                    *value = reduce(value, zkp_modulus);
                }
            }
        }

        Ok(Witness {
            secret_crt,
            y,
            expected_secret_commitment,
        })
    }
}

impl ConvertToJson for Configs {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

impl ConvertToJson for Bounds {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

impl ConvertToJson for Witness {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::computation::DkgInputType;
    use crate::dkg::share_computation::ShareComputationCircuitInput;
    use crate::sample::{prepare_sample_for_test, Sample};
    use crate::ConvertToJson;
    use e3_fhe_params::BfvPreset;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    fn share_computation_input_from_sample(
        sample: &Sample,
        dkg_input_type: DkgInputType,
    ) -> ShareComputationCircuitInput {
        ShareComputationCircuitInput {
            dkg_input_type,
            secret: sample.secret.as_ref().unwrap().clone(),
            secret_sss: sample.secret_sss.clone(),
            parity_matrix: sample
                .parity_matrix
                .iter()
                .map(|m| m.to_bigint_rows())
                .collect(),
            n_parties: sample.committee.n as u32,
            threshold: sample.committee.threshold as u32,
        }
    }

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SecretKey),
        )
        .unwrap();
        let input = share_computation_input_from_sample(&sample, DkgInputType::SecretKey);
        let bounds = Bounds::compute(DEFAULT_BFV_PRESET, &input).unwrap();
        let bits = Bits::compute(DEFAULT_BFV_PRESET, &bounds).unwrap();
        let expected_sk_bits = calculate_bit_width(&bounds.sk_bound.to_string()).unwrap();

        assert_eq!(bits.bit_sk_secret, expected_sk_bits);
    }

    #[test]
    fn test_witness_reduction_and_json_roundtrip() {
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SecretKey),
        )
        .unwrap();
        let input = share_computation_input_from_sample(&sample, DkgInputType::SecretKey);
        let witness = Witness::compute(DEFAULT_BFV_PRESET, &input).unwrap();
        let json = witness.convert_to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json).unwrap();

        assert_eq!(
            decoded.secret_crt.limbs.len(),
            witness.secret_crt.limbs.len()
        );
        assert_eq!(
            decoded.expected_secret_commitment,
            witness.expected_secret_commitment
        );
    }

    #[test]
    fn test_witness_smudging_noise_secret_consistency() {
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SmudgingNoise),
        )
        .unwrap();
        let input = share_computation_input_from_sample(&sample, DkgInputType::SmudgingNoise);
        let witness = Witness::compute(DEFAULT_BFV_PRESET, &input).unwrap();
        let degree = witness.secret_crt.limb(0).coefficients().len();
        let num_moduli = witness.secret_crt.limbs.len();
        for coeff_idx in 0..degree {
            for mod_idx in 0..num_moduli {
                let secret_coeff =
                    witness.secret_crt.limb(mod_idx).coefficients()[coeff_idx].clone();
                let y_secret = witness.y[coeff_idx][mod_idx][0].clone();
                assert_eq!(
                    secret_coeff, y_secret,
                    "secret consistency: secret_crt[{mod_idx}][{coeff_idx}] must equal y[{coeff_idx}][{mod_idx}][0]"
                );
            }
        }
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SecretKey),
        )
        .unwrap();
        let input = share_computation_input_from_sample(&sample, DkgInputType::SecretKey);
        let constants = Configs::compute(DEFAULT_BFV_PRESET, &input).unwrap();

        let json = constants.convert_to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}

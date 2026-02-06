// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the public key aggregation circuit: constants, bounds, bit widths, and witness.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) public key shares and aggregated public key. They implement [`Computation`] and are used by codegen.

use crate::bigint_1d_to_json_values;
use crate::compute_pk_aggregation_commitment;
use crate::compute_pk_bit;
use crate::crt_polynomial_to_toml_json;
use crate::get_zkp_modulus;
use crate::threshold::pk_aggregation::circuit::PkAggregationCircuit;
use crate::threshold::pk_aggregation::circuit::PkAggregationCircuitInput;
use crate::CircuitsErrors;
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::CrtPolynomial;
use num_bigint::BigInt;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`PkAggregationCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct PkAggregationComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`PkAggregationCircuit`].
impl CircuitComputation for PkAggregationCircuit {
    type Preset = BfvPreset;
    type Input = PkAggregationCircuitInput;
    type Output = PkAggregationComputationOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &())?;
        let witness = Witness::compute(preset, &input)?;

        Ok(PkAggregationComputationOutput {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub pk_bit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub pk_bound: BigUint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    pub expected_threshold_pk_commitments: Vec<BigInt>,
    pub pk0: Vec<CrtPolynomial>,
    pub pk1: Vec<CrtPolynomial>,
    pub pk0_agg: CrtPolynomial,
    pub pk1_agg: CrtPolynomial,
}

impl Computation for Configs {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, CircuitsErrors> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let moduli = threshold_params.moduli().to_vec();

        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &())?;

        Ok(Configs {
            n: threshold_params.degree(),
            l: moduli.len(),
            moduli,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let pk_bit = compute_pk_bit(&threshold_params);

        Ok(Bits { pk_bit })
    }
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let mut pk_bound_max = BigUint::from(0u32);

        for &qi in threshold_params.moduli() {
            let qi_bound: BigUint = (&BigUint::from(qi) - 1u32) / 2u32;

            if qi_bound > pk_bound_max {
                pk_bound_max = qi_bound;
            }
        }

        let bounds = Bounds {
            pk_bound: pk_bound_max,
        };

        Ok(bounds)
    }
}

impl Computation for Witness {
    type Preset = BfvPreset;
    type Input = PkAggregationCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let bit_pk = compute_pk_bit(&threshold_params);
        let moduli = threshold_params.moduli();
        let zkp_modulus = &get_zkp_modulus();

        // Coefficients must be in [0, q_i), not centered to (-q_i/2, q_i/2]. The circuit sums
        // party coefficients then applies reduce_mod to get a value in [0, q_l); the aggregated
        // key is also in [0, q_i). Centered representatives would make the sum before reduction
        // inconsistent and could break the aggregation check.

        let mut pk0: Vec<CrtPolynomial> = input.pk0_shares.clone();
        // pk1 is the same (common random polynomial a) for all parties
        let mut pk1: Vec<CrtPolynomial> = (0..input.committee.h).map(|_| input.a.clone()).collect();
        // Extract pk0_agg from aggregated public key
        let mut pk0_agg = CrtPolynomial::from_fhe_polynomial(&input.public_key.c.c[0]);
        let mut pk1_agg = input.a.clone();

        // Compute expected_threshold_pk_commitments for each honest party
        // Each commitment is computed from pk0[i] and pk1[i] for party i
        let mut expected_threshold_pk_commitments = Vec::new();

        pk0_agg.reverse();
        pk0_agg.reduce(moduli)?;
        pk0_agg.reduce_uniform(zkp_modulus);

        pk1_agg.reverse();
        pk1_agg.scalar_mul(&BigInt::from(input.committee.h));
        pk1_agg.reduce(moduli)?;
        pk1_agg.reduce_uniform(zkp_modulus);

        for party_index in 0..input.committee.h {
            pk0[party_index].reverse();
            pk0[party_index].reduce(moduli)?;
            pk0[party_index].reduce_uniform(zkp_modulus);

            pk1[party_index].reverse();
            pk1[party_index].reduce(moduli)?;
            pk1[party_index].reduce_uniform(zkp_modulus);

            let commitment =
                compute_pk_aggregation_commitment(&pk0[party_index], &pk1[party_index], bit_pk);

            expected_threshold_pk_commitments.push(commitment);
        }

        Ok(Witness {
            expected_threshold_pk_commitments,
            pk0,
            pk1,
            pk0_agg,
            pk1_agg,
        })
    }

    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        let pk0: Vec<Vec<serde_json::Value>> = self
            .pk0
            .iter()
            .map(|p| crt_polynomial_to_toml_json(p))
            .collect();
        let pk1: Vec<Vec<serde_json::Value>> = self
            .pk1
            .iter()
            .map(|p| crt_polynomial_to_toml_json(p))
            .collect();
        let pk0_agg = crt_polynomial_to_toml_json(&self.pk0_agg);
        let pk1_agg = crt_polynomial_to_toml_json(&self.pk1_agg);
        let expected_threshold_pk_commitments =
            bigint_1d_to_json_values(&self.expected_threshold_pk_commitments);

        let json = serde_json::json!({
            "expected_threshold_pk_commitments": expected_threshold_pk_commitments,
            "pk0": pk0,
            "pk1": pk1,
            "pk0_agg": pk0_agg,
            "pk1_agg": pk1_agg,
        });

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let preset = BfvPreset::InsecureThreshold512;
        let (threshold_params, _) = build_pair_for_preset(preset).unwrap();

        let bounds = Bounds::compute(preset, &()).unwrap();
        let bits = Bits::compute(preset, &()).unwrap();

        let expected_bits = compute_pk_bit(&threshold_params);

        assert_eq!(bounds.pk_bound, BigUint::from(34359701504u128));
        assert_eq!(bits.pk_bit, expected_bits);
    }
}

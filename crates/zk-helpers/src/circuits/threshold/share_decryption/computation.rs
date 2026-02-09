// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the public key generation circuit: constants, bounds, bit widths, and witness.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) a public key. They implement [`Computation`] and are used by codegen.

use crate::calculate_bit_width;
use crate::circuits::commitments::compute_aggregated_shares_commitment;
use crate::compute_pk_bit;
use crate::crt_polynomial_to_toml_json;
use crate::get_zkp_modulus;
use crate::threshold::share_decryption::circuit::ShareDecryptionCircuit;
use crate::threshold::share_decryption::circuit::ShareDecryptionCircuitInput;
use crate::CircuitsErrors;
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::CrtPolynomial;
use e3_polynomial::Polynomial;
use itertools::izip;
use num_bigint::BigInt;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use rayon::iter::ParallelBridge;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`PkGenerationCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct ShareDecryptionComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`PkGenerationCircuit`].
impl CircuitComputation for ShareDecryptionCircuit {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Output = ShareDecryptionComputationOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &bounds)?;
        let witness = Witness::compute(preset, input)?;

        Ok(ShareDecryptionComputationOutput {
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
    pub ct_bit: u32,
    pub sk_bit: u32,
    pub e_sm_bit: u32,
    pub r1_bit: u32,
    pub r2_bit: u32,
    pub d_bit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub r1_bounds: Vec<BigUint>,
    pub r2_bounds: Vec<BigUint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    pub ct0: CrtPolynomial,
    pub ct1: CrtPolynomial,
    pub sk: CrtPolynomial,
    pub e_sm: CrtPolynomial,
    pub r1: CrtPolynomial,
    pub r2: CrtPolynomial,
    pub d: CrtPolynomial,
    pub expected_sk_commitment: BigInt,
    pub expected_e_sm_commitment: BigInt,
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
        let bits = Bits::compute(preset, &bounds)?;

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
    type Input = Bounds;
    type Error = CircuitsErrors;

    fn compute(_: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        // For r1, use the maximum of all low and up bounds
        let mut r1_bit = 0;
        for bound in input.r1_bounds.iter() {
            r1_bit = r1_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        // For r2, use the maximum of all bounds
        let mut r2_bit = 0;
        for bound in &input.r2_bounds {
            r2_bit = r2_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        Ok(Bits {
            ct_bit: r2_bit,
            sk_bit: r2_bit,
            e_sm_bit: r2_bit,
            r1_bit,
            r2_bit,
            d_bit: r2_bit,
        })
    }
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let n = BigInt::from(threshold_params.degree());
        // Get cyclotomic degree and context at provided level
        let ctx = threshold_params.ctx_at_level(0)?;

        // Calculate bounds for each CRT basis
        let mut r1_bounds: Vec<BigInt> = Vec::new();
        let mut r2_bounds: Vec<BigInt> = Vec::new();
        let mut moduli: Vec<u64> = Vec::new();

        for qi in ctx.moduli_operators() {
            let qi_bigint = BigInt::from(qi.modulus());
            let qi_bound = (&qi_bigint - BigInt::from(1)) / BigInt::from(2);

            moduli.push(qi.modulus());

            // r_2j bounds: [- (q_j-1)/2 , (q_j-1)/2] (cyclotomic quotients)
            r2_bounds.push(qi_bound.clone());

            // r_1j bounds: based on the formula from the notes
            // r_1j: [(-(q_j-1)/2 * (BS.N+3) - Be) / q_j , ((q_j-1)/2 * (BS.N+3) + Be) / q_j]
            // Where BS = s_bound, Be = e_bound, N = n (degree)
            r1_bounds.push(
                (&qi_bound * (&qi_bound.clone() * &n + BigInt::from(3)) - &qi_bound.clone())
                    / &qi_bigint,
            );
        }

        let bounds = Bounds {
            r1_bounds: r1_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            r2_bounds: r2_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
        };

        Ok(bounds)
    }
}

impl Computation for Witness {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Other(e.to_string()))?;

        let moduli: Vec<BigInt> = threshold_params
            .moduli()
            .iter()
            .copied()
            .map(BigInt::from)
            .collect();
        let n = threshold_params.degree() as u64;

        // Extract and convert ciphertext polynomials
        let ct0 = CrtPolynomial::from_fhe_polynomial(&input.ciphertext.c[0]);
        let ct1 = CrtPolynomial::from_fhe_polynomial(&input.ciphertext.c[1]);

        // Create cyclotomic polynomial x^N + 1
        let mut cyclo = vec![BigInt::from(0u64); (n + 1) as usize];
        cyclo[0] = BigInt::from(1u64); // x^N term
        cyclo[n as usize] = BigInt::from(1u64); // x^0 term

        // Perform the main computation logic
        let mut results: Vec<(
            usize,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
        )> = izip!(
            moduli.clone(),
            ct0.limbs.clone(),
            ct1.limbs.clone(),
            input.s.limbs.clone(),
            input.e.limbs.clone(),
            input.d_share.limbs.clone(),
        )
        .enumerate()
        .par_bridge()
        .map(|(i, (qi, mut ct0, mut ct1, mut s, mut e, mut d_share))| {
            ct0.reverse();
            ct0.reduce(&qi);
            ct0.center(&qi);

            ct1.reverse();
            ct1.reduce(&qi);
            ct1.center(&qi);

            s.reverse();
            s.reduce(&qi);
            s.center(&qi);

            e.reverse();
            e.reduce(&qi);
            e.center(&qi);

            d_share.reverse();
            d_share.reduce(&qi);
            d_share.center(&qi);

            // Compute d_share_hat = ct0 + ct1 * s + e
            // This is the expected value before lifting to Z
            let d_share_hat = {
                // ct1 * s (degree 2*(n-1))
                let ct1_s_times = ct1.mul(&s);
                assert_eq!((ct1_s_times.coefficients().len() as u64) - 1, 2 * (n - 1));

                // ct0 + ct1 * s + e
                ct0.add(&ct1_s_times).add(&e)
            };
            assert_eq!((d_share_hat.coefficients().len() as u64) - 1, 2 * (n - 1));

            // Check whether d_share_hat mod R_qi (the ring) is equal to d_share
            let mut d_share_hat_mod_rqi = d_share_hat.reduce_by_cyclotomic(&cyclo).unwrap();

            d_share_hat_mod_rqi.reduce(&qi);
            d_share_hat_mod_rqi.center(&qi);

            assert_eq!(&d_share, &d_share_hat_mod_rqi);

            // Compute r2_numerator = d_share - d_share_hat (in Z)
            // This should be divisible by (X^N + 1) and q_i
            let r2_numerator = d_share.sub(&d_share_hat);

            let mut r2_numerator_centered = r2_numerator.clone();
            r2_numerator_centered.reduce(&qi);
            r2_numerator_centered.center(&qi);

            // First, compute r2 = (d_share - d_share_hat) / (X^N + 1) mod Z_qi
            let cyclo_polynomial = Polynomial::new(cyclo.clone());
            let (r2, r2_rem) = r2_numerator_centered.div(&cyclo_polynomial).unwrap();

            assert!(r2_rem.is_zero());
            assert_eq!((r2.coefficients().len() as u64) - 1, n - 2); // Order(r2) = N - 2

            // Assert that (d_share - d_share_hat) = (r2 * cyclo) mod Z_qi
            let r2_cyclo_times = r2.mul(&cyclo_polynomial);

            let mut r2_cyclo_times_centered = r2_cyclo_times.clone();
            r2_cyclo_times_centered.reduce(&qi);
            r2_cyclo_times_centered.center(&qi);

            assert_eq!(&r2_numerator_centered, &r2_cyclo_times_centered);
            assert_eq!(
                (r2_cyclo_times.coefficients().len() as u64) - 1,
                2 * (n - 1)
            );

            // Now compute r1 = (d_share - d_share_hat - r2 * cyclo) / q_i mod Z_p
            let r1_numerator = r2_numerator.sub(&r2_cyclo_times);

            assert_eq!((r1_numerator.coefficients().len() as u64) - 1, 2 * (n - 1));

            let qi_polynomial = Polynomial::new(vec![qi.clone()]);
            let (r1, r1_rem) = r1_numerator.div(&qi_polynomial).unwrap();

            assert!(r1_rem.is_zero());
            assert_eq!((r1.coefficients().len() as u64) - 1, 2 * (n - 1)); // Order(r1) = 2*(N-1)

            assert_eq!(&r1_numerator, &r1.mul(&qi_polynomial));

            // Assert that d_share = ct0 + ct1 * s + e + r2 * cyclo + r1 * q_i mod Z_p

            let r1_qi_times = r1.scalar_mul(&qi);
            let d_share_calculated = d_share_hat.add(&r1_qi_times).add(&r2_cyclo_times);

            assert_eq!(&d_share, &d_share_calculated.trim_leading_zeros());

            (i, ct0, ct1, s, e, d_share, r2, r1)
        })
        .collect();

        results.sort_by_key(|(i, _, _, _, _, _, _, _)| *i);

        let mut ct0 = CrtPolynomial::new(vec![]);
        let mut ct1 = CrtPolynomial::new(vec![]);
        let mut sk = CrtPolynomial::new(vec![]);
        let mut e_sm = CrtPolynomial::new(vec![]);
        let mut r1 = CrtPolynomial::new(vec![]);
        let mut r2 = CrtPolynomial::new(vec![]);
        let mut d = CrtPolynomial::new(vec![]);

        for (_i, ct0i, ct1i, si, ei, d_sharei, r2i, r1i) in results {
            ct0.add_limb(ct0i);
            ct1.add_limb(ct1i);
            sk.add_limb(si);
            e_sm.add_limb(ei);
            r1.add_limb(r1i);
            r2.add_limb(r2i);
            d.add_limb(d_sharei);
        }

        let zkp_modulus = &get_zkp_modulus();

        ct0.reduce_uniform(zkp_modulus);
        ct1.reduce_uniform(zkp_modulus);
        sk.reduce_uniform(zkp_modulus);
        e_sm.reduce_uniform(zkp_modulus);
        r1.reduce_uniform(zkp_modulus);
        r2.reduce_uniform(zkp_modulus);
        d.reduce_uniform(zkp_modulus);

        // Compute commitments to s and e (matches circuit's commitment functions)
        let pk_bit = compute_pk_bit(&threshold_params);
        let expected_sk_commitment = compute_aggregated_shares_commitment(&sk, pk_bit);
        let expected_e_sm_commitment = compute_aggregated_shares_commitment(&e_sm, pk_bit);

        Ok(Witness {
            ct0,
            ct1,
            sk,
            e_sm,
            r1,
            r2,
            d,
            expected_sk_commitment,
            expected_e_sm_commitment,
        })
    }

    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        let ct0 = crt_polynomial_to_toml_json(&self.ct0);
        let ct1 = crt_polynomial_to_toml_json(&self.ct1);
        let sk = crt_polynomial_to_toml_json(&self.sk);
        let e_sm = crt_polynomial_to_toml_json(&self.e_sm);
        let r1 = crt_polynomial_to_toml_json(&self.r1);
        let r2 = crt_polynomial_to_toml_json(&self.r2);
        let d = crt_polynomial_to_toml_json(&self.d);
        let expected_sk_commitment = self.expected_sk_commitment.to_string();
        let expected_e_sm_commitment = self.expected_e_sm_commitment.to_string();

        let json = serde_json::json!({
            "ct0": ct0,
            "ct1": ct1,
            "sk": sk,
            "e_sm": e_sm,
            "r1": r1,
            "r2": r2,
            "d": d,
            "expected_sk_commitment": expected_sk_commitment,
            "expected_e_sm_commitment": expected_e_sm_commitment,
        });

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let bounds = Bounds::compute(DEFAULT_BFV_PRESET, &()).unwrap();
        let bits = Bits::compute(DEFAULT_BFV_PRESET, &bounds).unwrap();

        let expected_bit =
            calculate_bit_width(BigInt::from(bounds.r2_bounds.iter().max().unwrap().clone()));

        assert_eq!(bits.d_bit, expected_bit);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let constants = Configs::compute(DEFAULT_BFV_PRESET, &()).unwrap();

        let json = constants.to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}

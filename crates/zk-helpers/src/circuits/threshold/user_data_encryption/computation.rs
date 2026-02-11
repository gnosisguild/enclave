// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the user data encryption circuit: constants, bounds, bit widths, and inputs.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Inputs`] are produced from BFV parameters
//! and (for input) a public key. They implement [`Computation`] and are used by codegen.

use crate::calculate_bit_width;
use crate::commitments::compute_pk_aggregation_commitment;
use crate::compute_ciphertext_commitment;
use crate::crt_polynomial_to_toml_json;
use crate::get_zkp_modulus;
use crate::math::{compute_k0is, compute_q_mod_t, compute_q_product};
use crate::math::{cyclotomic_polynomial, decompose_residue};
use crate::polynomial_to_toml_json;
use crate::threshold::user_data_encryption::circuit::UserDataEncryptionCircuit;
use crate::threshold::user_data_encryption::circuit::UserDataEncryptionCircuitInput;
use crate::utils::compute_modulus_bit;
use crate::CircuitsErrors;
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::center;
use e3_polynomial::reduce;
use e3_polynomial::CrtPolynomial;
use e3_polynomial::Polynomial;
use fhe::bfv::SecretKey;
use fhe_math::rq::Poly;
use fhe_math::rq::Representation;
use fhe_math::zq::Modulus;
use fhe_traits::Serialize as FheSerialize;
use itertools::izip;
use num_bigint::BigInt;
use num_bigint::BigUint;
use num_bigint::ToBigInt;
use num_traits::Signed;
use num_traits::ToPrimitive;
use rand::thread_rng;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Output of [`CircuitComputation::compute`] for [`UserDataEncryptionCircuit`]: bounds, bit widths, and inputs.
#[derive(Debug)]
pub struct UserDataEncryptionComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub inputs: Inputs,
}

/// Implementation of [`CircuitComputation`] for [`UserDataEncryptionCircuit`].
impl CircuitComputation for UserDataEncryptionCircuit {
    type Preset = BfvPreset;
    type Input = UserDataEncryptionCircuitInput;
    type Output = UserDataEncryptionComputationOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &bounds)?;
        let inputs = Inputs::compute(preset, input)?;

        Ok(UserDataEncryptionComputationOutput {
            bounds,
            bits,
            inputs,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub n: usize,
    pub l: usize,
    pub moduli: Vec<u64>,
    pub q_mod_t_mod_p: BigInt,
    pub k0is: Vec<u64>,
    pub bits: Bits,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub pk_bit: u32,
    pub ct_bit: u32,
    pub u_bit: u32,
    pub e0_bit: u32,
    pub e1_bit: u32,
    pub k_bit: u32,
    pub r1_bit: u32,
    pub r2_bit: u32,
    pub p1_bit: u32,
    pub p2_bit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub pk_bounds: Vec<BigUint>,
    pub u_bound: BigUint,
    pub e0_bound: BigUint,
    pub e1_bound: BigUint,
    pub k1_low_bound: BigUint,
    pub k1_up_bound: BigUint,
    pub r1_low_bounds: Vec<BigUint>,
    pub r1_up_bounds: Vec<BigUint>,
    pub r2_bounds: Vec<BigUint>,
    pub p1_bounds: Vec<BigUint>,
    pub p2_bounds: Vec<BigUint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inputs {
    pub pk0is: CrtPolynomial,
    pub pk1is: CrtPolynomial,
    pub ct0is: CrtPolynomial,
    pub ct1is: CrtPolynomial,
    pub r1is: CrtPolynomial,
    pub r2is: CrtPolynomial,
    pub p1is: CrtPolynomial,
    pub p2is: CrtPolynomial,
    pub e0is: CrtPolynomial,
    pub e0_quotients: CrtPolynomial,
    pub e0: Polynomial,
    pub e1: Polynomial,
    pub u: Polynomial,
    pub k1: Polynomial,
    pub pk_commitment: BigInt,
    pub ct_commitment: BigInt,
    pub ciphertext: Vec<u8>,
}

impl Computation for Configs {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, CircuitsErrors> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let moduli = threshold_params.moduli().to_vec();
        let plaintext = threshold_params.plaintext();
        let q = compute_q_product(&moduli);
        let q_mod_t_uint = compute_q_mod_t(&q, plaintext);
        let t = BigInt::from(plaintext);
        let p = get_zkp_modulus();

        let q_mod_t = center(&BigInt::from(q_mod_t_uint), &t);
        let q_mod_t_mod_p = reduce(&q_mod_t, &p);

        let k0is = compute_k0is(threshold_params.moduli(), threshold_params.plaintext())?;

        let bounds = Bounds::compute(preset, &())?;
        let bits = Bits::compute(preset, &bounds)?;

        Ok(Configs {
            n: threshold_params.degree(),
            l: moduli.len(),
            q_mod_t_mod_p,
            k0is,
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
        let max_pk_bound = input.pk_bounds.iter().max().unwrap();

        let pk_bit = calculate_bit_width(BigInt::from(max_pk_bound.clone()));
        // We can safely assume that the ct bound is the same as the pk bound.
        let ct_bit = calculate_bit_width(BigInt::from(max_pk_bound.clone()));
        let u_bit = calculate_bit_width(BigInt::from(input.u_bound.clone()));
        let e0_bit = calculate_bit_width(BigInt::from(input.e0_bound.clone()));
        let e1_bit = calculate_bit_width(BigInt::from(input.e1_bound.clone()));

        // For k1, use the maximum of low and up bounds
        let k1_low_bit = calculate_bit_width(BigInt::from(input.k1_low_bound.clone()));
        let k1_up_bit = calculate_bit_width(BigInt::from(input.k1_up_bound.clone()));
        let k_bit = k1_low_bit.max(k1_up_bit);

        // For r1, use the maximum of all low and up bounds
        let mut r1_bit = 0;
        for bound in input.r1_low_bounds.iter().chain(input.r1_up_bounds.iter()) {
            r1_bit = r1_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        // For r2, use the maximum of all bounds
        let mut r2_bit = 0;
        for bound in &input.r2_bounds {
            r2_bit = r2_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        // For p1, use the maximum of all bounds
        let mut p1_bit = 0;
        for bound in &input.p1_bounds {
            p1_bit = p1_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        // For p2, use the maximum of all bounds
        let mut p2_bit = 0;
        for bound in &input.p2_bounds {
            p2_bit = p2_bit.max(calculate_bit_width(BigInt::from(bound.clone())));
        }

        Ok(Bits {
            pk_bit,
            ct_bit,
            u_bit,
            e0_bit,
            e1_bit,
            k_bit,
            r1_bit,
            r2_bit,
            p1_bit,
            p2_bit,
        })
    }
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let n = BigInt::from(threshold_params.degree());
        let ctx = threshold_params.ctx_at_level(0)?;

        let t = BigInt::from(threshold_params.plaintext());

        // CBD bound
        let cbd_bound = (threshold_params.variance() * 2) as u64;
        // Uniform bound
        let uniform_bound = (threshold_params.get_error1_variance() * BigUint::from(3u32))
            .sqrt()
            .to_bigint()
            .ok_or_else(|| {
                CircuitsErrors::Other("Failed to convert uniform bound to BigInt".into())
            })?;

        let u_bound = SecretKey::sk_bound() as u128; // u_bound is the same as sk_bound

        // e0 = e1 in the fhe.rs
        let e0_bound: u128 = if threshold_params.get_error1_variance() <= &BigUint::from(16u32) {
            cbd_bound as u128
        } else {
            uniform_bound.to_u128().unwrap()
        };
        let e1_bound = cbd_bound; // e1 = e2 in the fhe.rs

        let ptxt_up_bound = (t.clone() - BigInt::from(1)) / BigInt::from(2);
        let ptxt_low_bound: BigInt = if (t.clone() % BigInt::from(2)) == BigInt::from(1) {
            -1 * ptxt_up_bound.clone()
        } else {
            -1 * ptxt_up_bound.clone() - BigInt::from(1)
        };

        let k1_low_bound: BigInt = BigInt::from(-1) * ptxt_low_bound.clone();
        let k1_up_bound: BigInt = ptxt_up_bound.clone();

        // Calculate bounds for each CRT basis
        let moduli: Vec<u64> = ctx
            .moduli_operators()
            .into_iter()
            .map(|q| q.modulus())
            .collect();
        let k0is = compute_k0is(&moduli, threshold_params.plaintext())?;

        let mut pk_bounds: Vec<BigInt> = Vec::new();
        let mut r1_low_bounds: Vec<BigInt> = Vec::new();
        let mut r1_up_bounds: Vec<BigInt> = Vec::new();
        let mut r2_bounds: Vec<BigInt> = Vec::new();
        let mut p1_bounds: Vec<BigInt> = Vec::new();
        let mut p2_bounds: Vec<BigInt> = Vec::new();

        for (i, qi) in ctx.moduli_operators().into_iter().enumerate() {
            let qi_bigint = BigInt::from(qi.modulus());
            let qi_bound = (&qi_bigint - BigInt::from(1)) / BigInt::from(2);

            let k0qi = BigInt::from(k0is[i]);

            // PK and R2 bounds (same as qi_bound)
            pk_bounds.push(qi_bound.clone());
            r2_bounds.push(qi_bound.clone());

            let e0_bound_i = e0_bound % qi_bigint.clone();

            // R1 bounds (more complex calculation)
            let r1_low: BigInt = (&ptxt_low_bound * k0qi.abs()
                - &((&n * u_bound + BigInt::from(2)) * &qi_bound + e0_bound_i.clone()))
                / &qi_bigint;
            let r1_up: BigInt = (&ptxt_up_bound * k0qi.abs()
                + ((&n * u_bound + BigInt::from(2)) * &qi_bound + e0_bound_i.clone()))
                / &qi_bigint;

            r1_low_bounds.push(BigInt::from(-1) * r1_low.clone());
            r1_up_bounds.push(r1_up.clone());

            // P1 and P2 bounds
            let p1_bound: BigInt =
                ((&n * u_bound + BigInt::from(2)) * &qi_bound + e1_bound) / &qi_bigint;
            p1_bounds.push(p1_bound.clone());
            p2_bounds.push(qi_bound.clone());
        }

        Ok(Bounds {
            pk_bounds: pk_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            u_bound: BigUint::from(u_bound as u64),
            e0_bound: BigUint::from(e0_bound),
            e1_bound: BigUint::from(e1_bound),
            k1_low_bound: BigUint::from(k1_low_bound.to_u128().unwrap()),
            k1_up_bound: BigUint::from(k1_up_bound.to_u128().unwrap()),
            r1_low_bounds: r1_low_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            r1_up_bounds: r1_up_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            r2_bounds: r2_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            p1_bounds: p1_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
            p2_bounds: p2_bounds
                .iter()
                .map(|b| BigUint::from(b.to_u128().unwrap()))
                .collect(),
        })
    }
}

impl Computation for Inputs {
    type Preset = BfvPreset;
    type Input = UserDataEncryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, input: &Self::Input) -> Result<Self, Self::Error> {
        let (threshold_params, _) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let pk_bit = compute_modulus_bit(&threshold_params);

        let pk = input.public_key.clone();
        let pt = input.plaintext.clone();

        // Encrypt using the provided public key to ensure ciphertext matches the key.
        let (ct, u_rns, e0_rns, e1_rns) = input
            .public_key
            .try_encrypt_extended(&input.plaintext, &mut thread_rng())?;

        // Context and plaintext modulus (use same ctx for e0 reconstruction and loop).
        let ctx = threshold_params.ctx_at_level(pt.level())?;

        // Reconstruct e0 in mod Q so that e0_poly row i matches e0_rns row i (same ctx).
        let mut e0_power = e0_rns.clone();
        e0_power.change_representation(Representation::PowerBasis);
        let e0_mod_q: Vec<BigUint> = Vec::<BigUint>::from(&e0_power);
        let e0_bigints: Vec<BigInt> = e0_mod_q.iter().map(|c| c.to_bigint().unwrap()).collect();
        let e0 = (*Poly::from_bigints(&e0_bigints, &ctx)
            .map_err(|e| CircuitsErrors::Other(e.to_string()))?)
        .clone();

        let t = Modulus::new(threshold_params.plaintext())
            .map_err(|e| CircuitsErrors::Fhe(fhe::Error::from(e)))?;
        let n: u64 = ctx.degree as u64;

        // Calculate k1 (independent of qi), center and reverse
        let q_mod_t = (ctx.modulus() % t.modulus()).to_u64().unwrap(); // [q]_t
        let mut k1_u64 = pt.value.deref().to_vec(); // m
        t.scalar_mul_vec(&mut k1_u64, q_mod_t); // k1 = [q*m]_t

        let mut k1 = Polynomial::from_u64_vector(k1_u64);

        k1.reverse();
        k1.center(&BigInt::from(t.modulus()));

        // Extract single vectors of u, e1, and e2 as Vec<BigInt>, center and reverse
        let mut u_rns_copy = u_rns.clone();
        let mut e0_rns_copy = e0_rns.clone();
        let mut e0_poly_copy = e0.clone();
        let mut e1_rns_copy = e1_rns.clone();

        u_rns_copy.change_representation(Representation::PowerBasis);
        e0_rns_copy.change_representation(Representation::PowerBasis);
        e0_poly_copy.change_representation(Representation::PowerBasis);
        e1_rns_copy.change_representation(Representation::PowerBasis);

        // Extract coefficients using the current API
        let u: Vec<BigInt> =
            unsafe {
                ctx.moduli_operators()[0]
                    .center_vec_vt(u_rns_copy.coefficients().row(0).as_slice().ok_or_else(
                        || CircuitsErrors::Other("Cannot center coefficients.".into()),
                    )?)
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect()
            };

        let mut e0_vec = Polynomial::new(e0_bigints.clone());

        e0_vec.reverse();

        // Center the coefficients mod Q
        let q_bigint = BigInt::from(ctx.modulus().clone());

        e0_vec.center(&q_bigint);

        let e1: Vec<BigInt> =
            unsafe {
                ctx.moduli_operators()[0]
                    .center_vec_vt(e1_rns_copy.coefficients().row(0).as_slice().ok_or_else(
                        || CircuitsErrors::Other("Cannot center coefficients.".into()),
                    )?)
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect()
            };

        // Extract and convert ciphertext and public key polynomials
        let mut ct0 = ct.c[0].clone();
        let mut ct1 = ct.c[1].clone();
        ct0.change_representation(Representation::PowerBasis);
        ct1.change_representation(Representation::PowerBasis);

        let mut pk0: Poly = pk.c.c[0].clone();
        let mut pk1: Poly = pk.c.c[1].clone();
        pk0.change_representation(Representation::PowerBasis);
        pk1.change_representation(Representation::PowerBasis);

        let cyclo = cyclotomic_polynomial(n);

        let ct0_coeffs = ct0.coefficients();
        let ct1_coeffs = ct1.coefficients();
        let pk0_coeffs = pk0.coefficients();
        let pk1_coeffs = pk1.coefficients();
        let e0_coeffs = e0_rns_copy.coefficients();
        let e0_poly_coeffs = e0_poly_copy.coefficients();

        let ct0_coeffs_rows = ct0_coeffs.rows();
        let ct1_coeffs_rows = ct1_coeffs.rows();
        let pk0_coeffs_rows = pk0_coeffs.rows();
        let pk1_coeffs_rows = pk1_coeffs.rows();
        let e0_coeffs_rows = e0_coeffs.rows();
        let e0_poly_coeffs_rows = e0_poly_coeffs.rows();

        // Perform the main computation logic
        let results: Vec<_> = izip!(
            ctx.moduli_operators(),
            ct0_coeffs_rows,
            ct1_coeffs_rows,
            pk0_coeffs_rows,
            pk1_coeffs_rows,
            e0_coeffs_rows,
            e0_poly_coeffs_rows,
        )
        .enumerate()
        .par_bridge()
        .map(
            |(
                i,
                (qi, ct0_coeffs, ct1_coeffs, pk0_coeffs, pk1_coeffs, e0_coeffs, e0_poly_coeffs),
            )| {
                // --------------------------------------------------- ct0i ---------------------------------------------------

                // Convert to vectors of bigint, center, and reverse order.
                let mut ct0i = Polynomial::from_u64_vector(ct0_coeffs.to_vec());
                let mut ct1i = Polynomial::from_u64_vector(ct1_coeffs.to_vec());
                let mut pk0i = Polynomial::from_u64_vector(pk0_coeffs.to_vec());
                let mut pk1i = Polynomial::from_u64_vector(pk1_coeffs.to_vec());

                ct0i.reverse();
                ct1i.reverse();
                pk0i.reverse();
                pk1i.reverse();

                let qi_bigint = BigInt::from(qi.modulus());

                ct0i.reduce(&qi_bigint);
                ct0i.center(&qi_bigint);
                ct1i.reduce(&qi_bigint);
                ct1i.center(&qi_bigint);
                pk0i.reduce(&qi_bigint);
                pk0i.center(&qi_bigint);
                pk1i.reduce(&qi_bigint);
                pk1i.center(&qi_bigint);

                let e0i: Vec<BigInt> = unsafe {
                    qi.center_vec_vt(
                        e0_coeffs
                            .as_slice()
                            .ok_or_else(|| "Cannot center coefficients.".to_string())
                            .unwrap(),
                    )
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect()
                };

                // Explicitly check e1is[i] == e1 mod qi (after centering and reversal)
                let e0i_from_poly: Vec<BigInt> = unsafe {
                    qi.center_vec_vt(
                        e0_poly_coeffs
                            .as_slice()
                            .ok_or_else(|| "Cannot center coefficients.".to_string())
                            .unwrap(),
                    )
                    .iter()
                    .rev()
                    .map(|&x| BigInt::from(x))
                    .collect()
                };

                // Check that e0i equals e0 reduced modulo q_i (from e0_poly)
                assert_eq!(e0i, e0i_from_poly);

                // Compute e0_quotients[i] = (e0 - e0i) / qi for each coefficient
                // This is used for CRT consistency check: e0[j] = e0i[j] + e0_quotients[i][j] * qi
                let e0_quotient: Vec<BigInt> = e0_vec
                    .coefficients()
                    .iter()
                    .zip(e0i.iter())
                    .map(|(e0_coeff, e0i_coeff)| {
                        let diff = e0_coeff - e0i_coeff;
                        // Division should be exact since e0 = e0i (mod qi)
                        let quotient = &diff / &qi_bigint;
                        // Verify the CRT relationship
                        assert_eq!(e0_coeff, &(e0i_coeff + &quotient * &qi_bigint));
                        quotient
                    })
                    .collect();

                // k0qi = -t^{-1} mod qi
                let koqi_u64 = qi.inv(qi.neg(t.modulus())).unwrap();
                let k0qi = BigInt::from(koqi_u64); // Do not need to center this

                // ki = k1 * k0qi
                let ki_poly = Polynomial::new(k1.coefficients().to_vec()).scalar_mul(&k0qi);
                let ki = ki_poly.coefficients().to_vec();

                // Calculate ct0i_hat = pk0 * ui + e0i + ki
                let ct0i_hat = {
                    let pk0i_poly = pk0i.clone();
                    let u_poly = Polynomial::new(u.clone());
                    let pk0i_times_u = pk0i_poly.mul(&u_poly);
                    assert_eq!((pk0i_times_u.coefficients().len() as u64) - 1, 2 * (n - 1));

                    let e0i_poly = Polynomial::new(e0i.clone());
                    let ki_poly = Polynomial::new(ki.clone());
                    let e0_plus_ki = e0i_poly.add(&ki_poly);
                    assert_eq!((e0_plus_ki.coefficients().len() as u64) - 1, n - 1);

                    pk0i_times_u.add(&e0_plus_ki).coefficients().to_vec()
                };
                assert_eq!((ct0i_hat.len() as u64) - 1, 2 * (n - 1));

                let ct0i_hat_poly = Polynomial::new(ct0i_hat.clone());
                let (r1i_poly, r2i_poly) =
                    decompose_residue(&ct0i, &ct0i_hat_poly, &qi_bigint, &cyclo, n);
                let r1i = r1i_poly.coefficients().to_vec();
                let r2i = r2i_poly.coefficients().to_vec();

                // --------------------------------------------------- ct1i ---------------------------------------------------

                // Calculate ct1i_hat = pk1i * ui + e1
                let ct1i_hat = {
                    let pk1i_poly = pk1i.clone();
                    let u_poly = Polynomial::new(u.clone());
                    let pk1i_times_u = pk1i_poly.mul(&u_poly);
                    assert_eq!((pk1i_times_u.coefficients().len() as u64) - 1, 2 * (n - 1));

                    let e1_poly = Polynomial::new(e1.clone());
                    pk1i_times_u.add(&e1_poly).coefficients().to_vec()
                };
                assert_eq!((ct1i_hat.len() as u64) - 1, 2 * (n - 1));

                let ct1i_hat_poly = Polynomial::new(ct1i_hat.clone());
                let (p1i_poly, p2i_poly) =
                    decompose_residue(&ct1i, &ct1i_hat_poly, &qi_bigint, &cyclo, n);
                let p1i = p1i_poly.coefficients().to_vec();
                let p2i = p2i_poly.coefficients().to_vec();

                (
                    i,
                    r2i,
                    r1i,
                    k0qi,
                    ct0i,
                    ct1i,
                    pk0i,
                    pk1i,
                    p1i,
                    p2i,
                    e0i,
                    e0_quotient,
                )
            },
        )
        .collect();

        // Sort by modulus index so CRT limbs are in order
        let mut results = results.clone();
        results.sort_by_key(|(i, ..)| *i);

        // results elements: (i, r2i, r1i, k0qi, ct0i, ct1i, pk0i, pk1i, p1i, p2i, e0i, e0_quotient)
        let mut pk0is = CrtPolynomial::from_bigint_vectors(
            results
                .iter()
                .map(|row| row.6.clone())
                .map(|pk0i| pk0i.coefficients().to_vec())
                .collect(),
        );
        let mut pk1is = CrtPolynomial::from_bigint_vectors(
            results
                .iter()
                .map(|row| row.7.clone())
                .map(|pk1i| pk1i.coefficients().to_vec())
                .collect(),
        );
        let mut ct0is = CrtPolynomial::from_bigint_vectors(
            results
                .iter()
                .map(|row| row.4.clone())
                .map(|ct0i| ct0i.coefficients().to_vec())
                .collect(),
        );
        let mut ct1is = CrtPolynomial::from_bigint_vectors(
            results
                .iter()
                .map(|row| row.5.clone())
                .map(|ct1i| ct1i.coefficients().to_vec())
                .collect(),
        );
        let mut r1is =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.2.clone()).collect());
        let mut r2is =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.1.clone()).collect());
        let mut p1is =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.8.clone()).collect());
        let mut p2is =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.9.clone()).collect());
        let mut e0is =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.10.clone()).collect());
        let mut e0_quotients =
            CrtPolynomial::from_bigint_vectors(results.iter().map(|row| row.11.clone()).collect());

        let mut e1 = Polynomial::new(e1);
        let mut u = Polynomial::new(u);

        let zkp_modulus = get_zkp_modulus();

        pk0is.reduce_uniform(&zkp_modulus);
        pk1is.reduce_uniform(&zkp_modulus);
        ct0is.reduce_uniform(&zkp_modulus);
        ct1is.reduce_uniform(&zkp_modulus);
        r1is.reduce_uniform(&zkp_modulus);
        r2is.reduce_uniform(&zkp_modulus);
        p1is.reduce_uniform(&zkp_modulus);
        p2is.reduce_uniform(&zkp_modulus);
        e0is.reduce_uniform(&zkp_modulus);
        e0_quotients.reduce_uniform(&zkp_modulus);
        e1.reduce(&zkp_modulus);
        u.reduce(&zkp_modulus);
        e0_vec.reduce(&zkp_modulus);
        k1.reduce(&zkp_modulus);

        let pk_commitment = compute_pk_aggregation_commitment(&pk0is, &pk1is, pk_bit);
        let ct_commitment = compute_ciphertext_commitment(&ct0is, &ct1is, pk_bit);

        Ok(Inputs {
            pk0is,
            pk1is,
            ct0is,
            ct1is,
            r1is,
            r2is,
            p1is,
            p2is,
            e0is,
            e0_quotients,
            e0: e0_vec,
            e1,
            u,
            k1: k1,
            pk_commitment,
            ct_commitment,
            ciphertext: ct.to_bytes(),
        })
    }

    // Used as input for Nargo execution.
    fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        let pk0is = crt_polynomial_to_toml_json(&self.pk0is);
        let pk1is = crt_polynomial_to_toml_json(&self.pk1is);
        let ct0is = crt_polynomial_to_toml_json(&self.ct0is);
        let ct1is = crt_polynomial_to_toml_json(&self.ct1is);
        let u = polynomial_to_toml_json(&self.u);
        let e0 = polynomial_to_toml_json(&self.e0);
        let e0is = crt_polynomial_to_toml_json(&self.e0is);
        let e0_quotients = crt_polynomial_to_toml_json(&self.e0_quotients);
        let e1 = polynomial_to_toml_json(&self.e1);
        let k1 = polynomial_to_toml_json(&self.k1);
        let r1is = crt_polynomial_to_toml_json(&self.r1is);
        let r2is = crt_polynomial_to_toml_json(&self.r2is);
        let p1is = crt_polynomial_to_toml_json(&self.p1is);
        let p2is = crt_polynomial_to_toml_json(&self.p2is);
        let pk_commitment = self.pk_commitment.to_string();

        let json = serde_json::json!({
            "pk0is": pk0is,
            "pk1is": pk1is,
            "ct0is": ct0is,
            "ct1is": ct1is,
            "u": u,
            "e0": e0,
            "e0is": e0is,
            "e0_quotients": e0_quotients,
            "e1": e1,
            "k1": k1,
            "r1is": r1is,
            "r2is": r2is,
            "p1is": p1is,
            "p2is": p2is,
            "pk_commitment": pk_commitment
        });

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let bounds = Bounds::compute(BfvPreset::InsecureThreshold512, &()).unwrap();
        let bits = Bits::compute(BfvPreset::InsecureThreshold512, &bounds).unwrap();

        let max_pk_bound = bounds.pk_bounds.iter().max().unwrap();
        let expected_bits = calculate_bit_width(BigInt::from(max_pk_bound.clone()));

        assert_eq!(max_pk_bound.clone(), BigUint::from(34359701504u64));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let constants = Configs::compute(BfvPreset::InsecureThreshold512, &()).unwrap();

        let json = constants.to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}

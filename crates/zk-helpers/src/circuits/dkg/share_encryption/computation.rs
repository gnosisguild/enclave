// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the share-encryption circuit: configs, bounds, bit widths, and input.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Inputs`] are produced from BFV parameters
//! and (for input) plaintext, ciphertext, and encryption randomness. Input values are
//! normalized for the ZKP field so the Noir circuit's range checks and commitment checks succeed.

use crate::circuits::commitments::{
    compute_dkg_pk_commitment, compute_share_encryption_commitment_from_message,
};
use std::ops::Deref;

use crate::dkg::share_encryption::ShareEncryptionCircuit;
use crate::dkg::share_encryption::ShareEncryptionCircuitData;
use crate::get_zkp_modulus;
use crate::math::{compute_k0is, compute_q_mod_t, compute_q_product};
use crate::math::{cyclotomic_polynomial, decompose_residue};
use crate::polynomial_to_toml_json;
use crate::utils::{compute_modulus_bit, compute_msg_bit};
use crate::CircuitsErrors;
use crate::{calculate_bit_width, crt_polynomial_to_toml_json};
use crate::{CircuitComputation, Computation};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_polynomial::Polynomial;
use e3_polynomial::{center, reduce, CrtPolynomial};
use fhe::bfv::SecretKey;
use fhe_math::rq::Poly;
use fhe_math::rq::Representation;
use fhe_math::zq::Modulus;
use itertools::izip;
use num_bigint::ToBigInt;
use num_bigint::{BigInt, BigUint};
use num_traits::{Signed, ToPrimitive};
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`ShareEncryptionCircuit`]: bounds, bit widths, and input.
#[derive(Debug)]
pub struct ShareEncryptionOutput {
    /// Coefficient bounds used to derive bit widths.
    pub bounds: Bounds,
    /// Bit widths used by the Noir prover for packing.
    pub bits: Bits,
    /// Input for the share-encryption circuit.
    pub inputs: Inputs,
}

/// Implementation of [`CircuitComputation`] for [`ShareEncryptionCircuit`].
impl CircuitComputation for ShareEncryptionCircuit {
    type Preset = BfvPreset;
    type Data = ShareEncryptionCircuitData;
    type Output = ShareEncryptionOutput;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, data: &Self::Data) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(preset, data)?;
        let bits = Bits::compute(preset, &bounds)?;
        let inputs = Inputs::compute(preset, data)?;

        Ok(ShareEncryptionOutput {
            bounds,
            bits,
            inputs,
        })
    }
}

/// Global configs for the share-encryption circuit: plaintext modulus, [q]_t, moduli, k0is, bits, and bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    /// Plaintext modulus (as usize).
    pub t: usize,
    /// [q]_t reduced to ZKP field modulus.
    pub q_mod_t: BigInt,
    /// CRT moduli (one per limb).
    pub moduli: Vec<u64>,
    /// k0_i = [1/q_i]_t per modulus, for scaling in the circuit.
    pub k0is: Vec<u64>,
    pub bits: Bits,
    pub bounds: Bounds,
}

/// Bit widths used by the Noir prover (e.g. for packing coefficients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub pk_bit: u32,
    pub ct_bit: u32,
    pub u_bit: u32,
    pub e0_bit: u32,
    pub e1_bit: u32,
    pub msg_bit: u32,
    pub r1_bit: u32,
    pub r2_bit: u32,
    pub p1_bit: u32,
    pub p2_bit: u32,
}

/// Coefficient bounds for polynomials (used to derive bit widths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub u_bound: BigUint,
    pub e0_bound: BigUint,
    pub e1_bound: BigUint,
    pub msg_bound: BigUint,
    pub pk_bounds: Vec<BigUint>,
    pub r1_low_bounds: Vec<BigUint>,
    pub r1_up_bounds: Vec<BigUint>,
    pub r2_bounds: Vec<BigUint>,
    pub p1_bounds: Vec<BigUint>,
    pub p2_bounds: Vec<BigUint>,
}

/// Input for the share-encryption circuit: CRT limbs for pk, ct, randomness, and message.
///
/// Coefficients are reduced to the ZKP field modulus for serialization. The circuit verifies
/// that the ciphertext and commitments match the public inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inputs {
    /// Public key and ciphertext polynomials in CRT form (per modulus).
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
    pub message: Polynomial,
    pub pk_commitment: BigInt,
    pub msg_commitment: BigInt,
}

impl Computation for Configs {
    type Preset = BfvPreset;
    type Data = ShareEncryptionCircuitData;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, data: &Self::Data) -> Result<Self, CircuitsErrors> {
        let (_, dkg_params) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let moduli = dkg_params.moduli().to_vec();
        let plaintext = dkg_params.plaintext();
        let q = compute_q_product(&moduli);
        let q_mod_t_uint = compute_q_mod_t(&q, plaintext);
        let t = BigInt::from(plaintext);
        let p = get_zkp_modulus();

        let q_mod_t = center(&BigInt::from(q_mod_t_uint), &t);
        let q_mod_t_mod_p = reduce(&q_mod_t, &p);

        let k0is = compute_k0is(&moduli, plaintext)?;

        let bounds = Bounds::compute(preset, data)?;
        let bits = Bits::compute(preset, &bounds)?;

        Ok(Configs {
            t: plaintext as usize,
            q_mod_t: q_mod_t_mod_p,
            moduli,
            k0is,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type Preset = BfvPreset;
    type Data = Bounds;
    type Error = crate::utils::ZkHelpersUtilsError;

    fn compute(_: Self::Preset, data: &Self::Data) -> Result<Self, Self::Error> {
        let max_pk_bound = data.pk_bounds.iter().max().unwrap();
        let max_r2_bound = data.r2_bounds.iter().max().unwrap();
        let max_p1_bound = data.p1_bounds.iter().max().unwrap();
        let max_p2_bound = data.p2_bounds.iter().max().unwrap();

        let pk_bit = calculate_bit_width(BigInt::from(max_pk_bound.clone()));
        let ct_bit = calculate_bit_width(BigInt::from(max_pk_bound.clone()));
        let u_bit = calculate_bit_width(BigInt::from(data.u_bound.clone()));
        let e0_bit = calculate_bit_width(BigInt::from(data.e0_bound.clone()));
        let e1_bit = calculate_bit_width(BigInt::from(data.e1_bound.clone()));
        let msg_bit = calculate_bit_width(BigInt::from(data.msg_bound.clone()));
        let r1_bit = calculate_bit_width(BigInt::from(
            data.r1_low_bounds
                .iter()
                .chain(data.r1_up_bounds.iter())
                .max()
                .unwrap()
                .clone(),
        ));
        let r2_bit = calculate_bit_width(BigInt::from(max_r2_bound.clone()));
        let p1_bit = calculate_bit_width(BigInt::from(max_p1_bound.clone()));
        let p2_bit = calculate_bit_width(BigInt::from(max_p2_bound.clone()));

        Ok(Bits {
            pk_bit,
            ct_bit,
            u_bit,
            e0_bit,
            e1_bit,
            msg_bit,
            r1_bit,
            r2_bit,
            p1_bit,
            p2_bit,
        })
    }
}

impl Computation for Bounds {
    type Preset = BfvPreset;
    type Data = ShareEncryptionCircuitData;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, _: &Self::Data) -> Result<Self, Self::Error> {
        let (_, dkg_params) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;

        let n = BigInt::from(dkg_params.degree());
        let ctx = dkg_params.ctx_at_level(0)?;

        let t = BigInt::from(dkg_params.plaintext());

        // CBD bound
        let cbd_bound = (dkg_params.variance() * 2) as u64;
        // Uniform bound
        let uniform_bound = (dkg_params.get_error1_variance() * BigUint::from(3u32))
            .sqrt()
            .to_bigint()
            .ok_or_else(|| {
                CircuitsErrors::Other("Failed to convert uniform bound to BigInt".into())
            })?;

        let u_bound = SecretKey::sk_bound() as u128; // u_bound is the same as sk_bound

        // e0 = e1 in the fhe.rs
        let e0_bound: u128 = if dkg_params.get_error1_variance() <= &BigUint::from(16u32) {
            cbd_bound as u128
        } else {
            uniform_bound.to_u128().unwrap()
        };
        let e1_bound = cbd_bound; // e1 = e2 in the fhe.rs

        // Message bound: message is in [0, t), so bound is t - 1
        let msg_bound = t.clone() - BigInt::from(1);

        let ptxt_up_bound = (t.clone() - BigInt::from(1)) / BigInt::from(2);
        let ptxt_low_bound: BigInt = if (t.clone() % BigInt::from(2)) == BigInt::from(1) {
            -1 * ptxt_up_bound.clone()
        } else {
            -1 * ptxt_up_bound.clone() - BigInt::from(1)
        };

        // Calculate bounds for each CRT basis
        let moduli: Vec<u64> = ctx
            .moduli_operators()
            .into_iter()
            .map(|q| q.modulus())
            .collect();
        let k0is = compute_k0is(&moduli, dkg_params.plaintext())?;

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
            msg_bound: BigUint::from(msg_bound.to_u128().unwrap()),
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
    type Data = ShareEncryptionCircuitData;
    type Error = CircuitsErrors;

    fn compute(preset: Self::Preset, data: &Self::Data) -> Result<Self, Self::Error> {
        let (_, dkg_params) =
            build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
        let ctx = dkg_params.ctx_at_level(data.plaintext.level())?;

        let pk_bit = compute_modulus_bit(&dkg_params);
        let msg_bit = compute_msg_bit(&dkg_params);

        let pk = data.public_key.clone();
        let pt = data.plaintext.clone();

        // Reconstruct e0 in mod Q so that e0_poly row i matches e0_rns row i (same ctx).
        let mut e0_power = data.e0_rns.clone();
        e0_power.change_representation(Representation::PowerBasis);
        let e0_mod_q: Vec<BigUint> = Vec::<BigUint>::from(&e0_power);
        let e0_bigints: Vec<BigInt> = e0_mod_q.iter().map(|c| c.to_bigint().unwrap()).collect();
        let e0 = (*Poly::from_bigints(&e0_bigints, &ctx)
            .map_err(|e| CircuitsErrors::Other(e.to_string()))?)
        .clone();

        let t = Modulus::new(dkg_params.plaintext())
            .map_err(|e| CircuitsErrors::Fhe(fhe::Error::from(e)))?;
        let n: u64 = ctx.degree as u64;

        let mut message = Polynomial::from_u64_vector(pt.value.deref().to_vec());
        message.reverse();

        // k1[i] = (q_mod_t * message[i]) mod t, centered to [-t/2, t/2)
        let q_mod_t = (ctx.modulus() % t.modulus()).to_u64().unwrap();
        let mut k1_u64: Vec<u64> = message
            .coefficients()
            .iter()
            .map(|c| c.to_u64().unwrap())
            .collect();
        t.scalar_mul_vec(&mut k1_u64, q_mod_t);

        let mut k1 = Polynomial::from_u64_vector(k1_u64);
        k1.center(&BigInt::from(t.modulus()));

        let mut u_rns_copy = data.u_rns.clone();
        let mut e0_rns_copy = data.e0_rns.clone();
        let mut e0_poly_copy = e0.clone();
        let mut e1_rns_copy = data.e1_rns.clone();

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
        let mut ct0 = data.ciphertext.c[0].clone(); // ct0
        let mut ct1 = data.ciphertext.c[1].clone(); // ct1
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

        let pk_commitment = compute_dkg_pk_commitment(&pk0is, &pk1is, pk_bit);
        let msg_commitment = compute_share_encryption_commitment_from_message(&message, msg_bit);

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
            message,
            pk_commitment,
            msg_commitment,
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
        let message = polynomial_to_toml_json(&self.message);
        let r1is = crt_polynomial_to_toml_json(&self.r1is);
        let r2is = crt_polynomial_to_toml_json(&self.r2is);
        let p1is = crt_polynomial_to_toml_json(&self.p1is);
        let p2is = crt_polynomial_to_toml_json(&self.p2is);
        let pk_commitment = self.pk_commitment.to_string();
        let msg_commitment = self.msg_commitment.to_string();

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
            "message": message,
            "r1is": r1is,
            "r2is": r2is,
            "p1is": p1is,
            "p2is": p2is,
            "expected_pk_commitment": pk_commitment,
            "expected_message_commitment": msg_commitment,
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
        let sd = BfvPreset::InsecureThreshold512.search_defaults().unwrap();
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareEncryptionCircuitData::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
            sd.z,
            sd.lambda,
        )
        .unwrap();

        let bounds = Bounds::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();
        let bits = Bits::compute(BfvPreset::InsecureThreshold512, &bounds).unwrap();

        let max_pk_bound = bounds.pk_bounds.iter().max().unwrap();
        let expected_bits = calculate_bit_width(BigInt::from(max_pk_bound.clone()));

        assert_eq!(max_pk_bound.clone(), BigUint::from(1125899906777088u128));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let sd = BfvPreset::InsecureThreshold512.search_defaults().unwrap();
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareEncryptionCircuitData::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
            sd.z,
            sd.lambda,
        )
        .unwrap();
        let constants = Configs::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();

        let json = constants.to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.t, constants.t);
        assert_eq!(decoded.q_mod_t, constants.q_mod_t);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.k0is, constants.k0is);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }

    #[test]
    fn test_input_message_consistency() {
        let sd = BfvPreset::InsecureThreshold512.search_defaults().unwrap();
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareEncryptionCircuitData::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
            sd.z,
            sd.lambda,
        )
        .unwrap();
        let inputs = Inputs::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();

        // inputs.message is plaintext coefficients (reversed, as used in circuit)
        let expected_message = Polynomial::from_u64_vector(sample.plaintext.value.deref().to_vec());
        let mut expected = expected_message;
        expected.reverse();

        assert_eq!(inputs.message.coefficients(), expected.coefficients());
    }
}

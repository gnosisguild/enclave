// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the user data encryption circuit: constants, bounds, bit widths, and witness.
//!
//! [`Configs`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) a public key. They implement [`Computation`] and are used by codegen.

use crate::calculate_bit_width;
use crate::commitments::compute_pk_aggregation_commitment;
use crate::compute_ciphertext_commitment;
use crate::get_zkp_modulus;
use crate::threshold::user_data_encryption::circuit::UserDataEncryptionCircuit;
use crate::threshold::user_data_encryption::circuit::UserDataEncryptionCircuitInput;
use crate::CircuitsErrors;
use crate::ConvertToJson;
use crate::{CircuitComputation, Computation};
use e3_polynomial::center;
use e3_polynomial::reduce;
use e3_polynomial::CrtPolynomial;
use e3_polynomial::Polynomial;
use fhe::bfv::BfvParameters;
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
use num_traits::Zero;
use rand::thread_rng;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Output of [`CircuitComputation::compute`] for [`UserDataEncryptionCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct UserDataEncryptionComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`UserDataEncryptionCircuit`].
impl CircuitComputation for UserDataEncryptionCircuit {
    type Params = BfvParameters;
    type Input = UserDataEncryptionCircuitInput;
    type Output = UserDataEncryptionComputationOutput;
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(params, &())?;
        let bits = Bits::compute(params, &bounds)?;
        let witness = Witness::compute(params, input)?;

        Ok(UserDataEncryptionComputationOutput {
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
    pub q_mod_t: BigInt,
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
pub struct Witness {
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
    type Params = BfvParameters;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, _: &Self::Input) -> Result<Self, CircuitsErrors> {
        let moduli = params.moduli().to_vec();
        let ctx = params.ctx_at_level(0)?;
        let modulus = BigInt::from(ctx.modulus().clone());
        let t = BigInt::from(params.plaintext());
        let p = get_zkp_modulus();

        let q_mod_t = center(&reduce(&modulus, &t), &t);
        let q_mod_t_mod_p = reduce(&q_mod_t, &p);

        let mut k0is: Vec<u64> = Vec::new();

        for qi in ctx.moduli_operators() {
            let k0qi = BigInt::from(qi.inv(qi.neg(params.plaintext())).ok_or_else(|| {
                CircuitsErrors::Fhe(fhe::Error::MathError(fhe_math::Error::Default(
                    "Failed to calculate modulus inverse for k0qi".into(),
                )))
            })?);

            k0is.push(k0qi.to_u64().unwrap_or(0));
        }

        let bounds = Bounds::compute(&params, &())?;
        let bits = Bits::compute(&params, &bounds)?;

        Ok(Configs {
            n: params.degree(),
            l: moduli.len(),
            q_mod_t,
            q_mod_t_mod_p,
            k0is,
            moduli,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type Params = BfvParameters;
    type Input = Bounds;
    type Error = crate::utils::ZkHelpersUtilsError;

    fn compute(_: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        let pk_bit = calculate_bit_width(&input.pk_bounds[0].to_string())?;
        // We can safely assume that the ct bound is the same as the pk bound.
        let ct_bit = calculate_bit_width(&input.pk_bounds[0].to_string())?;
        let u_bit = calculate_bit_width(&input.u_bound.to_string())?;
        let e0_bit = calculate_bit_width(&input.e0_bound.to_string())?;
        let e1_bit = calculate_bit_width(&input.e1_bound.to_string())?;

        // For k1, use the maximum of low and up bounds
        let k1_low_bit = calculate_bit_width(&input.k1_low_bound.to_string())?;
        let k1_up_bit = calculate_bit_width(&input.k1_up_bound.to_string())?;
        let k_bit = k1_low_bit.max(k1_up_bit);

        // For r1, use the maximum of all low and up bounds
        let mut r1_bit = 0;
        for bound in input.r1_low_bounds.iter().chain(input.r1_up_bounds.iter()) {
            r1_bit = r1_bit.max(calculate_bit_width(&bound.to_string())?);
        }

        // For r2, use the maximum of all bounds
        let mut r2_bit = 0;
        for bound in &input.r2_bounds {
            r2_bit = r2_bit.max(calculate_bit_width(&bound.to_string())?);
        }

        // For p1, use the maximum of all bounds
        let mut p1_bit = 0;
        for bound in &input.p1_bounds {
            p1_bit = p1_bit.max(calculate_bit_width(&bound.to_string())?);
        }

        // For p2, use the maximum of all bounds
        let mut p2_bit = 0;
        for bound in &input.p2_bounds {
            p2_bit = p2_bit.max(calculate_bit_width(&bound.to_string())?);
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
    type Params = BfvParameters;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, _: &Self::Input) -> Result<Self, Self::Error> {
        let n = BigInt::from(params.degree());
        let ctx = params.ctx_at_level(0)?;

        let t = BigInt::from(params.plaintext());

        // CBD bound
        let cbd_bound = (params.variance() * 2) as u64;
        // Uniform bound
        let uniform_bound = (params.get_error1_variance() * BigUint::from(3u32))
            .sqrt()
            .to_bigint()
            .ok_or_else(|| {
                CircuitsErrors::Other("Failed to convert uniform bound to BigInt".into())
            })?;

        let u_bound = SecretKey::sk_bound() as u128; // u_bound is the same as sk_bound

        // e0 = e1 in the fhe.rs
        let e0_bound: u128 = if params.get_error1_variance() <= &BigUint::from(16u32) {
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
        let _num_moduli = ctx.moduli().len();
        let mut pk_bounds: Vec<BigInt> = Vec::new();
        let mut r1_low_bounds: Vec<BigInt> = Vec::new();
        let mut r1_up_bounds: Vec<BigInt> = Vec::new();
        let mut r2_bounds: Vec<BigInt> = Vec::new();
        let mut p1_bounds: Vec<BigInt> = Vec::new();
        let mut p2_bounds: Vec<BigInt> = Vec::new();
        let mut moduli: Vec<u64> = Vec::new();
        let mut k0is: Vec<u64> = Vec::new();

        for qi in ctx.moduli_operators() {
            let qi_bigint = BigInt::from(qi.modulus());
            let qi_bound = (&qi_bigint - BigInt::from(1)) / BigInt::from(2);

            moduli.push(qi.modulus());

            // Calculate k0qi for bounds
            let k0qi = BigInt::from(qi.inv(qi.neg(params.plaintext())).ok_or_else(|| {
                CircuitsErrors::Fhe(fhe::Error::MathError(fhe_math::Error::Default(
                    "Failed to calculate modulus inverse for k0qi".into(),
                )))
            })?);
            k0is.push(k0qi.to_u64().unwrap_or(0));

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

impl Computation for Witness {
    type Params = BfvParameters;
    type Input = UserDataEncryptionCircuitInput;
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        let bounds = Bounds::compute(params, &())?;

        let bit_pk = calculate_bit_width(&bounds.pk_bounds[0].to_string())?;

        let pk = input.public_key.clone();
        let pt = input.plaintext.clone();

        // Encrypt using the provided public key to ensure ciphertext matches the key.
        let (ct, u_rns, e0_rns, e1_rns) = input
            .public_key
            .try_encrypt_extended(&input.plaintext, &mut thread_rng())?;

        // Context and plaintext modulus (use same ctx for e0 reconstruction and loop).
        let ctx = params.ctx_at_level(pt.level())?;

        // Reconstruct e0 in mod Q so that e0_poly row i matches e0_rns row i (same ctx).
        let mut e0_power = e0_rns.clone();
        e0_power.change_representation(Representation::PowerBasis);
        let e0_mod_q: Vec<BigUint> = Vec::<BigUint>::from(&e0_power);
        let e0_bigints: Vec<BigInt> = e0_mod_q.iter().map(|c| c.to_bigint().unwrap()).collect();
        let e0 = (*Poly::from_bigints(&e0_bigints, &ctx)
            .map_err(|e| CircuitsErrors::Other(e.to_string()))?)
        .clone();

        let t = Modulus::new(params.plaintext())
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

        // Create cyclotomic polynomial x^N + 1
        let mut cyclo = vec![BigInt::from(0u64); (n + 1) as usize];

        cyclo[0] = BigInt::from(1u64); // x^N term
        cyclo[n as usize] = BigInt::from(1u64); // x^0 term

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

                // Check whether ct0i_hat mod R_qi (the ring) is equal to ct0i
                let mut ct0i_hat_mod_rqi = Polynomial::new(ct0i_hat.clone());

                ct0i_hat_mod_rqi = ct0i_hat_mod_rqi.reduce_by_cyclotomic(&cyclo).unwrap();

                ct0i_hat_mod_rqi.reduce(&qi_bigint);
                ct0i_hat_mod_rqi.center(&qi_bigint);

                assert_eq!(&ct0i, &ct0i_hat_mod_rqi);

                // Compute r2i numerator = ct0i - ct0i_hat and reduce/center the polynomial
                let ct0i_poly = ct0i.clone();
                let ct0i_hat_poly = Polynomial::new(ct0i_hat.clone());
                let ct0i_minus_ct0i_hat = ct0i_poly.sub(&ct0i_hat_poly).coefficients().to_vec();
                assert_eq!((ct0i_minus_ct0i_hat.len() as u64) - 1, 2 * (n - 1));

                let mut ct0i_minus_ct0i_hat_mod_zqi = Polynomial::new(ct0i_minus_ct0i_hat.clone());

                ct0i_minus_ct0i_hat_mod_zqi.reduce(&qi_bigint);
                ct0i_minus_ct0i_hat_mod_zqi.center(&qi_bigint);

                // Compute r2i as the quotient of numerator divided by the cyclotomic polynomial
                // to produce: (ct0i - ct0i_hat) / (x^N + 1) mod Z_qi. Remainder should be empty.
                let ct0i_minus_ct0i_hat_poly = ct0i_minus_ct0i_hat_mod_zqi.clone();
                let cyclo_poly = Polynomial::new(cyclo.clone());
                let (r2i_poly, r2i_rem_poly) = ct0i_minus_ct0i_hat_poly.div(&cyclo_poly).unwrap();
                let r2i = r2i_poly.coefficients().to_vec();
                let r2i_rem = r2i_rem_poly.coefficients().to_vec();
                assert!(r2i_rem.iter().all(|x| x.is_zero()));
                assert_eq!((r2i.len() as u64) - 1, n - 2); // Order(r2i) = N - 2

                // Assert that (ct0i - ct0i_hat) = (r2i * cyclo) mod Z_qi
                let r2i_poly = Polynomial::new(r2i.clone());
                let r2i_times_cyclo = r2i_poly.mul(&cyclo_poly).coefficients().to_vec();

                let mut r2i_times_cyclo_mod_zqi = Polynomial::new(r2i_times_cyclo.clone());

                r2i_times_cyclo_mod_zqi.reduce(&qi_bigint);
                r2i_times_cyclo_mod_zqi.center(&qi_bigint);

                assert_eq!(&ct0i_minus_ct0i_hat_mod_zqi, &r2i_times_cyclo_mod_zqi);
                assert_eq!((r2i_times_cyclo.len() as u64) - 1, 2 * (n - 1));

                // Calculate r1i = (ct0i - ct0i_hat - r2i * cyclo) / qi mod Z_p. Remainder should be empty.
                let ct0i_minus_ct0i_hat_poly = Polynomial::new(ct0i_minus_ct0i_hat.clone());
                let r2i_times_cyclo_poly = Polynomial::new(r2i_times_cyclo.clone());
                let r1i_num = ct0i_minus_ct0i_hat_poly
                    .sub(&r2i_times_cyclo_poly)
                    .coefficients()
                    .to_vec();
                assert_eq!((r1i_num.len() as u64) - 1, 2 * (n - 1));

                let r1i_num_poly = Polynomial::new(r1i_num.clone());
                let qi_poly = Polynomial::new(vec![qi_bigint.clone()]);
                let (r1i_poly, r1i_rem_poly) = r1i_num_poly.div(&qi_poly).unwrap();
                let r1i = r1i_poly.coefficients().to_vec();
                let r1i_rem = r1i_rem_poly.coefficients().to_vec();
                assert!(r1i_rem.iter().all(|x| x.is_zero()));
                assert_eq!((r1i.len() as u64) - 1, 2 * (n - 1)); // Order(r1i) = 2*(N-1)
                let r1i_poly_check = Polynomial::new(r1i.clone());
                assert_eq!(
                    &r1i_num,
                    &r1i_poly_check.mul(&qi_poly).coefficients().to_vec()
                );

                // Assert that ct0i = ct0i_hat + r1i * qi + r2i * cyclo mod Z_p
                let r1i_poly = Polynomial::new(r1i.clone());
                let r1i_times_qi = r1i_poly.scalar_mul(&qi_bigint).coefficients().to_vec();
                let ct0i_hat_poly = Polynomial::new(ct0i_hat.clone());
                let r1i_times_qi_poly = Polynomial::new(r1i_times_qi.clone());
                let r2i_times_cyclo_poly = Polynomial::new(r2i_times_cyclo.clone());
                let mut ct0i_calculated = ct0i_hat_poly
                    .add(&r1i_times_qi_poly)
                    .add(&r2i_times_cyclo_poly)
                    .coefficients()
                    .to_vec();

                while !ct0i_calculated.is_empty() && ct0i_calculated[0].is_zero() {
                    ct0i_calculated.remove(0);
                }

                assert_eq!(&ct0i, &Polynomial::new(ct0i_calculated.clone()));

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

                // Check whether ct1i_hat mod R_qi (the ring) is equal to ct1i
                let mut ct1i_hat_mod_rqi = Polynomial::new(ct1i_hat.clone());

                ct1i_hat_mod_rqi = ct1i_hat_mod_rqi.reduce_by_cyclotomic(&cyclo).unwrap();
                ct1i_hat_mod_rqi.reduce(&qi_bigint);
                ct1i_hat_mod_rqi.center(&qi_bigint);

                assert_eq!(&ct1i, &ct1i_hat_mod_rqi);

                // Compute p2i numerator = ct1i - ct1i_hat
                let ct1i_poly = ct1i.clone();
                let ct1i_hat_poly = Polynomial::new(ct1i_hat.clone());
                let ct1i_minus_ct1i_hat = ct1i_poly.sub(&ct1i_hat_poly).coefficients().to_vec();
                assert_eq!((ct1i_minus_ct1i_hat.len() as u64) - 1, 2 * (n - 1));
                let mut ct1i_minus_ct1i_hat_mod_zqi = Polynomial::new(ct1i_minus_ct1i_hat.clone());

                ct1i_minus_ct1i_hat_mod_zqi.reduce(&qi_bigint);
                ct1i_minus_ct1i_hat_mod_zqi.center(&qi_bigint);

                // Compute p2i as the quotient of numerator divided by the cyclotomic polynomial,
                // and reduce/center the resulting coefficients to produce:
                // (ct1i - ct1i_hat) / (x^N + 1) mod Z_qi. Remainder should be empty.
                let ct1i_minus_ct1i_hat_poly = ct1i_minus_ct1i_hat_mod_zqi.clone();
                let (p2i_poly, p2i_rem_poly) =
                    ct1i_minus_ct1i_hat_poly.div(&cyclo_poly.clone()).unwrap();
                let p2i = p2i_poly.coefficients().to_vec();
                let p2i_rem = p2i_rem_poly.coefficients().to_vec();
                assert!(p2i_rem.iter().all(|x| x.is_zero()));
                assert_eq!((p2i.len() as u64) - 1, n - 2); // Order(p2i) = N - 2

                // Assert that (ct1i - ct1i_hat) = (p2i * cyclo) mod Z_qi
                let p2i_poly = Polynomial::new(p2i.clone());
                let p2i_times_cyclo: Vec<BigInt> =
                    p2i_poly.mul(&cyclo_poly).coefficients().to_vec();
                let mut p2i_times_cyclo_mod_zqi = Polynomial::new(p2i_times_cyclo.clone());

                p2i_times_cyclo_mod_zqi.reduce(&qi_bigint);
                p2i_times_cyclo_mod_zqi.center(&qi_bigint);

                assert_eq!(&ct1i_minus_ct1i_hat_mod_zqi, &p2i_times_cyclo_mod_zqi);
                assert_eq!((p2i_times_cyclo.len() as u64) - 1, 2 * (n - 1));

                // Calculate p1i = (ct1i - ct1i_hat - p2i * cyclo) / qi mod Z_p. Remainder should be empty.
                let ct1i_minus_ct1i_hat_poly = Polynomial::new(ct1i_minus_ct1i_hat.clone());
                let p2i_times_cyclo_poly = Polynomial::new(p2i_times_cyclo.clone());
                let p1i_num = ct1i_minus_ct1i_hat_poly
                    .sub(&p2i_times_cyclo_poly)
                    .coefficients()
                    .to_vec();
                assert_eq!((p1i_num.len() as u64) - 1, 2 * (n - 1));

                let p1i_num_poly = Polynomial::new(p1i_num.clone());
                let qi_poly = Polynomial::new(vec![BigInt::from(qi.modulus())]);
                let (p1i_poly, p1i_rem_poly) = p1i_num_poly.div(&qi_poly).unwrap();
                let p1i = p1i_poly.coefficients().to_vec();
                let p1i_rem = p1i_rem_poly.coefficients().to_vec();
                assert!(p1i_rem.iter().all(|x| x.is_zero()));
                assert_eq!((p1i.len() as u64) - 1, 2 * (n - 1)); // Order(p1i) = 2*(N-1)
                let p1i_poly_check = Polynomial::new(p1i.clone());
                assert_eq!(
                    &p1i_num,
                    &p1i_poly_check.mul(&qi_poly).coefficients().to_vec()
                );

                // Assert that ct1i = ct1i_hat + p1i * qi + p2i * cyclo mod Z_p
                let p1i_poly = Polynomial::new(p1i.clone());
                let p1i_times_qi = p1i_poly.scalar_mul(&qi_bigint).coefficients().to_vec();
                let ct1i_hat_poly = Polynomial::new(ct1i_hat.clone());
                let p1i_times_qi_poly = Polynomial::new(p1i_times_qi.clone());
                let p2i_times_cyclo_poly = Polynomial::new(p2i_times_cyclo.clone());
                let mut ct1i_calculated = ct1i_hat_poly
                    .add(&p1i_times_qi_poly)
                    .add(&p2i_times_cyclo_poly)
                    .coefficients()
                    .to_vec();

                while !ct1i_calculated.is_empty() && ct1i_calculated[0].is_zero() {
                    ct1i_calculated.remove(0);
                }

                assert_eq!(&ct1i, &Polynomial::new(ct1i_calculated.clone()));
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

        let pk_commitment = compute_pk_aggregation_commitment(&pk0is, &pk1is, bit_pk);
        let ct_commitment = compute_ciphertext_commitment(&ct0is, &ct1is, bit_pk);

        Ok(Witness {
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

    use crate::ConvertToJson;
    use crate::Sample;
    use e3_fhe_params::BfvParamSet;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let bounds = Bounds::compute(&params, &()).unwrap();
        let bits = Bits::compute(&params, &bounds).unwrap();
        let expected_bits = calculate_bit_width(&bounds.pk_bounds[0].to_string()).unwrap();

        assert_eq!(bounds.pk_bounds[0], BigUint::from(34359701504u64));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_witness_reduction_and_json_roundtrip() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let encryption_data = Sample::generate(&params);
        let witness = Witness::compute(
            &params,
            &UserDataEncryptionCircuitInput {
                public_key: encryption_data.public_key,
                plaintext: encryption_data.plaintext,
            },
        )
        .unwrap();
        let json = witness.convert_to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(decoded.pk0is, witness.pk0is);
        assert_eq!(decoded.pk1is, witness.pk1is);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let constants = Configs::compute(&params, &()).unwrap();

        let json = constants.convert_to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}

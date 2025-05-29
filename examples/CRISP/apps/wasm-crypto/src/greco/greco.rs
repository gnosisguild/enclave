use std::ops::Deref;
use std::sync::Arc;

use fhe_math::rq::{Poly, Representation};
use fhe_math::zq::Modulus;
use fhe_rs::bfv::{BfvParameters, Ciphertext, Plaintext, PublicKey};
use serde_json::json;

use crate::greco::poly::*;
use itertools::izip;
use num_bigint::BigInt;
use num_traits::*;
use rayon::iter::{ParallelBridge, ParallelIterator};

/// Set of vectors for input validation of a ciphertext
#[derive(Clone, Debug)]
pub struct InputValidationVectors {
    pk0is: Vec<Vec<BigInt>>,
    pk1is: Vec<Vec<BigInt>>,
    ct0is: Vec<Vec<BigInt>>,
    ct1is: Vec<Vec<BigInt>>,
    r1is: Vec<Vec<BigInt>>,
    r2is: Vec<Vec<BigInt>>,
    p1is: Vec<Vec<BigInt>>,
    p2is: Vec<Vec<BigInt>>,
    k0is: Vec<BigInt>,
    u: Vec<BigInt>,
    e0: Vec<BigInt>,
    e1: Vec<BigInt>,
    k1: Vec<BigInt>,
}

impl InputValidationVectors {
    /// Create a new `InputValidationVectors` with the given number of moduli and degree.
    ///
    /// # Arguments
    ///
    /// * `num_moduli` - The number of moduli, which determines the number of inner vectors in 2D vectors.
    /// * `degree` - The size of each inner vector in the 2D vectors.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `InputValidationVectors` with all fields initialized to zero.
    pub fn new(num_moduli: usize, degree: usize) -> Self {
        InputValidationVectors {
            pk0is: vec![vec![BigInt::zero(); degree]; num_moduli],
            pk1is: vec![vec![BigInt::zero(); degree]; num_moduli],
            ct0is: vec![vec![BigInt::zero(); degree]; num_moduli],
            ct1is: vec![vec![BigInt::zero(); degree]; num_moduli],
            r1is: vec![vec![BigInt::zero(); 2 * (degree - 1) + 1]; num_moduli],
            r2is: vec![vec![BigInt::zero(); degree - 1]; num_moduli],
            p1is: vec![vec![BigInt::zero(); 2 * (degree - 1) + 1]; num_moduli],
            p2is: vec![vec![BigInt::zero(); degree - 1]; num_moduli],
            k0is: vec![BigInt::zero(); num_moduli],
            u: vec![BigInt::zero(); degree],
            e0: vec![BigInt::zero(); degree],
            e1: vec![BigInt::zero(); degree],
            k1: vec![BigInt::zero(); degree],
        }
    }

    /// Assign and return all of the centered input validation vectors to the ZKP modulus `p`.
    ///
    /// # Arguments
    ///
    /// * `p` - ZKP modulus
    ///
    /// # Returns
    ///
    /// Returns a new `InputValidationVectors` struct with all coefficients reduced modulo `p`.
    pub fn standard_form(&self, p: &BigInt) -> Self {
        InputValidationVectors {
            pk0is: reduce_coefficients_2d(&self.pk0is, p),
            pk1is: reduce_coefficients_2d(&self.pk1is, p),
            ct0is: reduce_coefficients_2d(&self.ct0is, p),
            ct1is: reduce_coefficients_2d(&self.ct1is, p),
            r1is: reduce_coefficients_2d(&self.r1is, p),
            r2is: reduce_coefficients_2d(&self.r2is, p),
            p1is: reduce_coefficients_2d(&self.p1is, p),
            p2is: reduce_coefficients_2d(&self.p2is, p),
            k0is: self.k0is.clone(),
            u: reduce_coefficients(&self.u, p),
            e0: reduce_coefficients(&self.e0, p),
            e1: reduce_coefficients(&self.e1, p),
            k1: reduce_coefficients(&self.k1, p),
        }
    }

    /// Convert the `InputValidationVectors` to a JSON object.
    ///
    /// # Returns
    ///
    /// Returns a `serde_json::Value` representing the JSON serialization of the `InputValidationVectors`.
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "pk0is": to_string_2d_vec(&self.pk0is),
            "pk1is": to_string_2d_vec(&self.pk1is),
            "u": to_string_1d_vec(&self.u),
            "e0": to_string_1d_vec(&self.e0),
            "e1": to_string_1d_vec(&self.e1),
            "k1": to_string_1d_vec(&self.k1),
            "r2is": to_string_2d_vec(&self.r2is),
            "r1is": to_string_2d_vec(&self.r1is),
            "p2is": to_string_2d_vec(&self.p2is),
            "p1is": to_string_2d_vec(&self.p1is),
            "ct0is": to_string_2d_vec(&self.ct0is),
            "ct1is": to_string_2d_vec(&self.ct1is),
        })
    }

    /// Check whether all members of `self` have the correct length based on the provided `degree` and `num_moduli`.
    ///
    /// # Arguments
    ///
    /// * `num_moduli` - The expected number of moduli (outer vector length).
    /// * `degree` - The expected degree (inner vector length).
    ///
    /// # Returns
    ///
    /// Returns `true` if all vectors have the correct lengths, `false` otherwise.
    pub fn check_correct_lengths(&self, num_moduli: usize, degree: usize) -> bool {
        // Helper function to check 2D vector lengths
        let check_2d_lengths =
            |vec: &Vec<Vec<BigInt>>, expected_outer_len: usize, expected_inner_len: usize| {
                vec.len() == expected_outer_len && vec.iter().all(|v| v.len() == expected_inner_len)
            };

        // Helper function to check 1D vector lengths
        let check_1d_lengths = |vec: &Vec<BigInt>, expected_len: usize| vec.len() == expected_len;

        // Use all to combine all checks into a single statement
        [
            // 2D vector checks
            check_2d_lengths(&self.pk0is, num_moduli, degree),
            check_2d_lengths(&self.pk1is, num_moduli, degree),
            check_2d_lengths(&self.ct0is, num_moduli, degree),
            check_2d_lengths(&self.ct1is, num_moduli, degree),
            check_2d_lengths(&self.r1is, num_moduli, 2 * (degree - 1)),
            check_2d_lengths(&self.r2is, num_moduli, degree - 2),
            check_2d_lengths(&self.p1is, num_moduli, 2 * (degree - 1)),
            check_2d_lengths(&self.p2is, num_moduli, degree - 2),
            // 1D vector checks
            check_1d_lengths(&self.k0is, num_moduli),
            check_1d_lengths(&self.u, degree),
            check_1d_lengths(&self.e0, degree),
            check_1d_lengths(&self.e1, degree),
            check_1d_lengths(&self.k1, degree),
        ]
        .iter()
        .all(|&check| check)
    }

    /// Create the centered validation vectors necessary for creating an input validation proof according to Greco.
    /// For more information, please see https://eprint.iacr.org/2024/594.
    ///
    /// # Arguments
    ///
    /// * `pt` - Plaintext from fhe.rs.
    /// * `u_rns` - Private polynomial used in ciphertext sampled from secret key distribution.
    /// * `e0_rns` - Error polynomial used in ciphertext sampled from error distribution.
    /// * `e1_rns` - Error polynomioal used in cihpertext sampled from error distribution.
    /// * `ct` - Ciphertext from fhe.rs.
    /// * `pk` - Public Key from fhe.rs.
    pub fn compute(
        pt: &Plaintext,
        u_rns: &Poly,
        e0_rns: &Poly,
        e1_rns: &Poly,
        ct: &Ciphertext,
        pk: &PublicKey,
    ) -> Result<InputValidationVectors, Box<dyn std::error::Error>> {
        // Get context, plaintext modulus, and degree
        let params = &pk.par;
        let ctx = params.ctx_at_level(pt.level())?;
        let t = Modulus::new(params.plaintext())?;
        let N: u64 = ctx.degree as u64;

        // Calculate k1 (independent of qi), center and reverse
        let q_mod_t = (ctx.modulus() % t.modulus())
            .to_u64()
            .ok_or_else(|| "Cannot convert BigInt to u64.".to_string())?; // [q]_t
        let mut k1_u64 = pt.value.deref().to_vec(); // m

        t.scalar_mul_vec(&mut k1_u64, q_mod_t); // k1 = [q*m]_t

        let mut k1: Vec<BigInt> = k1_u64.iter().map(|&x| BigInt::from(x)).rev().collect();
        reduce_and_center_coefficients_mut(&mut k1, &BigInt::from(t.modulus()));

        // Extract single vectors of u, e1, and e2 as Vec<BigInt>, center and reverse
        let mut u_rns_copy = u_rns.clone();
        let mut e0_rns_copy = e0_rns.clone();
        let mut e1_rns_copy = e1_rns.clone();

        u_rns_copy.change_representation(Representation::PowerBasis);
        e0_rns_copy.change_representation(Representation::PowerBasis);
        e1_rns_copy.change_representation(Representation::PowerBasis);

        let u: Vec<BigInt> = unsafe {
            ctx.moduli_operators()[0]
                .center_vec_vt(
                    u_rns_copy
                        .coefficients()
                        .row(0)
                        .as_slice()
                        .ok_or_else(|| "Cannot center coefficients.".to_string())?,
                )
                .iter()
                .rev()
                .map(|&x| BigInt::from(x))
                .collect()
        };

        let e0: Vec<BigInt> = unsafe {
            ctx.moduli_operators()[0]
                .center_vec_vt(
                    e0_rns_copy
                        .coefficients()
                        .row(0)
                        .as_slice()
                        .ok_or_else(|| "Cannot center coefficients.".to_string())?,
                )
                .iter()
                .rev()
                .map(|&x| BigInt::from(x))
                .collect()
        };

        let e1: Vec<BigInt> = unsafe {
            ctx.moduli_operators()[0]
                .center_vec_vt(
                    e1_rns_copy
                        .coefficients()
                        .row(0)
                        .as_slice()
                        .ok_or_else(|| "Cannot center coefficients.".to_string())?,
                )
                .iter()
                .rev()
                .map(|&x| BigInt::from(x))
                .collect()
        };

        // Extract and convert ciphertext and plaintext polynomials
        let mut ct0 = ct.c[0].clone();
        let mut ct1 = ct.c[1].clone();
        ct0.change_representation(Representation::PowerBasis);
        ct1.change_representation(Representation::PowerBasis);

        let mut pk0: Poly = pk.c.c[0].clone();
        let mut pk1: Poly = pk.c.c[1].clone();
        pk0.change_representation(Representation::PowerBasis);
        pk1.change_representation(Representation::PowerBasis);

        // Create cyclotomic polynomial x^N + 1
        let mut cyclo = vec![BigInt::from(0u64); (N + 1) as usize];

        cyclo[0] = BigInt::from(1u64); // x^N term
        cyclo[N as usize] = BigInt::from(1u64); // x^0 term

        // Initialize matrices to store results
        let num_moduli = ctx.moduli().len();
        let mut res = InputValidationVectors::new(num_moduli, N as usize);

        // Perform the main computation logic
        let results: Vec<(
            usize,
            Vec<BigInt>,
            Vec<BigInt>,
            BigInt,
            Vec<BigInt>,
            Vec<BigInt>,
            Vec<BigInt>,
            Vec<BigInt>,
            Vec<BigInt>,
            Vec<BigInt>,
        )> = izip!(
            ctx.moduli_operators(),
            ct0.coefficients().rows(),
            ct1.coefficients().rows(),
            pk0.coefficients().rows(),
            pk1.coefficients().rows()
        )
        .enumerate()
        .par_bridge()
        .map(
            |(i, (qi, ct0_coeffs, ct1_coeffs, pk0_coeffs, pk1_coeffs))| {
                // --------------------------------------------------- ct0i ---------------------------------------------------

                // Convert to vectors of bigint, center, and reverse order.
                let mut ct0i: Vec<BigInt> =
                    ct0_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();
                let mut ct1i: Vec<BigInt> =
                    ct1_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();
                let mut pk0i: Vec<BigInt> =
                    pk0_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();
                let mut pk1i: Vec<BigInt> =
                    pk1_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();

                let qi_bigint = BigInt::from(qi.modulus());

                reduce_and_center_coefficients_mut(&mut ct0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut ct1i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut pk0i, &qi_bigint);
                reduce_and_center_coefficients_mut(&mut pk1i, &qi_bigint);

                // k0qi = -t^{-1} mod qi

                let koqi_u64 = qi.inv(qi.neg(t.modulus())).unwrap();
                let k0qi = BigInt::from(koqi_u64); // Do not need to center this

                // ki = k1 * k0qi
                let ki = poly_scalar_mul(&k1, &k0qi);

                // Calculate ct0i_hat = pk0 * ui + e0i + ki
                let ct0i_hat = {
                    let pk0i_times_u = poly_mul(&pk0i, &u);
                    assert_eq!((pk0i_times_u.len() as u64) - 1, 2 * (N - 1));
                    let e0_plus_ki = poly_add(&e0, &ki);
                    assert_eq!((e0_plus_ki.len() as u64) - 1, N - 1);

                    poly_add(&pk0i_times_u, &e0_plus_ki)
                };
                assert_eq!((ct0i_hat.len() as u64) - 1, 2 * (N - 1));

                // Check whether ct0i_hat mod R_qi (the ring) is equal to ct0i
                let mut ct0i_hat_mod_rqi = ct0i_hat.clone();
                reduce_in_ring(&mut ct0i_hat_mod_rqi, &cyclo, &qi_bigint);
                assert_eq!(&ct0i, &ct0i_hat_mod_rqi);

                // Compute r2i numerator = ct0i - ct0i_hat and reduce/center the polynomial
                let ct0i_minus_ct0i_hat = poly_sub(&ct0i, &ct0i_hat);
                assert_eq!((ct0i_minus_ct0i_hat.len() as u64) - 1, 2 * (N - 1));
                let mut ct0i_minus_ct0i_hat_mod_zqi = ct0i_minus_ct0i_hat.clone();
                reduce_and_center_coefficients_mut(&mut ct0i_minus_ct0i_hat_mod_zqi, &qi_bigint);

                // Compute r2i as the quotient of numerator divided by the cyclotomic polynomial
                // to produce: (ct0i - ct0i_hat) / (x^N + 1) mod Z_qi. Remainder should be empty.
                let (r2i, r2i_rem) = poly_div(&ct0i_minus_ct0i_hat_mod_zqi, &cyclo);
                assert!(r2i_rem.is_empty());
                assert_eq!((r2i.len() as u64) - 1, N - 2); // Order(r2i) = N - 2

                // Assert that (ct0i - ct0i_hat) = (r2i * cyclo) mod Z_qi
                let r2i_times_cyclo = poly_mul(&r2i, &cyclo);
                let mut r2i_times_cyclo_mod_zqi = r2i_times_cyclo.clone();
                reduce_and_center_coefficients_mut(&mut r2i_times_cyclo_mod_zqi, &qi_bigint);
                assert_eq!(&ct0i_minus_ct0i_hat_mod_zqi, &r2i_times_cyclo_mod_zqi);
                assert_eq!((r2i_times_cyclo.len() as u64) - 1, 2 * (N - 1));

                // Calculate r1i = (ct0i - ct0i_hat - r2i * cyclo) / qi mod Z_p. Remainder should be empty.
                let r1i_num = poly_sub(&ct0i_minus_ct0i_hat, &r2i_times_cyclo);
                assert_eq!((r1i_num.len() as u64) - 1, 2 * (N - 1));

                let (r1i, r1i_rem) = poly_div(&r1i_num, &[qi_bigint.clone()]);
                assert!(r1i_rem.is_empty());
                assert_eq!((r1i.len() as u64) - 1, 2 * (N - 1)); // Order(r1i) = 2*(N-1)
                assert_eq!(&r1i_num, &poly_mul(&r1i, &[qi_bigint.clone()]));

                // Assert that ct0i = ct0i_hat + r1i * qi + r2i * cyclo mod Z_p
                let r1i_times_qi = poly_scalar_mul(&r1i, &qi_bigint);
                let mut ct0i_calculated =
                    poly_add(&poly_add(&ct0i_hat, &r1i_times_qi), &r2i_times_cyclo);

                while ct0i_calculated.len() > 0 && ct0i_calculated[0].is_zero() {
                    ct0i_calculated.remove(0);
                }

                assert_eq!(&ct0i, &ct0i_calculated);

                // --------------------------------------------------- ct1i ---------------------------------------------------

                // Calculate ct1i_hat = pk1i * ui + e1i
                let ct1i_hat = {
                    let pk1i_times_u = poly_mul(&pk1i, &u);
                    assert_eq!((pk1i_times_u.len() as u64) - 1, 2 * (N - 1));

                    poly_add(&pk1i_times_u, &e1)
                };
                assert_eq!((ct1i_hat.len() as u64) - 1, 2 * (N - 1));

                // Check whether ct1i_hat mod R_qi (the ring) is equal to ct1i
                let mut ct1i_hat_mod_rqi = ct1i_hat.clone();
                reduce_in_ring(&mut ct1i_hat_mod_rqi, &cyclo, &qi_bigint);
                assert_eq!(&ct1i, &ct1i_hat_mod_rqi);

                // Compute p2i numerator = ct1i - ct1i_hat
                let ct1i_minus_ct1i_hat = poly_sub(&ct1i, &ct1i_hat);
                assert_eq!((ct1i_minus_ct1i_hat.len() as u64) - 1, 2 * (N - 1));
                let mut ct1i_minus_ct1i_hat_mod_zqi = ct1i_minus_ct1i_hat.clone();
                reduce_and_center_coefficients_mut(&mut ct1i_minus_ct1i_hat_mod_zqi, &qi_bigint);

                // Compute p2i as the quotient of numerator divided by the cyclotomic polynomial,
                // and reduce/center the resulting coefficients to produce:
                // (ct1i - ct1i_hat) / (x^N + 1) mod Z_qi. Remainder should be empty.
                let (p2i, p2i_rem) = poly_div(&ct1i_minus_ct1i_hat_mod_zqi, &cyclo.clone());
                assert!(p2i_rem.is_empty());
                assert_eq!((p2i.len() as u64) - 1, N - 2); // Order(p2i) = N - 2

                // Assert that (ct1i - ct1i_hat) = (p2i * cyclo) mod Z_qi
                let p2i_times_cyclo: Vec<BigInt> = poly_mul(&p2i, &cyclo);
                let mut p2i_times_cyclo_mod_zqi = p2i_times_cyclo.clone();
                reduce_and_center_coefficients_mut(&mut p2i_times_cyclo_mod_zqi, &qi_bigint);
                assert_eq!(&ct1i_minus_ct1i_hat_mod_zqi, &p2i_times_cyclo_mod_zqi);
                assert_eq!((p2i_times_cyclo.len() as u64) - 1, 2 * (N - 1));

                // Calculate p1i = (ct1i - ct1i_hat - p2i * cyclo) / qi mod Z_p. Remainder should be empty.
                let p1i_num = poly_sub(&ct1i_minus_ct1i_hat, &p2i_times_cyclo);
                assert_eq!((p1i_num.len() as u64) - 1, 2 * (N - 1));

                let (p1i, p1i_rem) = poly_div(&p1i_num, &[BigInt::from(qi.modulus())]);
                assert!(p1i_rem.is_empty());
                assert_eq!((p1i.len() as u64) - 1, 2 * (N - 1)); // Order(p1i) = 2*(N-1)
                assert_eq!(&p1i_num, &poly_mul(&p1i, &[qi_bigint.clone()]));

                // Assert that ct1i = ct1i_hat + p1i * qi + p2i * cyclo mod Z_p
                let p1i_times_qi = poly_scalar_mul(&p1i, &qi_bigint);
                let mut ct1i_calculated =
                    poly_add(&poly_add(&ct1i_hat, &p1i_times_qi), &p2i_times_cyclo);

                while ct1i_calculated.len() > 0 && ct1i_calculated[0].is_zero() {
                    ct1i_calculated.remove(0);
                }

                assert_eq!(&ct1i, &ct1i_calculated);
                (i, r2i, r1i, k0qi, ct0i, ct1i, pk0i, pk1i, p1i, p2i)
            },
        )
        .collect();

        // Merge results into the `res` structure after parallel execution
        for (i, r2i, r1i, k0i, ct0i, ct1i, pk0i, pk1i, p1i, p2i) in results.into_iter() {
            res.r2is[i] = r2i;
            res.r1is[i] = r1i;
            res.k0is[i] = k0i;
            res.ct0is[i] = ct0i;
            res.ct1is[i] = ct1i;
            res.pk0is[i] = pk0i;
            res.pk1is[i] = pk1i;
            res.p1is[i] = p1i;
            res.p2is[i] = p2i;
        }

        // Set final result vectors
        res.u = u;
        res.e0 = e0;
        res.e1 = e1;
        res.k1 = k1;

        Ok(res)
    }
}

/// The `InputValidationBounds` struct holds the bounds for various vectors and polynomials used in the input validation process.
/// These bounds are calculated from a set of BFV encryption parameters and represent limits on the values of different fields
/// to ensure that the inputs remain within valid ranges during operations.
#[derive(Clone, Debug)]
pub struct InputValidationBounds {
    u: BigInt,
    e: BigInt,
    k1_low: BigInt,
    k1_up: BigInt,
    pk: Vec<BigInt>,
    r1_low: Vec<BigInt>,
    r1_up: Vec<BigInt>,
    r2: Vec<BigInt>,
    p1: Vec<BigInt>,
    p2: Vec<BigInt>,
}

impl InputValidationBounds {
    /// Checks the constraints of the input validation vectors against the bounds stored in `InputValidationBounds`.
    ///
    /// # Arguments
    ///
    /// * `vecs` - A reference to `InputValidationVectors`, which contains the vectors to be validated.
    /// * `p` - The prime modulus used in the encryption scheme.
    ///
    /// This function checks whether the coefficients of the vectors `u`, `e0`, `e1`, `k1`, and others are within
    /// the specified ranges, using both centered and standard range checks. It asserts that the vectors stay within
    /// these predefined bounds.
    pub fn check_constraints(&self, vecs: &InputValidationVectors, p: &BigInt) {
        let vecs_std = vecs.standard_form(p);

        // constraint. The coefficients of u, e0, e1 should be in the range [-⌈6σ⌋, ⌈6σ⌋]
        // where ⌈6σ⌋ is the upper bound of the discrete Gaussian distribution
        assert!(range_check_centered(&vecs.u, &-&self.u, &self.u));
        assert!(range_check_centered(&vecs.e0, &-&self.e, &self.e));
        assert!(range_check_centered(&vecs.e1, &-&self.e, &self.e));
        assert!(range_check_standard(&vecs_std.u, &self.u, &p));
        assert!(range_check_standard(&vecs_std.e0, &self.e, &p));
        assert!(range_check_standard(&vecs_std.e1, &self.e, &p));

        // constraint. The coefficients of k1 should be in the range [-(t-1)]/2, (t-1)/2]
        assert!(range_check_centered(&vecs.k1, &self.k1_low, &self.k1_up));
        assert!(range_check_standard_2bounds(
            &vecs_std.k1,
            &self.k1_low,
            &self.k1_up,
            &p
        ));

        // Perform asserts for polynomials depending on each qi
        for i in 0..self.r2.len() {
            // constraint. The coefficients of ct0i and ct1i should be in the range [-(qi-1)/2, (qi-1)/2]
            assert!(range_check_centered(
                &vecs.ct0is[i],
                &-&self.pk[i],
                &self.pk[i]
            ));
            assert!(range_check_centered(
                &vecs.ct1is[i],
                &-&self.pk[i],
                &self.pk[i]
            ));

            // constraint. The coefficients of pk0i and pk1i should be in range [-(qi-1)/2 , (qi-1)/2]
            assert!(range_check_centered(
                &vecs.pk0is[i],
                &-&self.pk[i],
                &self.pk[i]
            ));
            assert!(range_check_centered(
                &vecs.pk1is[i],
                &-&self.pk[i],
                &self.pk[i]
            ));
            assert!(range_check_standard(&vecs_std.pk0is[i], &self.pk[i], &p));
            assert!(range_check_standard(&vecs_std.pk1is[i], &self.pk[i], &p));

            // constraint. The coefficients of r2i should be in the range [-(qi-1)/2, (qi-1)/2]
            assert!(range_check_centered(
                &vecs.r2is[i],
                &-&self.r2[i],
                &self.r2[i]
            ));
            assert!(range_check_standard(&vecs_std.r2is[i], &self.r2[i], &p));

            // constraint. The coefficients of (ct0i - ct0i_hat - r2i * cyclo) / qi = r1i should be in the range
            // $[
            //      \frac{ \frac{-(t - 1)}{2} \cdot |K_{0,i}| - ((N \cdot B +2) \cdot \frac{q_i - 1}{2} + B )}{q_i},
            //      \frac{ \frac{t - 1}{2} \cdot |K_{0,i}| +  (N \cdot B+2) \cdot \frac{q_i - 1}{2} + B }{q_i}
            // ]$
            assert!(range_check_centered(
                &vecs.r1is[i],
                &self.r1_low[i],
                &self.r1_up[i]
            ));
            assert!(range_check_standard_2bounds(
                &vecs_std.r1is[i],
                &self.r1_low[i],
                &self.r1_up[i],
                &p
            ));

            // constraint. The coefficients of p2 should be in the range [-(qi-1)/2, (qi-1)/2]
            assert!(range_check_centered(
                &vecs.p2is[i],
                &-&self.p2[i],
                &self.p2[i]
            ));
            assert!(range_check_standard(&vecs_std.p2is[i], &self.p2[i], &p));

            // constraint. The coefficients of (ct0i - ct0i_hat - p2i * cyclo) / qi = p1i should be in the range
            // $[
            //      \frac{- ((N \cdot B +2) \cdot \frac{q_i - 1}{2} + B)}{q_i},
            //      \frac{   (N \cdot B +2) \cdot \frac{q_i - 1}{2} + B }{q_i}
            // ]$
            assert!(range_check_centered(
                &vecs.p1is[i],
                &-&self.p1[i],
                &self.p1[i]
            ));
            assert!(range_check_standard(&vecs_std.p1is[i], &self.p1[i], &p));
        }
    }

    /// Compute the input validation bounds from a set of BFV encryption parameters.
    ///
    /// # Arguments
    ///
    /// * `params` - A reference to the BFV parameters.
    /// * `level` - The encryption level, which determines the number of moduli used.
    ///
    /// # Returns
    ///
    /// A new `InputValidationBounds` instance containing the bounds for vectors and polynomials
    /// based on the BFV parameters and the specified level.
    pub fn compute(
        params: &Arc<BfvParameters>,
        level: usize,
    ) -> Result<InputValidationBounds, Box<dyn std::error::Error>> {
        // Get cyclotomic degree and context at provided level
        let N = BigInt::from(params.degree());
        let t = BigInt::from(params.plaintext());
        let ctx = params.ctx_at_level(level)?;

        // Note: the secret key in fhe.rs is sampled from a discrete gaussian distribution
        // rather than a ternary distribution as in bfv.py.
        let gauss_bound = BigInt::from(
            f64::ceil(6_f64 * f64::sqrt(params.variance() as f64))
                .to_i64()
                .ok_or_else(|| "Failed to convert variance to i64".to_string())?,
        );
        let u_bound = gauss_bound.clone();
        let e_bound = gauss_bound.clone();

        //Note we have two different variables for lower bound and upper bound, as in the case
        //where the plaintext modulus is even, the lower bound cannot be calculated by just
        //negating the upper bound. For instance, if t = 8, then the lower bound will be -4 and the
        //upper bound will be 3
        let ptxt_up_bound = (t.clone() - BigInt::from(1)) / BigInt::from(2);
        let ptxt_low_bound = if (t.clone() % BigInt::from(2)) == BigInt::from(1) {
            (-&(t.clone() - BigInt::from(1))) / BigInt::from(2)
        } else {
            ((-&(t.clone() - BigInt::from(1))) / BigInt::from(2)) - BigInt::from(1)
        };

        let k1_low_bound = ptxt_low_bound.clone();
        let k1_up_bound = ptxt_up_bound.clone();

        // Calculate qi-based bounds
        let num_moduli = ctx.moduli().len();
        let mut pk_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        let mut r2_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        let mut r1_low_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        let mut r1_up_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        let mut p2_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        let mut p1_bounds: Vec<BigInt> = vec![BigInt::zero(); num_moduli];
        for (i, qi) in ctx.moduli_operators().iter().enumerate() {
            let qi_bigint = BigInt::from(qi.modulus());
            let qi_bound = (&qi_bigint - BigInt::from(1)) / BigInt::from(2);

            // Calculate the k0qi for the bounds (these are also constant wrt BFV params)
            let k0qi = BigInt::from(
                qi.inv(qi.neg(params.plaintext()))
                    .ok_or_else(|| "Failed to calculate modulus inverse for k0qi".to_string())?,
            );

            pk_bounds[i] = qi_bound.clone();
            r2_bounds[i] = qi_bound.clone();

            r1_low_bounds[i] = (&ptxt_low_bound * BigInt::abs(&k0qi)
                - &((&N * &gauss_bound + 2) * &qi_bound + &gauss_bound))
                / &qi_bigint;
            r1_up_bounds[i] = (&ptxt_up_bound * BigInt::abs(&k0qi)
                + ((&N * &gauss_bound + 2) * &qi_bound + &gauss_bound))
                / &qi_bigint;

            p2_bounds[i] = qi_bound.clone();
            p1_bounds[i] = ((&N * &gauss_bound + 2) * &qi_bound + &gauss_bound) / &qi_bigint;
        }

        Ok(InputValidationBounds {
            u: u_bound,
            e: e_bound,
            k1_low: k1_low_bound,
            k1_up: k1_up_bound,
            pk: pk_bounds,
            r1_low: r1_low_bounds,
            r1_up: r1_up_bounds,
            r2: r2_bounds,
            p1: p1_bounds,
            p2: p2_bounds,
        })
    }
}

fn to_string_1d_vec(poly: &Vec<BigInt>) -> Vec<String> {
    poly.iter().map(|coef| coef.to_string()).collect()
}

fn to_string_2d_vec(poly: &Vec<Vec<BigInt>>) -> Vec<Vec<String>> {
    poly.iter().map(|row| to_string_1d_vec(row)).collect()
}

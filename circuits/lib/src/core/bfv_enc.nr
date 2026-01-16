// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::{
    compute_bfv_enc_challenge_commitment, compute_pk_bfv_commitment,
    compute_shares_party_modulus_commitment, prepare_message_commitment_payload,
    prepare_pk_commitment_payload,
};
use crate::math::helpers::flatten;
use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for BFV encryption circuit.
pub struct Configs<let L: u32> {
    /// Plaintext modulus t
    pub t: Field,
    /// Q mod t (for scaling message)
    pub q_mod_t: Field,
    /// CRT moduli for each basis: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Scaling factors for each basis: [k0_0, k0_1, ..., k0_{L-1}]
    pub k0is: [Field; L],
    /// Bounds for public key polynomials for each CRT basis
    pub pk_bounds: [Field; L],
    /// Bounds for error polynomials (e0)
    pub e0_bound: Field,
    /// Bounds for error polynomials (e1)
    pub e1_bound: Field,
    /// Bound for secret polynomial u (ternary distribution)
    pub u_bound: Field,
    /// Lower bounds for r1 polynomials (modulus switching quotients)
    pub r1_low_bounds: [Field; L],
    /// Upper bounds for r1 polynomials (modulus switching quotients)
    pub r1_up_bounds: [Field; L],
    /// Bounds for r2 polynomials (cyclotomic reduction quotients)
    pub r2_bounds: [Field; L],
    /// Bounds for p1 polynomials (modulus switching quotients)
    pub p1_bounds: [Field; L],
    /// Bounds for p2 polynomials (cyclotomic reduction quotients)
    pub p2_bounds: [Field; L],
    /// Bound for message polynomial (m)
    pub msg_bound: Field,
}

impl<let L: u32> Configs<L> {
    pub fn new(
        t: Field,
        q_mod_t: Field,
        qis: [Field; L],
        k0is: [Field; L],
        pk_bounds: [Field; L],
        e0_bound: Field,
        e1_bound: Field,
        u_bound: Field,
        r1_low_bounds: [Field; L],
        r1_up_bounds: [Field; L],
        r2_bounds: [Field; L],
        p1_bounds: [Field; L],
        p2_bounds: [Field; L],
        msg_bound: Field,
    ) -> Self {
        Configs {
            t,
            q_mod_t,
            qis,
            k0is,
            pk_bounds,
            e0_bound,
            e1_bound,
            u_bound,
            r1_low_bounds,
            r1_up_bounds,
            r2_bounds,
            p1_bounds,
            p2_bounds,
            msg_bound,
        }
    }
}

/// BFV Encryption Circuit (Circuit 3).
///
/// Verifies:
/// 1. Public key commitment matches expected (from Circuit 0)
/// 2. Message commitment matches expected (from SK shares circuit)
/// 3. Correct BFV encryption: ct0[l] = pk0[l] * u + e0[l] + k1 * k0[l] + r1[l] * q[l] + r2[l] * (X^N + 1)
///    and ct1[l] = pk1[l] * u + e1 + p2[l] * (X^N + 1) + p1[l] * q[l]
pub struct EncryptionBfv<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_E1: u32, let BIT_MSG: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> {
    /// Circuit parameters
    configs: Configs<L>,
    /// Expected commitment to public key (from Circuit 0)
    expected_pk_commitment: Field,
    /// Expected commitment to message (from SK shares verification circuit)
    expected_message_commitment: Field,
    /// Public key component 0 for each CRT basis (public inputs)
    pk0is: [Polynomial<N>; L],
    /// Public key component 1 for each CRT basis (public inputs)
    pk1is: [Polynomial<N>; L],
    /// Ciphertext component 0 for each CRT basis (public inputs)
    ct0is: [Polynomial<N>; L],
    /// Ciphertext component 1 for each CRT basis (public inputs)
    ct1is: [Polynomial<N>; L],
    /// Random ternary polynomial u (secret witness)
    u: Polynomial<N>,
    /// Error polynomial e0 (secret witness)
    e0: Polynomial<N>,
    /// Per-basis error polynomials e0[l] (secret witnesses)
    e0is: [Polynomial<N>; L],
    /// CRT quotients for e0 (secret witnesses)
    e0_quotients: [Polynomial<N>; L],
    /// Error polynomial e1 (secret witness)
    e1: Polynomial<N>,
    /// Raw message polynomial (secret witness)
    message: Polynomial<N>,
    /// Modulus switching quotient polynomials r1 (secret witnesses, degree 2N-1)
    r1is: [Polynomial<(2 * N) - 1>; L],
    /// Cyclotomic reduction quotient polynomials r2 (secret witnesses, degree N-1)
    r2is: [Polynomial<N - 1>; L],
    /// Modulus switching quotient polynomials p1 (secret witnesses, degree 2N-1)
    p1is: [Polynomial<(2 * N) - 1>; L],
    /// Cyclotomic reduction quotient polynomials p2 (secret witnesses, degree N-1)
    p2is: [Polynomial<N - 1>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_E1: u32, let BIT_MSG: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> EncryptionBfv<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E0, BIT_E1, BIT_MSG, BIT_R1, BIT_R2, BIT_P1, BIT_P2> {
    pub fn new(
        configs: Configs<L>,
        expected_pk_commitment: Field,
        expected_message_commitment: Field,
        pk0is: [Polynomial<N>; L],
        pk1is: [Polynomial<N>; L],
        ct0is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e0: Polynomial<N>,
        e0is: [Polynomial<N>; L],
        e0_quotients: [Polynomial<N>; L],
        e1: Polynomial<N>,
        message: Polynomial<N>,
        r1is: [Polynomial<2 * N - 1>; L],
        r2is: [Polynomial<N - 1>; L],
        p1is: [Polynomial<2 * N - 1>; L],
        p2is: [Polynomial<N - 1>; L],
    ) -> Self {
        EncryptionBfv {
            configs,
            expected_pk_commitment,
            expected_message_commitment,
            pk0is,
            pk1is,
            ct0is,
            ct1is,
            u,
            e0,
            e0is,
            e0_quotients,
            e1,
            message,
            r1is,
            r2is,
            p1is,
            p2is,
        }
    }

    /// Verifies that the public key hashes to the expected commitment
    fn verify_pk_commitment(self) {
        let payload = prepare_pk_commitment_payload::<N, L, BIT_PK>(self.pk0is, self.pk1is);
        let computed_commitment = compute_pk_bfv_commitment(payload);
        assert(
            computed_commitment == self.expected_pk_commitment,
            "Public key commitment mismatch",
        );
    }

    /// Verifies that the message polynomial hashes to the expected commitment
    fn verify_message_commitment(self) {
        let payload = prepare_message_commitment_payload::<N>(self.message);
        let computed_commitment = compute_shares_party_modulus_commitment(payload);
        assert(
            computed_commitment == self.expected_message_commitment,
            "Message commitment mismatch",
        );
    }

    /// Computes the scaled message k1 from the raw message in centered form
    /// k1[i] = (q_mod_t * message[i]) mod t, centered to [-t/2, t/2)
    fn compute_scaled_message(self) -> Polynomial<N> {
        let t = self.configs.t;
        let t_mod = ModU128::new(t);
        let q_mod_t: Field = self.configs.q_mod_t;
        let mut k1_coeffs: [Field; N] = [0; N];

        // Integer division for t_half
        let t_half: u128 = (t as u128) / 2;

        for i in 0..N {
            let msg_i: Field = self.message.coefficients[i];
            let q_times_m_mod_t = t_mod.mul_mod(q_mod_t, msg_i);

            // Check if centering is needed (value > t/2 means negative in centered form)
            let needs_centering = (q_times_m_mod_t as u128) > t_half;

            k1_coeffs[i] = if needs_centering {
                // Value is in (t/2, t), negative in centered form
                // Represent as P - magnitude (i.e., 0 - magnitude in Field)
                let magnitude = t - q_times_m_mod_t;
                0 - magnitude
            } else {
                // Value is in [0, t/2], stays positive
                q_times_m_mod_t
            };
        }

        Polynomial { coefficients: k1_coeffs }
    }

    /// Flattens all polynomial coefficients into a single array for Fiat-Shamir challenge generation
    fn payload(self, k1: Polynomial<N>) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Use pk commitment instead of full pk polynomials
        inputs.push(self.expected_pk_commitment);

        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct0is);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct1is);

        inputs = flatten::<_, _, BIT_E0>(inputs, [self.e0]);
        inputs = flatten::<_, _, BIT_E1>(inputs, [self.e1]);
        inputs = flatten::<_, _, BIT_U>(inputs, [self.u]);

        // Use message commitment instead of full message
        inputs.push(self.expected_message_commitment);

        // Include computed k1 in payload
        for i in 0..N {
            inputs.push(k1.coefficients[i]);
        }

        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1is);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2is);
        inputs = flatten::<_, _, BIT_P1>(inputs, self.p1is);
        inputs = flatten::<_, _, BIT_P2>(inputs, self.p2is);

        inputs
    }

    /// Performs coefficient-wise CRT consistency check for the e0 error polynomial
    fn check_e0_crt_consistency(self) {
        for i in 0..L {
            for j in 0..N {
                assert(
                    self.e0.coefficients[j]
                        == self.e0is[i].coefficients[j]
                            + self.e0_quotients[i].coefficients[j] * self.configs.qis[i],
                );
            }
        }
    }

    /// Main verification function
    pub fn verify(self) -> bool {
        // Step 1: Verify public key commitment matches expected
        self.verify_pk_commitment();

        // Step 2: Verify message commitment matches expected
        self.verify_message_commitment();

        // Step 3: Perform range checks
        self.check_range_bounds();

        // Step 4: Check CRT consistency for e0
        self.check_e0_crt_consistency();

        // Step 5: Compute scaled message k1 from message
        let k1 = self.compute_scaled_message();

        // Step 6: Generate Fiat-Shamir challenges
        let gammas = self.generate_challenge(k1);

        // Step 7: Verify encryption constraints
        self.verify_evaluations(gammas, k1)
    }

    /// Performs range checks on all secret witness polynomial coefficients
    fn check_range_bounds(self) {
        self.u.range_check_2bounds::<BIT_U>(self.configs.u_bound, self.configs.u_bound);
        self.e0.range_check_2bounds::<BIT_E0>(self.configs.e0_bound, self.configs.e0_bound);
        self.e1.range_check_2bounds::<BIT_E1>(self.configs.e1_bound, self.configs.e1_bound);

        // Message should be in [0, t)
        self.message.range_check_standard::<BIT_MSG>(self.configs.msg_bound);

        for i in 0..L {
            self.pk0is[i].range_check_2bounds::<BIT_PK>(
                self.configs.pk_bounds[i],
                self.configs.pk_bounds[i],
            );
            self.pk1is[i].range_check_2bounds::<BIT_PK>(
                self.configs.pk_bounds[i],
                self.configs.pk_bounds[i],
            );

            self.r1is[i].range_check_2bounds::<BIT_R1>(
                self.configs.r1_up_bounds[i],
                self.configs.r1_low_bounds[i],
            );
            self.r2is[i].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[i],
                self.configs.r2_bounds[i],
            );

            self.p1is[i].range_check_2bounds::<BIT_P1>(
                self.configs.p1_bounds[i],
                self.configs.p1_bounds[i],
            );
            self.p2is[i].range_check_2bounds::<BIT_P2>(
                self.configs.p2_bounds[i],
                self.configs.p2_bounds[i],
            );
        }
    }

    /// Generates Fiat-Shamir challenge values using the SAFE cryptographic sponge
    fn generate_challenge(self, k1: Polynomial<N>) -> Vec<Field> {
        let inputs = self.payload(k1);

        compute_bfv_enc_challenge_commitment::<L>(inputs)
    }

    /// Verifies BFV encryption constraints using Fiat-Shamir challenges and the Schwartz-Zippel lemma
    fn verify_evaluations(self, gammas: Vec<Field>, k1: Polynomial<N>) -> bool {
        let gamma = gammas.get(0);
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;
        let u_at_gamma = self.u.eval(gamma);
        let e1_at_gamma = self.e1.eval(gamma);
        let k1_at_gamma = k1.eval(gamma);

        let mut sum = (0, 0);
        for i in 0..L {
            let pk0is_at_gamma = self.pk0is[i].eval(gamma);
            let r1i_at_gamma = self.r1is[i].eval(gamma);
            let r2i_at_gamma = self.r2is[i].eval(gamma);
            let e0is_at_gamma = self.e0is[i].eval(gamma);

            // Verify ct0 equation: ct0[i] = pk0[i]*u + e0[i] + k1*k0[i] + r1[i]*q[i] + r2[i]*cyclo
            let mut ct0_rhs = (pk0is_at_gamma * u_at_gamma) + e0is_at_gamma;
            ct0_rhs += k1_at_gamma * self.configs.k0is[i];
            ct0_rhs += r1i_at_gamma * self.configs.qis[i];
            ct0_rhs += r2i_at_gamma * cyclo_at_gamma;
            let ct0_lhs = self.ct0is[i].eval(gamma);

            // Verify ct1 equation: ct1[i] = pk1[i]*u + e1 + p2[i]*cyclo + p1[i]*q[i]
            let pk1is_at_gamma = self.pk1is[i].eval(gamma);
            let p1is_at_gamma = self.p1is[i].eval(gamma);
            let p2is_at_gamma = self.p2is[i].eval(gamma);
            let mut ct1_rhs = (pk1is_at_gamma * u_at_gamma) + e1_at_gamma;
            ct1_rhs += p2is_at_gamma * cyclo_at_gamma;
            ct1_rhs += p1is_at_gamma * self.configs.qis[i];
            let ct1_lhs = self.ct1is[i].eval(gamma);

            // Accumulate weighted sums for batch verification
            let gamma_i = if i == 0 { 1 } else { gammas.get(i) };
            sum = (
                sum.0 + ct0_lhs * gamma_i + ct1_lhs * gammas.get(i + L),
                sum.1 + ct0_rhs * gamma_i + ct1_rhs * gammas.get(i + L),
            );
        }

        sum.0 == sum.1
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;
use dep::bignum::BigNum;
use dep::bignum::bignum::to_field;
use dep::bignum::Enclave_TRBFV_8192;

/// Cryptographic parameters for decryption share aggregation circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Plaintext modulus (typically denoted as `t`)
    pub plaintext_modulus: Field,
    /// Precomputed value: `-Q^(-1) mod t` where Q is the product of all CRT moduli
    pub q_inverse_mod_t: Field,
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L], plaintext_modulus: Field, q_inverse_mod_t: Field) -> Self {
        Configs { qis, plaintext_modulus, q_inverse_mod_t }
    }
}

/// Decryption Share Aggregation Circuit (Circuit 7)
///
/// Verifies:
/// 1. Lagrange interpolation to compute u^{(l)} for each CRT basis
/// 2. CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global
/// 3. Decoding verification: message = -Q^{-1} * (t * u_global)_Q mod t
pub struct DecryptionSharesAggregation<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> {
    /// Circuit parameters including crypto constants
    configs: Configs<L>,

    /// Decryption shares from t+1 parties (public witnesses)
    decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],

    /// Party IDs (x-coordinates) for interpolation (public witnesses)
    /// Note: Must be in strictly increasing order for correct Lagrange sign computation
    party_ids: [Field; T + 1],

    /// Message polynomial m(x) (public witness)
    message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// Global u polynomial (secret witness)
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// CRT quotient polynomials (secret witnesses)
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
}

impl<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> DecryptionSharesAggregation<MAX_MSG_NON_ZERO_COEFFS, L, T, BIT_NOISE> {
    pub fn new(
        configs: Configs<L>,
        decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
        party_ids: [Field; T + 1],
        message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
    ) -> Self {
        DecryptionSharesAggregation {
            configs,
            decryption_shares,
            party_ids,
            message,
            u_global,
            crt_quotients,
        }
    }

    /// Main verification function
    pub fn verify(self) {
        // Step 1: Compute Lagrange coefficients in-circuit
        let lagrange_coeffs = self.compute_all_lagrange_coeffs();

        // Step 2: Compute u^{(l)} for each CRT basis via Lagrange interpolation
        let u_crts = self.compute_crt_components(lagrange_coeffs);

        // Step 3: Verify CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global
        self.verify_crt_reconstruction(u_crts);

        // Step 4: Verify decoding
        self.verify_decoding();
    }

    /// Alternative verification function using efficient modular arithmetic without BigNum.
    ///
    /// Uses `ModU128` for decoding verification instead of BigNum. More efficient when
    /// Q (the product of all CRT moduli) fits within u128 (e.g., 72 bits for insecure parameter sets).
    pub fn verify_no_bn(self) {
        // Step 1: Compute Lagrange coefficients in-circuit
        let lagrange_coeffs = self.compute_all_lagrange_coeffs();

        // Step 2: Compute u^{(l)} for each CRT basis via Lagrange interpolation
        let u_crts = self.compute_crt_components(lagrange_coeffs);

        // Step 3: Verify CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global
        self.verify_crt_reconstruction(u_crts);

        // Step 4: Verify decoding
        self.verify_decoding_no_bn();
    }

    /// Computes all Lagrange coefficients using optimized modular arithmetic
    fn compute_all_lagrange_coeffs(self) -> [[Field; T + 1]; L] {
        let mut lagrange_coeffs = [[0 as Field; T + 1]; L];

        // Step 1: Cache |x_i - x_j| factors for all party pairs
        let mut diffs = [[0 as Field; T + 1]; T + 1];
        for i in 0..(T + 1) {
            for j in (i + 1)..(T + 1) {
                let diff = self.party_ids[j] - self.party_ids[i];
                diffs[i][j] = diff;
                diffs[j][i] = diff;
            }
        }

        // Step 2: Determine signs (same for all parties within a basis)
        let numerator_sign_negative = (T % 2) == 1;

        // Step 3: For each CRT basis, compute Lagrange coefficients
        for basis_idx in 0..L {
            let q_l = self.configs.qis[basis_idx];
            let m = ModU128::new(q_l);

            // Compute product of all party IDs: PRODUCT(j=0..T) x_j mod q_l
            let mut product_x = 1 as Field;
            for j in 0..(T + 1) {
                product_x = m.mul_mod(product_x, self.party_ids[j]);
            }

            // For each party i, compute L_i(0) mod q_l
            for party_idx in 0..(T + 1) {
                // Numerator (absolute value): PRODUCT(j!=party_idx) x_j
                let numerator_abs = m.div_mod(product_x, self.party_ids[party_idx]);

                // Denominator (absolute value): PRODUCT(j!=party_idx) |x_party_idx - x_j|
                let mut denominator_abs = 1 as Field;
                for j in 0..(T + 1) {
                    if j != party_idx {
                        denominator_abs = m.mul_mod(denominator_abs, diffs[party_idx][j]);
                    }
                }

                // Determine denominator sign
                let num_greater = T - party_idx;
                let denom_sign_negative = (num_greater % 2) == 1;

                // Compute unsigned result: |numerator| / |denominator| mod q_l
                let result_abs = m.div_mod(numerator_abs, denominator_abs);

                // Apply combined sign
                let should_negate = numerator_sign_negative != denom_sign_negative;
                let result = if should_negate {
                    m.reduce_mod(q_l - result_abs)
                } else {
                    result_abs
                };

                lagrange_coeffs[basis_idx][party_idx] = result;
            }
        }

        lagrange_coeffs
    }

    /// Computes u^{(l)} for each CRT basis via Lagrange interpolation
    fn compute_crt_components(
        self,
        lagrange_coeffs: [[Field; T + 1]; L],
    ) -> [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L] {
        let mut u_crts: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L] =
            [Polynomial::new([0; MAX_MSG_NON_ZERO_COEFFS]); L];

        for basis_idx in 0..L {
            let q_l = self.configs.qis[basis_idx];
            let m = ModU128::new(q_l);
            let mut u_coeffs = [0 as Field; MAX_MSG_NON_ZERO_COEFFS];

            // For each coefficient position
            for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
                let mut u_coeff = 0 as Field;

                // Sum all contributions: u = SUM(i=0..T) [d_i * L_i(0)] mod q_l
                for party_idx in 0..(T + 1) {
                    let d_coeff =
                        self.decryption_shares[party_idx][basis_idx].coefficients[coeff_idx];
                    let l_i_0 = lagrange_coeffs[basis_idx][party_idx];

                    let term = m.mul_mod(d_coeff, l_i_0);
                    u_coeff = m.reduce_mod(u_coeff + term);
                }

                u_coeffs[coeff_idx] = u_coeff;
            }

            u_crts[basis_idx] = Polynomial::new(u_coeffs);
        }

        u_crts
    }

    /// Verifies CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global for all bases l
    fn verify_crt_reconstruction(self, u_crts: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]) {
        for basis_idx in 0..L {
            let q_l = self.configs.qis[basis_idx];

            // Compute r^{(l)} * q_l
            let r_times_q = self.crt_quotients[basis_idx].mul_scalar(q_l);

            // Compute u^{(l)} + r^{(l)} * q_l
            let reconstructed = u_crts[basis_idx].add(r_times_q);

            // Verify: u^{(l)} + r^{(l)} * q_l = u_global
            // Must hold for every coefficient
            for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
                assert(
                    reconstructed.coefficients[coeff_idx] == self.u_global.coefficients[coeff_idx],
                    "CRT reconstruction verification failed",
                );
            }
        }
    }

    /// Verifies decoding using BigNum for large Q values
    fn verify_decoding(self) {
        // Compute Q as product of all CRT moduli
        let mut q_modulus = 1 as Field;
        for l in 0..L {
            q_modulus *= self.configs.qis[l];
        }

        // For centered arithmetic
        let q_bn = Enclave_TRBFV_8192::from(q_modulus);

        let q_half_bn = q_bn.udiv(Enclave_TRBFV_8192::from(2));

        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            // Compute (t * u_global) mod Q using BigNum
            let u_bn = Enclave_TRBFV_8192::from(self.u_global.coefficients[coeff_idx]);
            let t_bn = Enclave_TRBFV_8192::from(self.configs.plaintext_modulus);
            let u_bn_mod_q = u_bn.umod(q_bn);
            let t_bn_mod_q = t_bn.umod(q_bn);
            let t_times_u_bn_q = (t_bn_mod_q * u_bn_mod_q).umod(q_bn);

            let m = ModU128::new(self.configs.plaintext_modulus);

            // Check if centering is needed
            let needs_centering = t_times_u_bn_q > q_half_bn;

            let computed_message = if needs_centering {
                // When (t*u) mod Q >= Q/2: treat as negative in centered form
                // Centered value is conceptually negative: (t_times_u_mod_q - Q)
                // -Q^{-1} * (negative) = positive result
                let centered_positive = q_bn - t_times_u_bn_q;
                let centered_positive_mod_t = centered_positive.umod(t_bn);
                let centered_field = to_field(centered_positive_mod_t);
                m.mul_mod(self.configs.q_inverse_mod_t, centered_field)
            } else {
                // When (t*u) mod Q < Q/2: stays positive in centered form
                // -Q^{-1} * (positive) = negative result = t - result
                let t_times_u_bn_t = t_times_u_bn_q.umod(t_bn);
                let t_times_u_field = to_field(t_times_u_bn_t);
                let product = m.mul_mod(self.configs.q_inverse_mod_t, t_times_u_field);
                if product == 0 {
                    0
                } else {
                    self.configs.plaintext_modulus - product
                }
            };

            // Verify: only check non-zero coefficients
            if self.message.coefficients[coeff_idx] != 0 {
                assert(computed_message == self.message.coefficients[coeff_idx]);
            }
        }
    }

    /// Alternative decoding verification using the formula:
    /// message = -Q^{-1} * (t * u_global)_Q mod t
    fn verify_decoding_no_bn(self) {
        let t: Field = self.configs.plaintext_modulus;
        // Compute Q as product of all CRT moduli
        let mut q_modulus = 1;
        for l in 0..L {
            q_modulus *= self.configs.qis[l];
        }

        // For centered arithmetic
        let q_half = q_modulus as u128 / 2;

        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            // Compute (t * u_global) mod Q using BigNum
            let q_mod = ModU128::new(q_modulus);
            let t_mod = ModU128::new(t);

            // Compute (t * u_global) mod Q using modular arithmetic functions
            let t_times_u_mod_q = q_mod.mul_mod(t, self.u_global.coefficients[coeff_idx]);
            let needs_centering = (t_times_u_mod_q as u128) > q_half;

            let computed_message = if needs_centering {
                // When (t*u) mod Q >= Q/2: treat as negative in centered form
                // Centered value is conceptually negative: (t_times_u_mod_q - Q)
                // -Q^{-1} * (negative) = positive result
                let centered_positive = q_modulus - t_times_u_mod_q;
                let centered_positive_mod_t = t_mod.reduce_mod(centered_positive);

                t_mod.mul_mod(self.configs.q_inverse_mod_t, centered_positive_mod_t)
            } else {
                // When (t*u) mod Q < Q/2: stays positive in centered form
                // -Q^{-1} * (positive) = negative result = t - result
                let t_times_u_mod_t = t_mod.reduce_mod(t_times_u_mod_q);
                let product = t_mod.mul_mod(self.configs.q_inverse_mod_t, t_times_u_mod_t);
                if product == 0 {
                    0
                } else {
                    t - product
                }
            };

            // Verify: only check non-zero coefficients
            if self.message.coefficients[coeff_idx] != 0 {
                assert(computed_message == self.message.coefficients[coeff_idx]);
            }
        }
    }
}

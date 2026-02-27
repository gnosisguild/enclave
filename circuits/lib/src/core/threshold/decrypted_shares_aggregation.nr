// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::modulo::U128::ModU128;
use crate::math::polynomial::Polynomial;
use dep::bignum::BigNum;
use dep::bignum::bignum::to_field;
use dep::bignum::SecureThreshold8192;

/// Cryptographic parameters for decryption share aggregation circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Plaintext modulus (typically denoted as `t`)
    pub plaintext_modulus: Field,
    /// Precomputed value: -Q^{-1} mod t, where Q = q_0 * q_1 * ... * q_{L-1}.
    /// Used in the final decoding step to recover the message from u_global.
    pub q_inverse_mod_t: Field,
}

impl<let L: u32> Configs<L> {
    pub fn new(qis: [Field; L], plaintext_modulus: Field, q_inverse_mod_t: Field) -> Self {
        Configs { qis, plaintext_modulus, q_inverse_mod_t }
    }
}

/// Decryption Share Aggregation Circuit - BigNum variant (C7).
///
/// Combines T+1 public decryption shares (from C6) into the final plaintext via
/// Lagrange interpolation, CRT reconstruction, and BFV decoding. This is the last
/// circuit in the Enclave protocol.
///
/// Use this variant when Q (the product of all CRT moduli) exceeds 128 bits,
/// as is the case for cryptographically secure parameter sets. For smaller parameter
/// sets where Q fits within 128 bits, use DecryptedSharesAggregationModular instead.
pub struct DecryptedSharesAggregationBigNum<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> {
    configs: Configs<L>,

    /// Public decryption shares from T+1 ciphernodes, as produced by C6.
    /// decryption_shares[party_idx][basis_idx] is party i's share for CRT basis l.
    /// (public witnesses)
    decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],

    /// Shamir x-coordinates for the T+1 participating parties.
    /// Must be in strictly increasing order, the sign computation in Lagrange
    /// interpolation depends on this ordering.
    /// (public witnesses)
    party_ids: [Field; T + 1],

    /// Claimed output plaintext polynomial.
    /// (public witness)
    message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// Global decryption value lifted from CRT representation.
    /// Verified against u^{(l)} via CRT reconstruction check.
    /// (secret witness)
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// Quotient polynomials witnessing the CRT lift:
    /// u^{(l)} + crt_quotients[l] * q_l == u_global for each basis l.
    /// (secret witnesses)
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
}

/// Decryption Share Aggregation Circuit - Modular variant (C7).
///
/// Identical verification logic to DecryptedSharesAggregationBigNum, but uses
/// ModU128 arithmetic for the decoding step instead of BigNum. More efficient
/// when Q (the product of all CRT moduli) fits within 128 bits, e.g. for smaller
/// or non-production parameter sets. For cryptographically secure parameter sets
/// where Q exceeds 128 bits, use DecryptedSharesAggregationBigNum instead.
pub struct DecryptedSharesAggregationModular<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> {
    configs: Configs<L>,

    /// Public decryption shares from T+1 ciphernodes, as produced by C6.
    /// decryption_shares[party_idx][basis_idx] is party i's share for CRT basis l.
    /// (public witnesses)
    decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],

    /// Shamir x-coordinates for the T+1 participating parties.
    /// Must be in strictly increasing order, the sign computation in Lagrange
    /// interpolation depends on this ordering.
    /// (public witnesses)
    party_ids: [Field; T + 1],

    /// Claimed output plaintext polynomial.
    /// (public witness)
    message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// Global decryption value lifted from CRT representation.
    /// Verified against u^{(l)} via CRT reconstruction check.
    /// (secret witness)
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,

    /// Quotient polynomials witnessing the CRT lift:
    /// u^{(l)} + crt_quotients[l] * q_l == u_global for each basis l.
    /// (secret witnesses)
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
}

impl<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> DecryptedSharesAggregationBigNum<MAX_MSG_NON_ZERO_COEFFS, L, T, BIT_NOISE> {
    pub fn new(
        configs: Configs<L>,
        decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
        party_ids: [Field; T + 1],
        message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
    ) -> Self {
        DecryptedSharesAggregationBigNum {
            configs,
            decryption_shares,
            party_ids,
            message,
            u_global,
            crt_quotients,
        }
    }

    /// Executes the decryption share aggregation circuit.
    ///
    /// Runs all four verification steps in order: Lagrange coefficient computation,
    /// interpolation, CRT reconstruction, and decoding.
    pub fn execute(self) {
        // Step 1: Compute Lagrange coefficients in-circuit
        let lagrange_coeffs = compute_all_lagrange_coeffs::<T, L>(self.configs.qis, self.party_ids);

        // Step 2: Compute u^{(l)} for each CRT basis via Lagrange interpolation
        let u_crts = compute_crt_components::<MAX_MSG_NON_ZERO_COEFFS, L, T>(
            self.configs.qis,
            self.decryption_shares,
            lagrange_coeffs,
        );

        // Step 3: Verify CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global
        verify_crt_reconstruction::<MAX_MSG_NON_ZERO_COEFFS, L>(
            self.configs.qis,
            self.u_global,
            self.crt_quotients,
            u_crts,
        );

        // Step 4: Verify decoding
        self.verify_decoding();
    }

    /// Verifies the BFV decoding step using BigNum arithmetic.
    ///
    /// For each coefficient, computes message = -Q^{-1} * (t * u_global)_Q mod t
    /// using SecureThreshold8192 to handle Q values that exceed 128 bits.
    ///
    /// Centered representation: if (t * u_global) mod Q >= Q/2, the value is treated
    /// as negative, flipping the sign of the -Q^{-1} multiplication. Only non-zero
    /// message coefficients are checked, zero coefficients are unconstrained.
    fn verify_decoding(self) {
        // Compute Q as product of all CRT moduli
        let mut q_modulus = 1 as Field;
        for l in 0..L {
            q_modulus *= self.configs.qis[l];
        }

        // For centered arithmetic
        let q_bn = SecureThreshold8192::from(q_modulus);

        let q_half_bn = q_bn.udiv(SecureThreshold8192::from(2));

        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            // Compute (t * u_global) mod Q using BigNum
            let u_bn = SecureThreshold8192::from(self.u_global.coefficients[coeff_idx]);
            let t_bn = SecureThreshold8192::from(self.configs.plaintext_modulus);
            let u_bn_mod_q = u_bn.umod(q_bn);
            let t_bn_mod_q = t_bn.umod(q_bn);
            let t_times_u_bn_q = (t_bn_mod_q * u_bn_mod_q).umod(q_bn);

            let m = ModU128::new(self.configs.plaintext_modulus);

            // Check if centering is needed
            let needs_centering = t_times_u_bn_q > q_half_bn;

            let computed_message = if needs_centering {
                // (t*u) mod Q >= Q/2: centered value is negative
                // -Q^{-1} * (Q - (t*u) mod Q) mod t
                let centered_positive = q_bn - t_times_u_bn_q;
                let centered_positive_mod_t = centered_positive.umod(t_bn);
                let centered_field = to_field(centered_positive_mod_t);
                m.mul_mod(self.configs.q_inverse_mod_t, centered_field)
            } else {
                // (t*u) mod Q < Q/2: centered value is positive
                // t - (-Q^{-1} * (t*u) mod Q mod t)
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
}

impl<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32, let BIT_NOISE: u32> DecryptedSharesAggregationModular<MAX_MSG_NON_ZERO_COEFFS, L, T, BIT_NOISE> {
    pub fn new(
        configs: Configs<L>,
        decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
        party_ids: [Field; T + 1],
        message: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
        crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
    ) -> Self {
        DecryptedSharesAggregationModular {
            configs,
            decryption_shares,
            party_ids,
            message,
            u_global,
            crt_quotients,
        }
    }

    /// Executes the decryption share aggregation circuit.
    ///
    /// Runs all four verification steps in order: Lagrange coefficient computation,
    /// interpolation, CRT reconstruction, and decoding.
    pub fn execute(self) {
        // Step 1: Compute Lagrange coefficients in-circuit
        let lagrange_coeffs = compute_all_lagrange_coeffs::<T, L>(self.configs.qis, self.party_ids);

        // Step 2: Compute u^{(l)} for each CRT basis via Lagrange interpolation
        let u_crts = compute_crt_components::<MAX_MSG_NON_ZERO_COEFFS, L, T>(
            self.configs.qis,
            self.decryption_shares,
            lagrange_coeffs,
        );

        // Step 3: Verify CRT reconstruction: u^{(l)} + r^{(l)} * q_l = u_global
        verify_crt_reconstruction::<MAX_MSG_NON_ZERO_COEFFS, L>(
            self.configs.qis,
            self.u_global,
            self.crt_quotients,
            u_crts,
        );
        // Step 4: Verify decoding
        self.verify_decoding();
    }

    /// Verifies the BFV decoding step using ModU128 arithmetic.
    ///
    /// For each coefficient, computes message = -Q^{-1} * (t * u_global)_Q mod t
    /// using u128 modular arithmetic. Only valid when Q fits within 128 bits.
    ///
    /// Centered representation: if (t * u_global) mod Q >= Q/2, the value is treated
    /// as negative, flipping the sign of the -Q^{-1} multiplication. Only non-zero
    /// message coefficients are checked, zero coefficients are unconstrained.
    fn verify_decoding(self) {
        let t: Field = self.configs.plaintext_modulus;
        // Compute Q as product of all CRT moduli
        let mut q_modulus = 1;
        for l in 0..L {
            q_modulus *= self.configs.qis[l];
        }

        // For centered arithmetic
        let q_half = q_modulus as u128 / 2;

        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            // Compute (t * u_global) mod Q using ModU128
            let q_mod = ModU128::new(q_modulus);
            let t_mod = ModU128::new(t);

            // Compute (t * u_global) mod Q using modular arithmetic functions
            let t_times_u_mod_q = q_mod.mul_mod(t, self.u_global.coefficients[coeff_idx]);
            let needs_centering = (t_times_u_mod_q as u128) > q_half;

            let computed_message = if needs_centering {
                // (t*u) mod Q >= Q/2: centered value is negative
                // -Q^{-1} * (Q - (t*u) mod Q) mod t
                let centered_positive = q_modulus - t_times_u_mod_q;
                let centered_positive_mod_t = t_mod.reduce_mod(centered_positive);

                t_mod.mul_mod(self.configs.q_inverse_mod_t, centered_positive_mod_t)
            } else {
                // (t*u) mod Q < Q/2: centered value is positive
                // t - (-Q^{-1} * (t*u) mod Q mod t)
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

/// Computes all Lagrange basis coefficients L_i(0) for each CRT basis.
///
/// For each CRT basis l and each party i, computes:
///   L_i(0) = PROD_{j!=i} (-x_j) / (x_i - x_j)  mod q_l
///
/// IMPORTANT: party_ids must be in strictly increasing order. The sign of each
/// Lagrange coefficient depends on this ordering, out-of-order IDs will silently
/// produce incorrect signs and produce wrong decryption results.
pub fn compute_all_lagrange_coeffs<let T: u32, let L: u32>(
    qis: [Field; L],
    party_ids: [Field; T + 1],
) -> [[Field; T + 1]; L] {
    let mut lagrange_coeffs = [[0 as Field; T + 1]; L];

    // Cache pairwise differences: diffs[i][j] = |x_j - x_i| for i < j
    let mut diffs = [[0 as Field; T + 1]; T + 1];
    for i in 0..(T + 1) {
        for j in (i + 1)..(T + 1) {
            let diff = party_ids[j] - party_ids[i];
            diffs[i][j] = diff;
            diffs[j][i] = diff;
        }
    }

    // Numerator sign: PROD_{j=0..T} (-x_j) contributes (-1)^{T+1} overall;
    // negative when T is odd
    let numerator_sign_negative = (T % 2) == 1;

    for basis_idx in 0..L {
        let q_l = qis[basis_idx];
        let m = ModU128::new(q_l);

        // Compute PROD(j=0..T) x_j mod q_l once; divide by x_i per party
        let mut product_x = 1 as Field;
        for j in 0..(T + 1) {
            product_x = m.mul_mod(product_x, party_ids[j]);
        }

        for party_idx in 0..(T + 1) {
            // |numerator| = PROD_{j!=i} x_j = product_x / x_i
            let numerator_abs = m.div_mod(product_x, party_ids[party_idx]);

            // |denominator| = PROD_{j!=i} |x_i - x_j|
            let mut denominator_abs = 1 as Field;
            for j in 0..(T + 1) {
                if j != party_idx {
                    denominator_abs = m.mul_mod(denominator_abs, diffs[party_idx][j]);
                }
            }

            // Denominator sign: negative when the number of j > party_idx is odd
            let num_greater = T - party_idx;
            let denom_sign_negative = (num_greater % 2) == 1;

            let result_abs = m.div_mod(numerator_abs, denominator_abs);

            // XOR signs: negate if exactly one of numerator/denominator is negative
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

/// Computes u^{(l)} for each CRT basis via Lagrange interpolation.
///
/// For each basis l and each coefficient position k:
///   u^{(l)}[k] = SUM_{i=0..T} d_i^{(l)}[k] * L_i(0)  mod q_l
///
/// This reconstructs the threshold decryption value at zero for each CRT basis
/// independently, coefficient by coefficient.
pub fn compute_crt_components<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32, let T: u32>(
    qis: [Field; L],
    decryption_shares: [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1],
    lagrange_coeffs: [[Field; T + 1]; L],
) -> [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L] {
    let mut u_crts: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L] =
        [Polynomial::new([0; MAX_MSG_NON_ZERO_COEFFS]); L];

    for basis_idx in 0..L {
        let q_l = qis[basis_idx];
        let m = ModU128::new(q_l);
        let mut u_coeffs = [0 as Field; MAX_MSG_NON_ZERO_COEFFS];

        // For each coefficient position
        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            let mut u_coeff = 0 as Field;

            // Sum all contributions: u = SUM(i=0..T) [d_i * L_i(0)] mod q_l
            for party_idx in 0..(T + 1) {
                let d_coeff = decryption_shares[party_idx][basis_idx].coefficients[coeff_idx];
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

/// Verifies CRT reconstruction coefficient-wise for all bases.
///
/// For each basis l and each coefficient k, asserts:
///   u^{(l)}[k] + crt_quotients[l][k] * q_l == u_global[k]
///
/// This proves that u_global is the unique integer lift of the CRT components,
/// without revealing the individual CRT values or the quotients.
pub fn verify_crt_reconstruction<let MAX_MSG_NON_ZERO_COEFFS: u32, let L: u32>(
    qis: [Field; L],
    u_global: Polynomial<MAX_MSG_NON_ZERO_COEFFS>,
    crt_quotients: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
    u_crts: [Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L],
) {
    for basis_idx in 0..L {
        let q_l = qis[basis_idx];

        // Compute r^{(l)} * q_l
        let r_times_q = crt_quotients[basis_idx].mul_scalar(q_l);

        // Compute u^{(l)} + r^{(l)} * q_l
        let reconstructed = u_crts[basis_idx].add(r_times_q);

        // Verify: u^{(l)} + r^{(l)} * q_l = u_global
        // Must hold for every coefficient
        for coeff_idx in 0..MAX_MSG_NON_ZERO_COEFFS {
            assert(
                reconstructed.coefficients[coeff_idx] == u_global.coefficients[coeff_idx],
                "CRT reconstruction verification failed",
            );
        }
    }
}

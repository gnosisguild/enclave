// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::{flatten, Polynomial};
use safe::SafeSponge;

/// Cryptographic parameters for Greco circuit.
/// Contains the core mathematical constants used in the encryption scheme.
pub struct CryptographicParams<let L: u32> {
    /// Plaintext modulus: q mod t
    pub q_mod_t: Field,
    /// CRT moduli for each basis: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Scaling factors for each basis: [k0_0, k0_1, ..., k0_{L-1}]
    pub k0is: [Field; L],
}

impl<let L: u32> CryptographicParams<L> {
    /// Creates new cryptographic parameters
    pub fn new(q_mod_t: Field, qis: [Field; L], k0is: [Field; L]) -> Self {
        CryptographicParams { q_mod_t, qis, k0is }
    }
}

/// Bound parameters for range checking.
/// Contains all the bounds used to validate polynomial coefficients.
pub struct BoundParams<let L: u32> {
    /// Bounds for public key polynomials for each CRT basis
    pub pk_bounds: [u64; L],
    /// Bound for error polynomials (e0, e1)
    pub e_bound: u64,
    /// Bound for secret polynomial u (ternary distribution)
    pub u_bound: u64,
    /// Lower bounds for r1 polynomials (modulus switching quotients)
    pub r1_low_bounds: [i64; L],
    /// Upper bounds for r1 polynomials (modulus switching quotients)
    pub r1_up_bounds: [u64; L],
    /// Bounds for r2 polynomials (cyclotomic reduction quotients)
    pub r2_bounds: [u64; L],
    /// Bounds for p1 polynomials (modulus switching quotients)
    pub p1_bounds: [u64; L],
    /// Bounds for p2 polynomials (cyclotomic reduction quotients)
    pub p2_bounds: [u64; L],
    /// Lower bound for k1 polynomial (scaled message)
    pub k1_low_bound: i64,
    /// Upper bound for k1 polynomial (scaled message)
    pub k1_up_bound: u64,
}

impl<let L: u32> BoundParams<L> {
    /// Creates new bound parameters
    pub fn new(
        pk_bounds: [u64; L],
        e_bound: u64,
        u_bound: u64,
        r1_low_bounds: [i64; L],
        r1_up_bounds: [u64; L],
        r2_bounds: [u64; L],
        p1_bounds: [u64; L],
        p2_bounds: [u64; L],
        k1_low_bound: i64,
        k1_up_bound: u64,
    ) -> Self {
        BoundParams {
            pk_bounds,
            e_bound,
            u_bound,
            r1_low_bounds,
            r1_up_bounds,
            r2_bounds,
            p1_bounds,
            p2_bounds,
            k1_low_bound,
            k1_up_bound,
        }
    }
}

/// Complete parameters for Greco circuit.
/// Combines cryptographic parameters, bounds, and circuit-specific parameters.
pub struct Params<let N: u32, let L: u32> {
    /// Cryptographic parameters (moduli, scaling factors)
    pub crypto: CryptographicParams<L>,
    /// Bound parameters for range checking
    pub bounds: BoundParams<L>,
}

impl<let N: u32, let L: u32> Params<N, L> {
    /// Creates new complete parameters
    pub fn new(
        q_mod_t: Field,
        pk_bounds: [u64; L],
        e_bound: u64,
        u_bound: u64,
        r1_low_bounds: [i64; L],
        r1_up_bounds: [u64; L],
        r2_bounds: [u64; L],
        p1_bounds: [u64; L],
        p2_bounds: [u64; L],
        k1_low_bound: i64,
        k1_up_bound: u64,
        qis: [Field; L],
        k0is: [Field; L],
    ) -> Self {
        let crypto = CryptographicParams::new(q_mod_t, qis, k0is);
        let bounds = BoundParams::new(
            pk_bounds,
            e_bound,
            u_bound,
            r1_low_bounds,
            r1_up_bounds,
            r2_bounds,
            p1_bounds,
            p2_bounds,
            k1_low_bound,
            k1_up_bound,
        );

        Params { crypto, bounds }
    }

    /// Convenience method to access cryptographic parameters
    pub fn crypto_params(self) -> CryptographicParams<L> {
        self.crypto
    }

    /// Convenience method to access bound parameters
    pub fn bound_params(self) -> BoundParams<L> {
        self.bounds
    }
}

/// Greco
/// Correct Encryption Circuit under BFV public key
///
/// This circuit implements a zero-knowledge proof for the correct formation of a ciphertext
/// resulting from BFV (Brakerski-Fan-Vercauteren) public key encryption. The circuit verifies
/// that the ciphertext components (ct0, ct1) are correctly computed from the public key
/// components (pk0, pk1) and the encryption randomness.
///
/// The circuit enforces the following core constraints:
/// 1. Range checks on all polynomial coefficients to ensure they are within expected bounds
/// 2. Correct encryption equations:
///    a. ct0i(gamma) = pk0i(gamma) * u(gamma) + e0(gamma) + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo(gamma)
///    b. ct1i(gamma) = pk1i(gamma) * u(gamma) + e1(gamma) + p1i(gamma) * qi + p2i(gamma) * cyclo(gamma)
///
/// DISCLAIMER
/// The circuit is a porting of the Halo2 circuit from Greco paper author @ PSE.
/// Halo2 implementation is available at https://github.com/privacy-scaling-explorations/greco
///
/// # Generic Parameters
/// * `N` - The degree of polynomials (ring dimension)
/// * `L` - The number of CRT (Chinese Remainder Theorem) bases
/// * `BIT_PK` - Bit-width bound per coefficient for public keys `pk0is`/`pk1is`
/// * `BIT_CT` - Bit-width bound per coefficient for cipher texts `ct0is`/`ct1is`
/// * `BIT_U` - Bit-width bound per coefficient for the ternary secret `u`
/// * `BIT_E` - Bit-width bound per coefficient for error polynomials `e0`/`e1`
/// * `BIT_K` - Bit-width bound per coefficient for the scaled message polynomial `k1`
/// * `BIT_R1` - Bit-width bound per coefficient for randomness polynomials `r1is`
/// * `BIT_R2` - Bit-width bound per coefficient for randomness polynomials `r2is`
/// * `BIT_P1` - Bit-width bound per coefficient for randomness polynomials `p1is`
/// * `BIT_P2` - Bit-width bound per coefficient for randomness polynomials `p2is`
///
/// # Circuit Inputs
/// * `pk0is`, `pk1is` - Public key polynomials
/// * `ct0is`, `ct1is` - Ciphertext polynomials
/// * `u` - Secret polynomial sampled from ternary distribution
/// * `e0`, `e1` - Error polynomials sampled from discrete Gaussian distribution
/// * `k1` - Scaled message polynomial
/// * `r1is`, `r2is` - Randomness polynomials for each i-th CRT basis.
/// * `p1is`, `p2is` - Randomness polynomials for each i-th CRT basis.
pub struct Greco<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> {
    /// Cryptographic parameters including bounds, moduli, and constants.
    params: Params<N, L>,
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e1: Polynomial<N>,
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_P1: u32, let BIT_P2: u32> Greco<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E, BIT_K, BIT_R1, BIT_R2, BIT_P1, BIT_P2> {
    /// Creates a new Greco instance.
    ///
    /// This constructor initializes all the polynomials and parameters needed for
    /// the zero-knowledge proof of correct ciphertext formation.
    ///
    /// # Arguments
    /// * `params` - Cryptographic parameters including bounds and moduli
    /// * `pk0is` - Public key polynomials pk0i = [ai*s + E] for each CRT basis
    /// * `pk1is` - Public key polynomials pk1i = -[ai] for each CRT basis
    /// * `ct0is` - First ciphertext component for each CRT basis
    /// * `ct1is` - Second ciphertext component for each CRT basis
    /// * `u` - Secret polynomial from ternary distribution
    /// * `e0` - Error polynomial sampled from discrete Gaussian distribution
    /// * `e1` - Error polynomial sampled from discrete Gaussian distribution
    /// * `k1` - Scaled message polynomial
    /// * `r1is` - Randomness polynomials for ct0 computation
    /// * `r2is` - Randomness polynomials for ct0 computation
    /// * `p1is` - Randomness polynomials for ct1 computation
    /// * `p2is` - Randomness polynomials for ct1 computation
    ///
    /// # Returns
    /// A new Greco instance ready for constraint checking
    pub fn new(
        params: Params<N, L>,
        pk0is: [Polynomial<N>; L],
        pk1is: [Polynomial<N>; L],
        ct0is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e0: Polynomial<N>,
        e1: Polynomial<N>,
        k1: Polynomial<N>,
        r1is: [Polynomial<2 * N - 1>; L],
        r2is: [Polynomial<N - 1>; L],
        p1is: [Polynomial<2 * N - 1>; L],
        p2is: [Polynomial<N - 1>; L],
    ) -> Greco<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E, BIT_K, BIT_R1, BIT_R2, BIT_P1, BIT_P2> {
        Greco { params, pk0is, pk1is, ct0is, ct1is, u, e0, e1, k1, r1is, r2is, p1is, p2is }
    }

    /// Flattens all polynomials coefficients into a single array for challenge generation.
    ///
    /// This function serializes all polynomial coefficients into a 1D array to enable
    /// the generation of random challenge values using the Fiat-Shamir transform.
    /// The coefficients are arranged in a specific order to ensure deterministic
    /// challenge generation.
    ///
    /// # Returns
    /// An array containing all polynomials coefficients in flattened form
    fn payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Flatten public key and ciphertext polynomials first (public inputs)
        inputs = flatten::<_, _, BIT_PK>(inputs, self.pk0is);
        inputs = flatten::<_, _, BIT_PK>(inputs, self.pk1is);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct0is);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct1is);

        // Flatten common polynomials (used across all CRT bases)
        inputs = flatten::<_, _, BIT_E>(inputs, [self.e0]);
        inputs = flatten::<_, _, BIT_E>(inputs, [self.e1]);
        inputs = flatten::<_, _, BIT_U>(inputs, [self.u]);
        inputs = flatten::<_, _, BIT_K>(inputs, [self.k1]);

        // Flatten randomness polynomials for each CRT basis
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1is);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2is);
        inputs = flatten::<_, _, BIT_P1>(inputs, self.p1is);
        inputs = flatten::<_, _, BIT_P2>(inputs, self.p2is);

        inputs
    }

    /// Verifies the correct encryption constraints for the Greco circuit.
    ///
    /// This function implements the core zero-knowledge proof by checking:
    /// 1. Binary constraint on k1 polynomial
    /// 2. Range constraints on all polynomials coefficients
    /// 3. Correct encryption equations
    ///
    /// The proof uses the Schwartz-Zippel lemma: if polynomial equations hold
    /// when evaluated at random points, then the polynomials are identical with
    /// high probability.
    ///
    /// # Encryption Equations
    /// For each CRT basis i:
    /// * ct0i(gamma) = pk0i(gamma) * u(gamma) + e0(gamma) + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo
    /// * ct1i(gamma) = pk1i(gamma) * u(gamma) + e1(gamma) + p1i(gamma) * qi + p2i(gamma) * cyclo
    ///
    /// Where:
    /// * cyclo(gamma) = gamma^N + 1 (cyclotomic polynomial)
    /// * qi, k0i are constants from the cryptographic parameters
    /// * r1i, r2i, p1i, p2i are randomness polynomials for each i-th CRT basis.
    pub fn verify_correct_ciphertext_encryption(self) -> Vec<Field> {
        // Step 1: Perform range checks on all polynomial coefficients
        self.check_range_bounds();

        // Step 2: Generate Fiat-Shamir challenges
        let gammas = self.generate_challenge();

        // Step 3: Verify encryption constraints using challenges
        self.check_encryption_constraints(gammas);

        gammas
    }

    /// Performs range checks on all polynomial coefficients
    ///
    /// Checks that all polynomial coefficients are within their expected bounds.
    /// This prevents attacks where coefficients are outside the valid range.
    fn check_range_bounds(self) {
        let bound_params = self.params.bound_params();

        // Common polynomials (used across all CRT bases)
        self.u.range_check_1bound::<BIT_U>(bound_params.u_bound);
        self.e0.range_check_1bound::<BIT_E>(bound_params.e_bound);
        self.e1.range_check_1bound::<BIT_E>(bound_params.e_bound);
        self.k1.range_check_2bounds::<BIT_K>(bound_params.k1_up_bound, bound_params.k1_low_bound);

        // CRT basis-specific polynomials
        for i in 0..L {
            // Public key polynomials must be within bounds for each CRT basis
            self.pk0is[i].range_check_1bound::<BIT_PK>(bound_params.pk_bounds[i]);
            self.pk1is[i].range_check_1bound::<BIT_PK>(bound_params.pk_bounds[i]);

            // Randomness polynomials for ct0 computation
            self.r1is[i].range_check_2bounds::<BIT_R1>(
                bound_params.r1_up_bounds[i],
                bound_params.r1_low_bounds[i],
            );
            self.r2is[i].range_check_1bound::<BIT_R2>(bound_params.r2_bounds[i]);

            // Randomness polynomials for ct1 computation
            self.p1is[i].range_check_1bound::<BIT_P1>(bound_params.p1_bounds[i]);
            self.p2is[i].range_check_1bound::<BIT_P2>(bound_params.p2_bounds[i]);
        }
    }

    /// Generates Fiat-Shamir challenge values using a cryptographic sponge
    ///
    /// The sponge absorbs all witness values and squeezes out deterministic random field elements
    /// that will be used to evaluate polynomials for the Schwartz-Zippel lemma.
    ///
    /// # Returns
    /// Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    fn generate_challenge(self) -> Vec<Field> {
        let inputs = self.payload();

        // Domain separator for Greco circuit - "Greco" in hex
        let domain_separator = [
            0x47, 0x72, 0x65, 0x63, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        // IO Pattern: ABSORB(input_size), SQUEEZE(2*L)
        let input_size = inputs.len();
        let io_pattern = [0x80000000 | input_size, 0x00000000 | (2 * L)];

        let mut sponge = SafeSponge::start(io_pattern, domain_separator);
        sponge.absorb(inputs);
        let gammas = sponge.squeeze();
        sponge.finish();
        gammas
    }

    /// Verifies encryption constraints using Fiat-Shamir challenges
    ///
    /// For each CRT basis i, proves that LHS(gamma) = RHS(gamma)
    /// This uses the Schwartz-Zippel lemma for polynomial identity testing.
    ///
    /// # Arguments
    /// * `gammas` - Vector of challenge values [gamma_0, gamma_1, ..., gamma_{2L-1}]
    fn check_encryption_constraints(self, gammas: Vec<Field>) {
        let crypto_params = self.params.crypto_params();

        for i in 0..L {
            // Verify first equation: ct0i = pk0i * u + e0 + k1 * k0i + r1i * qi + r2i * cyclo
            self.check_ct0_constraint(i, gammas.get(i), crypto_params);

            // Verify second equation: ct1i = pk1i * u + e1 + p1i * qi + p2i * cyclo
            self.check_ct1_constraint(i, gammas.get(i + L), crypto_params);
        }
    }

    /// Verifies the ct0 constraint for a specific CRT basis
    ///
    /// Checks that ct0i(gamma) = pk0i(gamma) * u(gamma) + e0(gamma) + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo(gamma)
    ///
    /// # Arguments
    /// * `i` - CRT basis index
    /// * `gamma` - Challenge value for this constraint
    /// * `crypto_params` - Cryptographic parameters
    fn check_ct0_constraint(self, i: u32, gamma: Field, crypto_params: CryptographicParams<L>) {
        // Cyclotomic polynomial evaluation: cyclo(x) = x^N + 1
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;

        // Evaluate all polynomials at gamma
        let u_at_gamma = self.u.eval(gamma);
        let e0_at_gamma = self.e0.eval(gamma);
        let k1_at_gamma = self.k1.eval(gamma);
        let pk0is_at_gamma = self.pk0is[i].eval(gamma);
        let r1i_at_gamma = self.r1is[i].eval(gamma);
        let r2i_at_gamma = self.r2is[i].eval(gamma);

        // Compute right-hand side step by step
        // Step 1: pk0_u = pk0i(gamma) * u(gamma) + e0(gamma)
        let pk0_u = (pk0is_at_gamma * u_at_gamma) + e0_at_gamma;

        // Step 2: rhs = pk0_u + k1(gamma) * k0i
        let rhs = pk0_u + (k1_at_gamma * crypto_params.k0is[i]);

        // Step 3: rhs = rhs + r1i(gamma) * qi
        let rhs = rhs + (r1i_at_gamma * crypto_params.qis[i]);

        // Step 4: rhs = rhs + r2i(gamma) * cyclo(gamma)
        let rhs = rhs + r2i_at_gamma * cyclo_at_gamma;

        // Left-hand side: ct0i(gamma)
        let lhs = self.ct0is[i].eval(gamma);

        // Enforce equality: LHS(gamma) = RHS(gamma)
        assert_eq(lhs, rhs);
    }

    /// Verifies the ct1 constraint for a specific CRT basis
    ///
    /// Checks that ct1i(gamma) = pk1i(gamma) * u(gamma) + e1(gamma) + p1i(gamma) * qi + p2i(gamma) * cyclo(gamma)
    ///
    /// # Arguments
    /// * `i` - CRT basis index
    /// * `gamma` - Challenge value for this constraint
    /// * `crypto_params` - Cryptographic parameters
    fn check_ct1_constraint(self, i: u32, gamma: Field, crypto_params: CryptographicParams<L>) {
        // Cyclotomic polynomial evaluation: cyclo(x) = x^N + 1
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;

        // Evaluate all polynomials at gamma
        let u_at_gamma = self.u.eval(gamma);
        let e1_at_gamma = self.e1.eval(gamma);
        let pk1is_at_gamma = self.pk1is[i].eval(gamma);
        let p1is_at_gamma = self.p1is[i].eval(gamma);
        let p2is_at_gamma = self.p2is[i].eval(gamma);

        // Compute right-hand side step by step
        // Step 1: pk1_u = pk1i(gamma) * u(gamma) + e1(gamma)
        let pk1_u = (pk1is_at_gamma * u_at_gamma) + e1_at_gamma;

        // Step 2: rhs = pk1_u + p2i(gamma) * cyclo(gamma)
        let rhs = pk1_u + p2is_at_gamma * cyclo_at_gamma;

        // Step 3: rhs = rhs + p1i(gamma) * qi
        let rhs = rhs + (p1is_at_gamma * crypto_params.qis[i]);

        // Left-hand side: ct1i(gamma)
        let lhs = self.ct1is[i].eval(gamma);

        // Enforce equality: LHS(gamma) = RHS(gamma)
        assert_eq(lhs, rhs);
    }
}

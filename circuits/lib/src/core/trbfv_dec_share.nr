use crate::math::commitments::{
    compute_aggregated_shares_commitment, compute_dec_share_challenge_commitment,
    prepare_aggregated_shares_from_values_commitment_payload,
};
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

/// Cryptographic parameters for TRBFV decryption share circuit.
pub struct Configs<let L: u32> {
    /// CRT moduli: [q_0, q_1, ..., q_{L-1}]
    pub qis: [Field; L],
    /// Bound for decryption share coefficients
    pub decryption_share_bound: Field,
    /// Bounds for r1 polynomials (modulus switching quotients) for each CRT basis
    pub r1_bounds: [Field; L],
    /// Bounds for r2 polynomials (cyclotomic reduction quotients) for each CRT basis
    pub r2_bounds: [Field; L],
}

impl<let L: u32> Configs<L> {
    pub fn new(
        qis: [Field; L],
        decryption_share_bound: Field,
        r1_bounds: [Field; L],
        r2_bounds: [Field; L],
    ) -> Self {
        Configs { qis, decryption_share_bound, r1_bounds, r2_bounds }
    }
}

/// TRBFV Decryption Share Correctness (Circuit 6).
///
/// Verifies:
/// 1. Commitment to s matches expected (from BFV decryption circuit)
/// 2. Commitment to e matches expected (from BFV decryption circuit)
/// 3. Correct computation: d = c_0 + c_1 * s + e + r_2 * (X^N + 1) + r_1 * q_i
pub struct DecryptionShare<let N: u32, let L: u32, let BIT_CT: u32, let BIT_S: u32, let BIT_E: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_D: u32> {
    /// Circuit parameters including bounds and cryptographic constants
    configs: Configs<L>,

    /// Expected commitment to aggregated shares s (from BFV decryption circuit)
    expected_s_commitment: Field,

    /// Expected commitment to aggregated noise e (from BFV decryption circuit)
    expected_e_commitment: Field,

    /// Ciphertext components (public inputs)
    /// c_0 components for each CRT basis (degree N-1 polynomials with N coefficients)
    c_0: [Polynomial<N>; L],
    /// c_1 components for each CRT basis (degree N-1 polynomials with N coefficients)
    c_1: [Polynomial<N>; L],

    /// Aggregated sum of shares s (secret witness)
    s: [Polynomial<N>; L],

    /// Aggregated sum of noise e (secret witness, direct input)
    /// e[basis] - sum of noise shares for each CRT basis (degree N-1 with N coefficients)
    e: [Polynomial<N>; L],

    /// Quotient polynomials for lifting to Z (secret witnesses)
    r_1: [Polynomial<2 * N - 1>; L],
    r_2: [Polynomial<N - 1>; L],

    /// Party's computed decryption share (public output)
    d: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_CT: u32, let BIT_S: u32, let BIT_E: u32, let BIT_R1: u32, let BIT_R2: u32, let BIT_D: u32> DecryptionShare<N, L, BIT_CT, BIT_S, BIT_E, BIT_R1, BIT_R2, BIT_D> {
    pub fn new(
        configs: Configs<L>,
        expected_s_commitment: Field,
        expected_e_commitment: Field,
        c_0: [Polynomial<N>; L],
        c_1: [Polynomial<N>; L],
        s: [Polynomial<N>; L],
        e: [Polynomial<N>; L],
        r_1: [Polynomial<2 * N - 1>; L],
        r_2: [Polynomial<N - 1>; L],
        d: [Polynomial<N>; L],
    ) -> Self {
        DecryptionShare {
            configs,
            expected_s_commitment,
            expected_e_commitment,
            c_0,
            c_1,
            s,
            e,
            r_1,
            r_2,
            d,
        }
    }

    /// Verifies that s hashes to expected_s_commitment
    fn verify_s_commitment(self) {
        let payload = prepare_aggregated_shares_from_values_commitment_payload::<N, L>(self.s);
        let computed_commitment = compute_aggregated_shares_commitment(payload);
        assert(computed_commitment == self.expected_s_commitment, "S commitment mismatch");
    }

    /// Verifies that e hashes to expected_e_commitment
    fn verify_e_commitment(self) {
        let payload = prepare_aggregated_shares_from_values_commitment_payload::<N, L>(self.e);
        let computed_commitment = compute_aggregated_shares_commitment(payload);
        assert(computed_commitment == self.expected_e_commitment, "E commitment mismatch");
    }

    /// Flattens all witness data into a single array for Fiat-Shamir challenge generation.
    ///
    /// This function serializes all polynomial coefficients (both public inputs and
    /// secret witnesses) into a 1D array in a deterministic order. The flattened data
    /// is used to generate the Fiat-Shamir challenge via the SAFE sponge API.
    ///
    /// The order of serialization is:
    /// 1. Ciphertext components `c_0` for each CRT basis
    /// 2. Ciphertext components `c_1` for each CRT basis
    /// 3. Aggregated secret shares `s` for each CRT basis
    /// 4. Aggregated noise shares `e` for each CRT basis
    /// 5. Quotient polynomials `r_1` for each CRT basis
    /// 6. Quotient polynomials `r_2` for each CRT basis
    /// 7. Decryption shares `d` for each CRT basis
    ///
    /// # Returns
    /// A vector containing all polynomial coefficients in flattened form, ready for
    /// hashing to generate the Fiat-Shamir challenge.
    fn payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();

        // Use commitments instead of full polynomials (saves constraints)
        inputs.push(self.expected_s_commitment);
        inputs.push(self.expected_e_commitment);

        // Flatten ciphertext components (public inputs)
        inputs = flatten::<_, _, BIT_CT>(inputs, self.c_0);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.c_1);

        // Flatten quotient polynomials (secret witnesses)
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r_1);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r_2);

        // Flatten decryption shares (public outputs)
        inputs = flatten::<_, _, BIT_D>(inputs, self.d);

        inputs
    }

    /// Main verification function
    pub fn verify(self) {
        // Step 1: Verify s commitment matches expected
        self.verify_s_commitment();

        // Step 2: Verify e commitment matches expected
        self.verify_e_commitment();

        // Step 3: Perform range checks on all secret witness values
        self.check_range_bounds();

        // Step 4: Generate Fiat-Shamir challenge from the transcript
        let gamma = self.generate_challenge();

        // Step 5: Verify decryption share computation for each CRT basis
        for i in 0..L {
            self.verify_decryption_share_computation(i, gamma);
        }
    }

    /// Performs range checks on all secret witness values.
    ///
    /// This function constrains all secret witnesses to be within their expected bounds
    /// as specified in the `configs`. This is critical for security because it prevents
    /// attacks where malicious provers provide out-of-range values that could break the
    /// security properties of the threshold BFV scheme.
    ///
    /// The function checks:
    /// - Aggregated secret shares `s` are within bounds for each CRT basis
    /// - Aggregated noise shares `e` are within bounds for each CRT basis
    /// - Quotient polynomials `r_1` and `r_2` are within bounds for each CRT basis
    ///
    /// # Panics
    /// This function will cause the circuit to fail if any coefficient is outside
    /// its expected bounds. The bounds are defined per polynomial type in the `configs`.
    fn check_range_bounds(self) {
        // Check aggregated sums are within bounds
        for basis_idx in 0..L {
            self.s[basis_idx].range_check_2bounds::<BIT_S>(
                self.configs.r2_bounds[basis_idx],
                self.configs.r2_bounds[basis_idx],
            );
            self.e[basis_idx].range_check_2bounds::<BIT_E>(
                self.configs.r2_bounds[basis_idx],
                self.configs.r2_bounds[basis_idx],
            );
        }

        // Check quotient polynomials are within bounds
        for basis_idx in 0..L {
            // r_1 quotients can be negative (modulus quotients)
            self.r_1[basis_idx].range_check_2bounds::<BIT_R1>(
                self.configs.r1_bounds[basis_idx],
                self.configs.r1_bounds[basis_idx],
            );
            // r_2 quotients (cyclotomic quotients)
            self.r_2[basis_idx].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[basis_idx],
                self.configs.r2_bounds[basis_idx],
            );
        }
    }

    /// Generates Fiat-Shamir challenge value using the SAFE cryptographic sponge.
    ///
    /// This function implements the Fiat-Shamir transform for the decryption share circuit:
    /// 1. Flattens all witness data (ciphertexts, secrets, quotients, shares) into a single array
    /// 2. Absorbs the flattened data into the SAFE sponge
    /// 3. Squeezes a single challenge value
    ///
    /// The challenge is used to evaluate polynomials for the Schwartz-Zippel lemma,
    /// which allows verification of polynomial equations by checking them at a random
    /// point rather than checking all coefficients.
    ///
    /// # Returns
    /// A single challenge value `gamma` used as the evaluation point for verifying
    /// the decryption share computation formula for all CRT bases.
    fn generate_challenge(self) -> Field {
        let inputs = self.payload();

        compute_dec_share_challenge_commitment(inputs)
    }

    /// Verifies the lifted decryption share computation formula for a specific CRT basis using the Schwartz-Zippel lemma.
    ///
    /// This function verifies that the decryption share for basis `i` satisfies:
    /// `d_i(gamma) = c_0i(gamma) + c_1i(gamma) * s(gamma) + e_i(gamma) + r_2_i(gamma) * cyclo(gamma) + r_1_i(gamma) * q_i`
    ///
    /// Where:
    /// - `c_0i`, `c_1i` are ciphertext components for basis i
    /// - `s` is the aggregated secret key shares
    /// - `e_i` is the aggregated noise shares
    /// - `r_1_i`, `r_2_i` are quotient witnesses
    /// - `cyclo(gamma) = gamma^N + 1` is the cyclotomic polynomial evaluated at gamma
    /// - `q_i` is the CRT modulus for basis i
    ///
    /// The Schwartz-Zippel lemma ensures that if this equation holds at a random point
    /// `gamma`, then the polynomials are identical with high probability.
    ///
    /// # Arguments
    /// * `basis_idx` - The index of the CRT basis to verify (0 <= basis_idx < L)
    /// * `gamma` - The Fiat-Shamir challenge value used as the evaluation point
    ///
    /// # Panics
    /// The circuit will fail if the decryption share computation formula doesn't hold for the specified basis.
    fn verify_decryption_share_computation(self, basis_idx: u32, gamma: Field) {
        // Evaluate ciphertext components at gamma
        let c_0_at_gamma = self.c_0[basis_idx].eval(gamma);
        let c_1_at_gamma = self.c_1[basis_idx].eval(gamma);

        // Evaluate aggregated sums at gamma
        let s_at_gamma = self.s[basis_idx].eval(gamma);
        let e_at_gamma = self.e[basis_idx].eval(gamma);

        // Evaluate quotient polynomials at gamma
        let r_1_at_gamma = self.r_1[basis_idx].eval(gamma);
        let r_2_at_gamma = self.r_2[basis_idx].eval(gamma);

        // Evaluate cyclotomic polynomial X^N + 1 at gamma
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;

        // Compute expected decryption share using the lifted formula:
        // d_i = c_0i + c_1i * s + e_i + r_2_i * (X^N + 1) + r_1_i * q_i
        let expected_decryption_share = c_0_at_gamma
            + c_1_at_gamma * s_at_gamma
            + e_at_gamma
            + r_2_at_gamma * cyclo_at_gamma
            + r_1_at_gamma * self.configs.qis[basis_idx];

        // Evaluate the party's claimed decryption share at gamma
        let computed_decryption_share = self.d[basis_idx].eval(gamma);

        // Enforce equality: computed decryption share must match expected value
        assert(
            computed_decryption_share == expected_decryption_share,
            "Decryption share computation failed",
        );
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// User data encryption circuit for ct0, which checks the correctness of ct0 encryption
// and generates commitment for u, which will be used in ct1 to check consistency of u
// across ct0 and ct1.

use crate::math::commitments::compute_commitment;
use crate::math::commitments::compute_user_data_encryption_ct0_challenge;
use crate::math::commitments::DS_PK_AGGREGATION;
use crate::math::commitments::multiple_polynomial_payload;
use crate::math::commitments::single_polynomial_payload;
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

pub struct Configs<let N: u32, let L: u32> {
    pub qis: [Field; L],
    pub k0is: [Field; L],
    pub e0_bound: Field,
    pub u_bound: Field,
    pub r1_low_bounds: [Field; L],
    pub r1_up_bounds: [Field; L],
    pub r2_bounds: [Field; L],
    pub k1_low_bound: Field,
    pub k1_up_bound: Field,
}

impl<let N: u32, let L: u32> Configs<N, L> {
    pub fn new(
        qis: [Field; L],
        k0is: [Field; L],
        e0_bound: Field,
        u_bound: Field,
        r1_low_bounds: [Field; L],
        r1_up_bounds: [Field; L],
        r2_bounds: [Field; L],
        k1_low_bound: Field,
        k1_up_bound: Field,
    ) -> Self {
        Configs {
            qis,
            k0is,
            e0_bound,
            u_bound,
            r1_low_bounds,
            r1_up_bounds,
            r2_bounds,
            k1_low_bound,
            k1_up_bound,
        }
    }
}

pub struct UserDataEncryptionCt0<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32> {
    configs: Configs<N, L>,
    /// Public input from PkAggregation
    pk0is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e0is: [Polynomial<N>; L],
    e0_quotients: [Polynomial<N>; L],
    k1: Polynomial<N>,
    r1is: [Polynomial<(2 * N) - 1>; L],
    r2is: [Polynomial<N - 1>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E0: u32, let BIT_K: u32, let BIT_R1: u32, let BIT_R2: u32> UserDataEncryptionCt0<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E0, BIT_K, BIT_R1, BIT_R2> {

    pub fn new(
        configs: Configs<N, L>,
        pk0is: [Polynomial<N>; L],
        ct0is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e0: Polynomial<N>,
        e0is: [Polynomial<N>; L],
        e0_quotients: [Polynomial<N>; L],
        k1: Polynomial<N>,
        r1is: [Polynomial<(2 * N) - 1>; L],
        r2is: [Polynomial<N - 1>; L],
    ) -> Self {
        UserDataEncryptionCt0 { configs, pk0is, ct0is, u, e0, e0is, e0_quotients, k1, r1is, r2is }
    }

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

    fn check_range_bounds(self) {
        self.u.range_check_2bounds::<BIT_U>(self.configs.u_bound, self.configs.u_bound);
        self.e0.range_check_2bounds::<BIT_E0>(self.configs.e0_bound, self.configs.e0_bound);
        self.k1.range_check_2bounds::<BIT_K>(self.configs.k1_up_bound, self.configs.k1_low_bound);

        for i in 0..L {
            self.r1is[i].range_check_2bounds::<BIT_R1>(
                self.configs.r1_up_bounds[i],
                self.configs.r1_low_bounds[i],
            );
            self.r2is[i].range_check_2bounds::<BIT_R2>(
                self.configs.r2_bounds[i],
                self.configs.r2_bounds[i],
            );
        }
    }

    fn generate_commitments(self) -> (Field, Field, Field, Field) {
        let pk0_payload = multiple_polynomial_payload::<N, L, BIT_PK>(Vec::new(), self.pk0is);
        let pk0_commitment = compute_commitment(pk0_payload, DS_PK_AGGREGATION);

        let ct0_payload = multiple_polynomial_payload::<N, L, BIT_CT>(Vec::new(), self.ct0is);
        let ct0_commitment = compute_commitment(ct0_payload, DS_PK_AGGREGATION);

        let k1_payload = single_polynomial_payload::<N, BIT_K>(Vec::new(), self.k1);
        let k1_commitment = compute_commitment(k1_payload, DS_PK_AGGREGATION);

        let u_payload = single_polynomial_payload::<N, BIT_U>(Vec::new(), self.u);
        let u_commitment = compute_commitment(u_payload, DS_PK_AGGREGATION);

        (pk0_commitment, ct0_commitment, k1_commitment, u_commitment)
    }

    fn generate_challenge(
        self,
        pk0_commitment: Field,
        ct0_commitment: Field,
        k1_commitment: Field,
        u_commitment: Field,
    ) -> Vec<Field> {
        let mut payload = Vec::new();

        payload.push(pk0_commitment);
        payload.push(ct0_commitment);
        payload.push(k1_commitment);
        payload.push(u_commitment);

        payload = flatten::<_, _, BIT_E0>(payload, [self.e0]);
        payload = flatten::<_, _, BIT_R1>(payload, self.r1is);
        payload = flatten::<_, _, BIT_R2>(payload, self.r2is);

        let gammas = compute_user_data_encryption_ct0_challenge::<L>(payload);

        gammas
    }

    fn verify_evaluations(self, gammas: Vec<Field>) -> bool {
        let gamma = gammas.get(0);
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;
        let u_at_gamma = self.u.eval(gamma);
        let k1_at_gamma = self.k1.eval(gamma);

        let mut sum = (0, 0);
        for i in 0..L {
            let pk0is_at_gamma = self.pk0is[i].eval(gamma);
            let r1i_at_gamma = self.r1is[i].eval(gamma);
            let r2i_at_gamma = self.r2is[i].eval(gamma);
            let e0is_at_gamma = self.e0is[i].eval(gamma);

            let mut ct0_rhs = pk0is_at_gamma * u_at_gamma + e0is_at_gamma;
            ct0_rhs += k1_at_gamma * self.configs.k0is[i];
            ct0_rhs += r1i_at_gamma * self.configs.qis[i];
            ct0_rhs += r2i_at_gamma * cyclo_at_gamma;

            let ct0_lhs = self.ct0is[i].eval(gamma);
            let gamma_i = if i == 0 { 1 } else { gammas.get(i) };

            sum = (sum.0 + ct0_lhs * gamma_i, sum.1 + ct0_rhs * gamma_i);
        }

        sum.0 == sum.1
    }

    /// Returns pk0_commitment, ct0_commitment, k1_commitment and u_commitment
    pub fn execute(self) -> (Field, Field, Field, Field) {
        self.check_range_bounds();
        self.check_e0_crt_consistency();

        let (pk0_commitment, ct0_commitment, k1_commitment, u_commitment) =
            self.generate_commitments();

        let gammas =
            self.generate_challenge(pk0_commitment, ct0_commitment, k1_commitment, u_commitment);

        assert(self.verify_evaluations(gammas), "ct0 encryption check failed");

        (pk0_commitment, ct0_commitment, k1_commitment, u_commitment)
    }
}

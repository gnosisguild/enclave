// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// User data encryption circuit for ct0, which checks the correctness of ct0 encryption
// and generates commitment for u, which will be used in ct1 to check consistency of u
// across ct0 and ct1.

use crate::math::commitments::{
    compute_user_data_encryption_ct0_challenge, compute_user_data_encryption_ct0_pk_commitment,
    compute_user_data_encryption_u_commitment,
};
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
    commit_pk0: Field,
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
        commit_pk0: Field,
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
        UserDataEncryptionCt0 {
            configs,
            commit_pk0,
            pk0is,
            ct0is,
            u,
            e0,
            e0is,
            e0_quotients,
            k1,
            r1is,
            r2is,
        }
    }

    fn verify_pk0_commitment(self) {
        let computed = compute_user_data_encryption_ct0_pk_commitment::<N, L, BIT_PK>(self.pk0is);
        assert(computed == self.commit_pk0, "pk0 commitment mismatch");
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

    fn gammas_payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();
        inputs.push(self.commit_pk0);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct0is);
        inputs = flatten::<_, _, BIT_E0>(inputs, [self.e0]);
        inputs = flatten::<_, _, BIT_U>(inputs, [self.u]);
        inputs = flatten::<_, _, BIT_K>(inputs, [self.k1]);
        inputs = flatten::<_, _, BIT_R1>(inputs, self.r1is);
        inputs = flatten::<_, _, BIT_R2>(inputs, self.r2is);
        inputs
    }

    fn generate_challenge(self) -> Vec<Field> {
        compute_user_data_encryption_ct0_challenge::<L>(self.gammas_payload())
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

    /// Returns commit_u so Circuit B can verify it uses the same u
    pub fn execute(self) -> Field {
        self.check_range_bounds();
        self.check_e0_crt_consistency();
        self.verify_pk0_commitment();
        let commit_u = compute_user_data_encryption_u_commitment::<N, BIT_U>(self.u);
        let gammas = self.generate_challenge();
        assert(self.verify_evaluations(gammas), "ct0 encryption check failed");
        commit_u
    }
}

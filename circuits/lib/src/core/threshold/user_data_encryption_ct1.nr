//user data encryption for ct1- the second part of the ciphertext
use crate::math::commitments::{
    compute_user_data_encryption_ct1_challenge, compute_user_data_encryption_ct1_pk_commitment,
    compute_user_data_encryption_u_commitment,
};
use crate::math::helpers::flatten;
use crate::math::polynomial::Polynomial;

pub struct Configs<let N: u32, let L: u32> {
    pub qis: [Field; L],
    pub e1_bound: Field,
    pub u_bound: Field,
    pub p1_bounds: [Field; L],
    pub p2_bounds: [Field; L],
}
impl<let N: u32, let L: u32> Configs<N, L> {
    pub fn new(
        qis: [Field; L],
        e1_bound: Field,
        u_bound: Field,
        p1_bounds: [Field; L],
        p2_bounds: [Field; L],
    ) -> Self {
        Configs { qis, e1_bound, u_bound, p1_bounds, p2_bounds }
    }
}

pub struct UserDataEncryptionCt1<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E1: u32, let BIT_P1: u32, let BIT_P2: u32> {
    configs: Configs<N, L>,
    /// Public input from PkAggregation
    commit_pk1: Field,
    /// Public input from Circuit A
    commit_u: Field,
    pk1is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    /// Re-witnessed privately, checked against commit_u from Circuit A
    u: Polynomial<N>,
    e1: Polynomial<N>,
    p1is: [Polynomial<(2 * N) - 1>; L],
    p2is: [Polynomial<N - 1>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32, let BIT_CT: u32, let BIT_U: u32, let BIT_E1: u32, let BIT_P1: u32, let BIT_P2: u32> UserDataEncryptionCt1<N, L, BIT_PK, BIT_CT, BIT_U, BIT_E1, BIT_P1, BIT_P2> {

    pub fn new(
        configs: Configs<N, L>,
        commit_pk1: Field,
        commit_u: Field,
        pk1is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e1: Polynomial<N>,
        p1is: [Polynomial<2 * N - 1>; L],
        p2is: [Polynomial<N - 1>; L],
    ) -> Self {
        UserDataEncryptionCt1 { configs, commit_pk1, commit_u, pk1is, ct1is, u, e1, p1is, p2is }
    }

    fn verify_pk1_commitment(self) {
        let computed = compute_user_data_encryption_ct1_pk_commitment::<N, L, BIT_PK>(self.pk1is);
        assert(computed == self.commit_pk1, "pk1 commitment mismatch");
    }

    fn verify_u_commitment(self) {
        let computed = compute_user_data_encryption_u_commitment::<N, BIT_U>(self.u);
        assert(computed == self.commit_u, "u commitment mismatch");
    }

    fn check_range_bounds(self) {
        self.e1.range_check_2bounds::<BIT_E1>(self.configs.e1_bound, self.configs.e1_bound);

        for i in 0..L {
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

    fn gammas_payload(self) -> Vec<Field> {
        let mut inputs = Vec::new();
        inputs.push(self.commit_pk1);
        inputs = flatten::<_, _, BIT_CT>(inputs, self.ct1is);
        inputs = flatten::<_, _, BIT_E1>(inputs, [self.e1]);
        inputs = flatten::<_, _, BIT_U>(inputs, [self.u]);
        inputs = flatten::<_, _, BIT_P1>(inputs, self.p1is);
        inputs = flatten::<_, _, BIT_P2>(inputs, self.p2is);
        inputs
    }

    fn generate_challenge(self) -> Vec<Field> {
        compute_user_data_encryption_ct1_challenge::<L>(self.gammas_payload())
    }

    fn verify_evaluations(self, gammas: Vec<Field>) -> bool {
        let gamma = gammas.get(0);
        let cyclo_at_gamma = gamma.pow_32(N as Field) + 1;
        let u_at_gamma = self.u.eval(gamma);
        let e1_at_gamma = self.e1.eval(gamma);

        let mut sum = (0, 0);
        for i in 0..L {
            let pk1is_at_gamma = self.pk1is[i].eval(gamma);
            let p1is_at_gamma = self.p1is[i].eval(gamma);
            let p2is_at_gamma = self.p2is[i].eval(gamma);

            let mut ct1_rhs = pk1is_at_gamma * u_at_gamma + e1_at_gamma;
            ct1_rhs += p1is_at_gamma * self.configs.qis[i];
            ct1_rhs += p2is_at_gamma * cyclo_at_gamma;

            let ct1_lhs = self.ct1is[i].eval(gamma);
            let gamma_i = gammas.get(i + L);

            sum = (sum.0 + ct1_lhs * gamma_i, sum.1 + ct1_rhs * gamma_i);
        }

        sum.0 == sum.1
    }

    pub fn execute(self) {
        self.check_range_bounds();
        self.verify_pk1_commitment();
        self.verify_u_commitment();
        let gammas = self.generate_challenge();
        assert(self.verify_evaluations(gammas), "ct1 encryption check failed");
    }
}

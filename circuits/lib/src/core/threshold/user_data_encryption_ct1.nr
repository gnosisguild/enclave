//user data encryption for ct1- the second part of the ciphertext
use crate::math::commitments::compute_commitment;
use crate::math::commitments::compute_user_data_encryption_ct1_challenge;
use crate::math::commitments::DS_PK_AGGREGATION;
use crate::math::commitments::multiple_polynomial_payload;
use crate::math::commitments::single_polynomial_payload;
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
        pk1is: [Polynomial<N>; L],
        ct1is: [Polynomial<N>; L],
        u: Polynomial<N>,
        e1: Polynomial<N>,
        p1is: [Polynomial<2 * N - 1>; L],
        p2is: [Polynomial<N - 1>; L],
    ) -> Self {
        UserDataEncryptionCt1 { configs, pk1is, ct1is, u, e1, p1is, p2is }
    }

    fn generate_commitments(self) -> (Field, Field) {
        let pk1_payload = multiple_polynomial_payload::<N, L, BIT_PK>(Vec::new(), self.pk1is);
        let pk1_commitment = compute_commitment(pk1_payload, DS_PK_AGGREGATION);

        let u_payload = single_polynomial_payload::<N, BIT_U>(Vec::new(), self.u);
        let u_commitment = compute_commitment(u_payload, DS_PK_AGGREGATION);

        (pk1_commitment, u_commitment)
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

    fn generate_challenge(self, pk1_commitment: Field, u_commitment: Field) -> Vec<Field> {
        let mut payload = Vec::new();

        payload.push(pk1_commitment);
        payload.push(u_commitment);

        payload = flatten::<_, _, BIT_CT>(payload, self.ct1is);
        payload = flatten::<_, _, BIT_E1>(payload, [self.e1]);
        payload = flatten::<_, _, BIT_P1>(payload, self.p1is);
        payload = flatten::<_, _, BIT_P2>(payload, self.p2is);

        compute_user_data_encryption_ct1_challenge::<L>(payload)
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

    /// Returns pk1_commitment and u_commitment
    pub fn execute(self) -> (Field, Field) {
        self.check_range_bounds();

        let (pk1_commitment, u_commitment) = self.generate_commitments();

        let gammas = self.generate_challenge(pk1_commitment, u_commitment);

        assert(self.verify_evaluations(gammas), "ct1 encryption check failed");

        (pk1_commitment, u_commitment)
    }
}

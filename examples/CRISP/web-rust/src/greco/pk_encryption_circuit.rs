use crate::greco::{
    constants::pk_enc_constants_2048_1x52_1032193::{
        E_BOUND, K0IS, K1_LOW_BOUND, K1_UP_BOUND, N, P1_BOUNDS, P2_BOUNDS, PK_BOUND, QIS,
        R1_LOW_BOUNDS, R1_UP_BOUNDS, R2_BOUNDS, U_BOUND,
    },
    greco::{to_string_1d_vec, to_string_2d_vec, InputValidationVectors},
    poly_circuit::{Poly, PolyAssigned},
};
use axiom_eth::halo2_base::{
    gates::{circuit::CircuitBuilderStage, GateInstructions, RangeChip, RangeInstructions},
    halo2_proofs::{
        halo2curves::bn256::{Bn256, Fr},
        plonk::{create_proof, keygen_pk, keygen_vk},
        poly::commitment::Params,
        poly::kzg::commitment::ParamsKZG,
        poly::kzg::multiopen::ProverSHPLONK,
        transcript::TranscriptWriterBuffer,
    },
    utils::ScalarField,
    QuantumCell::Constant,
};
use axiom_eth::rlc::{
    chip::RlcChip,
    circuit::{builder::RlcCircuitBuilder, instructions::RlcCircuitInstructions},
    utils::executor::RlcExecutor,
};

use halo2_solidity_verifier::{fr_to_u256, Keccak256Transcript};
use rand::{rngs::OsRng, rngs::StdRng, RngCore, SeedableRng};

use serde::Deserialize;

/// `BfvPkEncryptionCircuit` is a circuit that checks the correct formation of a ciphertext resulting from BFV public key encryption
/// All the polynomials coefficients and scalars are normalized to be in the range `[0, p)` where p is the modulus of the prime field of the circuit
///
/// pk_q1 = ( pk0i , pk1i )=( [ai*s + E] , -ai )
/// # Parameters:
/// * `pk0i`: publicly polynomial created by secret polynomial ([ai*s + E] )
/// * `pk1i`: publicly polynomial created by polynomial (-[ai])
/// * `u`: secret polynomial, sampled from ternary distribution.
/// * `e0`: error polynomial, sampled from discrete Gaussian distribution.
/// * `e1`: error polynomial, sampled from discrete Gaussian distribution.
/// * `k1`: scaled message polynomial.
/// * `r2is`: list of r2i polynomials for each i-th CRT basis .
/// * `r1is`: list of r1i polynomials for each CRT i-th CRT basis.
/// * `p2is`: list of p2i polynomials for each i-th CRT basis.
/// * `p1is`: list of p1i polynomials for each i-th CRT basis.
/// * `ct0is`: list of ct0i (first component of the ciphertext cti) polynomials for each CRT i-th CRT basis.
/// * `ct1is`: list of ct1i (second component of the ciphertext cti) polynomials for each CRT i-th CRT basis.

#[derive(Deserialize, Clone)]
pub struct BfvPkEncryptionCircuit {
    pk0i: Vec<Vec<String>>,
    pk1i: Vec<Vec<String>>,
    u: Vec<String>,
    e0: Vec<String>,
    e1: Vec<String>,
    k1: Vec<String>,
    r2is: Vec<Vec<String>>,
    r1is: Vec<Vec<String>>,
    p2is: Vec<Vec<String>>,
    p1is: Vec<Vec<String>>,
    ct0is: Vec<Vec<String>>,
    ct1is: Vec<Vec<String>>,
}

impl BfvPkEncryptionCircuit {
    pub fn create_empty_circuit(num_moduli: usize, degree: usize) -> Self {
        let zero_str = String::from("0");

        BfvPkEncryptionCircuit {
            pk0i: vec![vec![zero_str.clone(); degree]; num_moduli],
            pk1i: vec![vec![zero_str.clone(); degree]; num_moduli],
            ct0is: vec![vec![zero_str.clone(); degree]; num_moduli],
            ct1is: vec![vec![zero_str.clone(); degree]; num_moduli],
            r1is: vec![vec![zero_str.clone(); 2 * (degree - 1) + 1]; num_moduli],
            r2is: vec![vec![zero_str.clone(); degree - 1]; num_moduli],
            p1is: vec![vec![zero_str.clone(); 2 * (degree - 1) + 1]; num_moduli],
            p2is: vec![vec![zero_str.clone(); degree - 1]; num_moduli],
            u: vec![zero_str.clone(); degree],
            e0: vec![zero_str.clone(); degree],
            e1: vec![zero_str.clone(); degree],
            k1: vec![zero_str.clone(); degree],
        }
    }
}

impl From<InputValidationVectors> for BfvPkEncryptionCircuit {
    fn from(input: InputValidationVectors) -> Self {
        BfvPkEncryptionCircuit {
            pk0i: to_string_2d_vec(&input.pk0is),
            pk1i: to_string_2d_vec(&input.pk1is),
            ct0is: to_string_2d_vec(&input.ct0is),
            ct1is: to_string_2d_vec(&input.ct1is),
            r1is: to_string_2d_vec(&input.r1is),
            r2is: to_string_2d_vec(&input.r2is),
            p1is: to_string_2d_vec(&input.p1is),
            p2is: to_string_2d_vec(&input.p2is),
            u: to_string_1d_vec(&input.u),
            e0: to_string_1d_vec(&input.e0),
            e1: to_string_1d_vec(&input.e1),
            k1: to_string_1d_vec(&input.k1),
        }
    }
}

/// Payload returned by the first phase of the circuit to be reused in the second phase
pub struct Payload<F: ScalarField> {
    pk0i_assigned: Vec<PolyAssigned<F>>,
    pk1i_assigned: Vec<PolyAssigned<F>>,
    u_assigned: PolyAssigned<F>,
    e0_assigned: PolyAssigned<F>,
    e1_assigned: PolyAssigned<F>,
    k1_assigned: PolyAssigned<F>,
    r2is_assigned: Vec<PolyAssigned<F>>,
    r1is_assigned: Vec<PolyAssigned<F>>,
    p2is_assigned: Vec<PolyAssigned<F>>,
    p1is_assigned: Vec<PolyAssigned<F>>,
    ct0is_assigned: Vec<PolyAssigned<F>>,
    ct1is_assigned: Vec<PolyAssigned<F>>,
}

impl<F: ScalarField> RlcCircuitInstructions<F> for BfvPkEncryptionCircuit {
    type FirstPhasePayload = Payload<F>;

    /// #### Phase 0

    /// In this phase, the polynomials for each matrix $S_i$ are assigned to the circuit. Namely:
    /// * polynomials `u`,'e1, `e0`, `k1`, `pk0i`,`pk1_q1` are assigned to the witness table. This has to be done only once as these polynomial are common to each $S_i$ matrix
    /// * polynomials `r1i`, `r2i`,`p1i`,`p2i` are assigned to the witness table for each $S_i$ matrix
    /// * polynomials 'ct0is' and 'ct1is` are assigned to the witness table for each $Ct_i$

    /// Witness values are element of the finite field $\mod{p}$. Negative coefficients $-z$ are assigned as field elements $p - z$.

    /// At the end of phase 0, the witness generated so far is interpolated into a polynomial and committed by the prover. The hash of this commitment is used as challenge and will be used as a source of randomness $\gamma$ in Phase 1. This feature is made available by Halo2 [Challenge API](https://hackmd.io/@axiom/SJw3p-qX3).

    fn virtual_assign_phase0(
        &self,
        builder: &mut RlcCircuitBuilder<F>,
        _: &RangeChip<F>,
    ) -> Self::FirstPhasePayload {
        let ctx = builder.base.main(0);

        let mut public_inputs = vec![];

        let pk0i = self
            .pk0i
            .iter()
            .map(|pk0i| Poly::<F>::new(pk0i.clone()))
            .collect::<Vec<_>>();
        let pk0i_assigned = pk0i
            .into_iter()
            .map(|pk0i| PolyAssigned::new(ctx, pk0i))
            .collect::<Vec<_>>();

        let pk1i = self
            .pk1i
            .iter()
            .map(|pk1i| Poly::<F>::new(pk1i.clone()))
            .collect::<Vec<_>>();
        let pk1i_assigned = pk1i
            .into_iter()
            .map(|pk1i| PolyAssigned::new(ctx, pk1i))
            .collect::<Vec<_>>();

        let u = Poly::<F>::new(self.u.clone());
        let u_assigned = PolyAssigned::new(ctx, u);

        let e0 = Poly::<F>::new(self.e0.clone());
        let e0_assigned = PolyAssigned::new(ctx, e0);

        let e1 = Poly::<F>::new(self.e1.clone());
        let e1_assigned = PolyAssigned::new(ctx, e1);

        let k1 = Poly::<F>::new(self.k1.clone());
        let k1_assigned = PolyAssigned::new(ctx, k1);

        let r1is_assigned = self
            .r1is
            .iter()
            .map(|r1is| {
                let r1is = Poly::<F>::new(r1is.clone());
                PolyAssigned::new(ctx, r1is)
            })
            .collect::<Vec<_>>();

        let r2is_assigned = self
            .r2is
            .iter()
            .map(|r2is| {
                let r2is = Poly::<F>::new(r2is.clone());
                PolyAssigned::new(ctx, r2is)
            })
            .collect::<Vec<_>>();

        let p1is_assigned = self
            .p1is
            .iter()
            .map(|p1is| {
                let p1is = Poly::<F>::new(p1is.clone());
                PolyAssigned::new(ctx, p1is)
            })
            .collect::<Vec<_>>();

        let p2is_assigned = self
            .p2is
            .iter()
            .map(|p2is| {
                let p2is = Poly::<F>::new(p2is.clone());
                PolyAssigned::new(ctx, p2is)
            })
            .collect::<Vec<_>>();

        let ct0is_assigned = self
            .ct0is
            .iter()
            .map(|ct0is| {
                let ct0is = Poly::<F>::new(ct0is.clone());
                PolyAssigned::new(ctx, ct0is)
            })
            .collect::<Vec<_>>();

        let ct1is_assigned = self
            .ct1is
            .iter()
            .map(|ct1is| {
                let ct1is = Poly::<F>::new(ct1is.clone());
                PolyAssigned::new(ctx, ct1is)
            })
            .collect::<Vec<_>>();

        for pk0 in pk0i_assigned.iter() {
            public_inputs.push(pk0.assigned_coefficients[0]);
        }
        for pk1 in pk1i_assigned.iter() {
            public_inputs.push(pk1.assigned_coefficients[0]);
        }
        for ct0 in ct0is_assigned.iter() {
            public_inputs.push(ct0.assigned_coefficients[0]);
        }
        for ct1 in ct1is_assigned.iter() {
            public_inputs.push(ct1.assigned_coefficients[0]);
        }

        builder.base.assigned_instances[0] = public_inputs;

        Payload {
            pk0i_assigned,
            pk1i_assigned,
            u_assigned,
            e0_assigned,
            e1_assigned,
            k1_assigned,
            r2is_assigned,
            r1is_assigned,
            p2is_assigned,
            p1is_assigned,
            ct0is_assigned,
            ct1is_assigned,
        }
    }

    /// #### Phase 1

    /// In this phase, the following two core constraints are enforced:

    /// - The coefficients of $S_i$ are in the expected range.
    /// - $P_i(\gamma) \times S_i(\gamma) =Ct_{0,i}(\gamma)$

    /// ##### Range Check

    /// The coefficients of the private polynomials from each $i$-th matrix $S_i$ are checked to be in the correct range
    /// * Range check polynomials `u`, `e0`,`e1`,`k1`. This has to be done only once as these polynomial are common to each $S_i$ matrix
    /// * Range check polynomials `r1i`, `r2i` for each $S_i$ matrix
    /// * Range check polynomials `p1i`, `p2i` for each $S_i$ matrix
    /// * Range check polynomials `pk0`, `pk1` for each $U_i$ matrix

    /// Since negative coefficients `-z` are assigned as `p - z` to the circuit, this might result in very large coefficients. Performing the range check on such large coefficients requires large lookup tables. To avoid this, the coefficients (both negative and positive) are shifted by a constant to make them positive and then perform the range check.

    /// ##### Evaluation at $\gamma$ Constraint

    /// * Constrain the evaluation of the polynomials `u`, `e0`, `e1`, `k1` at $\gamma$. This has to be done only once as these polynomial are common to each $S_i$ matrix
    /// * Constrain the evaluation of the polynomials `r1i`, `r2i` at $\gamma$ for each $S_i$ matrix
    /// * Constrain the evaluation of the polynomials `p1i`, `p2i` at $\gamma$ for each $S_i$ matrix

    /// ##### Correct Encryption Constraint

    /// It is needed to prove that $P_i(\gamma) \times S_i(\gamma) =Ct_{0,i}(\gamma)$. This can be rewritten as `ct0i = ct0i_hat + r1i * qi + r2i * cyclo`, where `ct0i_hat = pk0i * u + e0 + k1 * k0i`.

    /// This constrained is enforced by proving that `LHS(gamma) = RHS(gamma)`. According to the Schwartz-Zippel lemma, if this relation between polynomial when evaluated at a random point holds true, then then the polynomials are identical with high probability. Note that `qi` and `k0i` (for each $U_i$ matrix) are constants to the circuit encoded during key generation.
    /// * Constrain that `ct0i(gamma) = ai(gamma) * s(gamma) + e(gamma) + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo(gamma)` for each $i$-th CRT basis
    ///

    fn virtual_assign_phase1(
        builder: &mut RlcCircuitBuilder<F>,
        range: &RangeChip<F>,
        rlc: &RlcChip<F>,
        payload: Self::FirstPhasePayload,
    ) {
        let Payload {
            pk0i_assigned,
            pk1i_assigned,
            u_assigned,
            e0_assigned,
            e1_assigned,
            k1_assigned,
            r2is_assigned,
            r1is_assigned,
            p2is_assigned,
            p1is_assigned,
            ct0is_assigned,
            ct1is_assigned,
        } = payload;

        // ASSIGNMENT

        let (ctx_gate, ctx_rlc) = builder.rlc_ctx_pair();
        let gate = range.gate();

        let mut qi_constants = vec![];
        let mut k0i_constants = vec![];

        for z in 0..ct0is_assigned.len() {
            let qi_constant = Constant(F::from_str_vartime(QIS[z]).unwrap());
            qi_constants.push(qi_constant);

            let k0i_constant = Constant(F::from_str_vartime(K0IS[z]).unwrap());
            k0i_constants.push(k0i_constant);
        }

        // cyclo poly is equal to x^N + 1
        let bits_used = (usize::BITS as usize) - (N.leading_zeros() as usize);
        rlc.load_rlc_cache((ctx_gate, ctx_rlc), gate, bits_used);
        let cyclo_at_gamma_assigned = rlc.rlc_pow_fixed(ctx_gate, gate, N);
        let cyclo_at_gamma_assigned =
            gate.add(ctx_gate, cyclo_at_gamma_assigned, Constant(F::from(1)));

        u_assigned.range_check_1bound(ctx_gate, range, U_BOUND);
        e0_assigned.range_check_1bound(ctx_gate, range, E_BOUND);
        e1_assigned.range_check_1bound(ctx_gate, range, E_BOUND);
        k1_assigned.range_check_2bounds(ctx_gate, range, K1_LOW_BOUND, K1_UP_BOUND);

        let _ = pk0i_assigned
            .iter()
            .enumerate()
            .map(|(i, pk_assigned)| pk_assigned.range_check_1bound(ctx_gate, range, PK_BOUND[i]));

        let _ = pk1i_assigned
            .iter()
            .enumerate()
            .map(|(i, pk_assigned)| pk_assigned.range_check_1bound(ctx_gate, range, PK_BOUND[i]));

        for z in 0..ct0is_assigned.len() {
            r2is_assigned[z].range_check_1bound(ctx_gate, range, R2_BOUNDS[z]);
            r1is_assigned[z].range_check_2bounds(
                ctx_gate,
                range,
                R1_LOW_BOUNDS[z],
                R1_UP_BOUNDS[z],
            );
            p2is_assigned[z].range_check_1bound(ctx_gate, range, P2_BOUNDS[z]);
            p1is_assigned[z].range_check_1bound(ctx_gate, range, P1_BOUNDS[z]);
        }

        let u_at_gamma = u_assigned.enforce_eval_at_gamma(ctx_rlc, rlc);
        let e0_at_gamma = e0_assigned.enforce_eval_at_gamma(ctx_rlc, rlc);
        let e1_at_gamma = e1_assigned.enforce_eval_at_gamma(ctx_rlc, rlc);
        let k1_at_gamma = k1_assigned.enforce_eval_at_gamma(ctx_rlc, rlc);
        let pk0i_at_gamma = pk0i_assigned
            .iter()
            .map(|pk_assigned| pk_assigned.enforce_eval_at_gamma(ctx_rlc, rlc))
            .collect::<Vec<_>>();
        let pk1i_at_gamma = pk1i_assigned
            .iter()
            .map(|pk_assigned| pk_assigned.enforce_eval_at_gamma(ctx_rlc, rlc))
            .collect::<Vec<_>>();
        let gate = range.gate();

        // For each `i` Prove that LHS(gamma) = RHS(gamma)
        // pk0_u = pk0i(gamma) * u(gamma) + e0(gamma)
        // LHS = ct0i(gamma)
        // RHS = pk0_u  + k1(gamma) * k0i + r1i(gamma) * qi + r2i(gamma) * cyclo(gamma)

        for z in 0..ct0is_assigned.len() {
            let pk0_u = gate.mul_add(ctx_gate, pk0i_at_gamma[z], u_at_gamma, e0_at_gamma);
            let r1i_at_gamma = r1is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);
            let r2i_at_gamma = r2is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);

            // CORRECT ENCRYPTION CONSTRAINT

            // rhs = pk0_u + k1(gamma) * k0i
            let rhs = gate.mul_add(ctx_gate, k1_at_gamma, k0i_constants[z], pk0_u);

            // rhs = rhs + r1i(gamma) * qi
            let rhs = gate.mul_add(ctx_gate, r1i_at_gamma, qi_constants[z], rhs);

            // rhs = rhs + r2i(gamma) * cyclo(gamma)
            let rhs = gate.mul_add(ctx_gate, r2i_at_gamma, cyclo_at_gamma_assigned, rhs);
            let lhs = ct0is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);

            // LHS(gamma) = RHS(gamma)
            let res = gate.is_equal(ctx_gate, lhs, rhs);
            gate.assert_is_const(ctx_gate, &res, &F::from(1));
        }

        for z in 0..ct1is_assigned.len() {
            let pk1_u = gate.mul_add(ctx_gate, pk1i_at_gamma[z], u_at_gamma, e1_at_gamma);

            let p1i_at_gamma = p1is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);
            let p2i_at_gamma = p2is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);

            //rhs = pk1_u + p2i * cyclo(gamma)
            let rhs = gate.mul_add(ctx_gate, p2i_at_gamma, cyclo_at_gamma_assigned, pk1_u);

            //rhs = rhs + p1s * qi
            let rhs = gate.mul_add(ctx_gate, p1i_at_gamma, qi_constants[z], rhs);

            let lhs = ct1is_assigned[z].enforce_eval_at_gamma(ctx_rlc, rlc);

            let res = gate.is_equal(ctx_gate, lhs, rhs);
            gate.assert_is_const(ctx_gate, &res, &F::from(1));
        }
    }

    fn instances(&self) -> Vec<Vec<F>> {
        let mut instance = vec![];
        for pk0 in self.pk0i.iter() {
            let pk0_poly = Poly::<F>::new(pk0.clone());
            instance.push(pk0_poly.coefficients[0]);
        }
        for pk1 in self.pk1i.iter() {
            let pk1_poly = Poly::<F>::new(pk1.clone());
            instance.push(pk1_poly.coefficients[0]);
        }
        for ct0i in self.ct0is.iter() {
            let ct0i_poly = Poly::<F>::new(ct0i.clone());
            instance.push(ct0i_poly.coefficients[0]);
        }
        for ct1i in self.ct1is.iter() {
            let ct1i_poly = Poly::<F>::new(ct1i.clone());
            instance.push(ct1i_poly.coefficients[0]);
        }
        vec![instance]
    }
}

const PARAMS_BIN: &[u8] = include_bytes!("../../params/kzg_bn254_15.bin");

pub fn load_params_from_memory() -> ParamsKZG<Bn256> {
    log::info!("Loading params from memory...");
    ParamsKZG::<Bn256>::read(&mut &PARAMS_BIN[..]).expect("Failed to parse ParamsKZG")
}

pub fn create_pk_enc_proof(input_val_vectors: InputValidationVectors) -> (Vec<u8>, Vec<Vec<u8>>) {
    // --------------------------------------------------
    // (A) Generate a proof
    // --------------------------------------------------
    let empty_pk_enc_circuit = BfvPkEncryptionCircuit::create_empty_circuit(1, 2048);

    let k = 15;
    let kzg_params = load_params_from_memory();

    // Build an RLC circuit for KeyGen
    let mut key_gen_builder =
        RlcCircuitBuilder::<Fr>::from_stage(CircuitBuilderStage::Keygen, 0).use_k(k as usize);
    key_gen_builder.base.set_lookup_bits((k - 1) as usize);
    key_gen_builder.base.set_instance_columns(1);

    let rlc_circuit_for_keygen = RlcExecutor::new(key_gen_builder, empty_pk_enc_circuit.clone());
    let rlc_circuit_params = rlc_circuit_for_keygen.0.calculate_params(Some(9));

    // Keygen VerifyingKey / ProvingKey
    let vk = keygen_vk(&kzg_params, &rlc_circuit_for_keygen).unwrap();
    let pk = keygen_pk(&kzg_params, vk, &rlc_circuit_for_keygen).unwrap();

    let break_points = rlc_circuit_for_keygen.0.builder.borrow().break_points();
    drop(rlc_circuit_for_keygen);

    // Convert input_val_vectors to BfvPkEncryptionCircuit
    let pk_enc_circuit: BfvPkEncryptionCircuit = input_val_vectors.into();
    let instances: Vec<Vec<Fr>> = pk_enc_circuit.instances();

    // Build the RLC circuit for the real data
    let mut builder: RlcCircuitBuilder<Fr> =
        RlcCircuitBuilder::from_stage(CircuitBuilderStage::Prover, 0)
            .use_params(rlc_circuit_params.clone());
    builder.base.set_lookup_bits((k - 1) as usize);
    builder.base.set_instance_columns(1);

    let rlc_prover_circuit = RlcExecutor::new(builder, pk_enc_circuit.clone());
    rlc_prover_circuit
        .0
        .builder
        .borrow_mut()
        .set_break_points(break_points);

    // Create a proof
    let mut rng = StdRng::seed_from_u64(OsRng.next_u64());
    let instance_refs = vec![instances[0].as_slice()];

    let proof = {
        let mut transcript = Keccak256Transcript::new(Vec::new());
        create_proof::<_, ProverSHPLONK<_>, _, _, _, _>(
            &kzg_params,
            &pk,
            &[rlc_prover_circuit],
            &[&instance_refs],
            &mut rng,
            &mut transcript,
        )
        .unwrap();
        transcript.finalize()
    };

    let instance_bytes: Vec<Vec<u8>> = instances[0]
        .iter()
        .map(|fr| {
            let fr_u256 = fr_to_u256(fr);
            let bytes: [u8; 32] = fr_u256.to_be_bytes();
            bytes.to_vec()
        })
        .collect();

    (proof, instance_bytes)
}

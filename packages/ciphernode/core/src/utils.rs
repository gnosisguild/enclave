use crate::SharedRng;
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder},
    mbfv::CommonRandomPoly,
};
use fhe_traits::Serialize;
use std::sync::Arc;

pub struct ParamsWithCrp {
    pub moduli: Vec<u64>,
    pub degree: usize,
    pub plaintext_modulus: u64,
    pub crp_bytes: Vec<u8>,
    pub params: Arc<BfvParameters>,
}

pub fn setup_crp_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
    rng: SharedRng,
) -> ParamsWithCrp {
    let params = setup_bfv_params(moduli, degree, plaintext_modulus);
    let crp = set_up_crp(params.clone(), rng);
    ParamsWithCrp {
        moduli: moduli.to_vec(),
        degree,
        plaintext_modulus,
        crp_bytes: crp.to_bytes(),
        params,
    }
}

fn setup_bfv_params(moduli: &[u64], degree: usize, plaintext_modulus: u64) -> Arc<BfvParameters> {
    BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build_arc()
        .unwrap()
}

fn set_up_crp(params: Arc<BfvParameters>, rng: SharedRng) -> CommonRandomPoly {
    CommonRandomPoly::new(&params, &mut *rng.lock().unwrap()).unwrap()
}

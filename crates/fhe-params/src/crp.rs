// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Common Random Polynomial (CRP) construction from BFV parameters.

use crate::builder::build_bfv_params_arc;
use e3_utils::SharedRng;
use fhe::bfv::BfvParameters;
use fhe::mbfv::CommonRandomPoly;
use fhe_traits::Serialize;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::sync::Arc;

/// Parameters plus serialized CRP, e.g. for setup or testing.
pub struct ParamsWithCrp {
    pub moduli: Vec<u64>,
    pub degree: usize,
    pub plaintext_modulus: u64,
    pub crp_bytes: Vec<u8>,
    pub params: Arc<BfvParameters>,
}

/// Builds BFV params and a CRP from raw parameter values and a RNG.
pub fn setup_crp_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
    rng: SharedRng,
) -> ParamsWithCrp {
    let params = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);
    let crp = create_crp(&params, rng);
    ParamsWithCrp {
        moduli: moduli.to_vec(),
        degree,
        plaintext_modulus,
        crp_bytes: crp.to_bytes(),
        params,
    }
}

/// Creates a Common Random Polynomial for the given BFV parameters.
pub fn create_crp(params: &Arc<BfvParameters>, rng: SharedRng) -> CommonRandomPoly {
    CommonRandomPoly::new(&params, &mut *rng.lock().unwrap()).unwrap()
}

/// Creates a Common Random Polynomial for the given BFV parameters and seed.
pub fn create_deterministic_crp_from_seed(
    params: &Arc<BfvParameters>,
    seed: [u8; 32],
) -> CommonRandomPoly {
    CommonRandomPoly::new_deterministic(&params, seed).unwrap()
}

/// Creates a Common Random Polynomial for the given BFV parameters and default seed.
pub fn create_deterministic_crp_from_default_seed(params: &Arc<BfvParameters>) -> CommonRandomPoly {
    create_deterministic_crp_from_seed(params, <ChaCha8Rng as SeedableRng>::Seed::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::insecure_512;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use std::sync::Mutex;

    #[test]
    fn setup_crp_params_returns_valid_structure() {
        let moduli = insecure_512::threshold::MODULI;
        let degree = insecure_512::DEGREE;
        let plaintext_modulus = insecure_512::threshold::PLAINTEXT_MODULUS;
        let rng = Arc::new(Mutex::new(ChaCha20Rng::from_seed(
            <ChaCha8Rng as SeedableRng>::Seed::default(),
        )));

        let ParamsWithCrp {
            moduli: out_moduli,
            degree: out_degree,
            plaintext_modulus: out_pt,
            crp_bytes,
            params,
        } = setup_crp_params(moduli, degree, plaintext_modulus, rng);

        assert_eq!(out_moduli, moduli);
        assert_eq!(out_degree, degree);
        assert_eq!(out_pt, plaintext_modulus);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert!(!crp_bytes.is_empty(), "CRP bytes should be non-empty");
    }

    #[test]
    fn crp_bytes_roundtrip_via_deserialize() {
        let params = build_bfv_params_arc(
            insecure_512::DEGREE,
            insecure_512::threshold::PLAINTEXT_MODULUS,
            insecure_512::threshold::MODULI,
            Some(insecure_512::threshold::ERROR1_VARIANCE),
        );
        let rng = Arc::new(Mutex::new(ChaCha20Rng::from_seed(
            <ChaCha8Rng as SeedableRng>::Seed::default(),
        )));
        let crp = create_crp(&params, rng);
        let bytes = crp.to_bytes();

        let restored = CommonRandomPoly::deserialize(&bytes, &params)
            .expect("CRP deserialization should succeed");
        let restored_bytes = restored.to_bytes();
        assert_eq!(bytes, restored_bytes, "CRP roundtrip should match");
    }

    #[test]
    fn deterministic_crp_same_seed_same_output() {
        let params = build_bfv_params_arc(
            insecure_512::DEGREE,
            insecure_512::threshold::PLAINTEXT_MODULUS,
            insecure_512::threshold::MODULI,
            Some(insecure_512::threshold::ERROR1_VARIANCE),
        );
        let seed = [42u8; 32];

        let crp1 = create_deterministic_crp_from_seed(&params, seed);
        let crp2 = create_deterministic_crp_from_seed(&params, seed);

        assert_eq!(crp1.to_bytes(), crp2.to_bytes());
    }

    #[test]
    fn deterministic_crp_different_seed_different_output() {
        let params = build_bfv_params_arc(
            insecure_512::DEGREE,
            insecure_512::threshold::PLAINTEXT_MODULUS,
            insecure_512::threshold::MODULI,
            Some(insecure_512::threshold::ERROR1_VARIANCE),
        );
        let seed1 = [1u8; 32];
        let mut seed2 = [2u8; 32];
        seed2[0] = 2;

        let crp1 = create_deterministic_crp_from_seed(&params, seed1);
        let crp2 = create_deterministic_crp_from_seed(&params, seed2);

        assert_ne!(crp1.to_bytes(), crp2.to_bytes());
    }
}

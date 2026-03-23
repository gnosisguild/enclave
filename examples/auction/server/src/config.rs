use auction_example::{build_eval_key, build_params, build_relin_key};
use fhe::bfv::{BfvParameters, EvaluationKey, PublicKey, RelinearizationKey, SecretKey};
use fhe_traits::Serialize;
use rand::rngs::OsRng;
use std::sync::Arc;

/// All FHE keys needed by the auction server.
pub struct FheKeys {
    pub params: Arc<BfvParameters>,
    pub sk: SecretKey,
    pub pk: PublicKey,
    pub pk_bytes: Vec<u8>,
    pub eval_key: EvaluationKey,
    pub relin_key: RelinearizationKey,
}

impl FheKeys {
    pub fn generate() -> Self {
        log::info!("Generating BFV parameters and keys...");
        let params = build_params();
        log::info!(
            "Parameters: N={}, t={}, L={} moduli",
            params.degree(),
            params.plaintext(),
            params.moduli().len()
        );

        let sk = SecretKey::random(&params, &mut OsRng);
        let pk = PublicKey::new(&sk, &mut OsRng);
        let pk_bytes = pk.to_bytes();
        log::info!("Public key size: {} bytes", pk_bytes.len());

        let eval_key = build_eval_key(&sk);
        let relin_key = build_relin_key(&sk);
        log::info!("Keys generated successfully");

        FheKeys {
            params,
            sk,
            pk,
            pk_bytes,
            eval_key,
            relin_key,
        }
    }
}

#![crate_name = "bfv"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]

mod util;

use std::{sync::Arc};
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Ciphertext, Encoding, Plaintext, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheDecoder, Serialize as FheSerialize, DeserializeParametrized};
use rand::{Rng, rngs::OsRng, thread_rng};
use util::timeit::{timeit};

pub struct EnclaveBFV {
    pub pk_share: PublicKeyShare,
    sk_share: SecretKey,
    pub params: Arc<BfvParameters>,
    pub crp: CommonRandomPoly,
}

impl EnclaveBFV {
    pub fn new(degree: usize, plaintext_modulus: u64, moduli: Vec<u64>) -> Self {
	    // let degree = 4096;
	    // let plaintext_modulus: u64 = 4096;
	    // let moduli = vec![0xffffee001, 0xffffc4001, 0x1ffffe0001];

	    // Generate the BFV parameters structure.
	    let params = timeit!(
	        "Parameters generation",
	        BfvParametersBuilder::new()
	            .set_degree(degree)
	            .set_plaintext_modulus(plaintext_modulus)
	            .set_moduli(&moduli)
	            .build_arc().unwrap()
	    );

	    let crp = CommonRandomPoly::new(&params, &mut thread_rng()).unwrap();
	    //TODO: save encrypted sk_share to disk?
        let sk_share = SecretKey::random(&params, &mut OsRng);
        let pk_share = PublicKeyShare::new(&sk_share, crp.clone(), &mut thread_rng()).unwrap();

        Self { pk_share, sk_share, params, crp }
    }

    pub fn get_pk_bytes(&mut self) -> Vec<u8> {
    	self.pk_share.to_bytes()
    }
}
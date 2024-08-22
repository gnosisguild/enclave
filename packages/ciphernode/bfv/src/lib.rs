#![crate_name = "bfv"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]

mod util;

use fhe::{
    bfv::{BfvParametersBuilder, Ciphertext, Encoding, Plaintext, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheDecoder, Serialize as FheSerialize, DeserializeParametrized};
use rand::{Rng, rngs::OsRng, thread_rng};
use util::timeit::{timeit};

pub struct EnclaveBFV {
	pub address: String,
	pub pk_share: Vec<u8>,
}

impl EnclaveBFV {
    pub fn new(address: String) -> Self {
	    let degree = 4096;
	    let plaintext_modulus: u64 = 4096;
	    let moduli = vec![0xffffee001, 0xffffc4001, 0x1ffffe0001];

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
	    //let crp_bytes = crp.to_bytes();
        let sk_share_1 = SecretKey::random(&params, &mut OsRng);
        let pk_share_1 = PublicKeyShare::new(&sk_share_1, crp.clone(), &mut thread_rng()).unwrap();
        // serialize pk_share
        let pk_share = pk_share_1.to_bytes();
        let sk_share = sk_share_1.coeffs.into_vec();

        Self { address, pk_share }
    }
}
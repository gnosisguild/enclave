#![crate_name = "bfv"]
#![crate_type = "lib"]
#![warn(missing_docs, unused_imports)]

mod util;

use std::{sync::Arc};
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Ciphertext, Encoding, Plaintext, SecretKey, PublicKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheDecoder, Serialize as FheSerialize, Deserialize, DeserializeParametrized};
use rand::{Rng, rngs::OsRng, thread_rng, SeedableRng};
use rand::rngs::StdRng;
use util::timeit::{timeit};

pub struct EnclaveBFV {
    pub pk_share: PublicKeyShare,
    sk_share: SecretKey,
    pub params: Arc<BfvParameters>,
    pub crp: CommonRandomPoly,
}

impl EnclaveBFV {
    pub fn new(degree: usize, plaintext_modulus: u64, moduli: Vec<u64>, seed: u64) -> Self {
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
        let mut r = StdRng::seed_from_u64(seed);
	    let crp = CommonRandomPoly::new(&params, &mut r).unwrap();
	    //TODO: save encrypted sk_share to disk?
        let sk_share = SecretKey::random(&params, &mut OsRng);
        let pk_share = PublicKeyShare::new(&sk_share, crp.clone(), &mut thread_rng()).unwrap();

        Self { pk_share, sk_share, params, crp }
    }

    pub fn serialize_pk(&mut self) -> Vec<u8> {
    	self.pk_share.to_bytes()
    }

    pub fn deserialize_pk(&mut self, bytes: Vec<u8>, par_bytes: Vec<u8>, crp_bytes: Vec<u8>) -> PublicKeyShare {
    	let params = Arc::new(BfvParameters::try_deserialize(&par_bytes).unwrap());
    	let crp = CommonRandomPoly::deserialize(&crp_bytes, &params).unwrap();
    	PublicKeyShare::deserialize(&bytes, &params, crp.clone()).unwrap()
    }

    pub fn serialize_crp(&mut self) -> Vec<u8> {
    	self.crp.to_bytes()
    }

    pub fn deserialize_crp(&mut self, bytes: Vec<u8>, par_bytes: Vec<u8>) -> CommonRandomPoly {
    	let params = Arc::new(BfvParameters::try_deserialize(&par_bytes).unwrap());
    	CommonRandomPoly::deserialize(&bytes, &params).unwrap()
    }

    pub fn serialize_params(&mut self) -> Vec<u8> {
    	self.params.to_bytes()
    }

    pub fn deserialize_params(&mut self, par_bytes: Vec<u8>) -> Arc<BfvParameters> {
    	Arc::new(BfvParameters::try_deserialize(&par_bytes).unwrap())
    }

    pub fn aggregate_pk(pk_shares: Vec<Vec<u8>>, params: Arc<BfvParameters>, crp: CommonRandomPoly) -> Vec<u8> {
        let mut parties :Vec<PublicKeyShare> = Vec::new();
        for i in 1..pk_shares.len() {
            println!("Aggregating PKShare... id {}", i);
            let data_des = PublicKeyShare::deserialize(&pk_shares[i as usize], &params, crp.clone()).unwrap();
            parties.push(data_des);
        }

        // Aggregation: this could be one of the parties or a separate entity. Or the
        // parties can aggregate cooperatively, in a tree-like fashion.
        let pk = timeit!("Public key aggregation", {
            let pk: PublicKey = parties.iter().map(|p| p.clone()).aggregate().unwrap();
            pk
        });
        //println!("{:?}", pk);
        println!("Multiparty Public Key Generated");
        pk.to_bytes()
    }
}
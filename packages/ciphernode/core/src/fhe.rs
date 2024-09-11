use crate::{
    ordered_set::OrderedSet,
    serializers::{
        CiphertextSerializer, DecryptionShareSerializer, PublicKeySerializer,
        PublicKeyShareSerializer, SecretKeySerializer,
    },
};
use actix::{Actor, Context, Handler, Message};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::FheDecoder;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{hash::Hash, sync::Arc};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Result<(Vec<u8>, Vec<u8>)>")]
pub struct GenerateKeyshare {
    // responder_pk: Vec<u8>, // TODO: use this to encrypt the secret data
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(Vec<u8>)>")]
pub struct GetAggregatePublicKey {
    pub keyshares: OrderedSet<Vec<u8>>,
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(Vec<u8>)>")]
pub struct GetAggregatePlaintext {
    pub decryptions: OrderedSet<Vec<u8>>,
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(Vec<u8>)>")]
pub struct DecryptCiphertext {
    pub unsafe_secret: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Fhe library adaptor. All FHE computations should happen through this actor.
pub struct Fhe {
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
    rng: ChaCha20Rng,
}

impl Actor for Fhe {
    type Context = Context<Self>;
}

impl Fhe {
    pub fn new(params: Arc<BfvParameters>, crp: CommonRandomPoly, rng: ChaCha20Rng) -> Self {
        Self { params, crp, rng }
    }

     pub fn try_default() -> Result<Self> {
        let moduli = &vec![0x3FFFFFFF000001];
        let degree = 2048usize;
        let plaintext_modulus = 1032193u64;
        let rng = ChaCha20Rng::from_entropy();

        Ok(Fhe::from_raw_params(
            moduli,
            degree,
            plaintext_modulus,
            rng,
        )?)
    }

    pub fn from_raw_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        mut rng: ChaCha20Rng,
    ) -> Result<Self> {
        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(&moduli)
            .build_arc()?;
        let crp = CommonRandomPoly::new(&params, &mut rng)?;

        Ok(Fhe::new(params, crp, rng))
    }
}

impl Handler<GenerateKeyshare> for Fhe {
    type Result = Result<(Vec<u8>, Vec<u8>)>;
    fn handle(&mut self, _event: GenerateKeyshare, _: &mut Self::Context) -> Self::Result {
        let sk_share = { SecretKey::random(&self.params, &mut self.rng) };
        let pk_share = { PublicKeyShare::new(&sk_share, self.crp.clone(), &mut self.rng)? };
        Ok((
            SecretKeySerializer::to_bytes(sk_share, self.params.clone())?,
            PublicKeyShareSerializer::to_bytes(pk_share, self.params.clone(), self.crp.clone())?,
        ))
    }
}

impl Handler<DecryptCiphertext> for Fhe {
    type Result = Result<Vec<u8>>;
    fn handle(&mut self, msg: DecryptCiphertext, _: &mut Self::Context) -> Self::Result {
        let DecryptCiphertext {
            unsafe_secret, // TODO: fix security issues with sending secrets between actors
            ciphertext,
        } = msg;

        let secret_key = SecretKeySerializer::from_bytes(&unsafe_secret)?;
        let ct = Arc::new(CiphertextSerializer::from_bytes(&ciphertext)?);
        let inner = DecryptionShare::new(&secret_key, &ct, &mut self.rng).unwrap();

        Ok(DecryptionShareSerializer::to_bytes(
            inner,
            self.params.clone(),
            ct.clone(),
        )?)
    }
}

impl Handler<GetAggregatePublicKey> for Fhe {
    type Result = Result<Vec<u8>>;

    fn handle(&mut self, msg: GetAggregatePublicKey, _: &mut Self::Context) -> Self::Result {
        let public_key: PublicKey = msg
            .keyshares
            .iter()
            .map(|k| PublicKeyShareSerializer::from_bytes(k))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .aggregate()?;

        Ok(PublicKeySerializer::to_bytes(
            public_key,
            self.params.clone(),
        )?)
    }
}

impl Handler<GetAggregatePlaintext> for Fhe {
    type Result = Result<Vec<u8>>;
    fn handle(&mut self, msg: GetAggregatePlaintext, _: &mut Self::Context) -> Self::Result {
        let plaintext: Plaintext = msg
            .decryptions
            .iter()
            .map(|k| DecryptionShareSerializer::from_bytes(k))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .aggregate()?;

        // XXX: how do we know what the expected output of the plaintext is in order to decrypt
        // here for serialization?
        // This would be dependent on the computation that is running.
        // For now assuming testcase of Vec<u64>
        // This could be determined based on the "program" config
        let decoded = Vec::<u64>::try_decode(&plaintext, Encoding::poly())?;
        let decoded = &decoded[0..2]; // TODO: this will be computation dependent
        Ok(bincode::serialize(&decoded)?)
    }
}

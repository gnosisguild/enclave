use crate::{
    ordered_set::OrderedSet,
    wrapped::{
        CiphertextSerializer, DecryptionShareSerializer, PlaintextSerializer, PublicKeySerializer,
        PublicKeyShareSerializer, SecretKeySerializer,
    },
};
use actix::{Actor, Context, Handler, Message};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Ciphertext, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
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
#[rtype(result = "Result<(PlaintextSerializer)>")]
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
    pub fn new(
        params: Arc<BfvParameters>,
        crp: CommonRandomPoly,
        rng: ChaCha20Rng,
    ) -> Result<Self> {
        Ok(Self { params, crp, rng })
    }
    pub fn try_default() -> Result<Self> {
        let moduli = &vec![0x3FFFFFFF000001];
        let degree = 2048usize;
        let plaintext_modulus = 1032193u64;
        let mut rng = ChaCha20Rng::from_entropy();
        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(&moduli)
            .build_arc()?;
        let crp = CommonRandomPoly::new(&params, &mut rng)?;

        Ok(Fhe::new(params, crp, rng)?)
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

// TODO: add this once we have decryption aggregation ready
// impl Handler<GetAggregatePlaintext> for Fhe {
//     type Result = Result<PlaintextSerializer>;
//     fn handle(&mut self, msg: GetAggregatePlaintext, _: &mut Self::Context) -> Self::Result {
//         let plaintext: Plaintext = msg
//             .decryptions
//             .iter()
//             .map(|k| k.clone().try_inner())
//             .collect::<Result<Vec<_>>>()? // NOTE: not optimal
//             .into_iter()
//             .aggregate()?;
//
//         Ok(PlaintextSerializer::to_bytes(plaintext))
//     }
// }

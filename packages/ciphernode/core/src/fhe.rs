use crate::{
    ordered_set::OrderedSet,
    serializers::{
        CiphertextSerializer, DecryptionShareSerializer, PublicKeyShareSerializer,
        SecretKeySerializer,
    },
    ActorFactory, E3Requested, EnclaveEvent,
};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{FheDecoder, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::sync::{Arc, Mutex};

pub struct GetAggregatePublicKey {
    pub keyshares: OrderedSet<Vec<u8>>,
}

pub struct GetAggregatePlaintext {
    pub decryptions: OrderedSet<Vec<u8>>,
}

pub struct DecryptCiphertext {
    pub unsafe_secret: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub type SharedRng = Arc<Mutex<ChaCha20Rng>>;

/// Fhe library adaptor.
#[derive(Clone)]
pub struct Fhe {
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
    rng: SharedRng,
}

impl Fhe {
    pub fn new(params: Arc<BfvParameters>, crp: CommonRandomPoly, rng: SharedRng) -> Self {
        Self { params, crp, rng }
    }

    // Deprecated
    pub fn try_default() -> Result<Self> {
        // TODO: The production bootstrapping of this will involve receiving a crp bytes and param
        // input form the event
        let moduli = &vec![0x3FFFFFFF000001];
        let degree = 2048usize;
        let plaintext_modulus = 1032193u64;
        let rng = Arc::new(Mutex::new(ChaCha20Rng::from_entropy()));
        let crp = CommonRandomPoly::new(
            &BfvParametersBuilder::new()
                .set_degree(degree)
                .set_plaintext_modulus(plaintext_modulus)
                .set_moduli(&moduli)
                .build_arc()?,
            &mut *rng.lock().unwrap(),
        )?
        .to_bytes();

        Ok(Fhe::from_raw_params(
            moduli,
            degree,
            plaintext_modulus,
            &crp,
            rng,
        )?)
    }

    pub fn from_raw_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        crp: &[u8],
        rng: Arc<Mutex<ChaCha20Rng>>,
    ) -> Result<Self> {
        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(moduli)
            .build_arc()?;

        Ok(Fhe::new(
            params.clone(),
            CommonRandomPoly::deserialize(crp, &params)?,
            rng,
        ))
    }

    pub fn generate_keyshare(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        let sk_share = { SecretKey::random(&self.params, &mut *self.rng.lock().unwrap()) };
        let pk_share =
            { PublicKeyShare::new(&sk_share, self.crp.clone(), &mut *self.rng.lock().unwrap())? };
        Ok((
            SecretKeySerializer::to_bytes(sk_share, self.params.clone())?,
            PublicKeyShareSerializer::to_bytes(pk_share, self.params.clone(), self.crp.clone())?,
        ))
    }

    pub fn decrypt_ciphertext(&self, msg: DecryptCiphertext) -> Result<Vec<u8>> {
        let DecryptCiphertext {
            unsafe_secret, // TODO: fix security issues with sending secrets between actors
            ciphertext,
        } = msg;

        let secret_key = SecretKeySerializer::from_bytes(&unsafe_secret)?;
        let ct = Arc::new(CiphertextSerializer::from_bytes(&ciphertext)?);
        let inner = DecryptionShare::new(&secret_key, &ct, &mut *self.rng.lock().unwrap()).unwrap();

        Ok(DecryptionShareSerializer::to_bytes(
            inner,
            self.params.clone(),
            ct.clone(),
        )?)
    }

    pub fn get_aggregate_public_key(&self, msg: GetAggregatePublicKey) -> Result<Vec<u8>> {
        let public_key: PublicKey = msg
            .keyshares
            .iter()
            .map(|k| PublicKeyShareSerializer::from_bytes(k))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .aggregate()?;

        Ok(public_key.to_bytes())
    }

    pub fn get_aggregate_plaintext(&self, msg: GetAggregatePlaintext) -> Result<Vec<u8>> {
        let plaintext: Plaintext = msg
            .decryptions
            .iter()
            .map(|k| DecryptionShareSerializer::from_bytes(k))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .aggregate()?;
    
        let decoded = Vec::<u64>::try_decode(&plaintext, Encoding::poly())?;
        let decoded = &decoded[0..2]; // TODO: remove this and leave it up to the caller
        Ok(bincode::serialize(&decoded)?)
    }
}

pub struct FheFactory;

impl FheFactory {
    pub fn create(rng: Arc<Mutex<ChaCha20Rng>>) -> ActorFactory {
        Box::new(move |ctx, evt| {
            // Saving the fhe on Committee Requested
            let EnclaveEvent::E3Requested { data, .. } = evt else {
                return;
            };
            let E3Requested {
                degree,
                moduli,
                plaintext_modulus,
                crp,
                ..
            } = data;

            ctx.fhe = Some(Arc::new(
                Fhe::from_raw_params(&moduli, degree, plaintext_modulus, &crp, rng.clone())
                    .unwrap(),
            ));
        })
    }
}

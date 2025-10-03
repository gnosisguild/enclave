// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use super::create_crp;
use anyhow::*;
use async_trait::async_trait;
use e3_bfv_helpers::{build_bfv_params_arc, decode_bfv_params_arc};
use e3_data::{FromSnapshotWithParams, Snapshot};
use e3_events::{OrderedSet, Seed};
use fhe::{
    bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{Deserialize, DeserializeParametrized, FheDecoder, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::sync::{Arc, Mutex};

pub struct GetAggregatePublicKey {
    pub keyshares: OrderedSet<Vec<u8>>,
}

pub struct GetAggregatePlaintext {
    pub decryptions: OrderedSet<Vec<u8>>,
    pub ciphertext_output: Vec<u8>,
}

pub struct DecryptCiphertext {
    pub unsafe_secret: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub type SharedRng = Arc<Mutex<ChaCha20Rng>>;

/// Fhe library adaptor.
#[derive(Clone)]
pub struct Fhe {
    pub params: Arc<BfvParameters>,
    pub crp: CommonRandomPoly,
    rng: SharedRng,
}

impl Fhe {
    pub fn new(params: Arc<BfvParameters>, crp: CommonRandomPoly, rng: SharedRng) -> Self {
        Self { params, crp, rng }
    }

    pub fn from_encoded(bytes: &[u8], seed: Seed, rng: SharedRng) -> Result<Self> {
        let params = decode_bfv_params_arc(bytes);
        let crp = create_crp(
            params.clone(),
            Arc::new(Mutex::new(ChaCha20Rng::from_seed(seed.into()))),
        );

        Ok(Fhe::new(params, crp, rng))
    }

    pub fn from_raw_params(
        moduli: &[u64],
        degree: usize,
        plaintext_modulus: u64,
        crp: &[u8],
        rng: Arc<Mutex<ChaCha20Rng>>,
    ) -> Result<Self> {
        let params = build_bfv_params_arc(degree, plaintext_modulus, moduli);

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
            SecretKeySerializer::to_bytes(sk_share)?,
            pk_share.to_bytes(),
        ))
    }

    pub fn decrypt_ciphertext(&self, msg: DecryptCiphertext) -> Result<Vec<u8>> {
        let DecryptCiphertext {
            unsafe_secret,
            ciphertext,
        } = msg;

        let secret_key = SecretKeySerializer::from_bytes(&unsafe_secret, self.params.clone())?;
        let ct = Arc::new(
            Ciphertext::from_bytes(&ciphertext, &self.params)
                .context("Error deserializing ciphertext")?,
        );
        let decryption_share =
            DecryptionShare::new(&secret_key, &ct, &mut *self.rng.lock().unwrap()).unwrap();
        Ok(decryption_share.to_bytes())
    }

    pub fn get_aggregate_public_key(&self, msg: GetAggregatePublicKey) -> Result<Vec<u8>> {
        let public_key: PublicKey = msg
            .keyshares
            .iter()
            .map(|k| PublicKeyShare::deserialize(k, &self.params, self.crp.clone()))
            .aggregate()?;

        Ok(public_key.to_bytes())
    }

    pub fn get_aggregate_plaintext(&self, msg: GetAggregatePlaintext) -> Result<Vec<u8>> {
        let arc_ct = Arc::new(Ciphertext::from_bytes(
            &msg.ciphertext_output,
            &self.params,
        )?);

        let plaintext: Plaintext = msg
            .decryptions
            .iter()
            .map(|k| DecryptionShare::deserialize(k, &self.params, arc_ct.clone()))
            .aggregate()?;
        let decoded = Vec::<u64>::try_decode(&plaintext, Encoding::poly())?;
        let mut bytes = Vec::with_capacity(decoded.len() * 8);
        for value in decoded {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        Ok(bytes)
    }
}

impl Snapshot for Fhe {
    type Snapshot = FheSnapshot;
    fn snapshot(&self) -> Result<Self::Snapshot> {
        Ok(FheSnapshot {
            crp: self.crp.to_bytes(),
            params: self.params.to_bytes(),
        })
    }
}

#[async_trait]
impl FromSnapshotWithParams for Fhe {
    type Params = SharedRng;
    async fn from_snapshot(rng: SharedRng, snapshot: FheSnapshot) -> Result<Self> {
        let params = Arc::new(BfvParameters::try_deserialize(&snapshot.params)?);
        let crp = CommonRandomPoly::deserialize(&snapshot.crp, &params)?;
        Ok(Fhe::new(params, crp, rng))
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FheSnapshot {
    crp: Vec<u8>,
    params: Vec<u8>,
}

struct SecretKeySerializer {
    pub inner: SecretKey,
}

impl SecretKeySerializer {
    pub fn to_bytes(inner: SecretKey) -> Result<Vec<u8>> {
        let value = Self { inner };
        Ok(value.unsafe_serialize()?)
    }

    pub fn from_bytes(bytes: &[u8], params: Arc<BfvParameters>) -> Result<SecretKey> {
        Ok(Self::deserialize(bytes, params)?.inner)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SecretKeyData {
    coeffs: Box<[i64]>,
}

impl SecretKeySerializer {
    pub fn unsafe_serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&SecretKeyData {
            coeffs: self.inner.coeffs.clone(),
        })?)
    }

    pub fn deserialize(bytes: &[u8], params: Arc<BfvParameters>) -> Result<SecretKeySerializer> {
        let SecretKeyData { coeffs } = bincode::deserialize(&bytes)?;
        Ok(Self {
            inner: SecretKey::new(coeffs.to_vec(), &params),
        })
    }
}

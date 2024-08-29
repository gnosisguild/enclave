use crate::{
    fhe::{WrappedPublicKey, WrappedPublicKeyShare},
    WrappedCiphertext, WrappedDecryptionShare,
};
use actix::Message;
use bincode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct E3id(pub String);
impl fmt::Display for E3id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl E3id {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<u32> for E3id {
    fn from(value: u32) -> Self {
        E3id::new(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub [u8; 32]);

impl EventId {
    fn from<T: Hash>(value: T) -> Self {
        let mut hasher = Sha256::new();
        let mut std_hasher = DefaultHasher::new();
        value.hash(&mut std_hasher);
        hasher.update(std_hasher.finish().to_le_bytes());
        let result = hasher.finalize();
        EventId(result.into())
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base58_string = bs58::encode(&self.0).into_string();
        write!(f, "eid_{}", base58_string)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvent {
    KeyshareCreated {
        id: EventId,
        data: KeyshareCreated,
    },
    ComputationRequested {
        id: EventId,
        data: ComputationRequested,
    },
    PublicKeyAggregated {
        id: EventId,
        data: PublicKeyAggregated,
    },
    DecryptionRequested {
        id: EventId,
        data: DecryptionRequested
    },
    DecryptionshareCreated {
        id: EventId,
        data: DecryptionshareCreated
    }
    // CommitteeSelected,
    // OutputDecrypted,
    // CiphernodeRegistered,
    // CiphernodeDeregistered,
}

impl EnclaveEvent {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    pub fn get_id(&self) -> EventId {
        self.clone().into()
    }
}

impl From<EnclaveEvent> for EventId {
    fn from(value: EnclaveEvent) -> Self {
        match value {
            EnclaveEvent::KeyshareCreated { id, .. } => id,
            EnclaveEvent::ComputationRequested { id, .. } => id,
            EnclaveEvent::PublicKeyAggregated { id, .. } => id,
            EnclaveEvent::DecryptionRequested { id, .. } => id,
            EnclaveEvent::DecryptionshareCreated { id, .. } => id
        }
    }
}

impl From<KeyshareCreated> for EnclaveEvent {
    fn from(data: KeyshareCreated) -> Self {
        EnclaveEvent::KeyshareCreated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<ComputationRequested> for EnclaveEvent {
    fn from(data: ComputationRequested) -> Self {
        EnclaveEvent::ComputationRequested {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<PublicKeyAggregated> for EnclaveEvent {
    fn from(data: PublicKeyAggregated) -> Self {
        EnclaveEvent::PublicKeyAggregated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}


impl From<DecryptionRequested> for EnclaveEvent {
    fn from(data: DecryptionRequested) -> Self {
        EnclaveEvent::DecryptionRequested {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<DecryptionshareCreated> for EnclaveEvent {
    fn from(data: DecryptionshareCreated) -> Self {
        EnclaveEvent::DecryptionshareCreated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl fmt::Display for EnclaveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}({})", self.event_type(), self.get_id()))
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct KeyshareCreated {
    pub pubkey: WrappedPublicKeyShare,
    pub e3_id: E3id,
}


#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct  DecryptionshareCreated {
    pub decryption_share: WrappedDecryptionShare,
    pub e3_id: E3id
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    pub pubkey: WrappedPublicKey,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputationRequested {
    pub e3_id: E3id,
    pub nodecount: usize,
    pub threshold: usize,
    pub sortition_seed: u32,
    // computation_type: ??, // TODO:
    // execution_model_type: ??, // TODO:
    // input_deadline: ??, // TODO:
    // availability_duration: ??, // TODO:
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DecryptionRequested {
    pub e3_id: E3id,
    pub ciphertext: WrappedCiphertext,
}

fn extract_enclave_event_name(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' || item == b'(' {
            return &s[..i];
        }
    }
    s
}

impl EnclaveEvent {
    pub fn event_type(&self) -> String {
        let s = format!("{:?}", self);
        extract_enclave_event_name(&s).to_string()
    }
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    use fhe::{
        bfv::{BfvParametersBuilder, SecretKey},
        mbfv::{CommonRandomPoly, PublicKeyShare},
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    use crate::{events::extract_enclave_event_name, E3id, KeyshareCreated, WrappedPublicKeyShare};

    use super::EnclaveEvent;

    #[test]
    fn test_extract_enum_name() {
        assert_eq!(
            extract_enclave_event_name("KeyshareCreated(KeyshareCreated { pubkey: [] })"),
            "KeyshareCreated"
        );
        assert_eq!(
            extract_enclave_event_name("CommitteeSelected(SomeStruct { t: 8 })"),
            "CommitteeSelected"
        );
    }

    #[test]
    fn test_deserialization() -> Result<(), Box<dyn Error>> {
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
        let sk_share = { SecretKey::random(&params, &mut rng) };
        let pk_share = { PublicKeyShare::new(&sk_share, crp.clone(), &mut rng)? };
        let pubkey = WrappedPublicKeyShare::from_fhe_rs(pk_share, params.clone(), crp.clone());
        let kse = EnclaveEvent::from(KeyshareCreated {
            e3_id: E3id::from(1001),
            pubkey,
        });
        let kse_bytes = kse.to_bytes()?;
        let _ = EnclaveEvent::from_bytes(&kse_bytes.clone());
        // deserialization occurred without panic!
        Ok(())
    }
}

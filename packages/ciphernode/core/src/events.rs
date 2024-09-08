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
    CommitteeRequested {
        id: EventId,
        data: CommitteeRequested,
    },
    PublicKeyAggregated {
        id: EventId,
        data: PublicKeyAggregated,
    },
    CiphertextOutputPublished {
        id: EventId,
        data: CiphertextOutputPublished,
    },
    DecryptionshareCreated {
        id: EventId,
        data: DecryptionshareCreated,
    },
    PlaintextAggregated {
        id: EventId,
        data: PlaintextAggregated,
    },
    CiphernodeSelected {
        id: EventId,
        data: CiphernodeSelected,
    },
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
            EnclaveEvent::CommitteeRequested { id, .. } => id,
            EnclaveEvent::PublicKeyAggregated { id, .. } => id,
            EnclaveEvent::CiphertextOutputPublished { id, .. } => id,
            EnclaveEvent::DecryptionshareCreated { id, .. } => id,
            EnclaveEvent::PlaintextAggregated { id, .. } => id,
            EnclaveEvent::CiphernodeSelected { id, .. } => id,
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

impl From<CommitteeRequested> for EnclaveEvent {
    fn from(data: CommitteeRequested) -> Self {
        EnclaveEvent::CommitteeRequested {
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

impl From<CiphertextOutputPublished> for EnclaveEvent {
    fn from(data: CiphertextOutputPublished) -> Self {
        EnclaveEvent::CiphertextOutputPublished {
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

impl From<PlaintextAggregated> for EnclaveEvent {
    fn from(data: PlaintextAggregated) -> Self {
        EnclaveEvent::PlaintextAggregated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphernodeSelected> for EnclaveEvent {
    fn from(data: CiphernodeSelected) -> Self {
        EnclaveEvent::CiphernodeSelected {
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
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct DecryptionshareCreated {
    pub decryption_share: Vec<u8>,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CommitteeRequested {
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
pub struct CiphernodeSelected {
    pub e3_id: E3id,
    pub nodecount: usize,
    pub threshold: usize,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphertextOutputPublished {
    pub e3_id: E3id,
    pub ciphertext_output: Vec<u8>,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PlaintextAggregated {
    pub e3_id: E3id,
    pub decrypted_output: Vec<u8>,
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
    use super::EnclaveEvent;
    use crate::{
        events::extract_enclave_event_name, serializers::PublicKeyShareSerializer, E3id,
        KeyshareCreated,
    };
    use fhe::{
        bfv::{BfvParametersBuilder, SecretKey},
        mbfv::{CommonRandomPoly, PublicKeyShare},
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;
    use std::error::Error;

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
        let pubkey = PublicKeyShareSerializer::to_bytes(pk_share, params.clone(), crp.clone())?;
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

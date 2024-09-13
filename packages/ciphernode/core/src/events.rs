use actix::Message;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EthAddr(pub Vec<u8>);

impl From<Address> for EthAddr {
    fn from(value: Address) -> Self {
        Self(value.to_vec())
    }
}

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
    CiphernodeAdded {
        id: EventId,
        data: CiphernodeAdded,
    },
    CiphernodeRemoved {
        id: EventId,
        data: CiphernodeRemoved,
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

    pub fn is_local_only(&self) -> bool {
        // Add a list of local events
        match self {
            EnclaveEvent::CiphernodeSelected { .. } => true,
            _ => false,
        }
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
            EnclaveEvent::CiphernodeAdded { id, .. } => id,
            EnclaveEvent::CiphernodeRemoved { id, .. } => id,
        }
    }
}

impl EnclaveEvent {
    pub fn get_e3_id(&self) -> Option<E3id> {
        match self.clone() {
            EnclaveEvent::KeyshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CommitteeRequested { data, .. } => Some(data.e3_id),
            EnclaveEvent::PublicKeyAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => Some(data.e3_id),
            EnclaveEvent::DecryptionshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::PlaintextAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphernodeSelected { data, .. } => Some(data.e3_id),
            _ => None,
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

impl From<CiphernodeAdded> for EnclaveEvent {
    fn from(data: CiphernodeAdded) -> Self {
        EnclaveEvent::CiphernodeAdded {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphernodeRemoved> for EnclaveEvent {
    fn from(data: CiphernodeRemoved) -> Self {
        EnclaveEvent::CiphernodeRemoved {
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
    pub sortition_seed: u64, // Should actually be much larger eg [u8;32]

    // fhe params
    pub moduli: Vec<u64>,
    pub degree: usize,
    pub plaintext_modulus: u64,
    pub crp: Vec<u8>,
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

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeAdded {
    pub address: Address,
    pub index: usize,
    pub num_nodes: usize,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeRemoved {
    pub address: Address,
    pub index: usize,
    pub num_nodes: usize,
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

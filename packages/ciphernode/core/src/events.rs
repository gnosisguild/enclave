use actix::Message;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::fhe::{WrappedPublicKey, WrappedPublicKeyShare};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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


#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
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
    // CommitteeSelected,
    // OutputDecrypted,
    // CiphernodeRegistered,
    // CiphernodeDeregistered,
}

impl From<EnclaveEvent> for EventId {
    fn from(value: EnclaveEvent) -> Self {
       match value {
           EnclaveEvent::KeyshareCreated { id, .. } => id,
           EnclaveEvent::ComputationRequested { id, .. } => id,
           EnclaveEvent::PublicKeyAggregated { id, .. } => id,
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
    fn from(data:ComputationRequested) -> Self {
        EnclaveEvent::ComputationRequested {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<PublicKeyAggregated> for EnclaveEvent{
    fn from(data:PublicKeyAggregated) -> Self {
        EnclaveEvent::PublicKeyAggregated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "anyhow::Result<()>")]
pub struct KeyshareCreated {
    pub pubkey: WrappedPublicKeyShare,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    pub pubkey: WrappedPublicKey,
    pub e3_id: E3id,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
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

    use crate::events::extract_enclave_event_name;

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
}

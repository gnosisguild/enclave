use actix::Message;
use alloy::{
    hex,
    primitives::{Uint, U256},
};
use alloy_primitives::ruint::ParseError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt::{self, Display},
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::OrderedSet;

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

impl From<String> for E3id {
    fn from(value: String) -> Self {
        E3id::new(value)
    }
}

impl From<&str> for E3id {
    fn from(value: &str) -> Self {
        E3id::new(value)
    }
}

impl TryFrom<E3id> for U256 {
    type Error = ParseError;
    fn try_from(value: E3id) -> Result<Self, Self::Error> {
        U256::from_str_radix(&value.0, 10)
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
        write!(f, "evt:{}", &base58_string[0..8])
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvent {
    KeyshareCreated {
        id: EventId,
        data: KeyshareCreated,
    },
    E3Requested {
        id: EventId,
        data: E3Requested,
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
    EnclaveError {
        id: EventId,
        data: EnclaveError,
    },
    E3RequestComplete {
        id: EventId,
        data: E3RequestComplete,
    },
    Shutdown {
        id: EventId,
        data: Shutdown,
    },
    /// This is a test event to use in testing
    TestEvent {
        id: EventId,
        data: TestEvent,
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
            EnclaveEvent::E3Requested { .. } => true,
            EnclaveEvent::CiphernodeAdded { .. } => true,
            EnclaveEvent::CiphernodeRemoved { .. } => true,
            EnclaveEvent::E3RequestComplete { .. } => true,
            EnclaveEvent::Shutdown { .. } => true,
            _ => false,
        }
    }
}

impl From<EnclaveEvent> for EventId {
    fn from(value: EnclaveEvent) -> Self {
        match value {
            EnclaveEvent::KeyshareCreated { id, .. } => id,
            EnclaveEvent::E3Requested { id, .. } => id,
            EnclaveEvent::PublicKeyAggregated { id, .. } => id,
            EnclaveEvent::CiphertextOutputPublished { id, .. } => id,
            EnclaveEvent::DecryptionshareCreated { id, .. } => id,
            EnclaveEvent::PlaintextAggregated { id, .. } => id,
            EnclaveEvent::CiphernodeSelected { id, .. } => id,
            EnclaveEvent::CiphernodeAdded { id, .. } => id,
            EnclaveEvent::CiphernodeRemoved { id, .. } => id,
            EnclaveEvent::EnclaveError { id, .. } => id,
            EnclaveEvent::E3RequestComplete { id, .. } => id,
            EnclaveEvent::Shutdown { id, .. } => id,
            EnclaveEvent::TestEvent { id, .. } => id,
        }
    }
}

impl EnclaveEvent {
    pub fn get_e3_id(&self) -> Option<E3id> {
        match self.clone() {
            EnclaveEvent::KeyshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::E3Requested { data, .. } => Some(data.e3_id),
            EnclaveEvent::PublicKeyAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => Some(data.e3_id),
            EnclaveEvent::DecryptionshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::PlaintextAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphernodeSelected { data, .. } => Some(data.e3_id),
            _ => None,
        }
    }
    pub fn get_data(&self) -> String {
        match self.clone() {
            EnclaveEvent::KeyshareCreated { data, .. } => format!("{}", data),
            EnclaveEvent::E3Requested { data, .. } => format!("{}", data),
            EnclaveEvent::PublicKeyAggregated { data, .. } => format!("{}", data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => format!("{}", data),
            EnclaveEvent::DecryptionshareCreated { data, .. } => format!("{}", data),
            EnclaveEvent::PlaintextAggregated { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeSelected { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeAdded { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeRemoved { data, .. } => format!("{}", data),
            EnclaveEvent::E3RequestComplete { data, .. } => format!("{}", data),
            EnclaveEvent::EnclaveError { data, .. } => format!("{:?}", data),
            EnclaveEvent::Shutdown { data, .. } => format!("{:?}", data),
            EnclaveEvent::TestEvent { data, .. } => format!("{:?}", data),
            // _ => "<omitted>".to_string(),
        }
    }
}

pub trait FromError {
    type Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self;
}

// TODO: These From traits should be handled by a macro
impl From<KeyshareCreated> for EnclaveEvent {
    fn from(data: KeyshareCreated) -> Self {
        EnclaveEvent::KeyshareCreated {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<E3Requested> for EnclaveEvent {
    fn from(data: E3Requested) -> Self {
        EnclaveEvent::E3Requested {
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

impl From<E3RequestComplete> for EnclaveEvent {
    fn from(data: E3RequestComplete) -> Self {
        EnclaveEvent::E3RequestComplete {
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

impl From<EnclaveError> for EnclaveEvent {
    fn from(data: EnclaveError) -> Self {
        EnclaveEvent::EnclaveError {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<Shutdown> for EnclaveEvent {
    fn from(data: Shutdown) -> Self {
        EnclaveEvent::Shutdown {
            id: EventId::from(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<TestEvent> for EnclaveEvent {
    fn from(value: TestEvent) -> Self {
        EnclaveEvent::TestEvent {
            id: EventId::from(value.clone()),
            data: value.clone(),
        }
    }
}

impl FromError for EnclaveEvent {
    type Error = anyhow::Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self {
        let error_event = EnclaveError::from_error(err_type, error);
        EnclaveEvent::from(error_event)
    }
}

impl fmt::Display for EnclaveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}({})", self.event_type(), self.get_data()))
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct KeyshareCreated {
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
    pub node: String,
}

impl Display for KeyshareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}, node: {}", self.e3_id, self.node,)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct DecryptionshareCreated {
    pub decryption_share: Vec<u8>,
    pub e3_id: E3id,
    pub node: String,
}

impl Display for DecryptionshareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}, node: {}", self.e3_id, self.node,)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
    pub nodes: OrderedSet<String>,
    pub src_chain_id: u64,
}

impl Display for PublicKeyAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, src_chain_id: {}, nodes: <omitted>, pubkey: <omitted>",
            self.e3_id, self.src_chain_id,
        )
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct E3Requested {
    pub e3_id: E3id,
    pub threshold_m: usize,
    pub seed: Seed,
    pub params: Vec<u8>,
    pub src_chain_id: u64,
}

impl Display for E3Requested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}, src_chain_id: {}, seed: {}, params: <omitted>",
            self.e3_id, self.threshold_m, self.src_chain_id, self.seed
        )
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeSelected {
    pub e3_id: E3id,
    pub threshold_m: usize,
}

impl Display for CiphernodeSelected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}",
            self.e3_id, self.threshold_m,
        )
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphertextOutputPublished {
    pub e3_id: E3id,
    pub ciphertext_output: Vec<u8>,
}

impl Display for CiphertextOutputPublished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}", self.e3_id,)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PlaintextAggregated {
    pub e3_id: E3id,
    pub decrypted_output: Vec<u8>,
    pub src_chain_id: u64,
}

impl Display for PlaintextAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, src_chain_id: {}",
            self.e3_id, self.src_chain_id
        )
    }
}

/// E3RequestComplete event is a local only event
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct E3RequestComplete {
    pub e3_id: E3id,
}

impl Display for E3RequestComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}", self.e3_id)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeAdded {
    pub address: String,
    pub index: usize,
    pub num_nodes: usize,
}

impl Display for CiphernodeAdded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "address: {}, index: {}, num_nodes: {}",
            self.address, self.index, self.num_nodes
        )
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeRemoved {
    pub address: String,
    pub index: usize,
    pub num_nodes: usize,
}

impl Display for CiphernodeRemoved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "address: {}, index: {}, num_nodes: {}",
            self.address, self.index, self.num_nodes
        )
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EnclaveError {
    pub err_type: EnclaveErrorType,
    pub message: String,
}

impl Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct Die;
impl Display for Die {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Die",)
    }
}

/// Represents a shutdown event triggered by SIG TERM
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct Shutdown;
impl Display for Shutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shutdown",)
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct TestEvent {
    pub msg: String,
}

#[cfg(test)]
impl Display for TestEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestEvent(msg: {})", self.msg)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Seed(pub [u8; 32]);
impl From<Seed> for u64 {
    fn from(value: Seed) -> Self {
        u64::from_le_bytes(value.0[..8].try_into().unwrap())
    }
}

impl From<Seed> for [u8; 32] {
    fn from(value: Seed) -> Self {
        value.0
    }
}

impl From<Uint<256, 4>> for Seed {
    fn from(value: Uint<256, 4>) -> Self {
        Seed(value.to_le_bytes())
    }
}

impl Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Seed(0x{})", hex::encode(self.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnclaveErrorType {
    Evm,
    KeyGeneration,
    PublickeyAggregation,
    IO,
    PlaintextAggregation,
    Decryption,
    Sortition,
    Data,
}

impl EnclaveError {
    pub fn new(err_type: EnclaveErrorType, message: &str) -> Self {
        Self {
            err_type,
            message: message.to_string(),
        }
    }
}

impl FromError for EnclaveError {
    type Error = anyhow::Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self {
        Self {
            err_type,
            message: error.to_string(),
        }
    }
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

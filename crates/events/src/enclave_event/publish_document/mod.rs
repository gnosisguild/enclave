// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod filter;

use std::fmt::{self, Display};

use actix::Message;
use chrono::{serde::ts_seconds, DateTime, Utc};
use filter::Filter;
use serde::{Deserialize, Serialize};

use crate::E3id;

pub type PartyId = u64;

/// Diambiguates the kind of document we are looking for
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentKind {
    TrBFV,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentMeta {
    /// We will only be interested in e3_ids we are included within
    pub e3_id: E3id,
    /// The kind of document we are looking for
    pub kind: DocumentKind,
    /// Filter based on specific ids or a range of ids who might be interested in the document.
    /// Empty Vector means there is no filter
    /// We need this to denote when payloads are too big and must be split between DHT documents
    pub filter: Vec<Filter<PartyId>>,
    /// Unix timestamp for purging
    #[serde(with = "ts_seconds")]
    pub expires_at: DateTime<Utc>,
}

impl DocumentMeta {
    pub fn new(
        e3_id: E3id,
        kind: DocumentKind,
        filter: Vec<Filter<PartyId>>,
        expires_at: DateTime<Utc>,
    ) -> DocumentMeta {
        Self {
            e3_id,
            expires_at,
            filter,
            kind,
        }
    }
}

/// EnclaveEvent for signaling that a document be published
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublishDocumentRequested {
    pub meta: DocumentMeta,
    /// Key will be a simple hash eg. Sha256Hash of the value so we need not put it here
    pub value: Vec<u8>, // TODO: ArcBytes from ry/599-multithread
}

impl Display for PublishDocumentRequested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.meta) // XXX:: apply ArcBytes and rely on debug once trbfv is merged
    }
}

/// EnclaveEvent for receiving a document
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DocumentReceived {
    /// Document metadata
    pub meta: DocumentMeta,
    /// Document value from kademlia
    pub value: Vec<u8>, // TODO: ArcBytes
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::E3id;

pub type PartyId = u64;
pub type Cid = Vec<u8>;

/// PartialOrd Filter. Can filter based on our rank in the committee (party_id) incase a payload is split between documents.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Filter<T> {
    /// Range is inclusive but means nothing for non PartialOrd T
    Range(Option<T>, Option<T>),
    /// Single item specifier
    Item(T),
}

impl<T: PartialOrd> Filter<T> {
    pub fn matches(&self, item: &T) -> bool {
        match self {
            Filter::Range(Some(start), Some(end)) => item >= start && item <= end,
            Filter::Range(Some(start), None) => item >= start,
            Filter::Range(None, Some(end)) => item <= end,
            Filter::Range(None, None) => true,
            Filter::Item(value) => item == value,
        }
    }
}

/// Metadata for a published document
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum DocumentMeta {
    TrBFVShares {
        /// We will only be interested in e3_ids we are included within
        e3_id: E3id,
        /// Filter based on specific ids or a range of ids who might be interested in the document.
        /// Empty Vector means there is no filter
        filter: Vec<Filter<PartyId>>,
        /// Unix timestamp for purging
        #[serde(with = "ts_seconds")]
        expires_at: DateTime<Utc>,
    },
    // TFHEShares ...
}

/// EnclaveEvent for signaling that a document be published
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublishDocumentRequested {
    meta: DocumentMeta,
    /// Key will be a simple hash eg. Sha256Hash of the value so we need not put it here
    value: Vec<u8>,
}

/// NetEvent payload sent/received from net command/event channel
pub struct DocumentPublished {
    meta: DocumentMeta,
    cid: Cid,
}

/// NetCommand payload sent over net command channel to actually publish the Kademlia Document
pub struct PublishDocument {
    meta: DocumentMeta,
    value: Vec<u8>,
    cid: Cid,
}

/// EnclaveEvent for receiving a document
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DocumentReceived {
    meta: DocumentMeta,
    value: Vec<u8>,
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod filter;

use actix::Message;
use chrono::{serde::ts_seconds, DateTime, Utc};
use filter::Filter;
use serde::{Deserialize, Serialize};

use crate::E3id;

pub type PartyId = u64;

/// Metadata for a published document
/// This is used by components to test interest in a published document
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentMeta {
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

/// EnclaveEvent for receiving a document
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DocumentReceived {
    meta: DocumentMeta,
    value: Vec<u8>,
}

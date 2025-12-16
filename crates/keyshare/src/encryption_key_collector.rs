// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Collector for BFV encryption keys from all parties.
//!
//! Before parties can encrypt their Shamir shares, they need to collect
//! the BFV public keys from all other parties. This actor handles that
//! collection process.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use actix::{Actor, Addr, Handler, Message};
use e3_events::{EncryptionKey, EncryptionKeyCreated};
use e3_trbfv::PartyId;
use tracing::info;

use crate::ThresholdKeyshare;

/// State of the collector
pub enum CollectorState {
    /// Currently collecting keys
    Collecting,
    /// All keys have been collected
    Finished,
}

/// Message sent when all encryption keys have been collected.
///
/// This contains all parties' BFV public keys, sorted by party_id,
/// ready to be used for encrypting shares.
#[derive(Message)]
#[rtype(result = "()")]
pub struct AllEncryptionKeysCollected {
    /// All collected encryption keys, sorted by party_id
    pub keys: Vec<Arc<EncryptionKey>>,
}

impl From<HashMap<u64, Arc<EncryptionKey>>> for AllEncryptionKeysCollected {
    fn from(value: HashMap<u64, Arc<EncryptionKey>>) -> Self {
        // Sort by party_id for deterministic ordering
        let mut keys: Vec<_> = value.into_values().collect();
        keys.sort_by_key(|k| k.party_id);
        AllEncryptionKeysCollected { keys }
    }
}

/// Actor that collects BFV encryption keys from all parties.
///
/// Once all keys are collected, it sends `AllEncryptionKeysCollected` to the parent
/// `ThresholdKeyshare` actor.
pub struct EncryptionKeyCollector {
    /// Set of party IDs we're still waiting for
    todo: HashSet<PartyId>,
    /// Parent actor to notify when collection is complete
    parent: Addr<ThresholdKeyshare>,
    /// Current state
    state: CollectorState,
    /// Collected keys indexed by party_id
    keys: HashMap<PartyId, Arc<EncryptionKey>>,
}

impl EncryptionKeyCollector {
    /// Create and start a new collector.
    ///
    /// # Arguments
    /// * `parent` - The ThresholdKeyshare actor to notify when collection is complete
    /// * `total` - Total number of parties (keys to collect)
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64) -> Addr<Self> {
        let addr = Self {
            todo: (0..total).collect(),
            parent,
            state: CollectorState::Collecting,
            keys: HashMap::new(),
        }
        .start();
        addr
    }
}

impl Actor for EncryptionKeyCollector {
    type Context = actix::Context<Self>;
}

impl Handler<EncryptionKeyCreated> for EncryptionKeyCollector {
    type Result = ();
    fn handle(&mut self, msg: EncryptionKeyCreated, _: &mut Self::Context) -> Self::Result {
        let start = Instant::now();
        info!("EncryptionKeyCollector: EncryptionKeyCreated received");

        if let CollectorState::Finished = self.state {
            info!("EncryptionKeyCollector is finished, ignoring");
            return;
        }

        let pid = msg.key.party_id;
        info!("EncryptionKeyCollector: party_id = {}", pid);

        let Some(_) = self.todo.take(&pid) else {
            info!(
                "Error: {} was not in encryption key collector's ID list",
                pid
            );
            return;
        };

        info!(
            "Inserting encryption key... waiting on: {}",
            self.todo.len()
        );
        self.keys.insert(pid, msg.key);

        if self.todo.is_empty() {
            info!("All encryption keys collected!");
            self.state = CollectorState::Finished;
            let event: AllEncryptionKeysCollected = self.keys.clone().into();
            self.parent.do_send(event);
        }

        info!(
            "Finished processing EncryptionKeyCreated in {:?}",
            start.elapsed()
        );
    }
}



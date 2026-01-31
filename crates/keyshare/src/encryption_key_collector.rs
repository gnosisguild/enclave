// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, SpawnHandle};
use e3_events::{
    E3id, EncryptionKey, EncryptionKeyCollectionFailed, EncryptionKeyCreated, TypedEvent,
};
use e3_trbfv::PartyId;
use tracing::{info, warn};

const DEFAULT_COLLECTION_TIMEOUT: Duration = Duration::from_secs(60);

use crate::ThresholdKeyshare;

/// State of the collector
pub enum CollectorState {
    /// Currently collecting keys
    Collecting,
    /// All keys have been collected
    Finished,
    /// Collection timed out
    TimedOut,
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

/// Message sent when encryption key collection times out.
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct EncryptionKeyCollectionTimeout;

/// Actor that collects BFV encryption keys from all parties.
///
/// Once all keys are collected, it sends `AllEncryptionKeysCollected` to the parent
/// `ThresholdKeyshare` actor. If collection times out, it sends `EncryptionKeyCollectionFailed`.
pub struct EncryptionKeyCollector {
    e3_id: E3id,
    todo: HashSet<PartyId>,
    parent: Addr<ThresholdKeyshare>,
    state: CollectorState,
    keys: HashMap<PartyId, Arc<EncryptionKey>>,
    timeout_handle: Option<SpawnHandle>,
}

impl EncryptionKeyCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64, e3_id: E3id) -> Addr<Self> {
        let collector = Self {
            e3_id,
            todo: (0..total).collect(),
            parent,
            state: CollectorState::Collecting,
            keys: HashMap::new(),
            timeout_handle: None,
        };
        collector.start()
    }
}

impl Actor for EncryptionKeyCollector {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            e3_id = %self.e3_id,
            "EncryptionKeyCollector started, scheduling timeout in {:?}",
            DEFAULT_COLLECTION_TIMEOUT
        );

        let handle = ctx.notify_later(EncryptionKeyCollectionTimeout, DEFAULT_COLLECTION_TIMEOUT);
        self.timeout_handle = Some(handle);
    }
}

impl Handler<TypedEvent<EncryptionKeyCreated>> for EncryptionKeyCollector {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<EncryptionKeyCreated>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        let start = Instant::now();
        info!("EncryptionKeyCollector: EncryptionKeyCreated received");

        // Ignore if already finished or timed out
        if !matches!(self.state, CollectorState::Collecting) {
            info!(
                "EncryptionKeyCollector is not collecting (state: {:?}), ignoring",
                match self.state {
                    CollectorState::Collecting => "Collecting",
                    CollectorState::Finished => "Finished",
                    CollectorState::TimedOut => "TimedOut",
                }
            );
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

            // Cancel the timeout since we're done
            if let Some(handle) = self.timeout_handle.take() {
                ctx.cancel_future(handle);
            }

            let event: TypedEvent<AllEncryptionKeysCollected> =
                TypedEvent::new(self.keys.clone().into(), ec);
            self.parent.do_send(event);
        }

        info!(
            "Finished processing EncryptionKeyCreated in {:?}",
            start.elapsed()
        );
    }
}

impl Handler<EncryptionKeyCollectionTimeout> for EncryptionKeyCollector {
    type Result = ();
    fn handle(
        &mut self,
        _: EncryptionKeyCollectionTimeout,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        // Only handle timeout if we're still collecting
        if !matches!(self.state, CollectorState::Collecting) {
            return;
        }

        warn!(
            e3_id = %self.e3_id,
            missing_parties = ?self.todo,
            "Encryption key collection timed out, {} parties missing",
            self.todo.len()
        );

        self.state = CollectorState::TimedOut;

        // Notify parent of failure
        let missing_parties: Vec<PartyId> = self.todo.iter().copied().collect();
        self.parent.do_send(EncryptionKeyCollectionFailed {
            e3_id: self.e3_id.clone(),
            reason: format!(
                "Timeout waiting for encryption keys from {} parties",
                missing_parties.len()
            ),
            missing_parties,
        });

        // Stop the actor
        ctx.stop();
    }
}

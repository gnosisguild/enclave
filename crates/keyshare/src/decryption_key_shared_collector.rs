// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, SpawnHandle};
use e3_events::{DecryptionKeyShared, E3id, TypedEvent};
use e3_utils::MAILBOX_LIMIT;
use tracing::{info, warn};

use crate::ThresholdKeyshare;

const DEFAULT_COLLECTION_TIMEOUT: Duration = Duration::from_secs(600);

enum CollectorState {
    Collecting,
    Finished,
    TimedOut,
}

/// Message sent when all expected DecryptionKeyShared events have been collected.
#[derive(Message)]
#[rtype(result = "()")]
pub struct AllDecryptionKeySharesCollected {
    pub shares: HashMap<u64, DecryptionKeyShared>,
}

/// Message sent when DecryptionKeyShared collection times out.
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct DecryptionKeySharedCollectionTimeout;

/// Message sent when DecryptionKeyShared collection fails.
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct DecryptionKeySharedCollectionFailed {
    pub e3_id: E3id,
    pub reason: String,
    pub missing_parties: Vec<u64>,
}

/// Collects `DecryptionKeyShared` events from expected parties in H (Exchange #3).
///
/// Once all expected events are collected, sends `AllDecryptionKeySharesCollected`
/// to the parent `ThresholdKeyshare` actor for C4 proof verification.
pub struct DecryptionKeySharedCollector {
    e3_id: E3id,
    /// Party IDs we expect to receive from (H minus self).
    expected: HashSet<u64>,
    parent: Addr<ThresholdKeyshare>,
    state: CollectorState,
    shares: HashMap<u64, DecryptionKeyShared>,
    timeout_handle: Option<SpawnHandle>,
}

impl DecryptionKeySharedCollector {
    pub fn setup(
        parent: Addr<ThresholdKeyshare>,
        expected_parties: HashSet<u64>,
        e3_id: E3id,
    ) -> Addr<Self> {
        let collector = Self {
            e3_id,
            expected: expected_parties,
            parent,
            state: CollectorState::Collecting,
            shares: HashMap::new(),
            timeout_handle: None,
        };
        collector.start()
    }
}

impl Actor for DecryptionKeySharedCollector {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
        info!(
            e3_id = %self.e3_id,
            "DecryptionKeySharedCollector started, expecting {} parties, timeout {:?}",
            self.expected.len(),
            DEFAULT_COLLECTION_TIMEOUT
        );
        let handle = ctx.notify_later(
            DecryptionKeySharedCollectionTimeout,
            DEFAULT_COLLECTION_TIMEOUT,
        );
        self.timeout_handle = Some(handle);
    }
}

impl Handler<TypedEvent<DecryptionKeyShared>> for DecryptionKeySharedCollector {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<DecryptionKeyShared>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        let start = Instant::now();

        if !matches!(self.state, CollectorState::Collecting) {
            return;
        }

        let pid = msg.party_id;
        if !self.expected.remove(&pid) {
            info!(
                "DecryptionKeySharedCollector: party {} not in expected set, ignoring",
                pid
            );
            return;
        }

        info!(
            "DecryptionKeySharedCollector: received from party {}, waiting on {}",
            pid,
            self.expected.len()
        );
        self.shares.insert(pid, msg);

        if self.expected.is_empty() {
            info!("All DecryptionKeyShared events collected");
            self.state = CollectorState::Finished;

            if let Some(handle) = self.timeout_handle.take() {
                ctx.cancel_future(handle);
            }

            let event: TypedEvent<AllDecryptionKeySharesCollected> = TypedEvent::new(
                AllDecryptionKeySharesCollected {
                    shares: std::mem::take(&mut self.shares),
                },
                ec,
            );
            self.parent.do_send(event);
        }

        info!(
            "Finished processing DecryptionKeyShared in {:?}",
            start.elapsed()
        );
    }
}

impl Handler<DecryptionKeySharedCollectionTimeout> for DecryptionKeySharedCollector {
    type Result = ();
    fn handle(
        &mut self,
        _: DecryptionKeySharedCollectionTimeout,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        if !matches!(self.state, CollectorState::Collecting) {
            return;
        }

        warn!(
            e3_id = %self.e3_id,
            missing_parties = ?self.expected,
            "DecryptionKeyShared collection timed out, {} parties missing",
            self.expected.len()
        );

        self.state = CollectorState::TimedOut;

        let missing_parties: Vec<u64> = self.expected.iter().copied().collect();
        self.parent.do_send(DecryptionKeySharedCollectionFailed {
            e3_id: self.e3_id.clone(),
            reason: format!(
                "Timeout waiting for DecryptionKeyShared from {} parties",
                missing_parties.len()
            ),
            missing_parties,
        });

        ctx.stop();
    }
}

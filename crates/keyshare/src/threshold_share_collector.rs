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
    E3id, ThresholdShare, ThresholdShareCollectionFailed, ThresholdShareCreated, TypedEvent,
};
use e3_trbfv::PartyId;
use e3_utils::MAILBOX_LIMIT;
use tracing::{info, warn};

use crate::{AllThresholdSharesCollected, ThresholdKeyshare};

const DEFAULT_COLLECTION_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) enum CollectorState {
    Collecting,
    Finished,
    TimedOut,
}

/// Message sent when threshold share collection times out.
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub struct ThresholdShareCollectionTimeout;

pub struct ThresholdShareCollector {
    /// The E3id for the round
    e3_id: E3id,
    /// The partys the collector expects to receive from
    todo: HashSet<PartyId>,
    /// The parent actor that has requested collection
    parent: Addr<ThresholdKeyshare>,
    /// The current state of the collector
    state: CollectorState,
    /// The shares collected
    shares: HashMap<PartyId, Arc<ThresholdShare>>,
    /// A timeout handle for when this collector will report failure
    timeout_handle: Option<SpawnHandle>,
}

impl ThresholdShareCollector {
    pub fn setup(parent: Addr<ThresholdKeyshare>, total: u64, e3_id: E3id) -> Addr<Self> {
        let collector = Self {
            e3_id,
            todo: (0..total).collect(),
            parent,
            state: CollectorState::Collecting,
            shares: HashMap::new(),
            timeout_handle: None,
        };
        collector.start()
    }
}

impl Actor for ThresholdShareCollector {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
        info!(
            e3_id = %self.e3_id,
            "ThresholdShareCollector started, scheduling timeout in {:?}",
            DEFAULT_COLLECTION_TIMEOUT
        );
        // Schedule timeout
        let handle = ctx.notify_later(ThresholdShareCollectionTimeout, DEFAULT_COLLECTION_TIMEOUT);
        self.timeout_handle = Some(handle);
    }
}

impl Handler<TypedEvent<ThresholdShareCreated>> for ThresholdShareCollector {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<ThresholdShareCreated>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        let start = Instant::now();
        info!("ThresholdShareCollector: ThresholdShareCreated received by collector");

        // Ignore if already finished or timed out
        if !matches!(self.state, CollectorState::Collecting) {
            info!(
                "ThresholdShareCollector is not collecting (state: {:?}), ignoring",
                match self.state {
                    CollectorState::Collecting => "Collecting",
                    CollectorState::Finished => "Finished",
                    CollectorState::TimedOut => "TimedOut",
                }
            );
            return;
        }

        let pid = msg.share.party_id;
        info!("ThresholdShareCollector party id: {}", pid);
        let Some(_) = self.todo.take(&pid) else {
            info!(
                "Error: {} was not in threshold share collector's ID list",
                pid
            );
            return;
        };
        info!("Inserting... waiting on: {}", self.todo.len());
        self.shares.insert(pid, msg.share);

        if self.todo.is_empty() {
            info!("We have received all threshold shares");
            self.state = CollectorState::Finished;

            // Cancel the timeout since we're done
            if let Some(handle) = self.timeout_handle.take() {
                ctx.cancel_future(handle);
            }

            let event: TypedEvent<AllThresholdSharesCollected> =
                TypedEvent::new(self.shares.clone().into(), ec);
            self.parent.do_send(event);
        }
        info!(
            "Finished processing ThresholdShareCreated in {:?}",
            start.elapsed()
        );
    }
}

impl Handler<ThresholdShareCollectionTimeout> for ThresholdShareCollector {
    type Result = ();
    fn handle(
        &mut self,
        _: ThresholdShareCollectionTimeout,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        // Only handle timeout if we're still collecting
        if !matches!(self.state, CollectorState::Collecting) {
            return;
        }

        warn!(
            e3_id = %self.e3_id,
            missing_parties = ?self.todo,
            "Threshold share collection timed out, {} parties missing",
            self.todo.len()
        );

        self.state = CollectorState::TimedOut;

        // Notify parent of failure
        let missing_parties: Vec<PartyId> = self.todo.iter().copied().collect();
        self.parent.do_send(ThresholdShareCollectionFailed {
            e3_id: self.e3_id.clone(),
            reason: format!(
                "Timeout waiting for threshold shares from {} parties",
                missing_parties.len()
            ),
            missing_parties,
        });

        // Stop the actor
        ctx.stop();
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use e3_events::{
    prelude::*, run_once, trap, BusHandle, CommitteeFinalizeRequested, CommitteeRequested,
    E3Failed, E3Stage, E3StageChanged, EType, EffectsEnabled, EnclaveEvent, EnclaveEventData,
    EventType, Shutdown, TypedEvent,
};
use e3_sortition::{GetLocalNodeSortitionRank, Sortition};
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

const FINALIZATION_BUFFER_SECONDS: u64 = 1;
const FINALIZATION_INTERVAL_SECONDS: u64 = 1;

/// CommitteeFinalizer is an actor that listens to CommitteeRequested events and dispatches
/// CommitteeFinalizeRequested events after the submission deadline has passed.
pub struct CommitteeFinalizer {
    bus: BusHandle,
    sortition: Addr<Sortition>,
    pending_committees: HashMap<String, SpawnHandle>,
}

impl CommitteeFinalizer {
    pub fn new(bus: &BusHandle, sortition: Addr<Sortition>) -> Self {
        Self {
            bus: bus.clone(),
            sortition,
            pending_committees: HashMap::new(),
        }
    }

    pub fn attach(bus: &BusHandle, sortition: Addr<Sortition>) -> Addr<Self> {
        let addr = CommitteeFinalizer::new(bus, sortition).start();

        // Subscribe to state-building / cleanup events immediately
        bus.subscribe_all(
            &[
                EventType::Shutdown,
                EventType::E3Failed,
                EventType::E3StageChanged,
            ],
            addr.clone().recipient(),
        );

        // Gate CommitteeRequested behind EffectsEnabled — finalization should not
        // be scheduled during historical event replay.
        bus.subscribe(
            EventType::EffectsEnabled,
            run_once::<EffectsEnabled>({
                let bus = bus.clone();
                let addr = addr.clone();
                move |_| {
                    bus.subscribe(EventType::CommitteeRequested, addr.into());
                    Ok(())
                }
            })
            .recipient(),
        );

        addr
    }
}

impl Actor for CommitteeFinalizer {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for CommitteeFinalizer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::CommitteeRequested(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::Shutdown(data) => self.notify_sync(ctx, data),
            EnclaveEventData::E3Failed(data) => self.notify_sync(ctx, TypedEvent::new(data, ec)),
            EnclaveEventData::E3StageChanged(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<CommitteeRequested>> for CommitteeFinalizer {
    type Result = ();

    // TODO: Remove all async from this function. Remove reliance on e3_evm package. Add unit test.
    fn handle(
        &mut self,
        msg: TypedEvent<CommitteeRequested>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let ec = msg.get_ctx().clone();
        let e3_id = msg.e3_id.clone();
        let committee_deadline = msg.committee_deadline;
        let local_rank_request = GetLocalNodeSortitionRank {
            e3_id: msg.e3_id.clone(),
            seed: msg.seed,
            threshold: msg.threshold,
            chain_id: msg.chain_id,
        };
        let sortition = self.sortition.clone();
        let e3_id_for_log = e3_id.clone();
        let fut = async move {
            // TODO: we should have no dependencies on e3_evm here. Reason being that this is core
            // functionality and evm is shell. Shell can depend on core but core MUST not depend on
            // shell. This means we should not hold an address to a shell actor even and should use
            // the eventbus to communicate.
            // see https://github.com/gnosisguild/enclave/issues/989
            let current_timestamp = match e3_evm::helpers::get_current_timestamp().await {
                Ok(timestamp) => timestamp,
                Err(e) => {
                    error!(
                        e3_id = %e3_id_for_log,
                        error = %e,
                        "Failed to get current timestamp from RPC"
                    );
                    return None;
                }
            };

            let local_rank = match sortition.send(local_rank_request).await {
                Ok(rank) => rank,
                Err(e) => {
                    error!(
                        e3_id = %e3_id_for_log,
                        error = %e,
                        "Failed to get local sortition rank for committee finalization"
                    );
                    return None;
                }
            };

            Some((current_timestamp, local_rank))
        };

        let e3_id_for_async = e3_id;
        ctx.spawn(
            fut.into_actor(self)
                .then(move |result, act, ctx| {
                    if let Some((current_timestamp, local_rank)) = result {
                        if let Some(rank) = local_rank {
                            let base_delay = if committee_deadline > current_timestamp {
                                (committee_deadline - current_timestamp)
                                    + FINALIZATION_BUFFER_SECONDS
                            } else {
                                info!(
                                    e3_id = %e3_id_for_async,
                                    committee_deadline = committee_deadline,
                                    current_timestamp = current_timestamp,
                                    "Submission deadline already passed, finalizing with fallback buffer"
                                );
                                FINALIZATION_BUFFER_SECONDS
                            };
                            let seconds_to_wait =
                                base_delay + (rank * FINALIZATION_INTERVAL_SECONDS);

                            info!(
                                e3_id = %e3_id_for_async,
                                committee_deadline = committee_deadline,
                                current_timestamp = current_timestamp,
                                rank = rank,
                                seconds_to_wait = seconds_to_wait,
                                "Scheduling committee finalization"
                            );

                            let bus = act.bus.clone();
                            let e3_id_clone = e3_id_for_async.clone();

                            let handle = ctx.run_later(
                                Duration::from_secs(seconds_to_wait),
                                move |act, _ctx| {
                                    info!(e3_id = %e3_id_clone, rank = rank, "Dispatching CommitteeFinalizeRequested event");

                                    trap(EType::Sortition, &act.bus.with_ec(&ec), || {
                                        bus.publish(CommitteeFinalizeRequested {
                                            e3_id: e3_id_clone.clone(),
                                        },ec)?;
                                        Ok(())
                                    });

                                    act.pending_committees.remove(&e3_id_clone.to_string());
                                },
                            );

                            if let Some(existing) = act
                                .pending_committees
                                .insert(e3_id_for_async.to_string(), handle)
                            {
                                ctx.cancel_future(existing);
                            }
                        } else {
                            info!(
                                e3_id = %e3_id_for_async,
                                "Node is outside the local pre-finalization committee ranking, skipping finalize scheduling"
                            );
                        }
                    } else {
                        error!(
                            e3_id = %e3_id_for_async,
                            "Skipping committee finalization due to timestamp or sortition lookup failure"
                        );
                    }

                    async {}.into_actor(act)
                }),
        );
    }
}

impl Handler<Shutdown> for CommitteeFinalizer {
    type Result = ();
    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        info!("Killing CommitteeFinalizer");
        // Cancel all pending finalization tasks
        for (_, handle) in self.pending_committees.drain() {
            ctx.cancel_future(handle);
        }
        ctx.stop();
    }
}

impl Handler<TypedEvent<E3Failed>> for CommitteeFinalizer {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<E3Failed>, ctx: &mut Self::Context) -> Self::Result {
        let e3_id_str = msg.e3_id.to_string();
        if let Some(handle) = self.pending_committees.remove(&e3_id_str) {
            info!(
                e3_id = %msg.e3_id,
                reason = ?msg.reason,
                "E3 failed — cancelling pending committee finalization timer"
            );
            ctx.cancel_future(handle);
        }
    }
}

impl Handler<TypedEvent<E3StageChanged>> for CommitteeFinalizer {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<E3StageChanged>, ctx: &mut Self::Context) -> Self::Result {
        match &msg.new_stage {
            E3Stage::Complete | E3Stage::Failed => {
                let e3_id_str = msg.e3_id.to_string();
                if let Some(handle) = self.pending_committees.remove(&e3_id_str) {
                    info!(
                        e3_id = %msg.e3_id,
                        stage = ?msg.new_stage,
                        "E3 reached terminal stage — cancelling pending committee finalization timer"
                    );
                    ctx.cancel_future(handle);
                }
            }
            _ => {}
        }
    }
}

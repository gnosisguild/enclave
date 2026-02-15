// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use e3_events::{
    prelude::*, trap, BusHandle, CommitteeFinalizeRequested, CommitteeRequested, EType,
    EnclaveEvent, EnclaveEventData, EventType, Shutdown, TypedEvent,
};
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

/// CommitteeFinalizer is an actor that listens to CommitteeRequested events and dispatches
/// CommitteeFinalizeRequested events after the submission deadline has passed.
pub struct CommitteeFinalizer {
    bus: BusHandle,
    pending_committees: HashMap<String, SpawnHandle>,
}

impl CommitteeFinalizer {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            pending_committees: HashMap::new(),
        }
    }

    pub fn attach(bus: &BusHandle) -> Addr<Self> {
        let addr = CommitteeFinalizer::new(bus).start();

        bus.subscribe_all(
            &[EventType::CommitteeRequested, EventType::Shutdown],
            addr.clone().recipient(),
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

        const FINALIZATION_BUFFER_SECONDS: u64 = 1;
        let e3_id_for_log = e3_id.clone();
        let fut = async move {
            // TODO: we should have no dependencies on e3_evm here. Reason being that this is core
            // functionality and evm is shell. Shell can depend on core but core MUST not depend on
            // shell. This means we should not hold an address to a shell actor even and should use
            // the eventbus to communicate.
            // see https://github.com/gnosisguild/enclave/issues/989
            match e3_evm::helpers::get_current_timestamp().await {
                Ok(timestamp) => Some(timestamp),
                Err(e) => {
                    error!(
                        e3_id = %e3_id_for_log,
                        error = %e,
                        "Failed to get current timestamp from RPC"
                    );
                    None
                }
            }
        };

        let e3_id_for_async = e3_id;
        ctx.spawn(
            fut.into_actor(self)
                .then(move |current_timestamp, act, ctx| {
                    if let Some(current_timestamp) = current_timestamp {
                        let seconds_until_deadline = if committee_deadline > current_timestamp {
                            (committee_deadline - current_timestamp) + FINALIZATION_BUFFER_SECONDS
                        } else {
                            info!(
                                e3_id = %e3_id_for_async,
                                committee_deadline = committee_deadline,
                                current_timestamp = current_timestamp,
                                "Submission deadline already passed, finalizing with buffer"
                            );
                            FINALIZATION_BUFFER_SECONDS
                        };

                        info!(
                            e3_id = %e3_id_for_async,
                            committee_deadline = committee_deadline,
                            current_timestamp = current_timestamp,
                            seconds_to_wait = seconds_until_deadline,
                            "Scheduling committee finalization"
                        );

                        let bus = act.bus.clone();
                        let e3_id_clone = e3_id_for_async.clone();

                        let handle = ctx.run_later(
                            Duration::from_secs(seconds_until_deadline),
                            move |act, _ctx| {
                                info!(e3_id = %e3_id_clone, "Dispatching CommitteeFinalizeRequested event");

                                trap(EType::Sortition, &act.bus.with_ec(&ec), || {
                                    bus.publish(CommitteeFinalizeRequested {
                                        e3_id: e3_id_clone.clone(),
                                    },ec)?;
                                    Ok(())
                                });

                                act.pending_committees.remove(&e3_id_clone.to_string());
                            },
                        );

                        act.pending_committees
                            .insert(e3_id_for_async.to_string(), handle);
                    } else {
                        error!(
                            e3_id = %e3_id_for_async,
                            "Skipping committee finalization due to timestamp fetch failure"
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

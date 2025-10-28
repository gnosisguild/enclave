// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use alloy::providers::Provider;
use e3_events::{CommitteeRequested, EnclaveEvent, EventBus, Shutdown, Subscribe};
use e3_evm::FinalizeCommittee;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info};

/// CommitteeFinalizer is an actor that listens to CommitteeRequested events and calls
/// finalizeCommittee on the registry after the submission deadline has passed.
pub struct CommitteeFinalizer<P: Provider + Clone + Unpin + 'static> {
    #[allow(dead_code)]
    bus: Addr<EventBus<EnclaveEvent>>,
    registry_writer: Recipient<FinalizeCommittee>,
    provider: P,
    pending_committees: HashMap<String, SpawnHandle>,
}

impl<P: Provider + Clone + Unpin + 'static> CommitteeFinalizer<P> {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        registry_writer: Recipient<FinalizeCommittee>,
        provider: P,
    ) -> Self {
        Self {
            bus: bus.clone(),
            registry_writer,
            provider,
            pending_committees: HashMap::new(),
        }
    }

    pub fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        registry_writer: Recipient<FinalizeCommittee>,
        provider: P,
    ) -> Addr<Self> {
        let addr = CommitteeFinalizer::new(bus, registry_writer, provider).start();

        bus.do_send(Subscribe::new(
            "CommitteeRequested",
            addr.clone().recipient(),
        ));
        bus.do_send(Subscribe::new("Shutdown", addr.clone().recipient()));

        addr
    }
}

impl<P: Provider + Clone + Unpin + 'static> Actor for CommitteeFinalizer<P> {
    type Context = Context<Self>;
}

impl<P: Provider + Clone + Unpin + 'static> Handler<EnclaveEvent> for CommitteeFinalizer<P> {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::CommitteeRequested { data, .. } => ctx.notify(data),
            EnclaveEvent::Shutdown { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl<P: Provider + Clone + Unpin + 'static> Handler<CommitteeRequested> for CommitteeFinalizer<P> {
    type Result = ();

    fn handle(&mut self, msg: CommitteeRequested, ctx: &mut Self::Context) -> Self::Result {
        let e3_id = msg.e3_id.clone();
        let submission_deadline = msg.submission_deadline;
        let provider = self.provider.clone();

        const FINALIZATION_BUFFER_SECONDS: u64 = 1;

        let e3_id_for_log = e3_id.clone();
        let fut = async move {
            match e3_evm::helpers::get_current_timestamp(&provider).await {
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
                        let seconds_until_deadline = if submission_deadline > current_timestamp {
                            (submission_deadline - current_timestamp) + FINALIZATION_BUFFER_SECONDS
                        } else {
                            info!(
                                e3_id = %e3_id_for_async,
                                submission_deadline = submission_deadline,
                                current_timestamp = current_timestamp,
                                "Submission deadline already passed, finalizing with buffer"
                            );
                            FINALIZATION_BUFFER_SECONDS
                        };

                        info!(
                            e3_id = %e3_id_for_async,
                            submission_deadline = submission_deadline,
                            current_timestamp = current_timestamp,
                            seconds_to_wait = seconds_until_deadline,
                            "Scheduling committee finalization"
                        );

                        let registry_writer = act.registry_writer.clone();
                        let e3_id_clone = e3_id_for_async.clone();

                        let handle = ctx.run_later(
                            Duration::from_secs(seconds_until_deadline),
                            move |act, _ctx| {
                                info!(e3_id = %e3_id_clone, "Calling finalizeCommittee");

                                registry_writer.do_send(FinalizeCommittee {
                                    e3_id: e3_id_clone.clone(),
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

impl<P: Provider + Clone + Unpin + 'static> Handler<Shutdown> for CommitteeFinalizer<P> {
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

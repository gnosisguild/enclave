// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_bfv_client::client::compute_pk_commitment;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, Die, E3id, EnclaveEvent, EnclaveEventData, EventContext,
    KeyshareCreated, OrderedSet, PublicKeyAggregated, Seed, Sequenced, TypedEvent,
};
use e3_events::{trap, EType};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_utils::NotifySync;
use e3_utils::{ArcBytes, MAILBOX_LIMIT};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_n: usize,
        keyshares: OrderedSet<ArcBytes>,
        seed: Seed,
        nodes: OrderedSet<String>,
    },
    Computing {
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
    },
    Complete {
        public_key: Vec<u8>,
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
    },
}

impl PublicKeyAggregatorState {
    pub fn init(threshold_n: usize, seed: Seed) -> Self {
        PublicKeyAggregatorState::Collecting {
            threshold_n,
            keyshares: OrderedSet::new(),
            seed,
            nodes: OrderedSet::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub keyshares: OrderedSet<ArcBytes>,
    pub e3_id: E3id,
}

pub struct PublicKeyAggregator {
    fhe: Arc<Fhe>,
    bus: BusHandle,
    e3_id: E3id,
    state: Persistable<PublicKeyAggregatorState>,
}

pub struct PublicKeyAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: BusHandle,
    pub e3_id: E3id,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id and aggregates the public key based on this and once done broadcasts
/// a EnclaveEvent::PublicKeyAggregated event on the event bus. Note events are hashed and
/// identical events will not be triggered twice.
/// It is expected to change this mechanism as we work through adversarial scenarios and write tests
/// for them.
impl PublicKeyAggregator {
    pub fn new(
        params: PublicKeyAggregatorParams,
        state: Persistable<PublicKeyAggregatorState>,
    ) -> Self {
        PublicKeyAggregator {
            fhe: params.fhe,
            bus: params.bus,
            e3_id: params.e3_id,
            state,
        }
    }

    pub fn add_keyshare(
        &mut self,
        keyshare: ArcBytes,
        node: String,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(&ec, |mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                keyshares,
                nodes,
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
            };

            keyshares.insert(keyshare);
            nodes.insert(node);
            info!(
                "PublicKeyAggregator got keyshares {}/{}",
                keyshares.len(),
                threshold_n
            );
            if keyshares.len() == *threshold_n {
                info!("Computing aggregate public key...");
                return Ok(PublicKeyAggregatorState::Computing {
                    keyshares: std::mem::take(keyshares),
                    nodes: std::mem::take(nodes),
                });
            }

            Ok(state)
        })
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>, ec: &EventContext<Sequenced>) -> Result<()> {
        self.state.try_mutate(ec, |mut state| {
            let PublicKeyAggregatorState::Computing { keyshares, nodes } = &mut state else {
                return Ok(state);
            };

            Ok(PublicKeyAggregatorState::Complete {
                public_key: pubkey,
                keyshares: std::mem::take(keyshares),
                nodes: std::mem::take(nodes),
            })
        })
    }

    pub fn handle_member_expelled(
        &mut self,
        node: &str,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(ec, |mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                keyshares,
                nodes,
                ..
            } = &mut state
            else {
                return Ok(state);
            };

            // Remove the expelled node from the nodes set so it won't appear in
            // PublicKeyAggregated.nodes (forwarded on-chain for reward distribution).
            // Note: the corresponding keyshare cannot be removed because the
            // keyshares OrderedSet is keyed by raw bytes with no node mapping.
            // This is acceptable because BFV public key aggregation is additive
            // and works correctly with any superset of valid keys.
            nodes.remove(&node.to_string());

            if *threshold_n > 0 {
                *threshold_n -= 1;
                info!(
                    "PublicKeyAggregator: reduced threshold_n to {} after expelling {}",
                    threshold_n, node
                );
            }

            if keyshares.len() == *threshold_n && *threshold_n > 0 {
                info!("PublicKeyAggregator: enough keyshares after expulsion, computing aggregate");
                return Ok(PublicKeyAggregatorState::Computing {
                    keyshares: std::mem::take(keyshares),
                    nodes: std::mem::take(nodes),
                });
            }

            Ok(state)
        })
    }
}

impl Actor for PublicKeyAggregator {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::KeyshareCreated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::E3RequestComplete(_) => self.notify_sync(ctx, Die),
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                let node_addr = data.node.to_string();

                if data.e3_id != self.e3_id {
                    error!("Wrong e3_id sent to PublicKeyAggregator for expulsion. This should not happen.");
                    return;
                }

                info!(
                    "PublicKeyAggregator: committee member expelled: {} for e3_id={}",
                    node_addr, data.e3_id
                );
                trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
                    let was_collecting = matches!(
                        self.state.get(),
                        Some(PublicKeyAggregatorState::Collecting { .. })
                    );

                    self.handle_member_expelled(&node_addr, &ec)?;

                    if was_collecting {
                        if let Some(PublicKeyAggregatorState::Computing { keyshares, .. }) =
                            &self.state.get()
                        {
                            self.notify_sync(
                                ctx,
                                TypedEvent::new(
                                    ComputeAggregate {
                                        keyshares: keyshares.clone(),
                                        e3_id: data.e3_id,
                                    },
                                    ec.clone(),
                                ),
                            );
                        }
                    }
                    Ok(())
                });
            }
            _ => (),
        };
    }
}

impl Handler<TypedEvent<KeyshareCreated>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        event: TypedEvent<KeyshareCreated>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (event, ec) = event.into_components();
        trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
            let e3_id = event.e3_id.clone();
            let pubkey = event.pubkey.clone();
            let node = event.node.clone();

            if e3_id != self.e3_id {
                error!("Wrong e3_id sent to aggregator. This should not happen.");
                return Ok(());
            }

            self.add_keyshare(pubkey, node, &ec)?;

            if let Some(PublicKeyAggregatorState::Computing { keyshares, .. }) = &self.state.get() {
                self.notify_sync(
                    ctx,
                    TypedEvent::new(
                        ComputeAggregate {
                            keyshares: keyshares.clone(),
                            e3_id,
                        },
                        ec,
                    ),
                )
            }

            Ok(())
        })
    }
}

impl Handler<TypedEvent<ComputeAggregate>> for PublicKeyAggregator {
    type Result = ();

    fn handle(&mut self, msg: TypedEvent<ComputeAggregate>, _: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
            info!("Computing Aggregate PublicKey...");
            let pubkey = self.fhe.get_aggregate_public_key(GetAggregatePublicKey {
                keyshares: msg.keyshares,
            })?;

            let public_key_hash = compute_pk_commitment(
                pubkey.clone(),
                self.fhe.params.degree(),
                self.fhe.params.plaintext(),
                self.fhe.params.moduli().to_vec(),
            )?;

            self.set_pubkey(pubkey, &ec)?;

            if let Some(PublicKeyAggregatorState::Complete {
                public_key: pubkey,
                nodes,
                ..
            }) = self.state.get()
            {
                info!("Notifying network of PublicKey");
                info!("Sending PublicKeyAggregated...");
                let event = PublicKeyAggregated {
                    pubkey,
                    public_key_hash,
                    e3_id: msg.e3_id,
                    nodes,
                };
                self.bus.publish(event, ec)?;
            }
            Ok(())
        })
    }
}

impl Handler<Die> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

use crate::{
    eventbus::EventBus,
    events::{E3id, EnclaveEvent, KeyshareCreated, PublicKeyAggregated},
    fhe::{Fhe, GetAggregatePublicKey},
    ordered_set::OrderedSet,
    ActorFactory, GetHasNode, Sortition,
};
use actix::prelude::*;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum PublicKeyAggregatorState {
    Collecting {
        nodecount: usize,
        keyshares: OrderedSet<Vec<u8>>,
        seed: u64,
    },
    Computing {
        keyshares: OrderedSet<Vec<u8>>,
    },
    Complete {
        public_key: Vec<u8>,
        keyshares: OrderedSet<Vec<u8>>,
    },
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub keyshares: OrderedSet<Vec<u8>>,
}

pub struct PublicKeyAggregator {
    fhe: Addr<Fhe>,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: PublicKeyAggregatorState,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id and aggregates the public key based on this and once done broadcasts
/// a EnclaveEvent::PublicKeyAggregated event on the event bus. Note events are hashed and
/// identical events will not be triggered twice.
/// It is expected to change this mechanism as we work through adversarial scenarios and write tests
/// for them.
impl PublicKeyAggregator {
    pub fn new(
        fhe: Addr<Fhe>,
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        e3_id: E3id,
        nodecount: usize,
        seed: u64,
    ) -> Self {
        PublicKeyAggregator {
            fhe,
            bus,
            e3_id,
            sortition,
            state: PublicKeyAggregatorState::Collecting {
                nodecount,
                keyshares: OrderedSet::new(),
                seed,
            },
        }
    }

    pub fn add_keyshare(&mut self, keyshare: Vec<u8>) -> Result<PublicKeyAggregatorState> {
        let PublicKeyAggregatorState::Collecting {
            nodecount,
            keyshares,
            ..
        } = &mut self.state
        else {
            return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
        };

        keyshares.insert(keyshare);
        if keyshares.len() == *nodecount {
            return Ok(PublicKeyAggregatorState::Computing {
                keyshares: keyshares.clone(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>) -> Result<PublicKeyAggregatorState> {
        let PublicKeyAggregatorState::Computing { keyshares } = &mut self.state else {
            return Ok(self.state.clone());
        };

        let keyshares = keyshares.to_owned();

        Ok(PublicKeyAggregatorState::Complete {
            public_key: pubkey,
            keyshares,
        })
    }
}

impl Actor for PublicKeyAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::KeyshareCreated { data, .. } = msg {
            ctx.notify(data)
        }
    }
}

impl Handler<KeyshareCreated> for PublicKeyAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: KeyshareCreated, _: &mut Self::Context) -> Self::Result {
        let PublicKeyAggregatorState::Collecting {
            nodecount, seed, ..
        } = self.state.clone()
        else {
            println!("Aggregator has been closed for collecting keyshares."); // TODO: log properly

            return Box::pin(fut::ready(Ok(())));
        };

        let size = nodecount;
        let address = event.node;
        let e3_id = event.e3_id.clone();
        let pubkey = event.pubkey.clone();

        Box::pin(
            self.sortition
                .send(GetHasNode {
                    address,
                    size,
                    seed,
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
                    // NOTE: Returning Ok(()) on errors as we probably dont need a result type here since
                    // we will not be doing a send
                    let has_node = res?;
                    if !has_node {
                        println!("Node not found in committee"); // TODO: log properly
                        return Ok(());
                    }

                    if e3_id != act.e3_id {
                        println!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // add the keyshare and
                    act.state = act.add_keyshare(pubkey)?;

                    // Check the state and if it has changed to the computing
                    if let PublicKeyAggregatorState::Computing { keyshares } = &act.state {
                        ctx.notify(ComputeAggregate {
                            keyshares: keyshares.clone(),
                        })
                    }

                    Ok(())
                }),
        )
    }
}

impl Handler<ComputeAggregate> for PublicKeyAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, msg: ComputeAggregate, _: &mut Self::Context) -> Self::Result {
        // Futures are awkward in Actix from what I can tell we should try and structure events so
        // that futures that don't require access to self like the following...
        Box::pin(
            // Run the async future.
            self.fhe
                .send(GetAggregatePublicKey {
                    keyshares: msg.keyshares.clone(),
                })
                // allow access to the actor
                .into_actor(self)
                // map into some sync stuff
                .map(|res, act, _| {
                    // We have to double unwrap here. Suggestions?
                    // 1st - Mailbox error.
                    // 2nd - GetAggregatePublicKey Response.
                    let pubkey = res??;

                    // Update the local state
                    act.state = act.set_pubkey(pubkey.clone())?;

                    // Dispatch the PublicKeyAggregated event
                    let event = EnclaveEvent::from(PublicKeyAggregated {
                        pubkey,
                        e3_id: act.e3_id.clone(),
                    });

                    act.bus.do_send(event);

                    // Return
                    Ok(())
                }),
        )
    }
}

pub struct PublicKeyAggregatorFactory;
impl PublicKeyAggregatorFactory {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> ActorFactory {
        Box::new(move |ctx, evt| {
            // Saving the publickey aggregator with deps on CommitteeRequested
            let EnclaveEvent::CommitteeRequested { data, .. } = evt else {
                return;
            };

            let Some(ref fhe) = ctx.fhe else {
                return;
            };
            let Some(ref meta) = ctx.meta else {
                return;
            };

            ctx.publickey = Some(
                PublicKeyAggregator::new(
                    fhe.clone(),
                    bus.clone(),
                    sortition.clone(),
                    data.e3_id,
                    meta.nodecount,
                    meta.seed,
                )
                .start(),
            );
        })
    }
}

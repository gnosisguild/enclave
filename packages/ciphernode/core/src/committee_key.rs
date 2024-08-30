use crate::{
    eventbus::EventBus,
    events::{E3id, EnclaveEvent, KeyshareCreated, PublicKeyAggregated},
    fhe::{Fhe, GetAggregatePublicKey}, ordered_set::OrderedSet, 
};
use actix::prelude::*;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub enum CommitteeKeyState {
    Collecting {
        nodecount: usize,
        keyshares: OrderedSet<Vec<u8>>,
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct Die;

pub struct CommitteeKey {
    fhe: Addr<Fhe>,
    bus: Addr<EventBus>,
    e3_id: E3id,
    state: CommitteeKeyState,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id and aggregates the public key based on this and once done broadcasts 
/// a EnclaveEvent::PublicKeyAggregated event on the event bus. Note events are hashed and
/// identical events will not be triggered twice.
/// It is expected to change this mechanism as we work through adversarial scenarios and write tests
/// for them.
impl CommitteeKey {
    pub fn new(fhe: Addr<Fhe>, bus: Addr<EventBus>, e3_id: E3id, nodecount: usize) -> Self {
        CommitteeKey {
            fhe,
            bus,
            e3_id,
            state: CommitteeKeyState::Collecting {
                nodecount,
                keyshares: OrderedSet::new(),
            },
        }
    }

    pub fn add_keyshare(&mut self, keyshare: Vec<u8>) -> Result<CommitteeKeyState> {
        let CommitteeKeyState::Collecting {
            nodecount,
            keyshares,
        } = &mut self.state
        else {
            return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
        };

        keyshares.insert(keyshare);
        if keyshares.len() == *nodecount {
            return Ok(CommitteeKeyState::Computing {
                keyshares: keyshares.clone(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>) -> Result<CommitteeKeyState> {
        let CommitteeKeyState::Computing { keyshares } = &mut self.state else {
            return Ok(self.state.clone());
        };

        let keyshares = keyshares.to_owned();

        Ok(CommitteeKeyState::Complete {
            public_key: pubkey,
            keyshares,
        })
    }
}

impl Actor for CommitteeKey {
    type Context = Context<Self>;
}

impl Handler<KeyshareCreated> for CommitteeKey {
    type Result = Result<()>;

    fn handle(&mut self, event: KeyshareCreated, ctx: &mut Self::Context) -> Self::Result {
        if event.e3_id != self.e3_id {
            return Err(anyhow!(
                "Wrong e3_id sent to aggregator. This should not happen."
            ));
        }

        let CommitteeKeyState::Collecting { .. } = self.state else {
            return Err(anyhow!(
                "Aggregator has been closed for collecting keyshares."
            ));
        };

        // add the keyshare and
        self.state = self.add_keyshare(event.pubkey)?;

        // Check the state and if it has changed to the computing
        if let CommitteeKeyState::Computing { keyshares } = &self.state {
            ctx.address().do_send(ComputeAggregate {
                keyshares: keyshares.clone(),
            })
        }

        Ok(())
    }
}

impl Handler<ComputeAggregate> for CommitteeKey {
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

impl Handler<Die> for CommitteeKey {
    type Result = ();

    fn handle(&mut self, _msg: Die, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

use crate::{
    ordered_set::OrderedSet, DecryptedOutputPublished, DecryptionshareCreated, Die, E3id, EnclaveEvent, EventBus, Fhe, GetAggregatePlaintext
};
use actix::prelude::*;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub enum DecryptionState {
    Collecting {
        nodecount: usize,
        shares: OrderedSet<Vec<u8>>,
    },
    Computing {
        shares: OrderedSet<Vec<u8>>,
    },
    Complete {
        decrypted: Vec<u8>,
        shares: OrderedSet<Vec<u8>>,
    },
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub shares: OrderedSet<Vec<u8>>,
}

pub struct Decryption {
    fhe: Addr<Fhe>,
    bus: Addr<EventBus>,
    e3_id: E3id,
    state: DecryptionState,
}

impl Decryption {
    pub fn new(fhe: Addr<Fhe>, bus: Addr<EventBus>, e3_id: E3id, nodecount: usize) -> Self {
        Decryption {
            fhe,
            bus,
            e3_id,
            state: DecryptionState::Collecting {
                nodecount,
                shares: OrderedSet::new(),
            },
        }
    }

    pub fn add_share(&mut self, share: Vec<u8>) -> Result<DecryptionState> {
        let DecryptionState::Collecting { nodecount, shares } = &mut self.state else {
            return Err(anyhow::anyhow!("Can only add share in Collecting state"));
        };

        shares.insert(share);
        if shares.len() == *nodecount {
            return Ok(DecryptionState::Computing {
                shares: shares.clone(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_decryption(&mut self, decrypted: Vec<u8>) -> Result<DecryptionState> {
        let DecryptionState::Computing { shares } = &mut self.state else {
            return Ok(self.state.clone());
        };

        let shares = shares.to_owned();

        Ok(DecryptionState::Complete { decrypted, shares })
    }
}

impl Actor for Decryption {
    type Context = Context<Self>;
}

impl Handler<DecryptionshareCreated> for Decryption {
    type Result = Result<()>;
    fn handle(&mut self, event: DecryptionshareCreated, ctx: &mut Self::Context) -> Self::Result {
        if event.e3_id != self.e3_id {
            return Err(anyhow!(
                "Wrong e3_id sent to aggregator. This should not happen."
            ));
        }
        let DecryptionState::Collecting { .. } = self.state else {
            return Err(anyhow!(
                "Aggregator has been closed for collecting keyshares."
            ));
        };

        // add the keyshare and
        self.state = self.add_share(event.decryption_share)?;

        // Check the state and if it has changed to the computing
        if let DecryptionState::Computing { shares } = &self.state {
            ctx.address().do_send(ComputeAggregate {
                shares: shares.clone(),
            })
        }

        Ok(())
    }
}

impl Handler<ComputeAggregate> for Decryption {
    type Result = ResponseActFuture<Self, Result<()>>;
    fn handle(&mut self, msg: ComputeAggregate, ctx: &mut Self::Context) -> Self::Result {
        Box::pin(
            self.fhe
                .send(GetAggregatePlaintext {
                    decryptions: msg.shares.clone(),
                })
                .into_actor(self)
                .map(|res, act, _| {
                    let decrypted_output = res??;
                    // Update the local state
                    act.state = act.set_decryption(decrypted_output.clone())?;

                    // Dispatch the PublicKeyAggregated event
                    let event = EnclaveEvent::from(DecryptedOutputPublished {
                        decrypted_output,
                        e3_id: act.e3_id.clone(),
                    });

                    act.bus.do_send(event);

                    Ok(())
                }),
        )
    }
}

impl Handler<Die> for Decryption {
    type Result = ();

    fn handle(&mut self, _msg: Die, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

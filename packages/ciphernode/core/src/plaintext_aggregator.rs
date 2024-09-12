use crate::{
    ordered_set::OrderedSet, PlaintextAggregated, DecryptionshareCreated, Die, E3id, EnclaveEvent, EventBus, Fhe, GetAggregatePlaintext
};
use actix::prelude::*;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub enum PlaintextAggregatorState {
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

pub struct PlaintextAggregator {
    fhe: Addr<Fhe>,
    bus: Addr<EventBus>,
    e3_id: E3id,
    state: PlaintextAggregatorState,
}

impl PlaintextAggregator {
    pub fn new(fhe: Addr<Fhe>, bus: Addr<EventBus>, e3_id: E3id, nodecount: usize) -> Self {
        PlaintextAggregator {
            fhe,
            bus,
            e3_id,
            state: PlaintextAggregatorState::Collecting {
                nodecount,
                shares: OrderedSet::new(),
            },
        }
    }

    pub fn add_share(&mut self, share: Vec<u8>) -> Result<PlaintextAggregatorState> {
        let PlaintextAggregatorState::Collecting { nodecount, shares } = &mut self.state else {
            return Err(anyhow::anyhow!("Can only add share in Collecting state"));
        };

        shares.insert(share);
        if shares.len() == *nodecount {
            return Ok(PlaintextAggregatorState::Computing {
                shares: shares.clone(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_decryption(&mut self, decrypted: Vec<u8>) -> Result<PlaintextAggregatorState> {
        let PlaintextAggregatorState::Computing { shares } = &mut self.state else {
            return Ok(self.state.clone());
        };

        let shares = shares.to_owned();

        Ok(PlaintextAggregatorState::Complete { decrypted, shares })
    }
}

impl Actor for PlaintextAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::DecryptionshareCreated { data, .. } => ctx.notify(data),
            _ => ()
        }
    }
}

impl Handler<DecryptionshareCreated> for PlaintextAggregator {
    type Result = Result<()>;
    fn handle(&mut self, event: DecryptionshareCreated, ctx: &mut Self::Context) -> Self::Result {
        if event.e3_id != self.e3_id {
            return Err(anyhow!(
                "Wrong e3_id sent to aggregator. This should not happen."
            ));
        }
        let PlaintextAggregatorState::Collecting { .. } = self.state else {
            return Err(anyhow!(
                "Aggregator has been closed for collecting keyshares."
            ));
        };

        // add the keyshare and
        self.state = self.add_share(event.decryption_share)?;

        // Check the state and if it has changed to the computing
        if let PlaintextAggregatorState::Computing { shares } = &self.state {
            ctx.notify(ComputeAggregate {
                shares: shares.clone(),
            })
        }

        Ok(())
    }
}

impl Handler<ComputeAggregate> for PlaintextAggregator {
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
                    let event = EnclaveEvent::from(PlaintextAggregated {
                        decrypted_output,
                        e3_id: act.e3_id.clone(),
                    });
                    act.bus.do_send(event);

                    Ok(())
                }),
        )
    }
}

impl Handler<Die> for PlaintextAggregator {
    type Result = ();

    fn handle(&mut self, _msg: Die, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

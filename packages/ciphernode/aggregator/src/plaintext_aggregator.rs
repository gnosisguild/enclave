use actix::prelude::*;
use anyhow::Result;
use enclave_core::{
    DecryptionshareCreated, E3id, EnclaveEvent, EventBus, OrderedSet, PlaintextAggregated, Seed,
};
use fhe::{Fhe, GetAggregatePlaintext};
use sortition::{GetHasNode, Sortition};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum PlaintextAggregatorState {
    Collecting {
        threshold_m: usize,
        shares: OrderedSet<Vec<u8>>,
        seed: Seed,
        ciphertext_output: Vec<u8>,
    },
    Computing {
        shares: OrderedSet<Vec<u8>>,
        ciphertext_output: Vec<u8>,
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
    pub ciphertext_output: Vec<u8>,
}

pub struct PlaintextAggregator {
    fhe: Arc<Fhe>,
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: PlaintextAggregatorState,
}

impl PlaintextAggregator {
    pub fn new(
        fhe: Arc<Fhe>,
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        e3_id: E3id,
        threshold_m: usize,
        seed: Seed,
        ciphertext_output: Vec<u8>,
    ) -> Self {
        PlaintextAggregator {
            fhe,
            bus,
            sortition,
            e3_id,
            state: PlaintextAggregatorState::Collecting {
                threshold_m,
                shares: OrderedSet::new(),
                seed,
                ciphertext_output,
            },
        }
    }

    pub fn add_share(&mut self, share: Vec<u8>) -> Result<PlaintextAggregatorState> {
        let PlaintextAggregatorState::Collecting {
            threshold_m,
            shares,
            ciphertext_output,
            ..
        } = &mut self.state
        else {
            return Err(anyhow::anyhow!("Can only add share in Collecting state"));
        };

        shares.insert(share);
        if shares.len() == *threshold_m {
            return Ok(PlaintextAggregatorState::Computing {
                shares: shares.clone(),
                ciphertext_output: ciphertext_output.to_vec(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_decryption(&mut self, decrypted: Vec<u8>) -> Result<PlaintextAggregatorState> {
        let PlaintextAggregatorState::Computing { shares, .. } = &mut self.state else {
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
        if let EnclaveEvent::DecryptionshareCreated { data, .. } = msg {
            ctx.notify(data)
        }
    }
}

impl Handler<DecryptionshareCreated> for PlaintextAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: DecryptionshareCreated, _: &mut Self::Context) -> Self::Result {
        let PlaintextAggregatorState::Collecting {
            threshold_m, seed, ..
        } = self.state
        else {
            println!("Aggregator has been closed for collecting.");
            return Box::pin(fut::ready(Ok(())));
        };

        let size = threshold_m;
        let address = event.node;
        let e3_id = event.e3_id.clone();
        let decryption_share = event.decryption_share.clone();

        Box::pin(
            self.sortition
                .send(GetHasNode {
                    address,
                    size,
                    seed,
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
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
                    act.state = act.add_share(decryption_share)?;

                    // Check the state and if it has changed to the computing
                    if let PlaintextAggregatorState::Computing {
                        shares,
                        ciphertext_output,
                    } = &act.state
                    {
                        ctx.notify(ComputeAggregate {
                            shares: shares.clone(),
                            ciphertext_output: ciphertext_output.to_vec(),
                        })
                    }

                    Ok(())
                }),
        )
    }
}

impl Handler<ComputeAggregate> for PlaintextAggregator {
    type Result = Result<()>;
    fn handle(&mut self, msg: ComputeAggregate, _: &mut Self::Context) -> Self::Result {
        let decrypted_output = self.fhe.get_aggregate_plaintext(GetAggregatePlaintext {
            decryptions: msg.shares.clone(),
            ciphertext_output: msg.ciphertext_output,
        })?;

        // Update the local state
        self.state = self.set_decryption(decrypted_output.clone())?;

        // Dispatch the PlaintextAggregated event
        let event = EnclaveEvent::from(PlaintextAggregated {
            decrypted_output,
            e3_id: self.e3_id.clone(),
        });

        self.bus.do_send(event);

        Ok(())
    }
}

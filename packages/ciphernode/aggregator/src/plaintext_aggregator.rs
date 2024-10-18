use actix::prelude::*;
use anyhow::Result;
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{
    DecryptionshareCreated, Die, E3id, EnclaveEvent, EventBus, OrderedSet, PlaintextAggregated,
    Seed,
};
use fhe::{Fhe, GetAggregatePlaintext};
use sortition::{GetHasNode, Sortition};
use std::sync::Arc;
use tracing::error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

impl PlaintextAggregatorState {
    pub fn init(threshold_m: usize, seed: Seed, ciphertext_output: Vec<u8>) -> Self {
        PlaintextAggregatorState::Collecting {
            threshold_m,
            shares: OrderedSet::new(),
            seed,
            ciphertext_output,
        }
    }
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
    store: Repository<PlaintextAggregatorState>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: PlaintextAggregatorState,
    src_chain_id: u64,
}

pub struct PlaintextAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: Addr<EventBus>,
    pub store: Repository<PlaintextAggregatorState>,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
    pub src_chain_id: u64,
}

impl PlaintextAggregator {
    pub fn new(params: PlaintextAggregatorParams, state: PlaintextAggregatorState) -> Self {
        PlaintextAggregator {
            fhe: params.fhe,
            bus: params.bus,
            store: params.store,
            sortition: params.sortition,
            e3_id: params.e3_id,
            src_chain_id: params.src_chain_id,
            state,
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
        match msg {
            EnclaveEvent::DecryptionshareCreated { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            _ => (),
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
            error!(state=?self.state, "Aggregator has been closed for collecting.");
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
                        error!("Node not found in committee");
                        return Ok(());
                    }

                    if e3_id != act.e3_id {
                        error!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // add the keyshare and
                    act.state = act.add_share(decryption_share)?;
                    act.checkpoint();

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
        self.checkpoint();

        // Dispatch the PlaintextAggregated event
        let event = EnclaveEvent::from(PlaintextAggregated {
            decrypted_output,
            e3_id: self.e3_id.clone(),
            src_chain_id: self.src_chain_id,
        });

        self.bus.do_send(event);

        Ok(())
    }
}

impl Handler<Die> for PlaintextAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}

impl Snapshot for PlaintextAggregator {
    type Snapshot = PlaintextAggregatorState;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }
}

#[async_trait]
impl FromSnapshotWithParams for PlaintextAggregator {
    type Params = PlaintextAggregatorParams;

    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        Ok(PlaintextAggregator::new(params, snapshot))
    }
}

impl Checkpoint for PlaintextAggregator {
    fn repository(&self) -> Repository<PlaintextAggregatorState> {
        self.store.clone()
    }
}

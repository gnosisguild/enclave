use alloy::{
    primitives::{Address, B256, Uint},
    providers::{Provider, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter, Log},
    sol_types::SolEvent,
    transports::BoxTransport,
    sol,
};
use eyre::Result;
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::marker::PhantomData;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ICiphernodeRegistry,
    "src/ABI/ICiphernodeRegistry.json"
);

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitteeRequestedEvent {
    e3Id: Uint<256, 4>,
    filter: Address,
    threshold: [u32; 2],
}

#[derive(Clone)]
pub struct EventListener {
    provider: Arc<RootProvider<BoxTransport>>,
    address: Address,
    evt_tx: Sender<Vec<u8>>,
}

impl EventListener {
    pub fn new(
        provider: Arc<RootProvider<BoxTransport>>,
        address: Address,
        sender: Sender<Vec<u8>>,
    ) -> Self {
        Self {
            provider,
            address,
            evt_tx: sender,
        }
    }

    pub async fn listen(&self) -> Result<()> {
        let filter = Filter::new()
            // By NOT specifying an `event` or `event_signature` we listen to ALL events of the
            // contract.
            .address(self.address)
            .from_block(BlockNumberOrTag::Latest);

        // Subscribe to logs.
        let sub = self.provider.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        while let Some(log) = stream.next().await {
            // Match on topic 0, the hash of the signature of the event.
            match log.topic0() {
                // Match the `Approval(address,address,uint256)` event.
                Some(&ICiphernodeRegistry::CommitteeRequested::SIGNATURE_HASH) => {
                    let ICiphernodeRegistry::CommitteeRequested { e3Id, filter, threshold } = log.log_decode()?.inner.data;
                    println!("CommitteeRequested with ID {e3Id} filter {filter} thresold {:?}", threshold);
                    let cevent = CommitteeRequestedEvent {
                        e3Id,
                        filter,
                        threshold
                    };
                    let msg_str = serde_json::to_string(&cevent).unwrap();
                    let msg_bytes = msg_str.into_bytes();
                    self.evt_tx.send(msg_bytes).await?;
                }
                // Match the `Transfer(address,address,uint256)` event.
                Some(&ICiphernodeRegistry::CiphernodeAdded::SIGNATURE_HASH) => {
                    let ICiphernodeRegistry::CiphernodeAdded { node, index, numNodes, size } = log.log_decode()?.inner.data;
                    println!("CiphernodeAdded node {node} index {index} numNodes {numNodes} size {size}");
                }
                // WETH9's `Deposit(address,uint256)` and `Withdrawal(address,uint256)` events are not
                // handled here.
                _ => (),
            }
        }

        Ok(())
    }
}
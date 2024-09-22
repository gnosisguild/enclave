use crate::EventListener;
use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter},
    transports::BoxTransport,
};
use eyre::Result;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct ContractManager {
    provider: Arc<RootProvider<BoxTransport>>,
    listeners: Vec<EventListener>,
    evt_tx: Sender<Vec<u8>>,
    cmd_rx: Receiver<Vec<u8>>
}

impl ContractManager {
    pub async fn new(rpc_url: &str, ) -> Result<(Self, Sender<Vec<u8>>, Receiver<Vec<u8>>), Box<dyn Error>> {
        let (evt_tx, evt_rx) = channel(100); // TODO : tune this param
        let (cmd_tx, cmd_rx) = channel(100); // TODO : tune this param
        let provider = ProviderBuilder::new().on_builtin(rpc_url).await?;
        Ok((Self {
            provider: Arc::new(provider),
            listeners: vec![],
            evt_tx,
            cmd_rx
        },
            cmd_tx,
            evt_rx
        ))
    }

    pub fn add_listener(&mut self, contract_address: Address) {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);
        let listener = EventListener::new(self.provider.clone(), filter, self.evt_tx.clone());
        self.listeners.push(listener);
    }
}

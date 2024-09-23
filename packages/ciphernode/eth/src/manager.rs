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
    pub listeners: Vec<EventListener>,
    pub evt_tx: Sender<Vec<u8>>,
    pub cmd_rx: Receiver<Vec<u8>>
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

    pub fn add_listener(&mut self, contract_address: Address) -> EventListener {
        let listener = EventListener::new(self.provider.clone(), contract_address, self.evt_tx.clone());
        self.listeners.push(listener.clone());
        listener
    }
}

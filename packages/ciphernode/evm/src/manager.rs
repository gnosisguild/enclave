use actix::{Actor, Addr, Context, Handler, Message};
use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter},
    transports::BoxTransport,
};
use anyhow::Result;
use enclave_core::EventBus;
use std::sync::Arc;

use super::{EvmEventListener, StartListening};

pub struct EvmContractManager {
    bus: Addr<EventBus>,
    provider: Arc<RootProvider<BoxTransport>>,
    listeners: Vec<Addr<EvmEventListener>>,
}

impl EvmContractManager {
    async fn new(bus: Addr<EventBus>, rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new().on_builtin(rpc_url).await?;
        Ok(Self {
            bus,
            provider: Arc::new(provider),
            listeners: vec![],
        })
    }

    pub async fn attach(bus: Addr<EventBus>, rpc_url: &str) -> Addr<Self> {
        EvmContractManager::new(bus.clone(), rpc_url)
            .await
            .unwrap()
            .start()
    }

    fn add_listener(&self, contract_address: Address) -> Addr<EvmEventListener> {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);
        let listener = EvmEventListener::new(self.provider.clone(), filter, self.bus.clone());
        listener.start()
    }
}

impl Actor for EvmContractManager {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Addr<EvmEventListener>")]
pub struct AddListener {
    pub contract_address: Address,
}

impl Handler<AddListener> for EvmContractManager {
    type Result = Addr<EvmEventListener>;

    fn handle(&mut self, msg: AddListener, _ctx: &mut Self::Context) -> Self::Result {
        let listener = self.add_listener(msg.contract_address);
        self.listeners.push(listener.clone());
        listener
    }
}

impl Handler<StartListening> for EvmContractManager {
    type Result = ();

    fn handle(&mut self, _: StartListening, _ctx: &mut Self::Context) -> Self::Result {
        for listener in &self.listeners {
            listener.do_send(StartListening);
        }
    }
}

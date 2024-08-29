use crate::{EventListener, StartListening};
use actix::{Actor, Addr, Context, Handler, Message};
use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter},
    transports::BoxTransport,
};
use enclave_core::EventBus;
use eyre::Result;
use std::sync::Arc;

pub struct ContractManager {
    bus: Addr<EventBus>,
    provider: Arc<RootProvider<BoxTransport>>,
    listeners: Vec<Addr<EventListener>>,
}

impl ContractManager {
    async fn new(bus: Addr<EventBus>, rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new().on_builtin(rpc_url).await?;
        Ok(Self {
            bus,
            provider: Arc::new(provider),
            listeners: vec![],
        })
    }

    pub async fn attach(bus: Addr<EventBus>, rpc_url: &str) -> Addr<Self> {
        let addr = ContractManager::new(bus.clone(), rpc_url).await.unwrap().start();
        addr
    }

    fn add_listener(&self, contract_address: Address) -> Addr<EventListener> {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);
        let listener = EventListener::new(self.provider.clone(), filter, self.bus.clone());
        let addr = listener.start();
        addr
    }
}

impl Actor for ContractManager {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Addr<EventListener>")]
pub struct AddListener {
    pub contract_address: Address,
}

impl Handler<AddListener> for ContractManager {
    type Result = Addr<EventListener>;

    fn handle(&mut self, msg: AddListener, _ctx: &mut Self::Context) -> Self::Result {
        let listener = self.add_listener(msg.contract_address);
        self.listeners.push(listener.clone());
        listener
    }
}

impl Handler<StartListening> for ContractManager {
    type Result = ();

    fn handle(&mut self, _: StartListening, _ctx: &mut Self::Context) -> Self::Result {
        for listener in &self.listeners {
            listener.do_send(StartListening);
        }
    }
}
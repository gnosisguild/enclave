use alloy::{
    primitives::Address,
    providers::{ProviderBuilder, RootProvider},
    rpc::types::{BlockNumberOrTag, Filter},
    transports::BoxTransport,
};
use eyre::Result;
use std::sync::Arc;
use crate::EventListener;

pub struct ContractManager {
    provider: Arc<RootProvider<BoxTransport>>,
}

impl ContractManager {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = ProviderBuilder::new().on_builtin(rpc_url).await?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub fn add_listener(&self, contract_address: Address) -> EventListener {
        let filter = Filter::new()
            .address(contract_address)
            .from_block(BlockNumberOrTag::Latest);

        EventListener::new(self.provider.clone(), filter)
    }
}
use crate::helpers::{create_signer, Signer};
use actix::prelude::*;
use actix::Addr;
use alloy::{primitives::Address, sol};
use alloy::{
    primitives::{Bytes, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::Result;
use enclave_core::{BusError, E3id, EnclaveErrorType, PlaintextAggregated, Subscribe};
use enclave_core::{EnclaveEvent, EventBus};
use std::env;

sol! {
    #[derive(Debug)]
    #[sol(rpc)]
    contract Enclave {
        function publishPlaintextOutput(uint256 e3Id, bytes memory plaintextOutput, bytes memory proof) external returns (bool success);
    }
}

/// Consumes events from the event bus and calls EVM methods on the Enclave.sol contract
pub struct EnclaveSolWriter {
    provider: Signer,
    contract_address: Address,
    bus: Addr<EventBus>,
}

impl EnclaveSolWriter {
    pub async fn new(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Self> {
        Ok(Self {
            provider: create_signer(rpc_url, env::var("PRIVATE_KEY")?).await?,
            contract_address,
            bus,
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: Address,
    ) -> Result<Addr<EnclaveSolWriter>> {
        let addr = EnclaveSolWriter::new(bus.clone(), rpc_url, contract_address)
            .await?
            .start();
        let _ = bus
            .send(Subscribe::new("PlaintextAggregated", addr.clone().into()))
            .await;

        Ok(addr)
    }
}

impl Actor for EnclaveSolWriter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EnclaveSolWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PlaintextAggregated { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<PlaintextAggregated> for EnclaveSolWriter {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: PlaintextAggregated, _: &mut Self::Context) -> Self::Result {
        Box::pin({
            let e3_id = msg.e3_id.clone();
            let decrypted_output = msg.decrypted_output.clone();
            let contract_address = self.contract_address.clone();
            let provider = self.provider.clone();
            let bus = self.bus.clone();

            async move {
                let result =
                    publish_plaintext_output(provider, contract_address, e3_id, decrypted_output)
                        .await;
                match result {
                    Ok(receipt) => {
                        println!("tx:{}", receipt.transaction_hash)
                    }
                    Err(err) => bus.err(EnclaveErrorType::Evm, err),
                }
            }
        })
    }
}

async fn publish_plaintext_output(
    provider: Signer,
    contract_address: Address,
    e3_id: E3id,
    decrypted_output: Vec<u8>,
) -> Result<TransactionReceipt> {
    let e3_id: U256 = e3_id.try_into()?;
    let decrypted_output = Bytes::from(decrypted_output);
    let proof = Bytes::from(vec![1]);
    let contract = Enclave::new(contract_address, &provider);
    let builder = contract.publishPlaintextOutput(e3_id, decrypted_output, proof);
    let receipt = builder.send().await?.get_receipt().await?;
    Ok(receipt)
}
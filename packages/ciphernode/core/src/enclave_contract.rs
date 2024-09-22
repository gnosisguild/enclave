use crate::events;
use crate::setup_crp_params;
use crate::EnclaveEvent;
use crate::ParamsWithCrp;
use actix::Actor;
use alloy::eips::BlockNumberOrTag;
use alloy::{
    primitives::Address,
    providers::{Provider, ProviderBuilder, RootProvider},
    rpc::types::Filter,
    sol,
    sol_types::SolEvent,
    transports::BoxTransport,
};
use anyhow::{Context, Result};
use futures_util::stream::StreamExt;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub struct EnclaveContract;

impl Actor for EnclaveContract {
    type Context = actix::Context<Self>;
}

sol! {
    #[derive(Debug)]
    event CommitteeRequested(
        uint256 indexed e3Id,
        address filter,
        uint32[2] threshold
    );

    #[derive(Debug)]
    event CiphernodeAdded(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphernodeRemoved(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    #[derive(Debug)]
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );
}
impl From<CommitteeRequested> for EnclaveEvent {
    fn from(value: CommitteeRequested) -> Self {
        // Tmp
        let ParamsWithCrp {
            moduli,
            degree,
            plaintext_modulus,
            crp_bytes,
            ..
        } = setup_crp_params(
            &[0x3FFFFFFF000001],
            2048,
            1032193,
            Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_entropy())),
        );

        EnclaveEvent::from(events::CommitteeRequested {
            crp: crp_bytes,
            e3_id: crate::E3id::from(value.e3Id),
            moduli,
            degree,
            nodecount: value.threshold[0],
            sortition_seed: 123,
            plaintext_modulus,
        })
    }
}
impl From<CiphernodeRemoved> for EnclaveEvent {
    fn from(value: CiphernodeRemoved) -> Self {
        EnclaveEvent::from(events::CiphernodeRemoved {
            index: value.index.try_into().expect("Could not truncate"),
            address: value.node.to_vec(),
            num_nodes: value.numNodes.try_into().expect("Could not truncate"),
        })
    }
}

impl From<CiphernodeAdded> for EnclaveEvent {
    fn from(value: CiphernodeAdded) -> Self {
        EnclaveEvent::from(events::CiphernodeAdded {
            index: value.index.try_into().expect("Could not truncate"),
            address: value.node.to_vec(),
            num_nodes: value.numNodes.try_into().expect("Could not truncate"),
        })
    }
}

impl From<CiphertextOutputPublished> for EnclaveEvent {
    fn from(value: CiphertextOutputPublished) -> Self {
        EnclaveEvent::from(events::CiphertextOutputPublished {
            e3_id: crate::E3id::from(value.e3Id),
            ciphertext_output: value.ciphertextOutput.to_vec(),
        })
    }
}

async fn create_provider(rpc_url: &str) -> Result<Arc<RootProvider<BoxTransport>>> {
    Ok(Arc::new(
        ProviderBuilder::new()
            .on_builtin(rpc_url)
            .await
            .context("Provider could not be created")?,
    ))
}

async fn listen_for_enclave_events(
    provider: Arc<RootProvider<BoxTransport>>,
    contract_address: Address,
    tx: Sender<EnclaveEvent>,
) -> Result<()> {
    let filter = Filter::new()
        .address(contract_address)
        .event(CommitteeRequested::SIGNATURE)
        .event(CiphernodeAdded::SIGNATURE)
        .event(CiphernodeRemoved::SIGNATURE)
        .event(CiphertextOutputPublished::SIGNATURE)
        .from_block(BlockNumberOrTag::Latest);

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();
    while let Some(log) = stream.next().await {
        if let Some(topic0) = log.topic0() {
            let event = if topic0 == &CommitteeRequested::SIGNATURE_HASH {
                EnclaveEvent::from(CommitteeRequested::decode_log(&log.inner, false)?.data)
            } else if topic0 == &CiphernodeAdded::SIGNATURE_HASH {
                EnclaveEvent::from(CiphernodeAdded::decode_log(&log.inner, false)?.data)
            } else if topic0 == &CiphernodeRemoved::SIGNATURE_HASH {
                EnclaveEvent::from(CiphernodeRemoved::decode_log(&log.inner, false)?.data)
            } else if topic0 == &CiphertextOutputPublished::SIGNATURE_HASH {
                EnclaveEvent::from(CiphertextOutputPublished::decode_log(&log.inner, false)?.data)
            } else {
                continue;
            };

            tx.send(event).await.expect("Failed to send event");
        };
    }
    Ok(())
}

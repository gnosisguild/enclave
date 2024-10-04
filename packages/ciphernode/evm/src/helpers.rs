use std::sync::Arc;

use actix::Recipient;
use alloy::{
    primitives::{LogData, B256},
    providers::{Provider,RootProvider},
    rpc::types::Filter,
    transports::BoxTransport,
};
use anyhow::Context;
use enclave_core::{EnclaveErrorType, EnclaveEvent, FromError};
use futures_util::stream::StreamExt;

pub async fn stream_from_evm(
    provider: Arc<RootProvider<BoxTransport>>,
    filter: Filter,
    bus: Recipient<EnclaveEvent>,
    extractor: fn(&LogData, Option<&B256>) -> Option<EnclaveEvent>,
) {
    match provider
        .subscribe_logs(&filter)
        .await
        .context("Could not subscribe to stream")
    {
        Ok(subscription) => {
            let mut stream = subscription.into_stream();
            while let Some(log) = stream.next().await {
                let Some(event) = extractor(log.data(), log.topic0()) else {
                    continue;
                };
                bus.do_send(event);
            }
        }
        Err(e) => {
            bus.do_send(EnclaveEvent::from_error(EnclaveErrorType::Evm, e));
        }
    }
}

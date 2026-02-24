// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use e3_events::CorrelationId;
use e3_utils::{retry_with_backoff, to_retry};
use tokio::sync::{broadcast, mpsc};

use crate::events::{call_and_await_response, NetCommand, NetEvent, OutgoingRequest, PeerTarget};

pub trait DirectRequesterOutput: TryFrom<Vec<u8>> + Send + Sync + 'static {}

pub trait DirectRequesterInput:
    TryInto<Vec<u8>> + Clone + Send + Sync + fmt::Debug + 'static
{
}

impl<T> DirectRequesterOutput for T where T: TryFrom<Vec<u8>> + Send + Sync + 'static {}

impl<T> DirectRequesterInput for T where
    T: TryInto<Vec<u8>> + Clone + Send + Sync + fmt::Debug + 'static
{
}

pub struct DirectRequester {
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    request_timeout: Duration,
    max_retries: u32,
    retry_timeout: Duration,
}

impl DirectRequester {
    /// Creates a new DirectRequester with custom timeouts.
    ///
    /// # Arguments
    /// * `net_cmds` - Channel to send network commands
    /// * `net_events` - Channel to receive network events
    /// * `request_timeout` - Timeout for each individual request attempt
    /// * `max_retries` - Maximum number of retry attempts
    /// * `retry_timeout` - Total timeout budget for all retries (used for backoff calculation)
    pub fn new(
        net_cmds: mpsc::Sender<NetCommand>,
        net_events: Arc<broadcast::Receiver<NetEvent>>,
        request_timeout: Duration,
        max_retries: u32,
        retry_timeout: Duration,
    ) -> Self {
        Self {
            net_cmds,
            net_events,
            request_timeout,
            max_retries,
            retry_timeout,
        }
    }

    /// Creates a new DirectRequester with default timeouts (30s request, 4 retries, 5s total retry budget).
    pub fn with_defaults(
        net_cmds: mpsc::Sender<NetCommand>,
        net_events: Arc<broadcast::Receiver<NetEvent>>,
    ) -> Self {
        Self::new(
            net_cmds,
            net_events,
            Duration::from_secs(30),
            4,
            Duration::from_millis(5000),
        )
    }

    /// Sends a request to a peer and retries on failure.
    ///
    /// Uses exponential backoff with the configured `max_retries` and `retry_timeout_ms`.
    /// Each attempt times out after `request_timeout`.
    ///
    /// # Arguments
    /// * `request` - The request payload (must implement `DirectRequesterInput`)
    /// * `peer` - The target peer to send the request to
    ///
    /// # Returns
    /// The response deserialized as type `T` (must implement `DirectRequesterOutput`)
    pub async fn request<T, R>(&self, request: R, peer: PeerTarget) -> Result<T>
    where
        T: DirectRequesterOutput,
        R: DirectRequesterInput,
    {
        let payload: Vec<u8> = request
            .clone()
            .try_into()
            .map_err(|_| anyhow!("Request serialization failed for request: {:?}", request))?;

        let response = self.request_with_retry(payload, peer).await?;

        let response: T = response
            .try_into()
            .map_err(|_| anyhow!("Response conversion failed"))?;

        Ok(response)
    }

    async fn request_with_retry(&self, payload: Vec<u8>, peer: PeerTarget) -> Result<Vec<u8>> {
        let request_timeout = self.request_timeout;
        retry_with_backoff(
            || {
                let net_cmds = self.net_cmds.clone();
                let net_events = self.net_events.clone();
                let payload = payload.clone();
                let request_timeout = request_timeout;
                async move {
                    do_request(net_cmds, net_events, peer, payload, request_timeout)
                        .await
                        .map_err(to_retry)
                }
            },
            self.max_retries,
            self.retry_timeout.as_millis() as u64,
        )
        .await
    }
}

async fn do_request(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    target: PeerTarget,
    payload: Vec<u8>,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let correlation_id = CorrelationId::new();

    let response: Vec<u8> = call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::OutgoingRequest(OutgoingRequest {
            correlation_id,
            payload,
            target,
        }),
        |e| match e {
            NetEvent::OutgoingRequestSucceeded(value) => {
                if value.correlation_id == correlation_id {
                    Some(Ok(value.payload.clone()))
                } else {
                    None
                }
            }
            NetEvent::OutgoingRequestFailed(value) => {
                if value.correlation_id == correlation_id {
                    Some(Err(anyhow!("Request failed: {}", value.error)))
                } else {
                    None
                }
            }
            _ => None,
        },
        timeout,
    )
    .await
    .map_err(|e| anyhow!("Request failed: {}", e))?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{OutgoingRequestSucceeded, PeerTarget};
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_successful_request() {
        let (net_cmds_tx, mut net_cmds_rx) = mpsc::channel::<NetCommand>(16);
        let (net_events_tx, net_events_rx) = broadcast::channel::<NetEvent>(16);
        let net_events = Arc::new(net_events_rx);

        let requester = DirectRequester::with_defaults(net_cmds_tx.clone(), net_events.clone());

        let net_events_tx_clone = net_events_tx.clone();
        let handle = tokio::spawn(async move {
            let cmd = net_cmds_rx.recv().await.unwrap();
            if let NetCommand::OutgoingRequest(req) = cmd {
                let response = OutgoingRequestSucceeded {
                    payload: vec![2, 2, 2],
                    correlation_id: req.correlation_id,
                };
                net_events_tx_clone
                    .send(NetEvent::OutgoingRequestSucceeded(response))
                    .unwrap();
            }
        });

        let response: Vec<u8> = requester
            .request(vec![1, 1, 1], PeerTarget::Random)
            .await
            .unwrap();

        handle.await.unwrap();

        assert_eq!(response, vec![2, 2, 2]);
    }

    #[tokio::test]
    async fn test_request_with_peer_target() {
        let (net_cmds_tx, mut net_cmds_rx) = mpsc::channel::<NetCommand>(16);
        let (net_events_tx, net_events_rx) = broadcast::channel::<NetEvent>(16);
        let net_events = Arc::new(net_events_rx);

        let requester = DirectRequester::with_defaults(net_cmds_tx, net_events);

        let net_events_tx_clone = net_events_tx.clone();
        let handle = tokio::spawn(async move {
            let cmd = net_cmds_rx.recv().await.unwrap();
            if let NetCommand::OutgoingRequest(req) = cmd {
                assert!(matches!(req.target, PeerTarget::Random));
                let response = OutgoingRequestSucceeded {
                    payload: vec![],
                    correlation_id: req.correlation_id,
                };
                net_events_tx_clone
                    .send(NetEvent::OutgoingRequestSucceeded(response))
                    .unwrap();
            }
        });

        let _: Vec<u8> = requester
            .request(vec![1], PeerTarget::Random)
            .await
            .unwrap();

        handle.await.unwrap();
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, Context, Handler};
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeResponse,
    ComputeResponseKind, CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey,
    EncryptionKeyCreated, EncryptionKeyPending, EncryptionKeyReceived, Event, EventPublisher,
    EventSubscriber, EventType, PkBfvProofRequest, ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

use crate::{ZkBackend, ZkProver};

#[derive(Clone, Debug)]
struct PendingEncryptionKey {
    e3_id: E3id,
    key: Arc<EncryptionKey>,
}

pub struct ZkActor {
    bus: BusHandle,
    verifier: Option<Arc<ZkProver>>,
    proofs_enabled: bool,
    pending: HashMap<CorrelationId, PendingEncryptionKey>,
}

impl ZkActor {
    pub fn new(bus: &BusHandle, backend: Option<&ZkBackend>) -> Self {
        Self {
            bus: bus.clone(),
            verifier: backend.map(|b| Arc::new(ZkProver::new(b))),
            proofs_enabled: backend.is_some(),
            pending: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, backend: Option<&ZkBackend>) -> Addr<Self> {
        let addr = Self::new(bus, backend).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::EncryptionKeyReceived, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        addr
    }

    fn handle_encryption_key_pending(&mut self, msg: EncryptionKeyPending) {
        if !self.proofs_enabled {
            info!(
                "ZK proofs disabled; publishing EncryptionKeyCreated without proof for party {}",
                msg.key.party_id
            );
            if let Err(err) = self.bus.publish(EncryptionKeyCreated {
                e3_id: msg.e3_id,
                key: msg.key,
                external: false,
            }) {
                error!("Failed to publish EncryptionKeyCreated: {err}");
            }
            return;
        }

        let correlation_id = CorrelationId::new();
        self.pending.insert(
            correlation_id.clone(),
            PendingEncryptionKey {
                e3_id: msg.e3_id.clone(),
                key: msg.key.clone(),
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::PkBfv(PkBfvProofRequest::new(msg.key.pk_bfv.clone(), msg.params)),
            correlation_id.clone(),
            msg.e3_id,
        );

        info!("Requesting T0 proof generation");
        if let Err(err) = self.bus.publish(request) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending.remove(&correlation_id);
        }
    }

    fn handle_encryption_key_received(&mut self, msg: EncryptionKeyReceived) {
        if let Some(ref verifier) = self.verifier {
            let Some(proof) = &msg.key.proof else {
                warn!(
                    "External key from party {} is missing T0 proof - rejecting",
                    msg.key.party_id
                );
                return;
            };

            let e3_id_str = msg.e3_id.to_string();
            match verifier.verify(proof, &e3_id_str) {
                Ok(true) => info!(
                    "T0 proof verified for party {} (circuit: {})",
                    msg.key.party_id, proof.circuit
                ),
                Ok(false) => {
                    error!(
                        "T0 proof verification FAILED for party {} - rejecting key",
                        msg.key.party_id
                    );
                    return;
                }
                Err(e) => {
                    error!(
                        "T0 proof verification error for party {}: {} - rejecting key",
                        msg.key.party_id, e
                    );
                    return;
                }
            }
        } else {
            warn!(
                "ZK backend not available - accepting key from party {} without verification",
                msg.key.party_id
            );
        }

        if let Err(err) = self.bus.publish(EncryptionKeyCreated {
            e3_id: msg.e3_id,
            key: msg.key,
            external: true,
        }) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }

    fn handle_compute_response(&mut self, msg: ComputeResponse) {
        let ComputeResponseKind::Zk(ZkResponse::PkBfv(resp)) = msg.response else {
            return;
        };

        let Some(pending) = self.pending.remove(&msg.correlation_id) else {
            return;
        };

        let mut key = (*pending.key).clone();
        key.proof = Some(resp.proof);

        if let Err(err) = self.bus.publish(EncryptionKeyCreated {
            e3_id: pending.e3_id,
            key: Arc::new(key),
            external: false,
        }) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }

    fn handle_compute_request_error(&mut self, msg: ComputeRequestError) {
        let ComputeRequestErrorKind::Zk(err) = msg.get_err() else {
            return;
        };

        if self.pending.remove(msg.correlation_id()).is_some() {
            warn!("ZK proof request failed: {err}");
        }
    }
}

impl Actor for ZkActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::EncryptionKeyPending(data) => self.notify_sync(ctx, data),
            EnclaveEventData::EncryptionKeyReceived(data) => self.notify_sync(ctx, data),
            EnclaveEventData::ComputeResponse(data) => self.notify_sync(ctx, data),
            EnclaveEventData::ComputeRequestError(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl Handler<EncryptionKeyPending> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: EncryptionKeyPending, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_encryption_key_pending(msg)
    }
}

impl Handler<EncryptionKeyReceived> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: EncryptionKeyReceived, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_encryption_key_received(msg)
    }
}

impl Handler<ComputeResponse> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: ComputeResponse, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_compute_response(msg)
    }
}

impl Handler<ComputeRequestError> for ZkActor {
    type Result = ();

    fn handle(&mut self, msg: ComputeRequestError, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_compute_request_error(msg)
    }
}

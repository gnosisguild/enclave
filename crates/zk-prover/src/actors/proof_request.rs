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
    EncryptionKeyCreated, EncryptionKeyPending, EventPublisher, EventSubscriber, EventType,
    PkBfvProofRequest, TypedEvent, ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

#[derive(Clone, Debug)]
struct PendingProofRequest {
    e3_id: E3id,
    key: Arc<EncryptionKey>,
}

/// Core actor that handles encryption key proof requests.
pub struct ProofRequestActor {
    bus: BusHandle,
    proofs_enabled: bool,
    pending: HashMap<CorrelationId, PendingProofRequest>,
}

impl ProofRequestActor {
    pub fn new(bus: &BusHandle, proofs_enabled: bool) -> Self {
        Self {
            bus: bus.clone(),
            proofs_enabled,
            pending: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, proofs_enabled: bool) -> Addr<Self> {
        let addr = Self::new(bus, proofs_enabled).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        addr
    }

    fn handle_encryption_key_pending(&mut self, msg: TypedEvent<EncryptionKeyPending>) {
        let (msg, ec) = msg.into_components();
        if !self.proofs_enabled {
            info!(
                "ZK proofs disabled; publishing EncryptionKeyCreated without proof for party {}",
                msg.key.party_id
            );
            if let Err(err) = self.bus.publish(
                EncryptionKeyCreated {
                    e3_id: msg.e3_id,
                    key: msg.key,
                    external: false,
                },
                ec,
            ) {
                error!("Failed to publish EncryptionKeyCreated: {err}");
            }
            return;
        }

        let correlation_id = CorrelationId::new();
        self.pending.insert(
            correlation_id,
            PendingProofRequest {
                e3_id: msg.e3_id.clone(),
                key: msg.key.clone(),
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::PkBfv(PkBfvProofRequest::new(
                msg.key.pk_bfv.clone(),
                msg.params_preset,
            )),
            correlation_id,
            msg.e3_id,
        );

        info!("Requesting T0 proof generation");
        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending.remove(&correlation_id);
        }
    }

    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, ec) = msg.into_components();
        let ComputeResponseKind::Zk(ZkResponse::PkBfv(resp)) = msg.response else {
            return;
        };

        let Some(pending) = self.pending.remove(&msg.correlation_id) else {
            return;
        };

        let mut key = (*pending.key).clone();
        key.proof = Some(resp.proof);

        if let Err(err) = self.bus.publish(
            EncryptionKeyCreated {
                e3_id: pending.e3_id,
                key: Arc::new(key),
                external: false,
            },
            ec,
        ) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }

    fn handle_compute_request_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let (msg, ec) = msg.into_components();
        let ComputeRequestErrorKind::Zk(err) = msg.get_err() else {
            return;
        };

        if let Some(pending) = self.pending.remove(msg.correlation_id()) {
            warn!("ZK proof request failed for E3 {}: {err}", pending.e3_id);

            // Publish EncryptionKeyCreated without proof to allow the system to continue
            // Applications can check the proof field to determine if validation occurred
            if let Err(err) = self.bus.publish(
                EncryptionKeyCreated {
                    e3_id: pending.e3_id,
                    key: pending.key,
                    external: false,
                },
                ec,
            ) {
                error!("Failed to publish EncryptionKeyCreated after ZK proof failure: {err}");
            }
        }
    }
}

impl Actor for ProofRequestActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::EncryptionKeyPending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeRequestError(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<EncryptionKeyPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<EncryptionKeyPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_encryption_key_pending(msg)
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_response(msg)
    }
}

impl Handler<TypedEvent<ComputeRequestError>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeRequestError>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_request_error(msg)
    }
}

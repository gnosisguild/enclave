// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, Context, Handler};
use alloy::signers::local::PrivateKeySigner;
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeResponse,
    ComputeResponseKind, CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey,
    EncryptionKeyCreated, EncryptionKeyPending, Event, EventPublisher, EventSubscriber, EventType,
    PkBfvProofRequest, ProofPayload, ProofType, SignedProofPayload, ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info};

#[derive(Clone, Debug)]
struct PendingProofRequest {
    e3_id: E3id,
    key: Arc<EncryptionKey>,
}

/// Core actor that handles encryption key proof requests.
///
/// Proofs are always wrapped in a [`SignedProofPayload`] before being published,
/// enabling fault attribution via the signed proof model.
/// A signer is required — if signing fails, the proof is not published.
pub struct ProofRequestActor {
    bus: BusHandle,
    signer: PrivateKeySigner,
    pending: HashMap<CorrelationId, PendingProofRequest>,
}

impl ProofRequestActor {
    pub fn new(bus: &BusHandle, signer: PrivateKeySigner) -> Self {
        Self {
            bus: bus.clone(),
            signer,
            pending: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, signer: PrivateKeySigner) -> Addr<Self> {
        let addr = Self::new(bus, signer).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        addr
    }

    fn handle_encryption_key_pending(&mut self, msg: EncryptionKeyPending) {
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
        if let Err(err) = self.bus.publish(request) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending.remove(&correlation_id);
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
        key.proof = Some(resp.proof.clone());

        // Always sign the proof payload — unsigned proofs are not published
        let payload = ProofPayload {
            e3_id: pending.e3_id.clone(),
            proof_type: ProofType::T0PkBfv,
            proof: resp.proof.clone(),
        };

        match SignedProofPayload::sign(payload, &self.signer) {
            Ok(signed) => {
                info!(
                    "Signed T0 proof for party {} (signer: {})",
                    key.party_id,
                    self.signer.address()
                );
                key.signed_payload = Some(signed);
            }
            Err(err) => {
                error!("Failed to sign T0 proof payload: {err} — proof will not be published");
                return;
            }
        }

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

        if let Some(pending) = self.pending.remove(msg.correlation_id()) {
            error!(
                "ZK proof request failed for E3 {}: {err} — key will not be published without proof",
                pending.e3_id
            );
        }
    }
}

impl Actor for ProofRequestActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::EncryptionKeyPending(data) => self.notify_sync(ctx, data),
            EnclaveEventData::ComputeResponse(data) => self.notify_sync(ctx, data),
            EnclaveEventData::ComputeRequestError(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl Handler<EncryptionKeyPending> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: EncryptionKeyPending, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_encryption_key_pending(msg)
    }
}

impl Handler<ComputeResponse> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: ComputeResponse, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_compute_response(msg)
    }
}

impl Handler<ComputeRequestError> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: ComputeRequestError, _ctx: &mut Self::Context) -> Self::Result {
        self.handle_compute_request_error(msg)
    }
}

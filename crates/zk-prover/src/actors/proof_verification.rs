// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Core business logic actor for verifying received encryption keys.
//! This actor verifies EncryptionKeyReceived events and converts them
//! to EncryptionKeyCreated events after validation.
//!
//! This is a CORE actor - it delegates IO operations (verification) to ZkActor.

use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Recipient};
use e3_events::{
    BusHandle, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EncryptionKeyReceived, Event, EventPublisher, EventSubscriber, EventType, Proof,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// Request to verify a ZK proof.
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationRequest {
    pub proof: Proof,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
    pub sender: Recipient<ZkVerificationResponse>,
}

/// Response from ZK proof verification with context.
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationResponse {
    pub verified: bool,
    pub error: Option<String>,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
}

/// Core actor that handles encryption key verification.
pub struct ProofVerificationActor {
    bus: BusHandle,
    verifier: Option<Recipient<ZkVerificationRequest>>,
}

impl ProofVerificationActor {
    pub fn new(bus: &BusHandle, verifier: Option<Recipient<ZkVerificationRequest>>) -> Self {
        Self {
            bus: bus.clone(),
            verifier,
        }
    }

    pub fn setup(
        bus: &BusHandle,
        verifier: Option<Recipient<ZkVerificationRequest>>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, verifier).start();
        bus.subscribe(EventType::EncryptionKeyReceived, addr.clone().into());
        addr
    }

    fn handle_encryption_key_received(&mut self, msg: EncryptionKeyReceived, ctx: &Context<Self>) {
        let Some(ref verifier) = self.verifier else {
            warn!(
                "ZK verifier not available - accepting key from party {} without verification",
                msg.key.party_id
            );
            self.publish_key_created(msg.e3_id, msg.key);
            return;
        };

        let Some(ref proof) = msg.key.proof else {
            warn!(
                "External key from party {} is missing T0 proof - rejecting",
                msg.key.party_id
            );
            return;
        };

        let request = ZkVerificationRequest {
            proof: proof.clone(),
            e3_id: msg.e3_id,
            key: msg.key,
            sender: ctx.address().recipient(),
        };

        verifier.do_send(request);
    }

    fn publish_key_created(&self, e3_id: E3id, key: Arc<EncryptionKey>) {
        if let Err(err) = self.bus.publish(EncryptionKeyCreated {
            e3_id,
            key,
            external: true,
        }) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }
}

impl Actor for ProofVerificationActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::EncryptionKeyReceived(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl Handler<EncryptionKeyReceived> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: EncryptionKeyReceived, ctx: &mut Self::Context) -> Self::Result {
        self.handle_encryption_key_received(msg, ctx)
    }
}

impl Handler<ZkVerificationResponse> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: ZkVerificationResponse, _ctx: &mut Self::Context) -> Self::Result {
        if msg.verified {
            info!(
                "T0 proof verified for party {} - accepting key",
                msg.key.party_id
            );
            self.publish_key_created(msg.e3_id, msg.key);
        } else {
            error!(
                "T0 proof verification FAILED for party {} - rejecting key: {}",
                msg.key.party_id,
                msg.error.unwrap_or_else(|| "unknown error".to_string())
            );
        }
    }
}

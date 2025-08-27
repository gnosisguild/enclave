// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use actix::prelude::*;
use actix::{Actor, Handler};
use anyhow::Result;
use e3_crypto::Cipher;
use e3_events::{ComputeRequest, ComputeRequested, EnclaveEvent, EventBus, Subscribe};
use e3_trbfv::calculate_decryption_key::calculate_decryption_key;
use e3_trbfv::calculate_decryption_share::calculate_decryption_share;
use e3_trbfv::calculate_threshold_decryption::calculate_threshold_decryption;
use e3_trbfv::gen_esi_sss::gen_esi_sss;
use e3_trbfv::gen_pk_share_and_sk_sss::gen_pk_share_and_sk_sss;
use e3_trbfv::{SharedRng, TrBFVRequest};

/// Multithread actor
pub struct Multithread {
    rng: SharedRng,
    bus: Addr<EventBus<EnclaveEvent>>,
    cipher: Arc<Cipher>,
}

impl Multithread {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent>>, rng: SharedRng, cipher: Arc<Cipher>) -> Self {
        Self {
            rng,
            cipher,
            bus: bus.clone(),
        }
    }

    pub fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        rng: SharedRng,
        cipher: Arc<Cipher>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, rng, cipher).start();
        bus.do_send(Subscribe::new("ComputeRequested", addr.clone().recipient()));
        addr
    }
}

impl Actor for Multithread {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Multithread {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::ComputeRequested { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<ComputeRequested> for Multithread {
    type Result = ResponseFuture<Result<()>>;

    fn handle(&mut self, msg: ComputeRequested, _ctx: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let bus = self.bus.clone();
        let rng = self.rng.clone();
        Box::pin(async move {
            let _ = handle_compute_request(rng, cipher, msg.request).await;
            // bus.do_send(EnclaveEvent::/* Shutdown { id: () */, data: () });
            Ok(())
        })
    }
}

async fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    request: ComputeRequest,
) -> Result<()> {
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(req)) => {
            let _ = gen_pk_share_and_sk_sss(&rng, &cipher, req).await?;
        }
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(req)) => {
            let _ = gen_esi_sss(&cipher, req).await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(req)) => {
            let _ = calculate_decryption_key(&cipher, req).await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(req)) => {
            let _ = calculate_decryption_share(&cipher, req).await;
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(req)) => {
            let _ = calculate_threshold_decryption(req).await;
        }
        _ => (),
    };
    Ok(())
}

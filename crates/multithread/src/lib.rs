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
use e3_events::{
    ComputeRequest, ComputeRequestError, ComputeResponse, EnclaveEvent, EventBus, Subscribe,
};
use e3_trbfv::calculate_decryption_key::calculate_decryption_key;
use e3_trbfv::calculate_decryption_share::calculate_decryption_share;
use e3_trbfv::calculate_threshold_decryption::calculate_threshold_decryption;
use e3_trbfv::gen_esi_sss::gen_esi_sss;
use e3_trbfv::gen_pk_share_and_sk_sss::gen_pk_share_and_sk_sss;
use e3_trbfv::{SharedRng, TrBFVError, TrBFVRequest, TrBFVResponse};

/// Multithread actor
pub struct Multithread {
    rng: SharedRng,
    cipher: Arc<Cipher>,
}

impl Multithread {
    pub fn new(rng: SharedRng, cipher: Arc<Cipher>) -> Self {
        Self { rng, cipher }
    }

    pub fn attach(rng: SharedRng, cipher: Arc<Cipher>) -> Addr<Self> {
        let addr = Self::new(rng, cipher).start();
        addr
    }
}

impl Actor for Multithread {
    type Context = actix::Context<Self>;
}

impl Handler<ComputeRequest> for Multithread {
    type Result = ResponseFuture<Result<ComputeResponse, ComputeRequestError>>;
    fn handle(&mut self, msg: ComputeRequest, ctx: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let rng = self.rng.clone();
        Box::pin(async move { handle_compute_request(rng, cipher, msg).await })
    }
}

async fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(req)) => {
            match gen_pk_share_and_sk_sss(&rng, &cipher, req).await {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(o))),
                Err(_) => Err(ComputeRequestError::TrBFV(TrBFVError::GenPkShareAndSkSss)),
            }
        }
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(req)) => {
            match gen_esi_sss(&rng, &cipher, req).await {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(o))),
                Err(_) => Err(ComputeRequestError::TrBFV(TrBFVError::GenEsiSss)),
            }
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(req)) => {
            match calculate_decryption_key(&cipher, req).await {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionKey(o),
                )),
                Err(_) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateDecryptionKey,
                )),
            }
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(req)) => {
            match calculate_decryption_share(&cipher, req).await {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionShare(o),
                )),
                Err(_) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateDecryptionShare,
                )),
            }
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(req)) => {
            match calculate_threshold_decryption(req).await {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateThresholdDecryption(o),
                )),
                Err(_) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateThresholdDecryption,
                )),
            }
        }
    }
}

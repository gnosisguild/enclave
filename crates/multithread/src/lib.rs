// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;
use std::time::Instant;

use actix::prelude::*;
use actix::{Actor, Handler};
use anyhow::Result;
use e3_crypto::Cipher;
use e3_events::{ComputeRequest, ComputeRequestError, ComputeResponse};
use e3_trbfv::calculate_decryption_key::calculate_decryption_key;
use e3_trbfv::calculate_decryption_share::calculate_decryption_share;
use e3_trbfv::calculate_threshold_decryption::calculate_threshold_decryption;
use e3_trbfv::gen_esi_sss::gen_esi_sss;
use e3_trbfv::gen_pk_share_and_sk_sss::gen_pk_share_and_sk_sss;
use e3_trbfv::{SharedRng, TrBFVError, TrBFVRequest, TrBFVResponse};
use rayon::{self, ThreadPool};

/// Multithread actor
pub struct Multithread {
    rng: SharedRng,
    cipher: Arc<Cipher>,
    thread_pool: Arc<ThreadPool>,
}

impl Multithread {
    pub fn new(rng: SharedRng, cipher: Arc<Cipher>, threads: usize) -> Self {
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("Failed to create Rayon thread pool"),
        );
        println!(
            "$$$$$$$$  Created threadpool with {} threads.",
            thread_pool.current_num_threads()
        );
        Self {
            rng,
            cipher,
            thread_pool,
        }
    }

    pub fn attach(rng: SharedRng, cipher: Arc<Cipher>, threads: usize) -> Addr<Self> {
        Self::new(rng.clone(), cipher.clone(), threads).start()
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
        let thread_pool = self.thread_pool.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            thread_pool.spawn(move || {
                let res = handle_compute_request(rng, cipher, msg);
                let _ = tx.send(res);
            });

            println!("returned from compute request!");
            let res = rx.await.unwrap()?;

            Ok(res)
        })
    }
}

// TODO: implement tracing for this
// This enabled us to get insight into the timing of our long running functions
fn timefunc<F>(name: &str, func: F) -> Result<ComputeResponse, ComputeRequestError>
where
    F: FnOnce() -> Result<ComputeResponse, ComputeRequestError>,
{
    println!("\n$$$$$$$$$ STARTING `{}`\n", name);
    let start = Instant::now();
    let out = func();
    let dur = start.elapsed();
    println!("\n$$$$$$$$$ FINISHED `{}` in {:?}\n", name, dur);
    out
}

fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(req)) => timefunc(
            "gen_pk_share_and_sk_sss",
            || match gen_pk_share_and_sk_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(o))),
                Err(_) => Err(ComputeRequestError::TrBFV(TrBFVError::GenPkShareAndSkSss)),
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(req)) => {
            timefunc("gen_esi_sss", || match gen_esi_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(o))),
                Err(_) => Err(ComputeRequestError::TrBFV(TrBFVError::GenEsiSss)),
            })
        }
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(req)) => timefunc(
            "calculate_decryption_key",
            || match calculate_decryption_key(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionKey(o),
                )),
                Err(e) => {
                    println!("Error calculating decryption key: {}", e);
                    Err(ComputeRequestError::TrBFV(
                        TrBFVError::CalculateDecryptionKey,
                    ))
                }
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(req)) => timefunc(
            "calculate_decryption_share",
            || match calculate_decryption_share(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionShare(o),
                )),
                Err(_) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateDecryptionShare,
                )),
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(req)) => timefunc(
            "calculate_threshold_decryption",
            || match calculate_threshold_decryption(req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateThresholdDecryption(o),
                )),
                Err(_) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateThresholdDecryption,
                )),
            },
        ),
    }
}

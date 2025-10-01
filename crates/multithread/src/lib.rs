// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
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
use rand::Rng;
use rayon::{self, ThreadPool};
use tracing::error;
use tracing::info;

/// Multithread actor
pub struct Multithread {
    rng: SharedRng,
    cipher: Arc<Cipher>,
    thread_pool: Option<Arc<ThreadPool>>,
}

impl Multithread {
    pub fn new(rng: SharedRng, cipher: Arc<Cipher>, threads: usize) -> Self {
        let thread_pool = if threads == 1 {
            None
        } else {
            let thread_pool = Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .expect("Failed to create Rayon thread pool"),
            );
            info!(
                "Created threadpool with {} threads.",
                thread_pool.current_num_threads()
            );

            Some(thread_pool)
        };

        Self {
            rng,
            cipher,
            thread_pool,
        }
    }

    pub fn get_max_threads_minus(amount: usize) -> usize {
        let total_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let threads_to_use = std::cmp::max(1, total_threads.saturating_sub(amount));
        threads_to_use
    }

    pub fn attach(rng: SharedRng, cipher: Arc<Cipher>, threads: usize) -> Addr<Self> {
        Self::new(rng.clone(), cipher.clone(), threads).start()
    }
}

impl Actor for Multithread {
    type Context = actix::Context<Self>;
}

static PENDING_TASKS: AtomicUsize = AtomicUsize::new(0);
static COMPLETED_TASKS: AtomicUsize = AtomicUsize::new(0);

impl Handler<ComputeRequest> for Multithread {
    type Result = ResponseFuture<Result<ComputeResponse, ComputeRequestError>>;
    fn handle(&mut self, msg: ComputeRequest, ctx: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let rng = self.rng.clone();
        let thread_pool = self.thread_pool.clone();
        Box::pin(async move {
            let pending = PENDING_TASKS.fetch_add(1, Ordering::Relaxed);

            info!(
                "Spawning task. Pending: {}, Completed: {}",
                pending + 1,
                COMPLETED_TASKS.load(Ordering::Relaxed)
            );

            let res = if let Some(pool) = thread_pool {
                let (tx, rx) = tokio::sync::oneshot::channel();
                pool.spawn(move || {
                    let res = handle_compute_request(rng, cipher, msg);
                    PENDING_TASKS.fetch_sub(1, Ordering::Relaxed);
                    COMPLETED_TASKS.fetch_add(1, Ordering::Relaxed);

                    let _ = tx.send(res);
                });
                rx.await.unwrap()
            } else {
                handle_compute_request(rng, cipher, msg)
            };

            res
        })
    }
}

// TODO: implement tracing for this
// This enabled us to get insight into the timing of our long running functions
fn timefunc<F>(name: &str, id: u8, func: F) -> Result<ComputeResponse, ComputeRequestError>
where
    F: FnOnce() -> Result<ComputeResponse, ComputeRequestError>,
{
    info!("\nSTARTING MULTITHREAD `{}({})`\n", name, id);
    let start = Instant::now();
    let out = func();
    let dur = start.elapsed();
    info!("\nFINISHED MULTITHREAD `{}`({}) in {:?}\n", name, id, dur);
    out
}

fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    let id: u8 = rand::thread_rng().gen();
    match request {
        ComputeRequest::TrBFV(TrBFVRequest::GenPkShareAndSkSss(req)) => timefunc(
            "gen_pk_share_and_sk_sss",
            id,
            || match gen_pk_share_and_sk_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenPkShareAndSkSss(o))),
                Err(e) => Err(ComputeRequestError::TrBFV(TrBFVError::GenPkShareAndSkSss(
                    e.to_string(),
                ))),
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::GenEsiSss(req)) => timefunc("gen_esi_sss", id, || {
            match gen_esi_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(TrBFVResponse::GenEsiSss(o))),
                Err(e) => Err(ComputeRequestError::TrBFV(TrBFVError::GenEsiSss(
                    e.to_string(),
                ))),
            }
        }),
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionKey(req)) => timefunc(
            "calculate_decryption_key",
            id,
            || match calculate_decryption_key(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionKey(o),
                )),
                Err(e) => {
                    error!("Error calculating decryption key: {}", e);
                    Err(ComputeRequestError::TrBFV(
                        TrBFVError::CalculateDecryptionKey(e.to_string()),
                    ))
                }
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::CalculateDecryptionShare(req)) => timefunc(
            "calculate_decryption_share",
            id,
            || match calculate_decryption_share(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateDecryptionShare(o),
                )),
                Err(e) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateDecryptionShare(e.to_string()),
                )),
            },
        ),
        ComputeRequest::TrBFV(TrBFVRequest::CalculateThresholdDecryption(req)) => timefunc(
            "calculate_threshold_decryption",
            id,
            || match calculate_threshold_decryption(req) {
                Ok(o) => Ok(ComputeResponse::TrBFV(
                    TrBFVResponse::CalculateThresholdDecryption(o),
                )),
                Err(e) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateThresholdDecryption(e.to_string()),
                )),
            },
        ),
    }
}

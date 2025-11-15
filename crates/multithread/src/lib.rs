// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod report;

use std::sync::Arc;
use std::thread;
use std::time::Duration;
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
use e3_trbfv::{TrBFVError, TrBFVRequest, TrBFVResponse};
use e3_utils::SharedRng;
use rand::Rng;
use rayon::{self, ThreadPool};
use report::MultithreadReport;
use tokio::sync::Semaphore;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Multithread actor
pub struct Multithread {
    rng: SharedRng,
    cipher: Arc<Cipher>,
    rayon_limit: Arc<Semaphore>,
    thread_pool: Arc<ThreadPool>,
    report: Option<MultithreadReport>,
}

impl Multithread {
    pub fn new(
        rng: SharedRng,
        cipher: Arc<Cipher>,
        rayon_threads: usize,
        max_simultaneous_rayon_tasks: usize,
        capture_events: bool,
    ) -> Self {
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(rayon_threads)
                .build()
                .expect("Failed to create Rayon thread pool"),
        );
        info!(
            "Created threadpool with {} threads.",
            thread_pool.current_num_threads()
        );
        let rayon_limit = Arc::new(Semaphore::new(max_simultaneous_rayon_tasks));

        Self {
            rng,
            cipher,
            thread_pool,
            rayon_limit,
            report: if capture_events {
                Some(MultithreadReport::new(
                    rayon_threads,
                    max_simultaneous_rayon_tasks,
                ))
            } else {
                None
            },
        }
    }

    /// Subtract the given amount from the total number of available threads and return the result
    pub fn get_max_threads_minus(amount: usize) -> usize {
        let total_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let threads_to_use = std::cmp::max(1, total_threads.saturating_sub(amount));
        threads_to_use
    }

    pub fn attach(
        rng: SharedRng,
        cipher: Arc<Cipher>,
        rayon_threads: usize,
        max_simultaneous_rayon_tasks: usize,
        capture_events: bool,
    ) -> Addr<Self> {
        Self::new(
            rng.clone(),
            cipher.clone(),
            rayon_threads,
            max_simultaneous_rayon_tasks,
            capture_events,
        )
        .start()
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
        let semaphore = self.rayon_limit.clone();
        let msg_string = msg.to_string();
        let self_addr = ctx.address();
        let capture_events = self.report.is_some();
        let job_name = msg_string.clone();
        Box::pin(async move {
            // Block until we have enough task slots available we have to do this this way as
            // because we use do_send() everywhere there is no backpressure on the actors
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| ComputeRequestError::SemaphoreError(msg_string.to_string()))?;

            // Warn of long running jobs
            let warning_handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                warn!(
                    "Job '{}' has been running for more than 10 seconds",
                    job_name
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                error!(
                    "Job '{}' has been running for more than 30 seconds",
                    job_name
                );
            });

            // This uses channels to track pending and complete tasks when
            // using the thread pool
            let (tx, rx) = tokio::sync::oneshot::channel();

            // We spawn a thread on rayon moving to "sync"-land
            thread_pool.spawn(move || {
                // Do the actual work this is gonna take a while...
                let (result, duration) = handle_compute_request(rng, cipher, msg);

                // try to return the result and it's duration note this is sync as it is a oneshot sender.
                if let Err(res) = tx.send((result, Some(duration))) {
                    error!(
                        "There was an error sending the result from the multithread actor: result = {:?}",
                        res
                    );
                }
            });
            // we are back in async io land...

            // await the oneshot
            let (result, duration) = rx.await.unwrap_or_else(|_| {
                (
                    Err(ComputeRequestError::RecvError(msg_string.to_string())),
                    None,
                )
            });

            warning_handle.abort();

            // incase we are collecting events for a report
            if capture_events {
                if let Some(dur) = duration {
                    self_addr.do_send(TrackDuration::new(msg_string, dur))
                }
            };

            result
        })
    }
}

impl Handler<TrackDuration> for Multithread {
    type Result = ();
    fn handle(&mut self, msg: TrackDuration, _: &mut Self::Context) -> Self::Result {
        // If the report is there we are tracking durations
        if let Some(report) = &mut self.report {
            report.track(msg);
        };
    }
}

impl Handler<GetReport> for Multithread {
    type Result = Option<String>;
    fn handle(&mut self, _: GetReport, _: &mut Self::Context) -> Self::Result {
        if let Some(ref report) = self.report {
            return Some(report.to_report().to_string());
        }
        None
    }
}

#[derive(Message, Debug)]
#[rtype("()")]
pub struct TrackDuration {
    name: String,
    duration: Duration,
}

impl TrackDuration {
    pub fn new(name: String, duration: Duration) -> Self {
        Self { name, duration }
    }
}

#[derive(Message, Debug)]
#[rtype("Option<String>")]
pub struct GetReport;

fn timefunc<F>(
    name: &str,
    id: u8,
    func: F,
) -> (Result<ComputeResponse, ComputeRequestError>, Duration)
where
    F: FnOnce() -> Result<ComputeResponse, ComputeRequestError>,
{
    info!("\nSTARTING MULTITHREAD `{}({})`\n", name, id);
    let start = Instant::now();
    let out = func();
    let dur = start.elapsed();
    info!("\nFINISHED MULTITHREAD `{}`({}) in {:?}\n", name, id, dur);
    (out, dur) // return output as well as timing info
}

/// Handle our compute request. This function is run on a rayon threadpool.
fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    request: ComputeRequest,
) -> (Result<ComputeResponse, ComputeRequestError>, Duration) {
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

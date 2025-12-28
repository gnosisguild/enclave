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
use e3_events::BusHandle;
use e3_events::EType;
use e3_events::EnclaveEvent;
use e3_events::EnclaveEventData;
use e3_events::ErrorDispatcher;
use e3_events::Event;
use e3_events::EventPublisher;
use e3_events::EventSubscriber;
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
pub use report::MultithreadReport;
pub use report::ToReport;
use report::TrackDuration;
use tokio::sync::Semaphore;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Multithread actor
pub struct Multithread {
    bus: BusHandle,
    rng: SharedRng,
    cipher: Arc<Cipher>,
    rayon_limit: Arc<Semaphore>,
    thread_pool: Arc<ThreadPool>,
    report: Option<Addr<MultithreadReport>>,
}

impl Multithread {
    pub fn new(
        bus: BusHandle,
        rng: SharedRng,
        cipher: Arc<Cipher>,
        thread_pool: Arc<ThreadPool>,
        max_simultaneous_rayon_tasks: usize,
        report: Option<Addr<MultithreadReport>>,
    ) -> Self {
        let rayon_limit = Arc::new(Semaphore::new(max_simultaneous_rayon_tasks));

        Self {
            bus,
            rng,
            cipher,
            thread_pool,
            rayon_limit,
            report,
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
        bus: &BusHandle,
        rng: SharedRng,
        cipher: Arc<Cipher>,
        thread_pool: Arc<ThreadPool>,
        max_simultaneous_rayon_tasks: usize,
        report: Option<Addr<MultithreadReport>>,
    ) -> Addr<Self> {
        let addr = Self::new(
            bus.clone(),
            rng.clone(),
            cipher.clone(),
            thread_pool,
            max_simultaneous_rayon_tasks,
            report,
        )
        .start();
        bus.subscribe("ComputeRequest", addr.clone().recipient());
        addr
    }

    pub fn create_thread_pool(threads: usize) -> Arc<ThreadPool> {
        Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("Failed to build thread pool"),
        )
    }
}

impl Actor for Multithread {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for Multithread {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        info!("Multithread received EnclaveEvent!");
        match msg.get_data() {
            EnclaveEventData::ComputeRequest(data) => ctx.notify(data.clone()),
            _ => (),
        }
    }
}

impl Handler<ComputeRequest> for Multithread {
    // type Result = ResponseFuture<Result<ComputeResponse, ComputeRequestError>>;
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: ComputeRequest, _: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let rng = self.rng.clone();
        let bus = self.bus.clone();
        let thread_pool = self.thread_pool.clone();
        let semaphore = self.rayon_limit.clone();
        let report = self.report.clone();
        // TODO: replace with trap_fut
        Box::pin(async move {
            match handle_compute_request_event(
                msg,
                bus,
                cipher,
                rng,
                thread_pool,
                semaphore,
                report,
            )
            .await
            {
                Ok(_) => (),
                Err(e) => error!("{e}"),
            }
        })
    }
}

async fn handle_compute_request_event(
    msg: ComputeRequest,
    bus: BusHandle,
    cipher: Arc<Cipher>,
    rng: SharedRng,
    thread_pool: Arc<ThreadPool>,
    semaphore: Arc<Semaphore>,
    report: Option<Addr<MultithreadReport>>,
) -> anyhow::Result<()> {
    let msg_string = msg.to_string();
    let job_name = msg_string.clone();

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
    if let Some(report) = report {
        if let Some(dur) = duration {
            report.do_send(TrackDuration::new(msg_string, dur))
        }
    };

    match result {
        Ok(val) => bus.publish(val)?,
        Err(e) => bus.err(EType::Computation, e),
    };

    Ok(())
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

    let ComputeRequest {
        correlation_id,
        e3_id,
        request,
    } = request;
    match request {
        TrBFVRequest::GenPkShareAndSkSss(req) => {
            timefunc(
                "gen_pk_share_and_sk_sss",
                id,
                || match gen_pk_share_and_sk_sss(&rng, &cipher, req) {
                    Ok(o) => Ok(ComputeResponse::new(
                        TrBFVResponse::GenPkShareAndSkSss(o),
                        correlation_id,
                        e3_id,
                    )),
                    Err(e) => Err(ComputeRequestError::TrBFV(TrBFVError::GenPkShareAndSkSss(
                        e.to_string(),
                    ))),
                },
            )
        }
        TrBFVRequest::GenEsiSss(req) => timefunc("gen_esi_sss", id, || {
            match gen_esi_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::new(
                    TrBFVResponse::GenEsiSss(o),
                    correlation_id,
                    e3_id,
                )),
                Err(e) => Err(ComputeRequestError::TrBFV(TrBFVError::GenEsiSss(
                    e.to_string(),
                ))),
            }
        }),
        TrBFVRequest::CalculateDecryptionKey(req) => timefunc(
            "calculate_decryption_key",
            id,
            || match calculate_decryption_key(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::new(
                    TrBFVResponse::CalculateDecryptionKey(o),
                    correlation_id,
                    e3_id,
                )),
                Err(e) => {
                    error!("Error calculating decryption key: {}", e);
                    Err(ComputeRequestError::TrBFV(
                        TrBFVError::CalculateDecryptionKey(e.to_string()),
                    ))
                }
            },
        ),
        TrBFVRequest::CalculateDecryptionShare(req) => timefunc(
            "calculate_decryption_share",
            id,
            || match calculate_decryption_share(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::new(
                    TrBFVResponse::CalculateDecryptionShare(o),
                    correlation_id,
                    e3_id,
                )),
                Err(e) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateDecryptionShare(e.to_string()),
                )),
            },
        ),
        TrBFVRequest::CalculateThresholdDecryption(req) => timefunc(
            "calculate_threshold_decryption",
            id,
            || match calculate_threshold_decryption(req) {
                Ok(o) => Ok(ComputeResponse::new(
                    TrBFVResponse::CalculateThresholdDecryption(o),
                    correlation_id,
                    e3_id,
                )),
                Err(e) => Err(ComputeRequestError::TrBFV(
                    TrBFVError::CalculateThresholdDecryption(e.to_string()),
                )),
            },
        ),
    }
}

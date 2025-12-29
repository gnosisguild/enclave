// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use crate::report::MultithreadReport;
use crate::report::TrackDuration;
use crate::TaskPool;
use crate::TaskTimeouts;
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
use tracing::error;
use tracing::info;

/// Multithread actor
pub struct Multithread {
    bus: BusHandle,
    rng: SharedRng,
    cipher: Arc<Cipher>,
    task_pool: TaskPool,
    report: Option<Addr<MultithreadReport>>,
}

impl Multithread {
    pub fn new(
        bus: BusHandle,
        rng: SharedRng,
        cipher: Arc<Cipher>,
        task_pool: TaskPool,
        report: Option<Addr<MultithreadReport>>,
    ) -> Self {
        Self {
            bus,
            rng,
            cipher,
            task_pool,
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
        task_pool: TaskPool,
        report: Option<Addr<MultithreadReport>>,
    ) -> Addr<Self> {
        let addr = Self::new(bus.clone(), rng.clone(), cipher.clone(), task_pool, report).start();
        bus.subscribe("ComputeRequest", addr.clone().recipient());
        addr
    }

    pub fn create_taskpool(threads: usize, max_tasks: usize) -> TaskPool {
        TaskPool::new(threads, max_tasks)
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
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: ComputeRequest, _: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let rng = self.rng.clone();
        let bus = self.bus.clone();
        let pool = self.task_pool.clone();
        let report = self.report.clone();
        // TODO: replace with trap_fut
        Box::pin(async move {
            match handle_compute_request_event(msg, bus, cipher, rng, pool, report).await {
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
    pool: TaskPool,
    report: Option<Addr<MultithreadReport>>,
) -> anyhow::Result<()> {
    let msg_string = msg.to_string();
    let job_name = msg_string.clone();

    // We spawn a thread on rayon moving to "sync"-land
    let (result, duration) = pool
        .spawn(job_name, TaskTimeouts::default(), move || {
            // Do the actual work this is gonna take a while...
            handle_compute_request(rng, cipher, msg)
        })
        .await?;
    // we are back in async io land...

    // incase we are collecting events for a report
    if let Some(report) = report {
        report.do_send(TrackDuration::new(msg_string, duration))
    };

    match result {
        Ok(val) => bus.publish(val)?,
        Err(e) => bus.err(EType::Computation, e),
    };
    Ok(())
}

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

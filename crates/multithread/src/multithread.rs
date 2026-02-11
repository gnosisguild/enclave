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
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeRequestKind,
    ComputeResponse, EType, EnclaveEvent, EnclaveEventData, ErrorDispatcher, Event, EventPublisher,
    EventSubscriber, EventType, PkBfvProofRequest, PkBfvProofResponse, ZkError as ZkEventError,
    ZkRequest, ZkResponse,
};
use e3_fhe_params::{BfvParamSet, BfvPreset};
use e3_trbfv::calculate_decryption_key::calculate_decryption_key;
use e3_trbfv::calculate_decryption_share::calculate_decryption_share;
use e3_trbfv::calculate_threshold_decryption::calculate_threshold_decryption;
use e3_trbfv::gen_esi_sss::gen_esi_sss;
use e3_trbfv::gen_pk_share_and_sk_sss::gen_pk_share_and_sk_sss;
use e3_trbfv::{TrBFVError, TrBFVRequest, TrBFVResponse};
use e3_utils::SharedRng;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitData};
use e3_zk_prover::{Provable, ZkBackend, ZkProver};
use fhe::bfv::PublicKey;
use fhe_traits::DeserializeParametrized;
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
    zk_prover: Option<Arc<ZkProver>>,
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
            zk_prover: None,
        }
    }

    /// Set the ZK prover for handling proof requests.
    pub fn with_zk_prover(mut self, prover: Arc<ZkProver>) -> Self {
        self.zk_prover = Some(prover);
        self
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
        bus.subscribe(EventType::ComputeRequest, addr.clone().recipient());
        addr
    }

    pub fn attach_with_zk(
        bus: &BusHandle,
        rng: SharedRng,
        cipher: Arc<Cipher>,
        task_pool: TaskPool,
        report: Option<Addr<MultithreadReport>>,
        zk_backend: &ZkBackend,
    ) -> Addr<Self> {
        let zk_prover = Arc::new(ZkProver::new(zk_backend));
        let actor = Self::new(bus.clone(), rng.clone(), cipher.clone(), task_pool, report)
            .with_zk_prover(zk_prover);
        let addr = actor.start();
        bus.subscribe(EventType::ComputeRequest, addr.clone().recipient());
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
        let zk_prover = self.zk_prover.clone();

        Box::pin(async move {
            match handle_compute_request_event(msg, bus, cipher, rng, pool, report, zk_prover).await
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
    pool: TaskPool,
    report: Option<Addr<MultithreadReport>>,
    zk_prover: Option<Arc<ZkProver>>,
) -> anyhow::Result<()> {
    let msg_string = msg.to_string();
    let job_name = msg_string.clone();

    let (result, duration) = pool
        .spawn(job_name, TaskTimeouts::default(), move || {
            handle_compute_request(rng, cipher, zk_prover, msg)
        })
        .await?;

    if let Some(report) = report {
        report.do_send(TrackDuration::new(msg_string, duration))
    };

    match result {
        Ok(val) => bus.publish(val)?,
        Err(e) => {
            // Publish ComputeRequestError so ProofRequestActor can handle it
            // and continue without proof if needed
            bus.publish(e)?
        }
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
    info!("STARTING MULTITHREAD `{}({})`", name, id);
    let start = Instant::now();
    let out = func();
    let dur = start.elapsed();
    info!("FINISHED MULTITHREAD `{}`({}) in {:?}", name, id, dur);
    (out, dur)
}

/// Handle compute request. This function is run on a rayon threadpool.
fn handle_compute_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    zk_prover: Option<Arc<ZkProver>>,
    request: ComputeRequest,
) -> (Result<ComputeResponse, ComputeRequestError>, Duration) {
    let id: u8 = rand::thread_rng().gen();

    match request.request.clone() {
        ComputeRequestKind::TrBFV(trbfv_req) => {
            handle_trbfv_request(rng, cipher, trbfv_req, request, id)
        }
        ComputeRequestKind::Zk(zk_req) => handle_zk_request(zk_prover, zk_req, request, id),
    }
}

fn handle_trbfv_request(
    rng: SharedRng,
    cipher: Arc<Cipher>,
    trbfv_req: TrBFVRequest,
    request: ComputeRequest,
    id: u8,
) -> (Result<ComputeResponse, ComputeRequestError>, Duration) {
    match trbfv_req {
        TrBFVRequest::GenPkShareAndSkSss(req) => {
            timefunc(
                "gen_pk_share_and_sk_sss",
                id,
                || match gen_pk_share_and_sk_sss(&rng, &cipher, req) {
                    Ok(o) => Ok(ComputeResponse::trbfv(
                        TrBFVResponse::GenPkShareAndSkSss(o),
                        request.correlation_id,
                        request.e3_id,
                    )),
                    Err(e) => Err(ComputeRequestError::new(
                        ComputeRequestErrorKind::TrBFV(TrBFVError::GenPkShareAndSkSss(
                            e.to_string(),
                        )),
                        request,
                    )),
                },
            )
        }
        TrBFVRequest::GenEsiSss(req) => timefunc("gen_esi_sss", id, || {
            match gen_esi_sss(&rng, &cipher, req) {
                Ok(o) => Ok(ComputeResponse::trbfv(
                    TrBFVResponse::GenEsiSss(o),
                    request.correlation_id,
                    request.e3_id,
                )),
                Err(e) => Err(ComputeRequestError::new(
                    ComputeRequestErrorKind::TrBFV(TrBFVError::GenEsiSss(e.to_string())),
                    request,
                )),
            }
        }),
        TrBFVRequest::CalculateDecryptionKey(req) => timefunc(
            "calculate_decryption_key",
            id,
            || match calculate_decryption_key(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::trbfv(
                    TrBFVResponse::CalculateDecryptionKey(o),
                    request.correlation_id,
                    request.e3_id,
                )),
                Err(e) => {
                    error!("Error calculating decryption key: {}", e);
                    Err(ComputeRequestError::new(
                        ComputeRequestErrorKind::TrBFV(TrBFVError::CalculateDecryptionKey(
                            e.to_string(),
                        )),
                        request,
                    ))
                }
            },
        ),
        TrBFVRequest::CalculateDecryptionShare(req) => timefunc(
            "calculate_decryption_share",
            id,
            || match calculate_decryption_share(&cipher, req) {
                Ok(o) => Ok(ComputeResponse::trbfv(
                    TrBFVResponse::CalculateDecryptionShare(o),
                    request.correlation_id,
                    request.e3_id,
                )),
                Err(e) => Err(ComputeRequestError::new(
                    ComputeRequestErrorKind::TrBFV(TrBFVError::CalculateDecryptionShare(
                        e.to_string(),
                    )),
                    request,
                )),
            },
        ),
        TrBFVRequest::CalculateThresholdDecryption(req) => timefunc(
            "calculate_threshold_decryption",
            id,
            || match calculate_threshold_decryption(req) {
                Ok(o) => Ok(ComputeResponse::trbfv(
                    TrBFVResponse::CalculateThresholdDecryption(o),
                    request.correlation_id,
                    request.e3_id,
                )),
                Err(e) => Err(ComputeRequestError::new(
                    ComputeRequestErrorKind::TrBFV(TrBFVError::CalculateThresholdDecryption(
                        e.to_string(),
                    )),
                    request,
                )),
            },
        ),
    }
}

fn handle_zk_request(
    zk_prover: Option<Arc<ZkProver>>,
    zk_req: ZkRequest,
    request: ComputeRequest,
    id: u8,
) -> (Result<ComputeResponse, ComputeRequestError>, Duration) {
    let Some(prover) = zk_prover else {
        return (
            Err(ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(
                    "ZK prover not configured".to_string(),
                )),
                request,
            )),
            Duration::ZERO,
        );
    };

    match zk_req {
        ZkRequest::PkBfv(req) => timefunc("zk_pk_bfv", id, || {
            handle_pk_bfv_proof(&prover, req, request.clone())
        }),
    }
}

fn handle_pk_bfv_proof(
    prover: &ZkProver,
    req: PkBfvProofRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    // NOTE: req.params_preset is expected to contain a DKG preset (e.g., InsecureDkg512)
    // because the proof is for the DKG circuit. This preset is converted to BFV parameters.
    let params = BfvParamSet::from(req.params_preset.clone()).build_arc();
    let pk_bfv = PublicKey::from_bytes(&req.pk_bfv, &params).map_err(|e| {
        ComputeRequestError::new(
            ComputeRequestErrorKind::Zk(ZkEventError::InvalidParams(format!(
                "Failed to deserialize pk_bfv: {:?}",
                e
            ))),
            request.clone(),
        )
    })?;

    let circuit = PkCircuit;
    let circuit_data = PkCircuitData { public_key: pk_bfv };
    let e3_id_str = request.e3_id.to_string();
    let preset_counterpart = req
        .params_preset
        .threshold_counterpart()
        .unwrap_or_else(|| BfvPreset::InsecureThreshold512);
    // But here we have to pass the InsecureThreshold512 preset because the underlaying witness generator
    // builds both params, but will only use the DKG one
    let proof = circuit
        .prove(prover, &preset_counterpart, &circuit_data, &e3_id_str)
        .map_err(|e| {
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(e.to_string())),
                request.clone(),
            )
        })?;

    Ok(ComputeResponse::zk(
        ZkResponse::PkBfv(PkBfvProofResponse::new(proof)),
        request.correlation_id,
        request.e3_id,
    ))
}

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
use e3_events::run_once;
use e3_events::trap_fut;
use e3_events::EType;
use e3_events::EffectsEnabled;
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeRequestKind,
    ComputeResponse, DkgShareDecryptionProofRequest, DkgShareDecryptionProofResponse, EnclaveEvent,
    EnclaveEventData, EventPublisher, EventSubscriber, EventType, PartyC4VerificationResult,
    PartyVerificationResult, PkBfvProofRequest, PkBfvProofResponse, PkGenerationProofRequest,
    PkGenerationProofResponse, ShareComputationProofRequest, ShareComputationProofResponse,
    ShareEncryptionProofRequest, ShareEncryptionProofResponse, TypedEvent, VerifyC4ProofsRequest,
    VerifyC4ProofsResponse, VerifyShareProofsRequest, VerifyShareProofsResponse,
    ZkError as ZkEventError, ZkRequest, ZkResponse,
};
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::{BfvParamSet, BfvPreset};
use e3_polynomial::CrtPolynomial;
use e3_trbfv::calculate_decryption_key::calculate_decryption_key;
use e3_trbfv::calculate_decryption_share::calculate_decryption_share;
use e3_trbfv::calculate_threshold_decryption::calculate_threshold_decryption;
use e3_trbfv::gen_esi_sss::gen_esi_sss;
use e3_trbfv::gen_pk_share_and_sk_sss::gen_pk_share_and_sk_sss;
use e3_trbfv::helpers::deserialize_secret_key;
use e3_trbfv::helpers::try_poly_from_bytes;
use e3_trbfv::shares::SharedSecret;
use e3_trbfv::{TrBFVError, TrBFVRequest, TrBFVResponse};
use e3_utils::SharedRng;
use e3_utils::MAILBOX_LIMIT;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitData};
use e3_zk_helpers::circuits::dkg::share_computation::utils::compute_parity_matrix;
use e3_zk_helpers::circuits::threshold::pk_generation::circuit::{
    PkGenerationCircuit, PkGenerationCircuitData,
};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_computation::{ShareComputationCircuit, ShareComputationCircuitData};
use e3_zk_helpers::dkg::share_decryption::{ShareDecryptionCircuit, ShareDecryptionCircuitData};
use e3_zk_helpers::dkg::share_encryption::{ShareEncryptionCircuit, ShareEncryptionCircuitData};
use e3_zk_prover::{Provable, ZkBackend, ZkProver};
use fhe::bfv::{Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::{DeserializeParametrized, FheEncoder};
use ndarray::Array2;
use num_bigint::BigInt;
use rand::rngs::OsRng;
use rand::Rng;
use tracing::{error, info};

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

        bus.subscribe(
            EventType::EffectsEnabled,
            run_once::<EffectsEnabled>({
                let bus = bus.clone();
                let addr = addr.clone();
                move |_| {
                    bus.subscribe(EventType::ComputeRequest, addr.clone().recipient());
                    info!("Multithread actor listening for events.");
                    Ok(())
                }
            })
            .recipient(),
        );

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
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for Multithread {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        info!("Multithread received EnclaveEvent!");
        let (data, ec) = msg.into_components();
        match data {
            EnclaveEventData::ComputeRequest(data) => ctx.notify(TypedEvent::new(data, ec)),
            _ => (),
        }
    }
}

impl Handler<TypedEvent<ComputeRequest>> for Multithread {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: TypedEvent<ComputeRequest>, _: &mut Self::Context) -> Self::Result {
        let cipher = self.cipher.clone();
        let rng = self.rng.clone();
        let bus = self.bus.clone();
        let pool = self.task_pool.clone();
        let report = self.report.clone();
        let zk_prover = self.zk_prover.clone();
        trap_fut(
            EType::Computation,
            &self.bus.clone(),
            handle_compute_request_event(msg, bus, cipher, rng, pool, report, zk_prover),
        )
    }
}

async fn handle_compute_request_event(
    msg: TypedEvent<ComputeRequest>,
    bus: BusHandle,
    cipher: Arc<Cipher>,
    rng: SharedRng,
    pool: TaskPool,
    report: Option<Addr<MultithreadReport>>,
    zk_prover: Option<Arc<ZkProver>>,
) -> anyhow::Result<()> {
    let msg_string = msg.to_string();
    let job_name = msg_string.clone();
    let (msg, ctx) = msg.into_components();
    let request_snapshot = msg.clone();

    let pool_result = pool
        .spawn(job_name, TaskTimeouts::default(), move || {
            handle_compute_request(rng, cipher, zk_prover, msg)
        })
        .await;

    let (result, duration) = match pool_result {
        Ok(v) => v,
        Err(pool_err) => {
            error!(
                "Task pool error for compute request '{}': {pool_err}",
                msg_string
            );
            let error_kind = match &request_snapshot.request {
                ComputeRequestKind::Zk(_) => ComputeRequestErrorKind::Zk(
                    ZkEventError::ProofGenerationFailed(format!("Pool error: {pool_err}")),
                ),
                ComputeRequestKind::TrBFV(ref trbfv_req) => {
                    let msg = format!("Pool error: {pool_err}");
                    ComputeRequestErrorKind::TrBFV(match trbfv_req {
                        TrBFVRequest::GenPkShareAndSkSss(_) => TrBFVError::GenPkShareAndSkSss(msg),
                        TrBFVRequest::GenEsiSss(_) => TrBFVError::GenEsiSss(msg),
                        TrBFVRequest::CalculateDecryptionKey(_) => {
                            TrBFVError::CalculateDecryptionKey(msg)
                        }
                        TrBFVRequest::CalculateDecryptionShare(_) => {
                            TrBFVError::CalculateDecryptionShare(msg)
                        }
                        TrBFVRequest::CalculateThresholdDecryption(_) => {
                            TrBFVError::CalculateThresholdDecryption(msg)
                        }
                    })
                }
            };
            bus.publish(ComputeRequestError::new(error_kind, request_snapshot), ctx)?;
            return Ok(());
        }
    };

    if let Some(report) = report {
        report.do_send(TrackDuration::new(msg_string, duration))
    };

    match result {
        Ok(val) => bus.publish(val, ctx)?,
        Err(e) => {
            // Publish ComputeRequestError so ProofRequestActor can handle it
            // and continue without proof if needed
            bus.publish(e, ctx)?
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
        ComputeRequestKind::Zk(zk_req) => handle_zk_request(cipher, zk_prover, zk_req, request, id),
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
    cipher: Arc<Cipher>,
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
        ZkRequest::PkGeneration(req) => timefunc("zk_pk_generation", id, || {
            handle_pk_generation_proof(&prover, &cipher, req, request.clone())
        }),
        ZkRequest::ShareComputation(req) => timefunc("zk_share_computation", id, || {
            handle_share_computation_proof(&prover, &cipher, req, request.clone())
        }),
        ZkRequest::ShareEncryption(req) => timefunc("zk_share_encryption", id, || {
            handle_share_encryption_proof(&prover, &cipher, req, request.clone())
        }),
        ZkRequest::DkgShareDecryption(req) => timefunc("zk_dkg_share_decryption", id, || {
            handle_dkg_share_decryption_proof(&prover, &cipher, req, request.clone())
        }),
        ZkRequest::VerifyShareProofs(req) => timefunc("zk_verify_share_proofs", id, || {
            handle_verify_share_proofs(&prover, req, request.clone())
        }),
        ZkRequest::VerifyC4Proofs(req) => timefunc("zk_verify_c4_proofs", id, || {
            handle_verify_c4_proofs(&prover, req, request.clone())
        }),
    }
}

/// Helper to reduce boilerplate for ZK errors
fn make_zk_error(request: &ComputeRequest, msg: String) -> ComputeRequestError {
    ComputeRequestError::new(
        ComputeRequestErrorKind::Zk(ZkEventError::InvalidParams(msg)),
        request.clone(),
    )
}

fn handle_share_computation_proof(
    prover: &ZkProver,
    cipher: &Cipher,
    req: ShareComputationProofRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    // 1. Build BFV threshold parameters
    let (threshold_params, _dkg_params) = build_pair_for_preset(req.params_preset.clone())
        .map_err(|e| make_zk_error(&request, format!("build_pair_for_preset: {}", e)))?;

    // 2. Decrypt sensitive witness fields
    let secret_bytes = req
        .secret_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("secret_raw decrypt: {}", e)))?;
    let secret_sss_bytes = req
        .secret_sss_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("secret_sss_raw decrypt: {}", e)))?;

    // 3. Deserialize secret polynomial
    let secret_poly = try_poly_from_bytes(&secret_bytes, &threshold_params)
        .map_err(|e| make_zk_error(&request, format!("secret_raw: {}", e)))?;
    let mut secret = CrtPolynomial::from_fhe_polynomial(&secret_poly);
    if req.dkg_input_type == DkgInputType::SecretKey {
        secret
            .center(threshold_params.moduli())
            .map_err(|e| make_zk_error(&request, format!("Failed to center polynomial: {}", e)))?;
    }

    // 4. Deserialize Shamir shares (bincode-encoded SharedSecret)
    let shared_secret: SharedSecret = bincode::deserialize(&secret_sss_bytes)
        .map_err(|e| make_zk_error(&request, format!("secret_sss_raw deserialize: {}", e)))?;

    // Convert Vec<Array2<u64>> → Vec<Array2<BigInt>>
    let secret_sss: Vec<Array2<BigInt>> = shared_secret
        .moduli_data()
        .iter()
        .map(|arr| arr.mapv(|v| BigInt::from(v)))
        .collect();

    // 4. Compute parity matrix
    let committee = req.committee_size.values();
    let parity_matrix =
        compute_parity_matrix(threshold_params.moduli(), committee.n, committee.threshold)
            .map_err(|e| make_zk_error(&request, format!("compute_parity_matrix: {}", e)))?;

    // 5. Build circuit data
    let circuit_data = ShareComputationCircuitData {
        dkg_input_type: req.dkg_input_type,
        secret,
        secret_sss,
        parity_matrix,
        n_parties: committee.n as u32,
        threshold: committee.threshold as u32,
    };

    // 6. Generate proof
    let circuit = ShareComputationCircuit;
    let e3_id_str = request.e3_id.to_string();

    let proof = circuit
        .prove(prover, &req.params_preset, &circuit_data, &e3_id_str)
        .map_err(|e| {
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(e.to_string())),
                request.clone(),
            )
        })?;

    // 7. Return response
    Ok(ComputeResponse::zk(
        ZkResponse::ShareComputation(ShareComputationProofResponse {
            proof,
            dkg_input_type: req.dkg_input_type,
        }),
        request.correlation_id,
        request.e3_id,
    ))
}

fn handle_pk_generation_proof(
    prover: &ZkProver,
    cipher: &Cipher,
    req: PkGenerationProofRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    // 1. Build BFV parameters from the threshold preset
    let params = BfvParamSet::from(req.params_preset.clone()).build_arc();

    // 2. Decrypt sensitive witness fields
    let sk_bytes = req
        .sk
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("sk decrypt: {}", e)))?;
    let eek_bytes = req
        .eek
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("eek decrypt: {}", e)))?;
    let e_sm_bytes = req
        .e_sm
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("e_sm decrypt: {}", e)))?;

    // 3. Deserialize raw polynomial bytes → Poly
    let pk0_share_poly = try_poly_from_bytes(&req.pk0_share, &params)
        .map_err(|e| make_zk_error(&request, format!("pk0_share: {}", e)))?;

    let sk_poly = try_poly_from_bytes(&sk_bytes, &params)
        .map_err(|e| make_zk_error(&request, format!("sk: {}", e)))?;

    let eek_poly = try_poly_from_bytes(&eek_bytes, &params)
        .map_err(|e| make_zk_error(&request, format!("eek: {}", e)))?;

    let e_sm_poly = try_poly_from_bytes(&e_sm_bytes, &params)
        .map_err(|e| make_zk_error(&request, format!("e_sm: {}", e)))?;

    // 3. Convert Poly → CrtPolynomial
    let pk0_share = CrtPolynomial::from_fhe_polynomial(&pk0_share_poly);
    let sk = CrtPolynomial::from_fhe_polynomial(&sk_poly);
    let eek = CrtPolynomial::from_fhe_polynomial(&eek_poly);
    let e_sm = CrtPolynomial::from_fhe_polynomial(&e_sm_poly);

    // 4. Build circuit data
    let committee = req.committee_size.values();
    let circuit_data = PkGenerationCircuitData {
        committee,
        pk0_share,
        eek,
        e_sm,
        sk,
    };

    // 5. Generate proof via Provable trait
    let circuit = PkGenerationCircuit;
    let e3_id_str = request.e3_id.to_string();

    let proof = circuit
        .prove(prover, &req.params_preset, &circuit_data, &e3_id_str)
        .map_err(|e| {
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(e.to_string())),
                request.clone(),
            )
        })?;

    // 6. Return response
    Ok(ComputeResponse::zk(
        ZkResponse::PkGeneration(PkGenerationProofResponse::new(proof)),
        request.correlation_id,
        request.e3_id,
    ))
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

fn handle_share_encryption_proof(
    prover: &ZkProver,
    cipher: &Cipher,
    req: ShareEncryptionProofRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    // 1. Build DKG params from threshold preset
    let (_threshold_params, dkg_params) = build_pair_for_preset(req.params_preset)
        .map_err(|e| make_zk_error(&request, format!("build_pair_for_preset: {}", e)))?;

    // 2. Decrypt sensitive witness data
    let share_row_bytes = req
        .share_row_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("share_row decrypt: {}", e)))?;
    let u_rns_bytes = req
        .u_rns_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("u_rns decrypt: {}", e)))?;
    let e0_rns_bytes = req
        .e0_rns_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("e0_rns decrypt: {}", e)))?;
    let e1_rns_bytes = req
        .e1_rns_raw
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("e1_rns decrypt: {}", e)))?;

    // 3. Deserialize share row and re-encode as Plaintext
    let share_row: Vec<u64> = bincode::deserialize(&share_row_bytes)
        .map_err(|e| make_zk_error(&request, format!("share_row: {}", e)))?;
    let plaintext = Plaintext::try_encode(&share_row, Encoding::poly(), &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("plaintext encode: {:?}", e)))?;

    // 4. Deserialize ciphertext, public key, polys using DKG params
    let ciphertext = Ciphertext::from_bytes(&req.ciphertext_raw, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("ciphertext: {:?}", e)))?;
    let public_key = PublicKey::from_bytes(&req.recipient_pk_raw, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("recipient_pk: {:?}", e)))?;
    let u_rns = try_poly_from_bytes(&u_rns_bytes, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("u_rns: {}", e)))?;
    let e0_rns = try_poly_from_bytes(&e0_rns_bytes, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("e0_rns: {}", e)))?;
    let e1_rns = try_poly_from_bytes(&e1_rns_bytes, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("e1_rns: {}", e)))?;

    // 4. Dummy SecretKey (not used by Inputs::compute)
    let dummy_sk = SecretKey::random(&dkg_params, &mut OsRng);

    // 5. Build circuit data
    let circuit_data = ShareEncryptionCircuitData {
        plaintext,
        ciphertext,
        public_key,
        secret_key: dummy_sk,
        u_rns,
        e0_rns,
        e1_rns,
        dkg_input_type: req.dkg_input_type,
    };

    // 6. Generate proof (preset = threshold preset; Inputs::compute derives DKG internally)
    let circuit = ShareEncryptionCircuit;
    let e3_id_str = request.e3_id.to_string();
    let proof = circuit
        .prove(prover, &req.params_preset, &circuit_data, &e3_id_str)
        .map_err(|e| {
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(e.to_string())),
                request.clone(),
            )
        })?;

    Ok(ComputeResponse::zk(
        ZkResponse::ShareEncryption(ShareEncryptionProofResponse {
            proof,
            dkg_input_type: req.dkg_input_type,
            recipient_party_id: req.recipient_party_id,
            row_index: req.row_index,
            esi_index: req.esi_index,
        }),
        request.correlation_id,
        request.e3_id,
    ))
}

fn handle_dkg_share_decryption_proof(
    prover: &ZkProver,
    cipher: &Cipher,
    req: DkgShareDecryptionProofRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    // 1. Build DKG params from preset
    let (_threshold_params, dkg_params) = build_pair_for_preset(req.params_preset)
        .map_err(|e| make_zk_error(&request, format!("build_pair_for_preset: {}", e)))?;

    // 2. Decrypt BFV secret key from SensitiveBytes
    let sk_bytes = req
        .sk_bfv
        .access_raw(cipher)
        .map_err(|e| make_zk_error(&request, format!("sk_bfv decrypt: {}", e)))?;
    let secret_key = deserialize_secret_key(&sk_bytes, &dkg_params)
        .map_err(|e| make_zk_error(&request, format!("sk_bfv deserialize: {}", e)))?;

    // 3. Deserialize ciphertexts from raw bytes [H * L] → Vec<Vec<Ciphertext>> [H][L]
    let h = req.num_honest_parties;
    let l = req.num_moduli;
    if req.honest_ciphertexts_raw.len() != h * l {
        return Err(make_zk_error(
            &request,
            format!(
                "Expected {} ciphertexts (H={} * L={}), got {}",
                h * l,
                h,
                l,
                req.honest_ciphertexts_raw.len()
            ),
        ));
    }

    let mut honest_ciphertexts: Vec<Vec<Ciphertext>> = Vec::with_capacity(h);
    for party_idx in 0..h {
        let mut party_cts = Vec::with_capacity(l);
        for mod_idx in 0..l {
            let raw = &req.honest_ciphertexts_raw[party_idx * l + mod_idx];
            let ct = Ciphertext::from_bytes(raw, &dkg_params).map_err(|e| {
                make_zk_error(
                    &request,
                    format!(
                        "ciphertext[{}][{}] deserialize: {:?}",
                        party_idx, mod_idx, e
                    ),
                )
            })?;
            party_cts.push(ct);
        }
        honest_ciphertexts.push(party_cts);
    }

    // 4. Build circuit data
    let circuit_data = ShareDecryptionCircuitData {
        secret_key,
        honest_ciphertexts,
        dkg_input_type: req.dkg_input_type,
    };

    // 5. Generate proof
    let circuit = ShareDecryptionCircuit;
    let e3_id_str = request.e3_id.to_string();
    let proof = circuit
        .prove(prover, &req.params_preset, &circuit_data, &e3_id_str)
        .map_err(|e| {
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkEventError::ProofGenerationFailed(e.to_string())),
                request.clone(),
            )
        })?;

    // 6. Return response
    Ok(ComputeResponse::zk(
        ZkResponse::DkgShareDecryption(DkgShareDecryptionProofResponse {
            proof,
            dkg_input_type: req.dkg_input_type,
        }),
        request.correlation_id,
        request.e3_id,
    ))
}

fn handle_verify_share_proofs(
    prover: &ZkProver,
    req: VerifyShareProofsRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    let e3_id_str = request.e3_id.to_string();

    let party_results: Vec<PartyVerificationResult> = req
        .party_proofs
        .into_iter()
        .map(|party| {
            let sender = party.sender_party_id;
            for signed_proof in &party.signed_proofs {
                let proof = &signed_proof.payload.proof;
                let result = prover.verify(proof, &e3_id_str, sender);
                match result {
                    Ok(true) => continue,
                    Ok(false) => {
                        info!(
                            "Proof verification failed for party {} ({:?})",
                            sender, signed_proof.payload.proof_type
                        );
                        return PartyVerificationResult {
                            sender_party_id: sender,
                            all_verified: false,
                            failed_proof_type: Some(signed_proof.payload.proof_type),
                            failed_signed_payload: Some(signed_proof.clone()),
                        };
                    }
                    Err(e) => {
                        info!(
                            "Proof verification error for party {} ({:?}): {}",
                            sender, signed_proof.payload.proof_type, e
                        );
                        return PartyVerificationResult {
                            sender_party_id: sender,
                            all_verified: false,
                            failed_proof_type: Some(signed_proof.payload.proof_type),
                            failed_signed_payload: Some(signed_proof.clone()),
                        };
                    }
                }
            }
            PartyVerificationResult {
                sender_party_id: sender,
                all_verified: true,
                failed_proof_type: None,
                failed_signed_payload: None,
            }
        })
        .collect();

    Ok(ComputeResponse::zk(
        ZkResponse::VerifyShareProofs(VerifyShareProofsResponse { party_results }),
        request.correlation_id,
        request.e3_id,
    ))
}

fn handle_verify_c4_proofs(
    prover: &ZkProver,
    req: VerifyC4ProofsRequest,
    request: ComputeRequest,
) -> Result<ComputeResponse, ComputeRequestError> {
    let e3_id_str = request.e3_id.to_string();

    let party_results: Vec<PartyC4VerificationResult> = req
        .party_proofs
        .into_iter()
        .map(|party| {
            let sender = party.sender_party_id;

            // Verify C4a proof
            let c4a_result = prover.verify(&party.c4a_proof, &e3_id_str, sender);
            match c4a_result {
                Ok(true) => {}
                Ok(false) | Err(_) => {
                    return PartyC4VerificationResult {
                        sender_party_id: sender,
                        all_verified: false,
                    };
                }
            }

            // Verify all C4b proofs
            for c4b_proof in &party.c4b_proofs {
                let result = prover.verify(c4b_proof, &e3_id_str, sender);
                match result {
                    Ok(true) => continue,
                    Ok(false) | Err(_) => {
                        return PartyC4VerificationResult {
                            sender_party_id: sender,
                            all_verified: false,
                        };
                    }
                }
            }

            PartyC4VerificationResult {
                sender_party_id: sender,
                all_verified: true,
            }
        })
        .collect();

    Ok(ComputeResponse::zk(
        ZkResponse::VerifyC4Proofs(VerifyC4ProofsResponse { party_results }),
        request.correlation_id,
        request.e3_id,
    ))
}

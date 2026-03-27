// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor for C2/C3/C4 share proof verification.
//!
//! Follows the same pattern as [`ProofVerificationActor`] (for C0) — sits
//! between the raw proof data and the verified result, handling ECDSA validation
//! and ZK verification orchestration.
//!
//! ## Flow
//!
//! 1. Receives [`ShareVerificationDispatched`] from [`ThresholdKeyshare`].
//! 2. Performs ECDSA validation (signature recovery, signer consistency, e3_id,
//!    circuit name) — lightweight, no thread pool needed.
//! 3. Dispatches ZK-only verification to multithread via [`ComputeRequest`].
//! 4. Receives [`ComputeResponse`] from multithread with pure ZK results.
//! 5. Combines ECDSA + ZK results.
//! 6. Emits [`SignedProofFailed`] for any failing proofs.
//! 7. Publishes [`ShareVerificationComplete`] with dishonest party set.

use std::collections::{BTreeSet, HashMap, HashSet};

use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::{keccak256, Address, Bytes};
use alloy::sol_types::SolValue;
use e3_events::{
    BusHandle, CommitmentConsistencyCheckComplete, CommitmentConsistencyCheckRequested,
    ComputeRequest, ComputeRequestError, ComputeResponse, ComputeResponseKind, CorrelationId, E3id,
    EnclaveEvent, EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType,
    PartyProofData, PartyProofsToVerify, PartyShareDecryptionProofsToVerify,
    PartyVerificationResult, ProofType, ProofVerificationFailed, ProofVerificationPassed,
    Sequenced, ShareVerificationComplete, ShareVerificationDispatched, SignedProofFailed,
    SignedProofPayload, TypedEvent, VerificationKind, VerifyShareDecryptionProofsRequest,
    VerifyShareProofsRequest, ZkRequest, ZkResponse,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// Trait for party types whose signed proofs can be ECDSA-validated and ZK-verified.
trait VerifiableParty: Clone {
    fn party_id(&self) -> u64;
    fn signed_proofs(&self) -> Vec<SignedProofPayload>;
}

impl VerifiableParty for PartyProofsToVerify {
    fn party_id(&self) -> u64 {
        self.sender_party_id
    }
    fn signed_proofs(&self) -> Vec<SignedProofPayload> {
        self.signed_proofs.clone()
    }
}

impl VerifiableParty for PartyShareDecryptionProofsToVerify {
    fn party_id(&self) -> u64 {
        self.sender_party_id
    }
    fn signed_proofs(&self) -> Vec<SignedProofPayload> {
        std::iter::once(self.signed_sk_decryption_proof.clone())
            .chain(self.signed_e_sm_decryption_proofs.iter().cloned())
            .collect()
    }
}

/// ECDSA validation result for a single party.
struct EcdsaPartyResult {
    passed: bool,
    /// The pair (signed_payload, recovered_address) of the first failing proof, if any.
    failed_payload: Option<(SignedProofPayload, Option<Address>)>,
}

/// Pending verification state — stored while ZK verification is in flight.
struct PendingVerification {
    e3_id: E3id,
    kind: VerificationKind,
    ec: EventContext<Sequenced>,
    /// Parties that failed ECDSA (dishonest before ZK runs).
    ecdsa_dishonest: HashSet<u64>,
    /// Pre-dishonest parties from the dispatch (missing/incomplete proofs).
    pre_dishonest: BTreeSet<u64>,
    /// Party IDs dispatched for ZK verification (for cross-checking results).
    dispatched_party_ids: HashSet<u64>,
    /// Recovered address for each party (from ECDSA step).
    party_addresses: HashMap<u64, Address>,
    /// Cached (proof_type, data_hash) per party — for emitting ProofVerificationPassed.
    party_proof_hashes: HashMap<u64, Vec<(ProofType, [u8; 32])>>,
    /// Cached (proof_type, public_signals) per party — for commitment consistency checking.
    party_public_signals: HashMap<u64, Vec<(ProofType, ArcBytes)>>,
}

/// Pending consistency check — stored between ECDSA pass and ZK dispatch.
///
/// After ECDSA validation, the actor publishes
/// [`CommitmentConsistencyCheckRequested`] and waits for the checker's
/// response. This struct buffers the ECDSA results and the original party
/// proofs so that ZK verification can be dispatched once the consistency
/// check completes.
///
/// Several fields overlap with [`PendingVerification`] (e3_id, kind, ec,
/// party_addresses, party_proof_hashes, party_public_signals). When the
/// consistency check completes, they are transferred to a new
/// `PendingVerification` entry for the ZK phase.
struct PendingConsistencyCheck {
    e3_id: E3id,
    kind: VerificationKind,
    ec: EventContext<Sequenced>,
    /// Parties that failed ECDSA (dishonest before consistency runs).
    ecdsa_dishonest: HashSet<u64>,
    /// Pre-dishonest parties from the dispatch (missing/incomplete proofs).
    pre_dishonest: BTreeSet<u64>,
    /// Recovered address per ECDSA-passed party.
    party_addresses: HashMap<u64, Address>,
    /// (proof_type, data_hash) per party — for ProofVerificationPassed after ZK.
    party_proof_hashes: HashMap<u64, Vec<(ProofType, [u8; 32])>>,
    /// (proof_type, public_signals) per party — for consistency & ZK.
    party_public_signals: HashMap<u64, Vec<(ProofType, ArcBytes)>>,
    /// Original ECDSA-passed share proofs for ZK dispatch.
    /// Populated for ShareProofs / ThresholdDecryptionProofs / PkGenerationProofs.
    ecdsa_passed_share_proofs: Vec<PartyProofsToVerify>,
    /// Original ECDSA-passed decryption proofs for ZK dispatch.
    /// Populated for DecryptionProofs.
    ecdsa_passed_decryption_proofs: Vec<PartyShareDecryptionProofsToVerify>,
}

/// Filter out inconsistent parties and collect dispatched party IDs.
/// Returns `None` if all parties were filtered out (nothing to verify).
fn filter_consistent<P>(
    proofs: Vec<P>,
    inconsistent: &BTreeSet<u64>,
    party_id_of: impl Fn(&P) -> u64,
) -> Option<(Vec<P>, HashSet<u64>)> {
    let passed: Vec<P> = proofs
        .into_iter()
        .filter(|p| !inconsistent.contains(&party_id_of(p)))
        .collect();
    if passed.is_empty() {
        return None;
    }
    let ids = passed.iter().map(|p| party_id_of(p)).collect();
    Some((passed, ids))
}

/// Actor that handles C1/C2/C3/C4/C6 share proof verification.
///
/// Three-stage pipeline:
/// 1. ECDSA validation (lightweight, done inline)
/// 2. Commitment consistency check (dispatched to per-E3 checker via event bus)
/// 3. ZK proof verification (heavyweight, delegated to multithread)
///
/// Emits [`SignedProofFailed`] for fault attribution and
/// [`ShareVerificationComplete`] with the final dishonest party set.
pub struct ShareVerificationActor {
    bus: BusHandle,
    /// Tracks pending ZK verifications by correlation ID.
    pending: HashMap<CorrelationId, PendingVerification>,
    /// Tracks pending consistency checks by correlation ID (between ECDSA and ZK).
    pending_consistency: HashMap<CorrelationId, PendingConsistencyCheck>,
}

impl ShareVerificationActor {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            pending: HashMap::new(),
            pending_consistency: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.subscribe(EventType::ShareVerificationDispatched, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        bus.subscribe(
            EventType::CommitmentConsistencyCheckComplete,
            addr.clone().into(),
        );
        addr
    }

    fn handle_share_verification_dispatched(
        &mut self,
        msg: TypedEvent<ShareVerificationDispatched>,
    ) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        info!(
            "handling ShareVerificationDispatched {:?}, {:?}",
            e3_id, msg.kind
        );

        match msg.kind {
            VerificationKind::ShareProofs
            | VerificationKind::ThresholdDecryptionProofs
            | VerificationKind::PkGenerationProofs => {
                let kind = msg.kind.clone();
                self.verify_proofs(
                    e3_id,
                    kind.clone(),
                    msg.share_proofs,
                    msg.pre_dishonest,
                    ec,
                    |pending, passed| {
                        pending.ecdsa_passed_share_proofs = passed;
                    },
                );
            }
            VerificationKind::DecryptionProofs => {
                self.verify_proofs(
                    e3_id,
                    VerificationKind::DecryptionProofs,
                    msg.decryption_proofs,
                    msg.pre_dishonest,
                    ec,
                    |pending, passed| {
                        pending.ecdsa_passed_decryption_proofs = passed;
                    },
                );
            }
        }
    }

    /// Generic ECDSA validation + consistency check dispatch.
    ///
    /// After ECDSA validation, publishes [`CommitmentConsistencyCheckRequested`]
    /// and stores a [`PendingConsistencyCheck`]. ZK verification is deferred
    /// until the consistency check response arrives.
    fn verify_proofs<P: VerifiableParty>(
        &mut self,
        e3_id: E3id,
        kind: VerificationKind,
        party_proofs: Vec<P>,
        pre_dishonest: BTreeSet<u64>,
        ec: EventContext<Sequenced>,
        store_passed_proofs: impl FnOnce(&mut PendingConsistencyCheck, Vec<P>),
    ) {
        let e3_id_str = e3_id.to_string();
        let label = match &kind {
            VerificationKind::ShareProofs => "C2/C3",
            VerificationKind::ThresholdDecryptionProofs => "C6",
            VerificationKind::PkGenerationProofs => "C1",
            VerificationKind::DecryptionProofs => "C4",
        };
        let mut ecdsa_dishonest = HashSet::new();
        let mut ecdsa_passed_parties = Vec::new();
        let mut party_addresses: HashMap<u64, Address> = HashMap::new();

        for party in &party_proofs {
            let proofs = party.signed_proofs();
            let result =
                self.ecdsa_validate_signed_proofs(party.party_id(), &proofs, &e3_id_str, label);
            if result.passed {
                ecdsa_passed_parties.push(party.clone());
            } else {
                ecdsa_dishonest.insert(party.party_id());
                if let Some((ref signed, addr)) = result.failed_payload {
                    self.emit_signed_proof_failed(&e3_id, signed, addr, party.party_id(), &ec);
                }
            }
        }

        // Store recovered addresses for passed parties
        for party in &party_proofs {
            if !ecdsa_dishonest.contains(&party.party_id()) {
                let proofs = party.signed_proofs();
                if let Some(first_signed) = proofs.first() {
                    if let Ok(addr) = first_signed.recover_address() {
                        party_addresses.insert(party.party_id(), addr);
                    }
                }
            }
        }

        if ecdsa_passed_parties.is_empty() {
            // All parties failed ECDSA — publish result immediately
            let mut all_dishonest: BTreeSet<u64> = pre_dishonest;
            all_dishonest.extend(ecdsa_dishonest);
            self.publish_complete(e3_id, kind, all_dishonest, ec);
            return;
        }

        // Compute proof hashes and public signals for ECDSA-passed parties
        let mut party_proof_hashes: HashMap<u64, Vec<(ProofType, [u8; 32])>> = HashMap::new();
        let mut party_public_signals: HashMap<u64, Vec<(ProofType, ArcBytes)>> = HashMap::new();
        for party in &ecdsa_passed_parties {
            let hashes: Vec<(ProofType, [u8; 32])> = party
                .signed_proofs()
                .iter()
                .map(|signed| {
                    let msg = (
                        Bytes::copy_from_slice(&signed.payload.proof.data),
                        Bytes::copy_from_slice(&signed.payload.proof.public_signals),
                    )
                        .abi_encode();
                    (signed.payload.proof_type, keccak256(&msg).into())
                })
                .collect();
            let signals: Vec<(ProofType, ArcBytes)> = party
                .signed_proofs()
                .iter()
                .map(|signed| {
                    (
                        signed.payload.proof_type,
                        signed.payload.proof.public_signals.clone(),
                    )
                })
                .collect();
            party_proof_hashes.insert(party.party_id(), hashes);
            party_public_signals.insert(party.party_id(), signals);
        }

        // Build consistency check request
        let correlation_id = CorrelationId::new();
        let party_proof_data: Vec<PartyProofData> = ecdsa_passed_parties
            .iter()
            .map(|party| {
                let signals = party_public_signals
                    .get(&party.party_id())
                    .cloned()
                    .unwrap_or_default();
                let hashes = party_proof_hashes
                    .get(&party.party_id())
                    .cloned()
                    .unwrap_or_default();
                let proofs = signals
                    .into_iter()
                    .zip(hashes)
                    .map(|((pt, ps), (_, dh))| (pt, ps, dh))
                    .collect();
                PartyProofData {
                    party_id: party.party_id(),
                    address: party_addresses
                        .get(&party.party_id())
                        .copied()
                        .unwrap_or_default(),
                    proofs,
                }
            })
            .collect();

        // Store pending consistency check with the original party proofs
        let mut pending = PendingConsistencyCheck {
            e3_id: e3_id.clone(),
            kind: kind.clone(),
            ec: ec.clone(),
            ecdsa_dishonest,
            pre_dishonest,
            party_addresses,
            party_proof_hashes,
            party_public_signals,
            ecdsa_passed_share_proofs: Vec::new(),
            ecdsa_passed_decryption_proofs: Vec::new(),
        };
        store_passed_proofs(&mut pending, ecdsa_passed_parties);
        self.pending_consistency.insert(correlation_id, pending);

        // Publish consistency check request
        if let Err(err) = self.bus.publish(
            CommitmentConsistencyCheckRequested {
                e3_id: e3_id.clone(),
                kind: kind.clone(),
                correlation_id,
                party_proofs: party_proof_data,
            },
            ec.clone(),
        ) {
            error!(
                "Failed to dispatch {} consistency check: {err} — treating all as dishonest",
                label
            );
            if let Some(pending) = self.pending_consistency.remove(&correlation_id) {
                let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
                all_dishonest.extend(pending.ecdsa_dishonest);
                for p in &pending.ecdsa_passed_share_proofs {
                    all_dishonest.insert(p.sender_party_id);
                }
                for p in &pending.ecdsa_passed_decryption_proofs {
                    all_dishonest.insert(p.sender_party_id);
                }
                self.publish_complete(e3_id, kind, all_dishonest, ec);
            }
        }
    }

    /// Handle consistency check response: add inconsistent parties to the
    /// dishonest set, then dispatch ZK verification for the remaining
    /// consistent parties.
    fn handle_consistency_check_complete(
        &mut self,
        msg: TypedEvent<CommitmentConsistencyCheckComplete>,
    ) {
        let (data, _ec) = msg.into_components();

        let Some(pending) = self.pending_consistency.remove(&data.correlation_id) else {
            return; // Not our correlation ID
        };

        let label = match &pending.kind {
            VerificationKind::ShareProofs => "C2/C3",
            VerificationKind::ThresholdDecryptionProofs => "C6",
            VerificationKind::PkGenerationProofs => "C1",
            VerificationKind::DecryptionProofs => "C4",
        };

        if !data.inconsistent_parties.is_empty() {
            warn!(
                "{} consistency check found {} inconsistent parties for E3 {}: {:?}",
                label,
                data.inconsistent_parties.len(),
                pending.e3_id,
                data.inconsistent_parties
            );
        }

        // Accumulate all dishonest parties discovered so far
        let mut dishonest_so_far: BTreeSet<u64> = pending.pre_dishonest.clone();
        dishonest_so_far.extend(&pending.ecdsa_dishonest);
        dishonest_so_far.extend(&data.inconsistent_parties);

        // Filter ECDSA-passed proofs to only consistent parties and dispatch ZK
        let inconsistent = &data.inconsistent_parties;
        let zk_correlation_id = CorrelationId::new();

        let (request, dispatched_party_ids) = match pending.kind {
            VerificationKind::ShareProofs
            | VerificationKind::ThresholdDecryptionProofs
            | VerificationKind::PkGenerationProofs => {
                let Some((passed, ids)) =
                    filter_consistent(pending.ecdsa_passed_share_proofs, inconsistent, |p| {
                        p.sender_party_id
                    })
                else {
                    self.publish_complete(
                        pending.e3_id,
                        pending.kind,
                        dishonest_so_far,
                        pending.ec,
                    );
                    return;
                };
                let req = ComputeRequest::zk(
                    ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                        party_proofs: passed,
                    }),
                    zk_correlation_id,
                    pending.e3_id.clone(),
                );
                (req, ids)
            }
            VerificationKind::DecryptionProofs => {
                let Some((passed, ids)) =
                    filter_consistent(pending.ecdsa_passed_decryption_proofs, inconsistent, |p| {
                        p.sender_party_id
                    })
                else {
                    self.publish_complete(
                        pending.e3_id,
                        pending.kind,
                        dishonest_so_far,
                        pending.ec,
                    );
                    return;
                };
                let req = ComputeRequest::zk(
                    ZkRequest::VerifyShareDecryptionProofs(VerifyShareDecryptionProofsRequest {
                        party_proofs: passed,
                    }),
                    zk_correlation_id,
                    pending.e3_id.clone(),
                );
                (req, ids)
            }
        };

        // Only keep proof hashes/signals/addresses for parties going to ZK
        let party_addresses: HashMap<u64, Address> = pending
            .party_addresses
            .into_iter()
            .filter(|(pid, _)| dispatched_party_ids.contains(pid))
            .collect();
        let party_proof_hashes: HashMap<u64, Vec<(ProofType, [u8; 32])>> = pending
            .party_proof_hashes
            .into_iter()
            .filter(|(pid, _)| dispatched_party_ids.contains(pid))
            .collect();
        let party_public_signals: HashMap<u64, Vec<(ProofType, ArcBytes)>> = pending
            .party_public_signals
            .into_iter()
            .filter(|(pid, _)| dispatched_party_ids.contains(pid))
            .collect();

        // Store pending ZK verification state.
        // All prior dishonest parties (pre_dishonest + ECDSA + consistency) are
        // folded into `pre_dishonest` so that `handle_compute_response` produces
        // the correct final dishonest set when it adds ZK failures.
        self.pending.insert(
            zk_correlation_id,
            PendingVerification {
                e3_id: pending.e3_id.clone(),
                kind: pending.kind.clone(),
                ec: pending.ec.clone(),
                ecdsa_dishonest: HashSet::new(),
                pre_dishonest: dishonest_so_far,
                dispatched_party_ids: dispatched_party_ids.clone(),
                party_addresses,
                party_proof_hashes,
                party_public_signals,
            },
        );

        if let Err(err) = self.bus.publish(request, pending.ec.clone()) {
            error!(
                "Failed to dispatch {} ZK verification after consistency check: {err}",
                label
            );
            if let Some(zk_pending) = self.pending.remove(&zk_correlation_id) {
                let mut all_dishonest: BTreeSet<u64> = zk_pending.pre_dishonest;
                all_dishonest.extend(zk_pending.ecdsa_dishonest);
                all_dishonest.extend(zk_pending.dispatched_party_ids);
                self.publish_complete(
                    zk_pending.e3_id,
                    zk_pending.kind,
                    all_dishonest,
                    zk_pending.ec,
                );
            }
        }
    }

    /// Validate ECDSA properties for a set of signed proofs from one party:
    /// 1. e3_id match
    /// 2. Signature recovery (valid ECDSA)
    /// 3. Signer consistency (all proofs from same address)
    /// 4. Circuit name matches expected ProofType circuits
    fn ecdsa_validate_signed_proofs(
        &self,
        sender_party_id: u64,
        signed_proofs: &[SignedProofPayload],
        e3_id_str: &str,
        label: &str,
    ) -> EcdsaPartyResult {
        let mut expected_addr: Option<Address> = None;

        for signed in signed_proofs {
            // 1. e3_id match
            if signed.payload.e3_id.to_string() != e3_id_str {
                info!(
                    "{} proof from party {} has wrong e3_id ({} vs {})",
                    label, sender_party_id, signed.payload.e3_id, e3_id_str
                );
                return EcdsaPartyResult {
                    passed: false,
                    failed_payload: Some((signed.clone(), expected_addr)),
                };
            }

            // 2. Signature recovery
            match signed.recover_address() {
                Ok(addr) => {
                    // 3. Signer consistency
                    match &expected_addr {
                        Some(ea) if *ea != addr => {
                            info!(
                                "{} inconsistent signer for party {}",
                                label, sender_party_id
                            );
                            return EcdsaPartyResult {
                                passed: false,
                                failed_payload: Some((signed.clone(), Some(addr))),
                            };
                        }
                        None => expected_addr = Some(addr),
                        _ => {}
                    }
                }
                Err(e) => {
                    info!(
                        "{} signature recovery failed for party {} ({:?}): {}",
                        label, sender_party_id, signed.payload.proof_type, e
                    );
                    return EcdsaPartyResult {
                        passed: false,
                        failed_payload: Some((signed.clone(), expected_addr)),
                    };
                }
            }

            // 4. Circuit name validation
            let expected_circuits = signed.payload.proof_type.circuit_names();
            if !expected_circuits.contains(&signed.payload.proof.circuit) {
                info!(
                    "{} circuit mismatch for party {}: expected {:?}, got {:?}",
                    label, sender_party_id, expected_circuits, signed.payload.proof.circuit
                );
                return EcdsaPartyResult {
                    passed: false,
                    failed_payload: Some((signed.clone(), expected_addr)),
                };
            }
        }

        EcdsaPartyResult {
            passed: true,
            failed_payload: None,
        }
    }

    /// Handle ZK verification response from multithread.
    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, _ec) = msg.into_components();

        let correlation_id = msg.correlation_id;
        let Some(pending) = self.pending.remove(&correlation_id) else {
            return; // Not our correlation ID
        };

        let zk_results: Vec<PartyVerificationResult> = match (&pending.kind, msg.response) {
            (
                VerificationKind::ShareProofs
                | VerificationKind::ThresholdDecryptionProofs
                | VerificationKind::PkGenerationProofs,
                ComputeResponseKind::Zk(ZkResponse::VerifyShareProofs(r)),
            ) => r.party_results,
            (
                VerificationKind::DecryptionProofs,
                ComputeResponseKind::Zk(ZkResponse::VerifyShareDecryptionProofs(r)),
            ) => r.party_results,
            _ => {
                error!("Unexpected ComputeResponse kind for verification — treating all dispatched parties as dishonest");
                let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
                all_dishonest.extend(pending.ecdsa_dishonest);
                all_dishonest.extend(pending.dispatched_party_ids);
                self.publish_complete(pending.e3_id, pending.kind, all_dishonest, pending.ec);
                return;
            }
        };

        let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
        all_dishonest.extend(&pending.ecdsa_dishonest);

        // Cross-check: every dispatched party must appear in results.
        // If any party is missing from the ZK response, treat as dishonest (defense-in-depth).
        let returned_party_ids: HashSet<u64> =
            zk_results.iter().map(|r| r.sender_party_id).collect();
        for &dispatched_pid in &pending.dispatched_party_ids {
            if !returned_party_ids.contains(&dispatched_pid) {
                warn!(
                    "Party {} was dispatched for ZK verification but missing from results — treating as dishonest",
                    dispatched_pid
                );
                all_dishonest.insert(dispatched_pid);
            }
        }

        for result in &zk_results {
            // Ignore results for parties we never dispatched (defense-in-depth)
            if !pending
                .dispatched_party_ids
                .contains(&result.sender_party_id)
            {
                warn!(
                    "ZK result for party {} was not dispatched — ignoring",
                    result.sender_party_id
                );
                continue;
            }
            if !result.all_verified {
                all_dishonest.insert(result.sender_party_id);

                // Emit SignedProofFailed for ZK failure
                if let Some(ref signed) = result.failed_signed_payload {
                    let addr = pending
                        .party_addresses
                        .get(&result.sender_party_id)
                        .copied();
                    self.emit_signed_proof_failed(
                        &pending.e3_id,
                        signed,
                        addr,
                        result.sender_party_id,
                        &pending.ec,
                    );
                }
            } else {
                // Emit ProofVerificationPassed for each proof type from this party
                if let Some(hashes) = pending.party_proof_hashes.get(&result.sender_party_id) {
                    let addr = pending
                        .party_addresses
                        .get(&result.sender_party_id)
                        .copied()
                        .unwrap_or_default();
                    let signals = pending.party_public_signals.get(&result.sender_party_id);
                    for (i, &(proof_type, data_hash)) in hashes.iter().enumerate() {
                        let public_signals = signals
                            .and_then(|s| s.get(i))
                            .map(|(_, ps)| ps.clone())
                            .unwrap_or_default();
                        if let Err(err) = self.bus.publish(
                            ProofVerificationPassed {
                                e3_id: pending.e3_id.clone(),
                                party_id: result.sender_party_id,
                                address: addr,
                                proof_type,
                                data_hash,
                                public_signals,
                            },
                            pending.ec.clone(),
                        ) {
                            error!("Failed to publish ProofVerificationPassed: {err}");
                        }
                    }
                }
            }
        }

        self.publish_complete(pending.e3_id, pending.kind, all_dishonest, pending.ec);
    }

    fn emit_signed_proof_failed(
        &self,
        e3_id: &E3id,
        signed_payload: &SignedProofPayload,
        recovered_addr: Option<Address>,
        party_id: u64,
        ec: &EventContext<Sequenced>,
    ) {
        let faulting_node = match recovered_addr {
            Some(addr) => addr,
            None => match signed_payload.recover_address() {
                Ok(addr) => addr,
                Err(err) => {
                    warn!(
                        "Signature recovery failed for party {} — using zero address for fault attribution: {err}",
                        party_id
                    );
                    Address::ZERO
                }
            },
        };

        if let Err(err) = self.bus.publish(
            SignedProofFailed {
                e3_id: e3_id.clone(),
                faulting_node,
                proof_type: signed_payload.payload.proof_type,
                signed_payload: signed_payload.clone(),
            },
            ec.clone(),
        ) {
            error!("Failed to publish SignedProofFailed: {err}");
        }

        // Also emit ProofVerificationFailed for AccusationManager
        let data_hash: [u8; 32] = {
            let msg = (
                Bytes::copy_from_slice(&signed_payload.payload.proof.data),
                Bytes::copy_from_slice(&signed_payload.payload.proof.public_signals),
            )
                .abi_encode();
            keccak256(&msg).into()
        };
        if let Err(err) = self.bus.publish(
            ProofVerificationFailed {
                e3_id: e3_id.clone(),
                accused_party_id: party_id,
                accused_address: faulting_node,
                proof_type: signed_payload.payload.proof_type,
                data_hash,
                signed_payload: signed_payload.clone(),
            },
            ec.clone(),
        ) {
            error!("Failed to publish ProofVerificationFailed: {err}");
        }
    }

    /// Handle computation error from multithread — clean up pending state and
    /// publish ShareVerificationComplete treating all dispatched parties as dishonest.
    fn handle_compute_request_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let (msg, _ec) = msg.into_components();

        let correlation_id = msg.correlation_id();
        let Some(pending) = self.pending.remove(correlation_id) else {
            return;
        };

        error!(
            "ZK verification computation failed for E3 {} ({:?}): {} — treating all dispatched parties as dishonest",
            pending.e3_id, pending.kind, msg
        );

        let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
        all_dishonest.extend(pending.ecdsa_dishonest);
        all_dishonest.extend(pending.dispatched_party_ids);
        self.publish_complete(pending.e3_id, pending.kind, all_dishonest, pending.ec);
    }

    fn publish_complete(
        &self,
        e3_id: E3id,
        kind: VerificationKind,
        dishonest_parties: BTreeSet<u64>,
        ec: EventContext<Sequenced>,
    ) {
        if let Err(err) = self.bus.publish(
            ShareVerificationComplete {
                e3_id,
                kind,
                dishonest_parties,
            },
            ec,
        ) {
            error!("Failed to publish ShareVerificationComplete: {err}");
        }
    }
}

impl Actor for ShareVerificationActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ShareVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::ShareVerificationDispatched(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeRequestError(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::CommitmentConsistencyCheckComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<ShareVerificationDispatched>> for ShareVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ShareVerificationDispatched>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_share_verification_dispatched(msg)
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ShareVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_response(msg)
    }
}

impl Handler<TypedEvent<ComputeRequestError>> for ShareVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeRequestError>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_request_error(msg)
    }
}

impl Handler<TypedEvent<CommitmentConsistencyCheckComplete>> for ShareVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitmentConsistencyCheckComplete>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_consistency_check_complete(msg)
    }
}

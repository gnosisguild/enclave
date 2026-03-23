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
    BusHandle, ComputeRequest, ComputeRequestError, ComputeResponse, ComputeResponseKind,
    CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EventContext, EventPublisher,
    EventSubscriber, EventType, PartyProofsToVerify, PartyShareDecryptionProofsToVerify,
    PartyVerificationResult, ProofType, ProofVerificationFailed, ProofVerificationPassed,
    Sequenced, ShareVerificationComplete, ShareVerificationDispatched, SignedProofFailed,
    SignedProofPayload, TypedEvent, VerificationKind, VerifyShareDecryptionProofsRequest,
    VerifyShareProofsRequest, ZkRequest, ZkResponse,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// Cached C4 return commitments for a single party.
#[derive(Debug, Clone)]
struct C4Commitments {
    /// C4a: commitment to aggregated SK shares.
    sk_commitment: ArcBytes,
    /// C4b: commitment(s) to aggregated ESM shares, one per ESI.
    e_sm_commitments: Vec<ArcBytes>,
}

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

/// Actor that handles C2/C3/C4 share proof verification.
///
/// Separates ECDSA validation (lightweight, done inline) from ZK proof
/// verification (heavyweight, delegated to multithread). Emits
/// [`SignedProofFailed`] for fault attribution and [`ShareVerificationComplete`]
/// with the final dishonest party set.
pub struct ShareVerificationActor {
    bus: BusHandle,
    /// Tracks pending verifications by correlation ID.
    pending: HashMap<CorrelationId, PendingVerification>,
    /// Cached C4 return commitments per party, keyed by E3 ID.
    /// Populated after C4 verification passes; consumed during C6 verification.
    c4_cache: HashMap<E3id, HashMap<u64, C4Commitments>>,
}

impl ShareVerificationActor {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            pending: HashMap::new(),
            c4_cache: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.subscribe(EventType::ShareVerificationDispatched, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
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
            VerificationKind::ShareProofs | VerificationKind::PkGenerationProofs => {
                let kind = msg.kind.clone();
                self.verify_proofs(
                    e3_id,
                    kind.clone(),
                    msg.share_proofs,
                    msg.pre_dishonest,
                    ec,
                    |passed, corr_id, e3| {
                        ComputeRequest::zk(
                            ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                                party_proofs: passed,
                            }),
                            corr_id,
                            e3,
                        )
                    },
                );
            }
            VerificationKind::ThresholdDecryptionProofs => {
                // C4→C6 cross-check: compare C6 expected commitments against cached C4 values.
                // Mismatched parties are added to pre_dishonest before ZK dispatch.
                let mut pre_dishonest = msg.pre_dishonest;
                for party in &msg.share_proofs {
                    if let Some(mismatch_signed) = self.check_c6_party_against_c4(&e3_id, party) {
                        pre_dishonest.insert(party.sender_party_id);
                        self.emit_signed_proof_failed(
                            &e3_id,
                            &mismatch_signed,
                            None,
                            party.sender_party_id,
                            &ec,
                        );
                    }
                }

                // Filter out parties already marked dishonest by the cross-check
                // to avoid wasting ZK verification on them.
                let share_proofs: Vec<_> = msg
                    .share_proofs
                    .into_iter()
                    .filter(|p| !pre_dishonest.contains(&p.sender_party_id))
                    .collect();

                self.verify_proofs(
                    e3_id,
                    VerificationKind::ThresholdDecryptionProofs,
                    share_proofs,
                    pre_dishonest,
                    ec,
                    |passed, corr_id, e3| {
                        ComputeRequest::zk(
                            ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                                party_proofs: passed,
                            }),
                            corr_id,
                            e3,
                        )
                    },
                );
            }
            VerificationKind::DecryptionProofs => {
                // Cache C4 return commitments for later C6 cross-check.
                self.cache_c4_commitments(&e3_id, &msg.decryption_proofs);

                self.verify_proofs(
                    e3_id,
                    VerificationKind::DecryptionProofs,
                    msg.decryption_proofs,
                    msg.pre_dishonest,
                    ec,
                    |passed, corr_id, e3| {
                        ComputeRequest::zk(
                            ZkRequest::VerifyShareDecryptionProofs(
                                VerifyShareDecryptionProofsRequest {
                                    party_proofs: passed,
                                },
                            ),
                            corr_id,
                            e3,
                        )
                    },
                );
            }
        }
    }

    /// Generic ECDSA + ZK verification: validates signed proofs for each party,
    /// then dispatches ZK verification for ECDSA-passed parties.
    fn verify_proofs<P: VerifiableParty>(
        &mut self,
        e3_id: E3id,
        kind: VerificationKind,
        party_proofs: Vec<P>,
        pre_dishonest: BTreeSet<u64>,
        ec: EventContext<Sequenced>,
        build_request: impl FnOnce(Vec<P>, CorrelationId, E3id) -> ComputeRequest,
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

        // Dispatch ZK-only verification to multithread
        let correlation_id = CorrelationId::new();
        let dispatched_party_ids: HashSet<u64> =
            ecdsa_passed_parties.iter().map(|p| p.party_id()).collect();

        // Compute proof hashes for ECDSA-passed parties (for ProofVerificationPassed on success)
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

        self.pending.insert(
            correlation_id,
            PendingVerification {
                e3_id: e3_id.clone(),
                kind: kind.clone(),
                ec: ec.clone(),
                ecdsa_dishonest,
                pre_dishonest,
                dispatched_party_ids,
                party_addresses,
                party_proof_hashes,
                party_public_signals,
            },
        );

        let request = build_request(ecdsa_passed_parties, correlation_id, e3_id.clone());

        if let Err(err) = self.bus.publish(request, ec.clone()) {
            error!("Failed to dispatch {} ZK verification: {err}", label);
            if let Some(pending) = self.pending.remove(&correlation_id) {
                let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
                all_dishonest.extend(pending.ecdsa_dishonest);
                // Dispatched parties were never ZK-verified — treat as dishonest
                all_dishonest.extend(pending.dispatched_party_ids);
                self.publish_complete(e3_id, kind, all_dishonest, ec);
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

    /// Cache C4 return commitments from decryption proofs for later C6 cross-check.
    fn cache_c4_commitments(
        &mut self,
        e3_id: &E3id,
        parties: &[PartyShareDecryptionProofsToVerify],
    ) {
        let cache = self.c4_cache.entry(e3_id.clone()).or_default();
        for party in parties {
            let sk = party
                .signed_sk_decryption_proof
                .payload
                .proof
                .extract_output("commitment");
            let esm: Vec<ArcBytes> = party
                .signed_e_sm_decryption_proofs
                .iter()
                .filter_map(|p| p.payload.proof.extract_output("commitment"))
                .collect();

            if let Some(sk_commitment) = sk {
                cache.insert(
                    party.sender_party_id,
                    C4Commitments {
                        sk_commitment,
                        e_sm_commitments: esm,
                    },
                );
            }
        }
        info!(
            "Cached C4 commitments for {} parties (E3 {})",
            cache.len(),
            e3_id
        );
    }

    /// Check one party's C6 expected commitments against cached C4 return values.
    /// Returns the first mismatched signed proof (for fault attribution), or None if OK.
    fn check_c6_party_against_c4(
        &self,
        e3_id: &E3id,
        party: &PartyProofsToVerify,
    ) -> Option<SignedProofPayload> {
        let c4_cache = self.c4_cache.get(e3_id)?;
        let c4 = c4_cache.get(&party.sender_party_id)?;
        let first_proof = party.signed_proofs.first()?;
        let proof = &first_proof.payload.proof;

        // Extract C6 expected commitments using the input layout
        let c6_sk = proof.extract_input("expected_sk_commitment")?;
        let c6_esm = proof.extract_input("expected_e_sm_commitment")?;

        if c4.sk_commitment[..] != c6_sk[..] {
            warn!(
                "C4→C6 SK commitment mismatch for party {}",
                party.sender_party_id
            );
            return Some(first_proof.clone());
        }

        if let Some(c4_esm) = c4.e_sm_commitments.first() {
            if c4_esm[..] != c6_esm[..] {
                warn!(
                    "C4→C6 ESM commitment mismatch for party {}",
                    party.sender_party_id
                );
                return Some(first_proof.clone());
            }
        }

        None
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

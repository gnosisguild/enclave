// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor for C2/C3/C4 share proof verification.
//!
//! Follows the same pattern as [`ProofVerificationActor`] (for C0/T0) — sits
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
use alloy::primitives::Address;
use e3_events::{
    BusHandle, ComputeRequest, ComputeResponse, ComputeResponseKind, CorrelationId, E3id,
    EnclaveEvent, EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType,
    PartyProofsToVerify, PartyShareDecryptionProofsToVerify, PartyVerificationResult, Sequenced,
    ShareVerificationComplete, ShareVerificationDispatched, SignedProofFailed, SignedProofPayload,
    TypedEvent, VerificationKind, VerifyShareDecryptionProofsRequest,
    VerifyShareDecryptionProofsResponse, VerifyShareProofsRequest, VerifyShareProofsResponse,
    ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// ECDSA validation result for a single party.
struct EcdsaPartyResult {
    sender_party_id: u64,
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
    /// Signed payloads for each party, indexed by party_id.
    /// Used for SignedProofFailed emission when ZK also fails.
    party_signed_payloads: HashMap<u64, Vec<SignedProofPayload>>,
    /// Recovered address for each party (from ECDSA step).
    party_addresses: HashMap<u64, Address>,
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
}

impl ShareVerificationActor {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            pending: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.subscribe(EventType::ShareVerificationDispatched, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        addr
    }

    fn handle_share_verification_dispatched(
        &mut self,
        msg: TypedEvent<ShareVerificationDispatched>,
    ) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        match msg.kind {
            VerificationKind::ShareProofs => {
                self.verify_share_proofs(e3_id, msg.share_proofs, msg.pre_dishonest, ec);
            }
            VerificationKind::DecryptionProofs => {
                self.verify_decryption_proofs(e3_id, msg.decryption_proofs, msg.pre_dishonest, ec);
            }
        }
    }

    /// C2/C3 verification: ECDSA check on each party, then dispatch ZK.
    fn verify_share_proofs(
        &mut self,
        e3_id: E3id,
        party_proofs: Vec<PartyProofsToVerify>,
        pre_dishonest: BTreeSet<u64>,
        ec: EventContext<Sequenced>,
    ) {
        let e3_id_str = e3_id.to_string();
        let mut ecdsa_dishonest = HashSet::new();
        let mut ecdsa_passed_parties = Vec::new();
        let mut party_signed_payloads: HashMap<u64, Vec<SignedProofPayload>> = HashMap::new();
        let mut party_addresses: HashMap<u64, Address> = HashMap::new();

        for party in &party_proofs {
            let result = self.ecdsa_validate_signed_proofs(
                party.sender_party_id,
                &party.signed_proofs,
                &e3_id_str,
                "C2/C3",
            );
            party_signed_payloads.insert(party.sender_party_id, party.signed_proofs.clone());
            if result.passed {
                ecdsa_passed_parties.push(party.clone());
                if let Some((_, Some(addr))) = &result.failed_payload {
                    party_addresses.insert(party.sender_party_id, *addr);
                }
            } else {
                ecdsa_dishonest.insert(party.sender_party_id);
                // Emit SignedProofFailed for ECDSA failure
                if let Some((ref signed, addr)) = result.failed_payload {
                    self.emit_signed_proof_failed(&e3_id, signed, addr, &ec);
                }
            }
        }

        // Store recovered addresses for passed parties
        for party in &party_proofs {
            if !ecdsa_dishonest.contains(&party.sender_party_id) {
                if let Some(first_signed) = party.signed_proofs.first() {
                    if let Ok(addr) = first_signed.recover_address() {
                        party_addresses.insert(party.sender_party_id, addr);
                    }
                }
            }
        }

        if ecdsa_passed_parties.is_empty() {
            // All parties failed ECDSA — publish result immediately
            let mut all_dishonest: BTreeSet<u64> = pre_dishonest;
            all_dishonest.extend(ecdsa_dishonest);
            self.publish_complete(e3_id, VerificationKind::ShareProofs, all_dishonest, ec);
            return;
        }

        // Dispatch ZK-only verification to multithread
        let correlation_id = CorrelationId::new();
        self.pending.insert(
            correlation_id,
            PendingVerification {
                e3_id: e3_id.clone(),
                kind: VerificationKind::ShareProofs,
                ec: ec.clone(),
                ecdsa_dishonest,
                pre_dishonest,
                party_signed_payloads,
                party_addresses,
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                party_proofs: ecdsa_passed_parties,
            }),
            correlation_id,
            e3_id,
        );

        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to dispatch ZK verification: {err}");
            self.pending.remove(&correlation_id);
        }
    }

    /// C4 verification: ECDSA check on each party, then dispatch ZK.
    fn verify_decryption_proofs(
        &mut self,
        e3_id: E3id,
        party_proofs: Vec<PartyShareDecryptionProofsToVerify>,
        pre_dishonest: BTreeSet<u64>,
        ec: EventContext<Sequenced>,
    ) {
        let e3_id_str = e3_id.to_string();
        let mut ecdsa_dishonest = HashSet::new();
        let mut ecdsa_passed_parties = Vec::new();
        let mut party_signed_payloads: HashMap<u64, Vec<SignedProofPayload>> = HashMap::new();
        let mut party_addresses: HashMap<u64, Address> = HashMap::new();

        for party in &party_proofs {
            // Flatten all signed proofs (SK + ESMs)
            let all_signed: Vec<&SignedProofPayload> =
                std::iter::once(&party.signed_sk_decryption_proof)
                    .chain(party.signed_esm_decryption_proofs.iter())
                    .collect();
            let all_signed_cloned: Vec<SignedProofPayload> =
                all_signed.iter().map(|s| (*s).clone()).collect();

            let result = self.ecdsa_validate_signed_proofs(
                party.sender_party_id,
                &all_signed_cloned,
                &e3_id_str,
                "C4",
            );
            party_signed_payloads.insert(party.sender_party_id, all_signed_cloned);

            if result.passed {
                ecdsa_passed_parties.push(party.clone());
                if let Some((_, Some(addr))) = &result.failed_payload {
                    party_addresses.insert(party.sender_party_id, *addr);
                }
            } else {
                ecdsa_dishonest.insert(party.sender_party_id);
                if let Some((ref signed, addr)) = result.failed_payload {
                    self.emit_signed_proof_failed(&e3_id, signed, addr, &ec);
                }
            }
        }

        // Store recovered addresses for passed parties
        for party in &party_proofs {
            if !ecdsa_dishonest.contains(&party.sender_party_id) {
                if let Ok(addr) = party.signed_sk_decryption_proof.recover_address() {
                    party_addresses.insert(party.sender_party_id, addr);
                }
            }
        }

        if ecdsa_passed_parties.is_empty() {
            let mut all_dishonest: BTreeSet<u64> = pre_dishonest;
            all_dishonest.extend(ecdsa_dishonest);
            self.publish_complete(e3_id, VerificationKind::DecryptionProofs, all_dishonest, ec);
            return;
        }

        let correlation_id = CorrelationId::new();
        self.pending.insert(
            correlation_id,
            PendingVerification {
                e3_id: e3_id.clone(),
                kind: VerificationKind::DecryptionProofs,
                ec: ec.clone(),
                ecdsa_dishonest,
                pre_dishonest,
                party_signed_payloads,
                party_addresses,
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::VerifyShareDecryptionProofs(VerifyShareDecryptionProofsRequest {
                party_proofs: ecdsa_passed_parties,
            }),
            correlation_id,
            e3_id,
        );

        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to dispatch C4 ZK verification: {err}");
            self.pending.remove(&correlation_id);
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
                    sender_party_id,
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
                                sender_party_id,
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
                        sender_party_id,
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
                    sender_party_id,
                    passed: false,
                    failed_payload: Some((signed.clone(), expected_addr)),
                };
            }
        }

        EcdsaPartyResult {
            sender_party_id,
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
                VerificationKind::ShareProofs,
                ComputeResponseKind::Zk(ZkResponse::VerifyShareProofs(r)),
            ) => r.party_results,
            (
                VerificationKind::DecryptionProofs,
                ComputeResponseKind::Zk(ZkResponse::VerifyShareDecryptionProofs(r)),
            ) => r.party_results,
            _ => {
                error!("Unexpected ComputeResponse kind for verification");
                return;
            }
        };

        let mut all_dishonest: BTreeSet<u64> = pending.pre_dishonest;
        all_dishonest.extend(&pending.ecdsa_dishonest);

        for result in &zk_results {
            if !result.all_verified {
                all_dishonest.insert(result.sender_party_id);

                // Emit SignedProofFailed for ZK failure
                if let Some(ref signed) = result.failed_signed_payload {
                    let addr = pending
                        .party_addresses
                        .get(&result.sender_party_id)
                        .copied();
                    self.emit_signed_proof_failed(&pending.e3_id, signed, addr, &pending.ec);
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
        ec: &EventContext<Sequenced>,
    ) {
        let faulting_node = match recovered_addr {
            Some(addr) => addr,
            None => match signed_payload.recover_address() {
                Ok(addr) => addr,
                Err(err) => {
                    warn!("Cannot attribute fault — signature recovery failed: {err}");
                    return;
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

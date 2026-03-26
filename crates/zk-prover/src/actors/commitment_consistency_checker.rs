// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor that cross-checks commitment values across different circuit proofs.
//!
//! Subscribes to [`ProofVerificationPassed`] events and, for each registered
//! [`CommitmentLink`], compares commitment field values extracted from public
//! signals of related proof types.
//!
//! ## Architecture
//!
//! - Caches verified proof outputs keyed by `(Address, ProofType)`.
//! - On each new event, evaluates every registered link to see if both sides
//!   (source and target) are now available.
//! - For **same-party** links, compares proofs from the same Ethereum address.
//! - For **cross-party** links (e.g. per-node C1 vs aggregator C5), checks all
//!   cached source proofs against the newly arrived target (or vice versa).
//! - Logs warnings on mismatch. Future iterations may emit an accusation event.

use super::commitment_links::{CommitmentLink, LinkScope};
use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::Address;
use e3_events::{
    BusHandle, E3id, EnclaveEvent, EnclaveEventData, EventPublisher, EventSubscriber, EventType,
    ProofType, ProofVerificationPassed, SignedProofFailed, SignedProofPayload, TypedEvent,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Cached data from a verified proof.
struct VerifiedProofData {
    party_id: u64,
    address: Address,
    public_signals: ArcBytes,
    signed_payload: SignedProofPayload,
}

/// Per-E3 actor that enforces cross-circuit commitment consistency.
pub struct CommitmentConsistencyChecker {
    bus: BusHandle,
    e3_id: E3id,
    links: Vec<Box<dyn CommitmentLink>>,
    /// Verified proof outputs: `(address, proof_type) → data`.
    ///
    /// For cross-party links the target proof type may come from a different
    /// address than the source, so lookups iterate over all entries whose
    /// `proof_type` matches.
    verified: HashMap<(Address, ProofType), VerifiedProofData>,
}

impl CommitmentConsistencyChecker {
    pub fn new(bus: &BusHandle, e3_id: E3id, links: Vec<Box<dyn CommitmentLink>>) -> Self {
        Self {
            bus: bus.clone(),
            e3_id,
            links,
            verified: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, e3_id: E3id, links: Vec<Box<dyn CommitmentLink>>) -> Addr<Self> {
        let actor = Self::new(bus, e3_id, links);
        let addr = actor.start();
        bus.subscribe(EventType::ProofVerificationPassed, addr.clone().into());
        addr
    }

    /// Emit SignedProofFailed for a party whose proof is inconsistent.
    fn emit_fault(
        &self,
        data: &VerifiedProofData,
        ec: &e3_events::EventContext<e3_events::Sequenced>,
    ) {
        if let Err(e) = self.bus.publish(
            SignedProofFailed {
                e3_id: self.e3_id.clone(),
                faulting_node: data.address,
                proof_type: data.signed_payload.payload.proof_type,
                signed_payload: data.signed_payload.clone(),
            },
            ec.clone(),
        ) {
            error!("Failed to publish SignedProofFailed: {e}");
        }
    }

    /// Evaluate all registered links given a newly arrived proof.
    fn check_links(
        &self,
        new_proof_type: ProofType,
        new_address: Address,
        ec: &e3_events::EventContext<e3_events::Sequenced>,
    ) {
        for link in &self.links {
            match link.scope() {
                LinkScope::SameParty => {
                    self.check_same_party_link(link.as_ref(), new_proof_type, new_address, ec);
                }
                LinkScope::CrossParty => {
                    self.check_cross_party_link(link.as_ref(), new_proof_type, ec);
                }
            }
        }
    }

    /// Same-party: compare source and target from the same address.
    fn check_same_party_link(
        &self,
        link: &dyn CommitmentLink,
        new_proof_type: ProofType,
        address: Address,
        ec: &e3_events::EventContext<e3_events::Sequenced>,
    ) {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();

        // Only run when the newly arrived proof completes a pair.
        if new_proof_type != src_type && new_proof_type != tgt_type {
            return;
        }

        let source = self.verified.get(&(address, src_type));
        let target = self.verified.get(&(address, tgt_type));

        if let (Some(src), Some(tgt)) = (source, target) {
            let source_values = link.extract_source_values(&src.public_signals);
            if !link.check_consistency(&source_values, &tgt.public_signals) {
                warn!(
                    "[{}] Commitment mismatch for E3 {} — party {} ({}): \
                     source {:?} vs target {:?} from same address",
                    link.name(),
                    self.e3_id,
                    src.party_id,
                    address,
                    src_type,
                    tgt_type,
                );
                // Report the target proof as faulting — its inputs don't match
                // the source's outputs.
                self.emit_fault(tgt, ec);
            }
        }
    }

    /// Cross-party: check all cached sources against the target (or the new
    /// source against all cached targets).
    fn check_cross_party_link(
        &self,
        link: &dyn CommitmentLink,
        new_proof_type: ProofType,
        ec: &e3_events::EventContext<e3_events::Sequenced>,
    ) {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();

        if new_proof_type != src_type && new_proof_type != tgt_type {
            return;
        }

        // Collect all entries matching the source proof type.
        let sources: Vec<&VerifiedProofData> = self
            .verified
            .iter()
            .filter(|((_, pt), _)| *pt == src_type)
            .map(|(_, v)| v)
            .collect();

        // Collect all entries matching the target proof type.
        let targets: Vec<&VerifiedProofData> = self
            .verified
            .iter()
            .filter(|((_, pt), _)| *pt == tgt_type)
            .map(|(_, v)| v)
            .collect();

        // For each (source, target) pair, check consistency.
        for src in &sources {
            let source_values = link.extract_source_values(&src.public_signals);
            if source_values.is_empty() {
                continue;
            }
            for tgt in &targets {
                if !link.check_consistency(&source_values, &tgt.public_signals) {
                    warn!(
                        "[{}] Commitment mismatch for E3 {} — source party {} ({}) {:?} \
                         not consistent with target party {} ({}) {:?}",
                        link.name(),
                        self.e3_id,
                        src.party_id,
                        src.address,
                        src_type,
                        tgt.party_id,
                        tgt.address,
                        tgt_type,
                    );
                    self.emit_fault(src, ec);
                }
            }
        }
    }
}

impl Actor for CommitmentConsistencyChecker {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!(
            "CommitmentConsistencyChecker started for E3 {} with {} link(s)",
            self.e3_id,
            self.links.len()
        );
    }
}

impl Handler<EnclaveEvent> for CommitmentConsistencyChecker {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        if let EnclaveEventData::ProofVerificationPassed(data) = msg {
            self.notify_sync(ctx, TypedEvent::new(data, ec));
        }
    }
}

impl Handler<TypedEvent<ProofVerificationPassed>> for CommitmentConsistencyChecker {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ProofVerificationPassed>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();

        let proof_type = data.proof_type;
        let address = data.address;
        let public_signals = data.public_signals;
        let signed_payload = data.signed_payload;

        self.verified.insert(
            (address, proof_type),
            VerifiedProofData {
                party_id: data.party_id,
                address,
                public_signals,
                signed_payload,
            },
        );

        self.check_links(proof_type, address, &ec);
    }
}

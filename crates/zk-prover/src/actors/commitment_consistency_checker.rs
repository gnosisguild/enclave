// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor that cross-checks commitment values across different circuit proofs.
//!
//! Has two roles:
//!
//! 1. **Pre-ZK gating** (request/response): Subscribes to
//!    [`CommitmentConsistencyCheckRequested`] from [`ShareVerificationActor`],
//!    caches each party's public signals, evaluates all registered
//!    [`CommitmentLink`]s, and responds with
//!    [`CommitmentConsistencyCheckComplete`]. Inconsistent parties are excluded
//!    from ZK verification.
//!
//! 2. **Post-ZK cross-circuit checking**: Subscribes to
//!    [`ProofVerificationPassed`] events and, for each registered link,
//!    compares commitment values across different circuit proofs. On mismatch,
//!    publishes [`CommitmentConsistencyViolation`] for the accusation pipeline.
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
    BusHandle, CommitmentConsistencyCheckComplete, CommitmentConsistencyCheckRequested,
    CommitmentConsistencyViolation, E3id, EnclaveEvent, EnclaveEventData, EventContext,
    EventPublisher, EventSubscriber, EventType, ProofType, ProofVerificationPassed, Sequenced,
    TypedEvent,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use std::collections::{BTreeSet, HashMap};
use tracing::{error, info, warn};

/// Cached data from a verified proof.
struct VerifiedProofData {
    party_id: u64,
    address: Address,
    public_signals: ArcBytes,
    data_hash: [u8; 32],
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
        bus.subscribe(
            EventType::CommitmentConsistencyCheckRequested,
            addr.clone().into(),
        );
        bus.subscribe(EventType::ProofVerificationPassed, addr.clone().into());
        addr
    }

    /// Evaluate all registered links given a newly arrived proof.
    fn check_links(
        &self,
        new_proof_type: ProofType,
        new_address: Address,
        ec: &EventContext<Sequenced>,
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
        ec: &EventContext<Sequenced>,
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
                self.emit_violation(src.party_id, address, src_type, src.data_hash, ec);
            }
        }
    }

    /// Cross-party: check all cached sources against the target (or the new
    /// source against all cached targets).
    fn check_cross_party_link(
        &self,
        link: &dyn CommitmentLink,
        new_proof_type: ProofType,
        ec: &EventContext<Sequenced>,
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
                    self.emit_violation(src.party_id, src.address, src_type, src.data_hash, ec);
                }
            }
        }
    }

    /// Check if a same-party link has a mismatch for the given address.
    /// Returns `true` if both sides are cached AND inconsistent.
    fn check_same_party_mismatch(&self, link: &dyn CommitmentLink, address: Address) -> bool {
        let source = self.verified.get(&(address, link.source_proof_type()));
        let target = self.verified.get(&(address, link.target_proof_type()));
        if let (Some(src), Some(tgt)) = (source, target) {
            let source_values = link.extract_source_values(&src.public_signals);
            !link.check_consistency(&source_values, &tgt.public_signals)
        } else {
            false
        }
    }

    /// Check if a cross-party link has a mismatch involving the given party.
    /// Returns `true` if any (source, target) pair is inconsistent where
    /// the source party_id matches `party_id`.
    fn check_cross_party_mismatch(&self, link: &dyn CommitmentLink, party_id: u64) -> bool {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();

        let sources: Vec<&VerifiedProofData> = self
            .verified
            .iter()
            .filter(|((_, pt), v)| *pt == src_type && v.party_id == party_id)
            .map(|(_, v)| v)
            .collect();

        let targets: Vec<&VerifiedProofData> = self
            .verified
            .iter()
            .filter(|((_, pt), _)| *pt == tgt_type)
            .map(|(_, v)| v)
            .collect();

        for src in &sources {
            let source_values = link.extract_source_values(&src.public_signals);
            if source_values.is_empty() {
                continue;
            }
            for tgt in &targets {
                if !link.check_consistency(&source_values, &tgt.public_signals) {
                    return true;
                }
            }
        }
        false
    }

    /// Publish a [`CommitmentConsistencyViolation`] for the accusation pipeline.
    fn emit_violation(
        &self,
        accused_party_id: u64,
        accused_address: Address,
        proof_type: ProofType,
        data_hash: [u8; 32],
        ec: &EventContext<Sequenced>,
    ) {
        let violation = CommitmentConsistencyViolation {
            e3_id: self.e3_id.clone(),
            accused_party_id,
            accused_address,
            proof_type,
            data_hash,
        };
        if let Err(err) = self.bus.publish(violation, ec.clone()) {
            error!("Failed to publish CommitmentConsistencyViolation: {err}");
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
        match msg {
            EnclaveEventData::CommitmentConsistencyCheckRequested(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ProofVerificationPassed(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
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

        self.verified.insert(
            (address, proof_type),
            VerifiedProofData {
                party_id: data.party_id,
                address,
                public_signals: data.public_signals,
                data_hash: data.data_hash,
            },
        );

        self.check_links(proof_type, address, &ec);
    }
}

impl Handler<TypedEvent<CommitmentConsistencyCheckRequested>> for CommitmentConsistencyChecker {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitmentConsistencyCheckRequested>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();

        let mut inconsistent_parties = BTreeSet::new();

        // Cache each party's proof data and evaluate links.
        for party in &data.party_proofs {
            for (proof_type, public_signals) in &party.proofs {
                // Cache for link evaluation (use zero data_hash — pre-ZK data
                // doesn't have a meaningful hash yet; violations use the
                // post-ZK ProofVerificationPassed data_hash instead).
                self.verified.insert(
                    (party.address, *proof_type),
                    VerifiedProofData {
                        party_id: party.party_id,
                        address: party.address,
                        public_signals: public_signals.clone(),
                        data_hash: [0u8; 32],
                    },
                );
            }
        }

        // Now evaluate links for each party's newly cached proofs.
        for party in &data.party_proofs {
            for (proof_type, _) in &party.proofs {
                for link in &self.links {
                    let is_relevant = *proof_type == link.source_proof_type()
                        || *proof_type == link.target_proof_type();
                    if !is_relevant {
                        continue;
                    }

                    let mismatch = match link.scope() {
                        LinkScope::SameParty => {
                            self.check_same_party_mismatch(link.as_ref(), party.address)
                        }
                        LinkScope::CrossParty => {
                            self.check_cross_party_mismatch(link.as_ref(), party.party_id)
                        }
                    };

                    if mismatch {
                        warn!(
                            "[{}] Pre-ZK commitment mismatch for E3 {} — party {} ({})",
                            link.name(),
                            self.e3_id,
                            party.party_id,
                            party.address,
                        );
                        inconsistent_parties.insert(party.party_id);
                    }
                }
            }
        }

        // Respond to ShareVerificationActor.
        if let Err(err) = self.bus.publish(
            CommitmentConsistencyCheckComplete {
                e3_id: data.e3_id,
                kind: data.kind,
                correlation_id: data.correlation_id,
                inconsistent_parties,
            },
            ec,
        ) {
            error!("Failed to publish CommitmentConsistencyCheckComplete: {err}");
        }
    }
}

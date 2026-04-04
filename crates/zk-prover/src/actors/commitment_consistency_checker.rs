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

/// Describes a source entry whose commitments are inconsistent with a target.
struct Mismatch {
    party_id: u64,
    address: Address,
    proof_type: ProofType,
    data_hash: [u8; 32],
}

/// Per-E3 actor that enforces cross-circuit commitment consistency.
pub struct CommitmentConsistencyChecker {
    bus: BusHandle,
    e3_id: E3id,
    links: Vec<Box<dyn CommitmentLink>>,
    /// Verified proof outputs: `(address, proof_type) → data`.
    /// Multiple proofs per key are supported (e.g. N-1 C3a proofs per sender).
    verified: HashMap<(Address, ProofType), Vec<VerifiedProofData>>,
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

    /// Insert a proof into the cache, deduplicating by `data_hash` to avoid
    /// double-counting when the same proof arrives via both the pre-ZK batch
    /// and the post-ZK `ProofVerificationPassed` path.
    fn insert_verified(
        &mut self,
        address: Address,
        proof_type: ProofType,
        data: VerifiedProofData,
    ) {
        let entries = self.verified.entry((address, proof_type)).or_default();
        if !entries.iter().any(|e| e.data_hash == data.data_hash) {
            entries.push(data);
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

    /// Find all source entries whose commitments are inconsistent with cached
    /// targets for a given link.
    fn find_mismatches(&self, link: &dyn CommitmentLink) -> Vec<Mismatch> {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();

        match link.scope() {
            // Same address: each source entry must be consistent with each
            // target entry from the same address.
            LinkScope::SameParty => {
                let mut mismatches = Vec::new();
                for ((addr, pt), srcs) in &self.verified {
                    if *pt != src_type {
                        continue;
                    }
                    let Some(tgts) = self.verified.get(&(*addr, tgt_type)) else {
                        continue;
                    };
                    for src in srcs {
                        let vals = link.extract_source_values(&src.public_signals);
                        for tgt in tgts {
                            if !link.check_consistency(&vals, &tgt.public_signals) {
                                mismatches.push(Mismatch {
                                    party_id: src.party_id,
                                    address: *addr,
                                    proof_type: src_type,
                                    data_hash: src.data_hash,
                                });
                                break; // one mismatch per source entry is enough
                            }
                        }
                    }
                }
                mismatches
            }

            // Cross-party: each source's extracted value must appear in at
            // least one target's public signals. Fault the source if no match.
            // If no targets are cached yet, skip — the check will run again
            // when a target arrives.
            LinkScope::CrossParty => {
                let all_targets: Vec<&VerifiedProofData> = self
                    .verified
                    .iter()
                    .filter(|((_, pt), _)| *pt == tgt_type)
                    .flat_map(|(_, entries)| entries)
                    .collect();

                if all_targets.is_empty() {
                    return Vec::new();
                }

                let mut mismatches = Vec::new();
                for ((_, pt), srcs) in &self.verified {
                    if *pt != src_type {
                        continue;
                    }
                    for src in srcs {
                        let vals = link.extract_source_values(&src.public_signals);
                        if vals.is_empty() {
                            continue;
                        }
                        // Source must match AT LEAST ONE target.
                        let found = all_targets
                            .iter()
                            .any(|tgt| link.check_consistency(&vals, &tgt.public_signals));
                        if !found {
                            mismatches.push(Mismatch {
                                party_id: src.party_id,
                                address: src.address,
                                proof_type: src_type,
                                data_hash: src.data_hash,
                            });
                        }
                    }
                }
                mismatches
            }

            // Each source claims a value that must exist among any target's
            // outputs. Fault the source (e.g. C3) when no target (e.g. C0)
            // matches. If no targets are cached yet, skip — the check will
            // run when a target arrives via post-ZK ProofVerificationPassed.
            LinkScope::SourceMustExistInTargets => {
                let all_targets: Vec<&VerifiedProofData> = self
                    .verified
                    .iter()
                    .filter(|((_, pt), _)| *pt == tgt_type)
                    .flat_map(|(_, entries)| entries)
                    .collect();

                if all_targets.is_empty() {
                    return Vec::new();
                }

                let mut mismatches = Vec::new();
                for ((_, pt), srcs) in &self.verified {
                    if *pt != src_type {
                        continue;
                    }
                    for src in srcs {
                        let vals = link.extract_source_values(&src.public_signals);
                        if vals.is_empty() {
                            continue;
                        }
                        let found = all_targets
                            .iter()
                            .any(|tgt| link.check_consistency(&vals, &tgt.public_signals));
                        if !found {
                            mismatches.push(Mismatch {
                                party_id: src.party_id,
                                address: src.address,
                                proof_type: src_type,
                                data_hash: src.data_hash,
                            });
                        }
                    }
                }
                mismatches
            }
        }
    }

    /// Post-ZK: evaluate links relevant to a newly arrived proof and emit
    /// violations on mismatch.
    fn check_links(&self, new_proof_type: ProofType, ec: &EventContext<Sequenced>) {
        for link in &self.links {
            if new_proof_type != link.source_proof_type()
                && new_proof_type != link.target_proof_type()
            {
                continue;
            }
            for m in self.find_mismatches(link.as_ref()) {
                // Defense-in-depth: skip entries with unresolved data_hash
                // (should not happen now that pre-ZK caching uses real hashes,
                // but guards against future regressions).
                if m.data_hash == [0u8; 32] {
                    warn!(
                        "[{}] Skipping mismatch with zero data_hash for party {} ({}) {:?}",
                        link.name(),
                        m.party_id,
                        m.address,
                        m.proof_type,
                    );
                    continue;
                }
                warn!(
                    "[{}] Commitment mismatch for E3 {} — party {} ({}) {:?}",
                    link.name(),
                    self.e3_id,
                    m.party_id,
                    m.address,
                    m.proof_type,
                );
                self.emit_violation(m.party_id, m.address, m.proof_type, m.data_hash, ec);
            }
        }
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

        self.insert_verified(
            address,
            proof_type,
            VerifiedProofData {
                party_id: data.party_id,
                address,
                public_signals: data.public_signals,
                data_hash: data.data_hash,
            },
        );

        self.check_links(proof_type, &ec);
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

        // Cache each party's proof data for link evaluation.
        for party in &data.party_proofs {
            for (proof_type, public_signals, data_hash) in &party.proofs {
                self.insert_verified(
                    party.address,
                    *proof_type,
                    VerifiedProofData {
                        party_id: party.party_id,
                        address: party.address,
                        public_signals: public_signals.clone(),
                        data_hash: *data_hash,
                    },
                );
            }
        }

        // Evaluate every link and collect inconsistent parties.
        // Also emit violations so AccusationManager can initiate the quorum
        // protocol — parties excluded pre-ZK would otherwise never trigger a
        // post-ZK violation.
        for link in &self.links {
            for m in self.find_mismatches(link.as_ref()) {
                warn!(
                    "[{}] Pre-ZK commitment mismatch for E3 {} — party {} ({})",
                    link.name(),
                    self.e3_id,
                    m.party_id,
                    m.address,
                );
                inconsistent_parties.insert(m.party_id);
                self.emit_violation(m.party_id, m.address, m.proof_type, m.data_hash, &ec);
            }
        }

        // Remove cached entries for inconsistent parties so they don't
        // participate in future post-ZK `find_mismatches` evaluations.
        if !inconsistent_parties.is_empty() {
            self.verified.retain(|_, entries| {
                entries.retain(|v| !inconsistent_parties.contains(&v.party_id));
                !entries.is_empty()
            });
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

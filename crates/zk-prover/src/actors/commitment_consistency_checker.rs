// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor that cross-checks commitment values across different circuit proofs.
//!
//! ## Architecture
//!
//! - Receives [`CommitmentConsistencyCheckRequested`] from the
//!   [`ShareVerificationActor`] after ECDSA validation but **before** ZK
//!   proof verification.
//! - Caches the ECDSA-validated proof outputs keyed by `(Address, ProofType)`.
//! - Evaluates every registered [`CommitmentLink`] to detect cross-circuit
//!   commitment mismatches.
//! - Responds with [`CommitmentConsistencyCheckComplete`] carrying the set of
//!   inconsistent party IDs. Only parties not in this set proceed to ZK
//!   verification.
//!
//! ## Link types
//!
//! - **Same-party** links compare proofs from the same Ethereum address across
//!   different circuits (e.g. C1 vs C5 from the same node).
//! - **Cross-party** links compare proofs from different addresses (e.g.
//!   per-node C1 vs aggregator C5).
//!
//! ## Lifecycle
//!
//! One instance per E3, created by [`CommitmentConsistencyCheckerExtension`]
//! when [`CommitteeFinalized`] fires. Proof data accumulates across
//! verification phases (C1, C2/C3, C4, C6) so that cross-phase links
//! (e.g. C1→C5) can be evaluated when the target circuit's data arrives.

use super::commitment_links::{CommitmentLink, LinkScope};
use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::Address;
use e3_events::{
    BusHandle, CommitmentConsistencyCheckComplete, CommitmentConsistencyCheckRequested,
    CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EventPublisher, EventSubscriber,
    EventType, ProofType, TypedEvent,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use std::collections::{BTreeSet, HashMap, HashSet};
use tracing::{error, info, warn};

/// Cached data from an ECDSA-validated (pre-ZK) proof.
struct CachedProofData {
    party_id: u64,
    address: Address,
    public_signals: ArcBytes,
}

/// Per-E3 actor that enforces cross-circuit commitment consistency.
pub struct CommitmentConsistencyChecker {
    bus: BusHandle,
    e3_id: E3id,
    links: Vec<Box<dyn CommitmentLink>>,
    /// Proof outputs keyed by `(address, proof_type)`.
    ///
    /// For cross-party links the target proof type may come from a different
    /// address than the source, so lookups iterate over all entries whose
    /// `proof_type` matches.
    cached: HashMap<(Address, ProofType), CachedProofData>,
    /// Correlation IDs already processed (idempotency guard against double
    /// delivery from both bus subscription and E3Context forwarding).
    processed: HashSet<CorrelationId>,
}

impl CommitmentConsistencyChecker {
    pub fn new(bus: &BusHandle, e3_id: E3id, links: Vec<Box<dyn CommitmentLink>>) -> Self {
        Self {
            bus: bus.clone(),
            e3_id,
            links,
            cached: HashMap::new(),
            processed: HashSet::new(),
        }
    }

    pub fn setup(bus: &BusHandle, e3_id: E3id, links: Vec<Box<dyn CommitmentLink>>) -> Addr<Self> {
        let actor = Self::new(bus, e3_id, links);
        let addr = actor.start();
        bus.subscribe(
            EventType::CommitmentConsistencyCheckRequested,
            addr.clone().into(),
        );
        addr
    }

    /// Handle a consistency check request: cache proof data, evaluate links,
    /// and respond with the set of inconsistent parties.
    fn handle_check_requested(&mut self, msg: TypedEvent<CommitmentConsistencyCheckRequested>) {
        let (data, ec) = msg.into_components();

        if data.e3_id != self.e3_id {
            return;
        }

        // Idempotency: skip if we already processed this correlation_id.
        if !self.processed.insert(data.correlation_id) {
            return;
        }

        info!(
            "CommitmentConsistencyChecker: processing check for E3 {} kind {:?} ({} parties)",
            self.e3_id,
            data.kind,
            data.party_proofs.len()
        );

        // Cache all proof data from the request.
        for party in &data.party_proofs {
            for (proof_type, public_signals) in &party.proofs {
                self.cached.insert(
                    (party.address, *proof_type),
                    CachedProofData {
                        party_id: party.party_id,
                        address: party.address,
                        public_signals: public_signals.clone(),
                    },
                );
            }
        }

        // Evaluate all links against the full cache.
        let inconsistent = self.check_all_links();

        if inconsistent.is_empty() {
            info!(
                "CommitmentConsistencyChecker: all links consistent for E3 {} {:?}",
                self.e3_id, data.kind
            );
        } else {
            warn!(
                "CommitmentConsistencyChecker: {} inconsistent parties for E3 {} {:?}: {:?}",
                inconsistent.len(),
                self.e3_id,
                data.kind,
                inconsistent
            );
        }

        if let Err(err) = self.bus.publish(
            CommitmentConsistencyCheckComplete {
                e3_id: self.e3_id.clone(),
                kind: data.kind,
                correlation_id: data.correlation_id,
                inconsistent_parties: inconsistent,
            },
            ec,
        ) {
            error!("Failed to publish CommitmentConsistencyCheckComplete: {err}");
        }
    }

    /// Evaluate all registered links across all cached proofs.
    /// Returns the set of party IDs with at least one inconsistency.
    fn check_all_links(&self) -> BTreeSet<u64> {
        let mut inconsistent = BTreeSet::new();
        for link in &self.links {
            match link.scope() {
                LinkScope::SameParty => {
                    let src_type = link.source_proof_type();
                    let tgt_type = link.target_proof_type();
                    let addresses: BTreeSet<Address> = self
                        .cached
                        .keys()
                        .filter(|(_, pt)| *pt == src_type || *pt == tgt_type)
                        .map(|(addr, _)| *addr)
                        .collect();
                    for addr in addresses {
                        inconsistent.extend(self.check_same_party_link(link.as_ref(), addr));
                    }
                }
                LinkScope::CrossParty => {
                    inconsistent.extend(self.check_cross_party_link(link.as_ref()));
                }
            }
        }
        inconsistent
    }

    /// Same-party: compare source and target from the same address.
    fn check_same_party_link(&self, link: &dyn CommitmentLink, address: Address) -> BTreeSet<u64> {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();
        let mut inconsistent = BTreeSet::new();

        let source = self.cached.get(&(address, src_type));
        let target = self.cached.get(&(address, tgt_type));

        if let (Some(src), Some(tgt)) = (source, target) {
            let source_values = link.extract_source_values(&src.public_signals);
            if !link.check_consistency(&source_values, &tgt.public_signals) {
                warn!(
                    "[{}] Commitment mismatch for E3 {} — party {} ({}): \
                     source {:?} vs target {:?}",
                    link.name(),
                    self.e3_id,
                    src.party_id,
                    address,
                    src_type,
                    tgt_type,
                );
                inconsistent.insert(src.party_id);
            }
        }
        inconsistent
    }

    /// Cross-party: check all cached sources against all cached targets.
    fn check_cross_party_link(&self, link: &dyn CommitmentLink) -> BTreeSet<u64> {
        let src_type = link.source_proof_type();
        let tgt_type = link.target_proof_type();
        let mut inconsistent = BTreeSet::new();

        let sources: Vec<&CachedProofData> = self
            .cached
            .iter()
            .filter(|((_, pt), _)| *pt == src_type)
            .map(|(_, v)| v)
            .collect();

        let targets: Vec<&CachedProofData> = self
            .cached
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
                    inconsistent.insert(src.party_id);
                }
            }
        }
        inconsistent
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
                self.notify_sync(ctx, TypedEvent::new(data, ec));
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<CommitmentConsistencyCheckRequested>> for CommitmentConsistencyChecker {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitmentConsistencyCheckRequested>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_check_requested(msg);
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Off-chain accusation quorum protocol for fault attribution.
//!
//! When a node detects a ZK proof failure from another committee member, it
//! broadcasts a [`ProofFailureAccusation`] over gossip. Other committee members
//! independently check the same proof and respond with [`AccusationVote`]s.
//! Once a quorum of M (the cryptographic threshold) votes is reached, the
//! actor emits [`AccusationQuorumReached`] for downstream consumers (aggregator
//! exclusion, on-chain slash submission).
//!
//! ## Proof-type-specific behavior
//!
//! | Proof   | Attestation                | Notes                                      |
//! |---------|----------------------------|--------------------------------------------|
//! | C0      | All nodes independently    | Everyone receives via DHT                  |
//! | C1      | All nodes independently    | Bundled in ThresholdShareCreated            |
//! | C2a/C2b | All nodes independently    | Same proof bytes for all recipients         |
//! | C3a/C3b | Forwarding required        | Per-recipient; accuser forwards payload     |
//! | C4      | All nodes independently    | Broadcast via gossip                        |
//! | C5      | Committee attests          | Aggregator-generated; nodes verify off-chain|
//! | C6      | All nodes independently    | Broadcast via gossip                        |
//! | C7      | On-chain verification      | Not handled here (on-chain verifier)        |

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use actix::{Actor, Addr, AsyncContext, Context, Handler, SpawnHandle};
use alloy::primitives::{keccak256, Address, Bytes, U256};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::SignerSync;
use alloy::sol_types::SolValue;
use e3_events::{
    AccusationOutcome, AccusationQuorumReached, AccusationVote, BusHandle, ComputeRequest,
    ComputeRequestError, ComputeResponse, ComputeResponseKind, CorrelationId, E3id, EnclaveEvent,
    EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType,
    PartyProofsToVerify, ProofFailureAccusation, ProofType, ProofVerificationFailed,
    ProofVerificationPassed, Sequenced, SignedProofPayload, TypedEvent, VerifyShareProofsRequest,
    ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// How long to wait for votes before declaring the accusation inconclusive.
const DEFAULT_VOTE_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// An active accusation awaiting votes from committee members.
struct PendingAccusation {
    accusation: ProofFailureAccusation,
    votes_for: Vec<AccusationVote>,
    votes_against: Vec<AccusationVote>,
    /// Handle to the timeout future so it can be cancelled on early quorum.
    timeout_handle: Option<SpawnHandle>,
    /// The EventContext from when this accusation was created — used for timeout emission.
    ec: EventContext<Sequenced>,
}

/// Cached verification result for a proof from a specific (accused, proof_type) pair.
/// Populated as proofs are received and verified (pass or fail).
struct ReceivedProofData {
    data_hash: [u8; 32],
    /// `true` if our local verification passed, `false` if it failed.
    verification_passed: bool,
}

/// Tracks an in-flight ZK re-verification for a forwarded C3a/C3b proof.
struct PendingReVerification {
    accusation_id: [u8; 32],
    data_hash: [u8; 32],
    accused: Address,
    proof_type: ProofType,
}

/// Manages the off-chain accusation quorum protocol.
///
/// Subscribes to:
/// - [`ProofVerificationFailed`] — local proof failure detection
/// - [`ProofFailureAccusation`] — incoming accusations from other nodes via gossip
/// - [`AccusationVote`] — incoming votes from other nodes via gossip
///
/// Publishes:
/// - [`ProofFailureAccusation`] — broadcast own accusations via gossip
/// - [`AccusationVote`] — broadcast own votes via gossip
/// - [`AccusationQuorumReached`] — quorum decision for downstream consumers
pub struct AccusationManager {
    bus: BusHandle,
    e3_id: E3id,
    my_address: Address,
    signer: PrivateKeySigner,

    /// All committee member addresses for this E3.
    committee: Vec<Address>,
    /// Quorum threshold — matches the cryptographic threshold M.
    threshold_m: usize,

    /// Active accusations keyed by accusation_id (keccak256 of accusation fields).
    pending: HashMap<[u8; 32], PendingAccusation>,

    /// Dedup: (accused, proof_type) pairs we've already created an accusation for.
    /// Prevents duplicate accusations when multiple local failure events fire.
    accused_proofs: HashSet<(Address, ProofType)>,

    /// Cache of received data hashes per (accused, proof_type).
    /// Populated by ProofVerificationFailed (failures) and ProofVerificationPassed (successes)
    /// so the node can vote on accusations from other nodes.
    received_data: HashMap<(Address, ProofType), ReceivedProofData>,

    /// Votes received before the corresponding accusation — replayed on accusation arrival.
    buffered_votes: HashMap<[u8; 32], Vec<AccusationVote>>,

    /// In-flight C3a/C3b ZK re-verifications, keyed by CorrelationId.
    pending_reverifications: HashMap<CorrelationId, PendingReVerification>,

    /// Vote timeout duration.
    vote_timeout: Duration,
}

impl AccusationManager {
    pub fn new(
        bus: &BusHandle,
        e3_id: E3id,
        signer: PrivateKeySigner,
        committee: Vec<Address>,
        threshold_m: usize,
    ) -> Self {
        let my_address = signer.address();
        Self {
            bus: bus.clone(),
            e3_id,
            my_address,
            signer,
            committee,
            threshold_m,
            pending: HashMap::new(),
            accused_proofs: HashSet::new(),
            received_data: HashMap::new(),
            buffered_votes: HashMap::new(),
            pending_reverifications: HashMap::new(),
            vote_timeout: DEFAULT_VOTE_TIMEOUT,
        }
    }

    pub fn setup(
        bus: &BusHandle,
        e3_id: E3id,
        signer: PrivateKeySigner,
        committee: Vec<Address>,
        threshold_m: usize,
    ) -> Addr<Self> {
        let addr = Self::new(bus, e3_id, signer, committee, threshold_m).start();
        bus.subscribe(EventType::ProofVerificationFailed, addr.clone().into());
        bus.subscribe(EventType::ProofVerificationPassed, addr.clone().into());
        bus.subscribe(EventType::ProofFailureAccusation, addr.clone().into());
        bus.subscribe(EventType::AccusationVote, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        addr
    }

    // ─── Accusation ID computation ───────────────────────────────────────

    /// Compute a deterministic ID for an accusation based on its key fields.
    /// This ensures that the same (e3_id, accused, proof_type) produces the
    /// same ID regardless of who the accuser is, enabling deduplication.
    ///
    /// `keccak256(abi.encodePacked(chainId, e3Id, accused, proofType))`
    fn accusation_id(accusation: &ProofFailureAccusation) -> [u8; 32] {
        let e3_id_u256: U256 = accusation
            .e3_id
            .clone()
            .try_into()
            .expect("E3id should be valid U256");
        let msg = (
            U256::from(accusation.e3_id.chain_id()),
            e3_id_u256,
            accusation.accused,
            U256::from(accusation.proof_type as u8),
        )
            .abi_encode_packed();
        keccak256(&msg).into()
    }

    // ─── Signing / Verification ──────────────────────────────────────────

    fn sign_accusation_digest(&self, accusation: &ProofFailureAccusation) -> Vec<u8> {
        let digest = Self::accusation_digest(accusation);
        self.signer
            .sign_message_sync(&digest)
            .map(|sig| sig.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// Structured digest for ECDSA signing of accusations.
    ///
    /// Uses a typehash + `abi.encode` pattern matching `ProofPayload::digest()`:
    /// ```text
    /// keccak256(abi.encode(
    ///     ACCUSATION_TYPEHASH,
    ///     chainId, e3Id, accuser, accused, proofType,
    ///     dataHash
    /// ))
    /// ```
    fn accusation_digest(accusation: &ProofFailureAccusation) -> [u8; 32] {
        let e3_id_u256: U256 = accusation
            .e3_id
            .clone()
            .try_into()
            .expect("E3id should be valid U256");
        let typehash: [u8; 32] = keccak256(
            "ProofFailureAccusation(uint256 chainId,uint256 e3Id,address accuser,address accused,uint256 proofType,bytes32 dataHash)"
        ).into();
        let encoded = (
            typehash,
            U256::from(accusation.e3_id.chain_id()),
            e3_id_u256,
            accusation.accuser,
            accusation.accused,
            U256::from(accusation.proof_type as u8),
            accusation.data_hash,
        )
            .abi_encode();
        keccak256(&encoded).into()
    }

    fn verify_accusation_signature(&self, accusation: &ProofFailureAccusation) -> bool {
        let digest = Self::accusation_digest(accusation);
        let sig = match alloy::primitives::Signature::try_from(accusation.signature.as_slice()) {
            Ok(s) => s,
            Err(_) => return false,
        };
        match sig.recover_address_from_msg(&digest) {
            Ok(addr) => addr == accusation.accuser,
            Err(_) => false,
        }
    }

    fn sign_vote_digest(&self, vote: &AccusationVote) -> Vec<u8> {
        let digest = Self::vote_digest(vote);
        self.signer
            .sign_message_sync(&digest)
            .map(|sig| sig.as_bytes().to_vec())
            .unwrap_or_default()
    }

    /// Structured digest for ECDSA signing of votes.
    ///
    /// ```text
    /// keccak256(abi.encode(
    ///     VOTE_TYPEHASH,
    ///     chainId, e3Id, accusationId, voter, agrees,
    ///     dataHash
    /// ))
    /// ```
    fn vote_digest(vote: &AccusationVote) -> [u8; 32] {
        let e3_id_u256: U256 = vote
            .e3_id
            .clone()
            .try_into()
            .expect("E3id should be valid U256");
        let typehash: [u8; 32] = keccak256(
            "AccusationVote(uint256 chainId,uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)"
        ).into();
        let encoded = (
            typehash,
            U256::from(vote.e3_id.chain_id()),
            e3_id_u256,
            vote.accusation_id,
            vote.voter,
            vote.agrees,
            vote.data_hash,
        )
            .abi_encode();
        keccak256(&encoded).into()
    }

    fn verify_vote_signature(&self, vote: &AccusationVote) -> bool {
        let digest = Self::vote_digest(vote);
        let sig = match alloy::primitives::Signature::try_from(vote.signature.as_slice()) {
            Ok(s) => s,
            Err(_) => return false,
        };
        match sig.recover_address_from_msg(&digest) {
            Ok(addr) => addr == vote.voter,
            Err(_) => false,
        }
    }

    // ─── Core Protocol ───────────────────────────────────────────────────

    /// Called when the local node detects a proof failure.
    ///
    /// Creates and broadcasts a `ProofFailureAccusation`, casts own vote,
    /// and begins vote collection with a timeout.
    fn on_local_proof_failure(
        &mut self,
        event: ProofVerificationFailed,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        let key = (event.accused_address, event.proof_type);

        // Cache the failed verification result
        self.received_data.insert(
            key,
            ReceivedProofData {
                data_hash: event.data_hash,
                verification_passed: false,
            },
        );

        // Dedup: don't create multiple accusations for the same (accused, proof_type)
        if !self.accused_proofs.insert(key) {
            info!(
                "Already accused {:?} for {:?} — skipping duplicate",
                event.accused_address, event.proof_type
            );
            return;
        }

        // For C3a/C3b, include the signed payload so other nodes can re-verify
        let forwarded_payload = match event.proof_type {
            ProofType::C3aSkShareEncryption | ProofType::C3bESmShareEncryption => {
                Some(event.signed_payload.clone())
            }
            _ => None,
        };

        // Create the accusation
        let mut accusation = ProofFailureAccusation {
            e3_id: self.e3_id.clone(),
            accuser: self.my_address,
            accused: event.accused_address,
            accused_party_id: event.accused_party_id,
            proof_type: event.proof_type,
            data_hash: event.data_hash,
            signed_payload: forwarded_payload,
            signature: Vec::new(),
        };
        accusation.signature = self.sign_accusation_digest(&accusation);

        let accusation_id = Self::accusation_id(&accusation);

        info!(
            "Broadcasting accusation against {} for {:?} proof failure",
            event.accused_address, event.proof_type
        );

        // Broadcast accusation via gossip
        if let Err(err) = self.bus.publish(accusation.clone(), ec.clone()) {
            error!("Failed to broadcast ProofFailureAccusation: {err}");
            return;
        }

        // Cast own vote (agrees: true)
        let mut own_vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id,
            voter: self.my_address,
            agrees: true,
            data_hash: event.data_hash,
            signature: Vec::new(),
        };
        own_vote.signature = self.sign_vote_digest(&own_vote);

        if let Err(err) = self.bus.publish(own_vote.clone(), ec.clone()) {
            error!("Failed to broadcast own AccusationVote: {err}");
        }

        // Start timeout
        let timeout_handle = ctx.run_later(self.vote_timeout, move |act, _ctx| {
            act.on_vote_timeout(accusation_id);
        });

        // Store pending accusation with own vote
        self.pending.insert(
            accusation_id,
            PendingAccusation {
                accusation,
                votes_for: vec![own_vote],
                votes_against: Vec::new(),
                timeout_handle: Some(timeout_handle),
                ec: ec.clone(),
            },
        );

        // Replay any votes that arrived before this accusation
        if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
            for vote in buffered {
                self.on_vote_received(vote, ec);
            }
        }

        // Check quorum immediately (in case threshold_m == 1)
        self.check_quorum(accusation_id, ec);
    }

    /// Called when we receive an accusation from another node via gossip.
    ///
    /// Validates the accuser, checks our own verification cache, and casts a vote.
    fn on_accusation_received(
        &mut self,
        accusation: ProofFailureAccusation,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        // Ignore accusations for other E3s
        if accusation.e3_id != self.e3_id {
            return;
        }

        // Verify accuser is in committee
        if !self.committee.contains(&accusation.accuser) {
            warn!(
                "Ignoring accusation from non-committee member {}",
                accusation.accuser
            );
            return;
        }

        // Verify accused is a committee member (defense-in-depth)
        if !self.committee.contains(&accusation.accused) {
            warn!(
                "Ignoring accusation against non-committee member {}",
                accusation.accused
            );
            return;
        }

        // Ignore our own accusations (we already voted)
        if accusation.accuser == self.my_address {
            return;
        }

        // Verify accuser's ECDSA signature
        if !self.verify_accusation_signature(&accusation) {
            warn!(
                "Invalid signature on accusation from {} — ignoring",
                accusation.accuser
            );
            return;
        }

        let accusation_id = Self::accusation_id(&accusation);

        // Don't process duplicate accusations
        if self.pending.contains_key(&accusation_id) {
            return;
        }

        // Determine our vote based on our local verification state
        let key = (accusation.accused, accusation.proof_type);
        let (agrees, our_data_hash) = if let Some(received) = self.received_data.get(&key) {
            // We have the data — did our verification also fail?
            (!received.verification_passed, received.data_hash)
        } else if let Some(ref forwarded) = accusation.signed_payload {
            // C3a/C3b case: we didn't receive this proof directly.
            // Validate the forwarded payload's ECDSA, then dispatch async ZK re-verification.
            let forwarded_valid = match forwarded.recover_address() {
                Ok(addr) => {
                    if addr != accusation.accused {
                        warn!(
                            "Forwarded C3a/C3b payload signer {} != accused {} — cannot verify",
                            addr, accusation.accused
                        );
                        false
                    } else if forwarded.payload.e3_id != self.e3_id {
                        warn!("Forwarded C3a/C3b payload e3_id mismatch — cannot verify");
                        false
                    } else {
                        let expected = forwarded.payload.proof_type.circuit_names();
                        expected.contains(&forwarded.payload.proof.circuit)
                    }
                }
                Err(e) => {
                    warn!("Forwarded C3a/C3b payload signature invalid: {e} — cannot verify");
                    false
                }
            };

            if !forwarded_valid {
                // Can't trust the forwarded proof — abstain
                return;
            }

            let data_hash = Self::compute_payload_hash(forwarded);
            let accused_party_id = accusation.accused_party_id;
            let forwarded_clone = forwarded.clone();

            // Create PendingAccusation without our vote — it arrives after ZK completes
            let timeout_handle = ctx.run_later(self.vote_timeout, move |act, _ctx| {
                act.on_vote_timeout(accusation_id);
            });
            self.pending.insert(
                accusation_id,
                PendingAccusation {
                    accusation,
                    votes_for: Vec::new(),
                    votes_against: Vec::new(),
                    timeout_handle: Some(timeout_handle),
                    ec: ec.clone(),
                },
            );

            // Replay any buffered votes
            if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
                for vote in buffered {
                    self.on_vote_received(vote, ec);
                }
            }

            // Dispatch ZK re-verification
            let correlation_id = CorrelationId::new();
            self.pending_reverifications.insert(
                correlation_id,
                PendingReVerification {
                    accusation_id,
                    data_hash,
                    accused: key.0,
                    proof_type: key.1,
                },
            );

            let party_proof = PartyProofsToVerify {
                sender_party_id: accused_party_id,
                signed_proofs: vec![forwarded_clone],
            };
            let request = ComputeRequest::zk(
                ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                    party_proofs: vec![party_proof],
                }),
                correlation_id,
                self.e3_id.clone(),
            );

            if let Err(err) = self.bus.publish(request, ec.clone()) {
                error!("Failed to dispatch C3a/C3b ZK re-verification: {err}");
                self.pending_reverifications.remove(&correlation_id);
            }

            // Vote deferred — return without falling through to the normal vote path
            return;
        } else {
            // We don't have the data and no payload was forwarded — abstain
            info!(
                "No local data for accused {} proof {:?} — abstaining from vote",
                accusation.accused, accusation.proof_type
            );
            return;
        };

        // Cast vote
        let mut vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id,
            voter: self.my_address,
            agrees,
            data_hash: our_data_hash,
            signature: Vec::new(),
        };
        vote.signature = self.sign_vote_digest(&vote);

        info!(
            "Voting {} on accusation against {} for {:?}",
            if agrees { "AGREE" } else { "DISAGREE" },
            accusation.accused,
            accusation.proof_type
        );

        // Broadcast vote via gossip
        if let Err(err) = self.bus.publish(vote.clone(), ec.clone()) {
            error!("Failed to broadcast AccusationVote: {err}");
        }

        // Start timeout for this accusation
        let timeout_handle = ctx.run_later(self.vote_timeout, move |act, _ctx| {
            act.on_vote_timeout(accusation_id);
        });

        // Record in pending
        let pending = PendingAccusation {
            accusation,
            votes_for: if agrees {
                vec![vote.clone()]
            } else {
                Vec::new()
            },
            votes_against: if agrees { Vec::new() } else { vec![vote] },
            timeout_handle: Some(timeout_handle),
            ec: ec.clone(),
        };
        self.pending.insert(accusation_id, pending);

        // Replay any votes that arrived before this accusation
        if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
            for vote in buffered {
                self.on_vote_received(vote, ec);
            }
        }

        // Check quorum
        self.check_quorum(accusation_id, ec);
    }

    /// Called when we receive a vote from another node via gossip.
    fn on_vote_received(&mut self, vote: AccusationVote, ec: &EventContext<Sequenced>) {
        // Ignore votes for other E3s
        if vote.e3_id != self.e3_id {
            return;
        }

        // Verify voter is in committee
        if !self.committee.contains(&vote.voter) {
            warn!("Ignoring vote from non-committee member {}", vote.voter);
            return;
        }

        // Ignore our own votes (already recorded)
        if vote.voter == self.my_address {
            return;
        }

        // Verify voter's ECDSA signature
        if !self.verify_vote_signature(&vote) {
            warn!("Invalid signature on vote from {} — ignoring", vote.voter);
            return;
        }

        let vote_accusation_id = vote.accusation_id;

        // Find the pending accusation
        let Some(pending) = self.pending.get_mut(&vote_accusation_id) else {
            // Unknown accusation — buffer the vote for replay when the accusation arrives
            self.buffered_votes
                .entry(vote_accusation_id)
                .or_default()
                .push(vote);
            return;
        };

        // Dedup: don't count same voter twice
        let already_voted = pending
            .votes_for
            .iter()
            .chain(pending.votes_against.iter())
            .any(|v| v.voter == vote.voter);
        if already_voted {
            return;
        }

        if vote.agrees {
            pending.votes_for.push(vote);
        } else {
            pending.votes_against.push(vote);
        }

        // Check if quorum reached
        self.check_quorum(vote_accusation_id, ec);
    }

    /// Evaluate whether we have enough votes to decide.
    ///
    /// Quorum logic:
    /// - Need >= M agreeing votes → AccusedFaulted
    /// - If impossible to reach M even with remaining voters → early exit
    /// - data_hash comparison detects equivocation vs false accusation
    fn check_quorum(&mut self, accusation_id: [u8; 32], ec: &EventContext<Sequenced>) {
        let Some(pending) = self.pending.get(&accusation_id) else {
            return;
        };

        let agree_count = pending.votes_for.len();
        let disagree_count = pending.votes_against.len();
        let total_votes = agree_count + disagree_count;

        // CASE A: Majority says proof is bad → accused is at fault
        // But first check for equivocation: if agreeing voters saw different data,
        // the accused sent different payloads to different nodes.
        if agree_count >= self.threshold_m {
            let agree_hashes: HashSet<[u8; 32]> =
                pending.votes_for.iter().map(|v| v.data_hash).collect();
            if agree_hashes.len() > 1 {
                info!(
                    "Equivocation detected at quorum: {} unique data hashes among {} agreeing voters for {} {:?}",
                    agree_hashes.len(),
                    agree_count,
                    pending.accusation.accused,
                    pending.accusation.proof_type
                );
                self.emit_quorum_reached(accusation_id, AccusationOutcome::Equivocation, ec);
            } else {
                info!(
                    "Quorum reached: {} votes confirm {} sent bad {:?} proof — AccusedFaulted",
                    agree_count, pending.accusation.accused, pending.accusation.proof_type
                );
                self.emit_quorum_reached(accusation_id, AccusationOutcome::AccusedFaulted, ec);
            }
            return;
        }

        // Check if quorum is still possible
        let remaining = self.committee.len().saturating_sub(total_votes);
        if agree_count + remaining < self.threshold_m {
            // Even if all remaining voters agree, can't reach quorum.
            // Collect all unique data hashes (from all votes + the accusation itself)
            let all_hashes: HashSet<[u8; 32]> = pending
                .votes_for
                .iter()
                .chain(pending.votes_against.iter())
                .map(|v| v.data_hash)
                .chain(std::iter::once(pending.accusation.data_hash))
                .collect();

            if all_hashes.len() > 1 {
                // Different nodes received different data → equivocation by the accused
                info!(
                    "Equivocation detected: {} unique data hashes for {} {:?}",
                    all_hashes.len(),
                    pending.accusation.accused,
                    pending.accusation.proof_type
                );
                self.emit_quorum_reached(accusation_id, AccusationOutcome::Equivocation, ec);
            } else if agree_count <= 1 && disagree_count > 0 {
                // Same data, only accuser says bad, others say good → AccuserLied
                info!(
                    "Accuser {} appears to have lied about {} {:?}",
                    pending.accusation.accuser,
                    pending.accusation.accused,
                    pending.accusation.proof_type
                );
                self.emit_quorum_reached(accusation_id, AccusationOutcome::AccuserLied, ec);
            } else {
                self.emit_quorum_reached(accusation_id, AccusationOutcome::Inconclusive, ec);
            }
        }
        // Otherwise: still waiting for more votes — timeout will handle it
    }

    /// Called when the vote timeout expires for an accusation.
    fn on_vote_timeout(&mut self, accusation_id: [u8; 32]) {
        let Some(pending) = self.pending.remove(&accusation_id) else {
            return; // Already resolved
        };

        let outcome = if pending.votes_for.len() >= self.threshold_m {
            AccusationOutcome::AccusedFaulted
        } else {
            AccusationOutcome::Inconclusive
        };

        warn!(
            "Accusation against {} for {:?} timed out with {} for / {} against — outcome: {:?}",
            pending.accusation.accused,
            pending.accusation.proof_type,
            pending.votes_for.len(),
            pending.votes_against.len(),
            outcome
        );

        if let Err(err) = self.bus.publish(
            AccusationQuorumReached {
                e3_id: self.e3_id.clone(),
                accuser: pending.accusation.accuser,
                accused: pending.accusation.accused,
                proof_type: pending.accusation.proof_type,
                votes_for: pending.votes_for,
                votes_against: pending.votes_against,
                outcome,
            },
            pending.ec,
        ) {
            error!("Failed to publish AccusationQuorumReached on timeout: {err}");
        }
    }

    fn emit_quorum_reached(
        &mut self,
        accusation_id: [u8; 32],
        outcome: AccusationOutcome,
        ec: &EventContext<Sequenced>,
    ) {
        let Some(pending) = self.pending.remove(&accusation_id) else {
            return;
        };

        // Cancel the timeout if it exists
        // (SpawnHandle can't be cancelled directly in actix without ctx,
        //  but removing from pending prevents the timeout handler from acting)

        info!(
            "Accusation quorum reached for {} {:?}: {} for, {} against — outcome: {}",
            pending.accusation.accused,
            pending.accusation.proof_type,
            pending.votes_for.len(),
            pending.votes_against.len(),
            outcome
        );

        if let Err(err) = self.bus.publish(
            AccusationQuorumReached {
                e3_id: self.e3_id.clone(),
                accuser: pending.accusation.accuser,
                accused: pending.accusation.accused,
                proof_type: pending.accusation.proof_type,
                votes_for: pending.votes_for,
                votes_against: pending.votes_against,
                outcome,
            },
            ec.clone(),
        ) {
            error!("Failed to publish AccusationQuorumReached: {err}");
        }
    }

    /// Cache a successful proof verification result for a specific (accused, proof_type).
    /// This allows the node to vote on accusations from other nodes.
    pub fn cache_verification_result(
        &mut self,
        accused: Address,
        proof_type: ProofType,
        data_hash: [u8; 32],
        passed: bool,
    ) {
        self.received_data.insert(
            (accused, proof_type),
            ReceivedProofData {
                data_hash,
                verification_passed: passed,
            },
        );
    }

    /// Compute a keccak256 hash of a SignedProofPayload for data_hash comparison.
    ///
    /// `keccak256(abi.encode(zkProof, publicSignals))`
    fn compute_payload_hash(payload: &SignedProofPayload) -> [u8; 32] {
        let msg = (
            Bytes::copy_from_slice(&payload.payload.proof.data),
            Bytes::copy_from_slice(&payload.payload.proof.public_signals),
        )
            .abi_encode();
        keccak256(&msg).into()
    }

    /// Handle ZK re-verification response for a forwarded C3a/C3b proof.
    ///
    /// Dispatched by `on_accusation_received` when the accused's forwarded proof
    /// needs async ZK verification. Casts our vote based on the ZK result.
    fn handle_reverification_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, _ec) = msg.into_components();

        let correlation_id = msg.correlation_id;
        let Some(reverif) = self.pending_reverifications.remove(&correlation_id) else {
            return; // Not our correlation ID
        };

        let zk_passed = match msg.response {
            ComputeResponseKind::Zk(ZkResponse::VerifyShareProofs(r)) => {
                if r.party_results.is_empty() {
                    warn!("Empty ZK re-verification results — abstaining");
                    return;
                }
                r.party_results.first().is_some_and(|r| r.all_verified)
            }
            _ => {
                warn!("Unexpected ComputeResponse kind for C3a/C3b re-verification — abstaining");
                return;
            }
        };

        let agrees = !zk_passed; // ZK failed → proof is bad → agree with accusation

        // Cache the result for future accusations
        self.cache_verification_result(
            reverif.accused,
            reverif.proof_type,
            reverif.data_hash,
            zk_passed,
        );

        // Get ec from the PendingAccusation
        let ec = match self.pending.get(&reverif.accusation_id) {
            Some(pending) => pending.ec.clone(),
            None => {
                // Accusation already resolved (timeout/quorum) before ZK finished
                return;
            }
        };

        // Cast vote
        let mut vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id: reverif.accusation_id,
            voter: self.my_address,
            agrees,
            data_hash: reverif.data_hash,
            signature: Vec::new(),
        };
        vote.signature = self.sign_vote_digest(&vote);

        info!(
            "C3a/C3b re-verification complete — voting {} on accusation against {:?}",
            if agrees { "AGREE" } else { "DISAGREE" },
            reverif.proof_type
        );

        // Broadcast vote via gossip
        if let Err(err) = self.bus.publish(vote.clone(), ec.clone()) {
            error!("Failed to broadcast C3a/C3b AccusationVote: {err}");
        }

        // Record in pending
        if let Some(pending) = self.pending.get_mut(&reverif.accusation_id) {
            if agrees {
                pending.votes_for.push(vote);
            } else {
                pending.votes_against.push(vote);
            }
        }

        // Check quorum
        self.check_quorum(reverif.accusation_id, &ec);
    }

    /// Handle ZK re-verification error for a forwarded C3a/C3b proof.
    fn handle_reverification_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let (msg, _ec) = msg.into_components();

        let correlation_id = msg.correlation_id();
        let Some(reverif) = self.pending_reverifications.remove(correlation_id) else {
            return; // Not our correlation ID
        };

        error!(
            "C3a/C3b ZK re-verification failed for {:?} — abstaining from vote",
            reverif.proof_type
        );
        // Don't vote — effectively abstain
    }
}

impl Actor for AccusationManager {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for AccusationManager {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::ProofVerificationFailed(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ProofVerificationPassed(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ProofFailureAccusation(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::AccusationVote(data) => {
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

impl Handler<TypedEvent<ProofVerificationFailed>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ProofVerificationFailed>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();
        self.on_local_proof_failure(data, &ec, ctx);
    }
}

impl Handler<TypedEvent<ProofVerificationPassed>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ProofVerificationPassed>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, _ec) = msg.into_components();
        // Cache successful verification for voting on future accusations
        self.received_data.insert(
            (data.address, data.proof_type),
            ReceivedProofData {
                data_hash: data.data_hash,
                verification_passed: true,
            },
        );
    }
}

impl Handler<TypedEvent<ProofFailureAccusation>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ProofFailureAccusation>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();
        self.on_accusation_received(data, &ec, ctx);
    }
}

impl Handler<TypedEvent<AccusationVote>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<AccusationVote>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();
        self.on_vote_received(data, &ec);
    }
}

impl Handler<TypedEvent<ComputeResponse>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_reverification_response(msg);
    }
}

impl Handler<TypedEvent<ComputeRequestError>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeRequestError>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_reverification_error(msg);
    }
}

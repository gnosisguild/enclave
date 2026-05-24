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
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix::{Actor, Addr, AsyncContext, Context, Handler, SpawnHandle};
use alloy::primitives::{keccak256, Address, Bytes, U256};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::SignerSync;
use alloy::sol_types::SolValue;
use e3_events::{
    AccusationOutcome, AccusationQuorumReached, AccusationVote, BusHandle,
    CommitmentConsistencyViolation, ComputeRequest, ComputeRequestError, ComputeResponse,
    ComputeResponseKind, CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EventContext,
    EventPublisher, EventSubscriber, EventType, PartyProofsToVerify, ProofFailureAccusation,
    ProofType, ProofVerificationFailed, ProofVerificationPassed, Sequenced, SignedProofPayload,
    SlashExecuted, TypedEvent, VerifyShareProofsRequest, ZkRequest, ZkResponse, VOTE_DOMAIN_NAME,
    VOTE_DOMAIN_VERSION, VOTE_TYPEHASH_STR,
};
use e3_utils::{ArcBytes, NotifySync};
use tracing::{error, info, warn};

/// How long to wait for votes before declaring the accusation inconclusive.
const DEFAULT_VOTE_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Abstraction over wall-clock time so the deadline-stamping logic is
/// deterministically testable. Production uses [`SystemClock`], which reads
/// `SystemTime::now()`; tests can inject a mock clock that returns fixed
/// timestamps.
pub trait Clock: Send + Sync + 'static {
    /// Current Unix time in seconds. Returns `0` if the platform clock is
    /// pre-`UNIX_EPOCH` (a broken clock should not silently produce
    /// signatures that look valid forever — the on-chain check will then
    /// reject the resulting deadline immediately).
    fn unix_now_secs(&self) -> u64;
}

/// Production clock backed by `SystemTime::now()`.
pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_now_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// An active accusation awaiting agreement votes from committee members.
///
/// There is no `votes_against` field: a peer who finds the disputed proof
/// passes simply stays silent rather than broadcasting a signed disagreement
/// (see `AccusationVote` docstring for rationale). The accusation runs to
/// quorum or to `vote_timeout`.
struct PendingAccusation {
    accusation: ProofFailureAccusation,
    votes_for: Vec<AccusationVote>,
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
    /// Raw `abi.encode(proof.data, proof.public_signals)` — preimage of
    /// `data_hash`. Forwarded to the on-chain slashing contract so it can
    /// recompute and verify the dataHash bound in voter signatures. Empty
    /// only on paths where the raw bytes weren't available locally; those
    /// paths can still slash, but they fall back to off-chain trust for
    /// the evidence binding.
    evidence: Bytes,
}

/// Tracks an in-flight ZK re-verification for a forwarded C3a/C3b proof.
struct PendingReVerification {
    accusation_id: [u8; 32],
    data_hash: [u8; 32],
    accused: Address,
    proof_type: ProofType,
    /// Evidence preimage bytes from the forwarded proof, used to populate
    /// `ReceivedProofData.evidence` after ZK re-verification completes.
    evidence: Bytes,
}

/// Manages the off-chain accusation quorum protocol.
///
/// **Lifecycle**: One instance per E3 computation. Created by
/// [`AccusationManagerExtension`] when [`CommitteeFinalized`] fires and
/// destroyed when the E3 completes or the node shuts down. All internal
/// state (pending accusations, votes, caches) is therefore naturally
/// scoped to a single E3 — no cross-E3 data contamination is possible.
///
/// **Ephemeral**: This actor does *not* persist state across restarts.
/// In-flight accusations are lost on node restart (accepted trade-off:
/// they would have timed out within [`DEFAULT_VOTE_TIMEOUT`] anyway).
/// A strategic node restart can delay slash submission but cannot
/// prevent it, because other committee members independently maintain
/// their own `AccusationManager` instances and will continue voting.
///
/// Subscribes to:
/// - [`ProofVerificationFailed`] — local proof failure detection
/// - [`ProofVerificationPassed`] — cache successful verification for voting
/// - [`ProofFailureAccusation`] — incoming accusations from other nodes via gossip
/// - [`AccusationVote`] — incoming votes from other nodes via gossip
/// - [`SlashExecuted`] — on-chain slash confirmation for committee updates
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

    /// On-chain `SlashingManager` address (EIP-712 `verifyingContract` for vote signatures).
    slashing_manager: Address,

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

    /// Registry-wide off-chain freshness window (seconds) applied when stamping
    /// `AccusationVote.deadline`. Fetched once per process from
    /// `CiphernodeRegistry.accusationVoteValidity()` so a governance change
    /// requires a node restart to take effect — same lifecycle as the fold
    /// attestation verifier.
    vote_validity_secs: u64,

    /// Wall-clock source used to derive accusation deadlines. Production uses
    /// [`SystemClock`]; tests can inject a deterministic mock.
    clock: Arc<dyn Clock>,

    /// BFV preset for circuit artifact resolution.
    params_preset: e3_fhe_params::BfvPreset,
}

impl AccusationManager {
    /// Construct an actor with the production [`SystemClock`]. Use
    /// [`AccusationManager::new_with_clock`] in tests that need deterministic
    /// timestamps.
    pub fn new(
        bus: &BusHandle,
        e3_id: E3id,
        signer: PrivateKeySigner,
        slashing_manager: Address,
        committee: Vec<Address>,
        threshold_m: usize,
        vote_validity_secs: u64,
        params_preset: e3_fhe_params::BfvPreset,
    ) -> Self {
        Self::new_with_clock(
            bus,
            e3_id,
            signer,
            slashing_manager,
            committee,
            threshold_m,
            vote_validity_secs,
            params_preset,
            Arc::new(SystemClock),
        )
    }

    /// Construct an actor with an explicit [`Clock`]. Allows unit tests to
    /// drive deadline computation without touching wall-clock time.
    pub fn new_with_clock(
        bus: &BusHandle,
        e3_id: E3id,
        signer: PrivateKeySigner,
        slashing_manager: Address,
        committee: Vec<Address>,
        threshold_m: usize,
        vote_validity_secs: u64,
        params_preset: e3_fhe_params::BfvPreset,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let my_address = signer.address();
        Self {
            bus: bus.clone(),
            e3_id,
            my_address,
            signer,
            slashing_manager,
            committee,
            threshold_m,
            pending: HashMap::new(),
            accused_proofs: HashSet::new(),
            received_data: HashMap::new(),
            buffered_votes: HashMap::new(),
            pending_reverifications: HashMap::new(),
            vote_timeout: DEFAULT_VOTE_TIMEOUT,
            vote_validity_secs,
            clock,
            params_preset,
        }
    }

    pub fn setup(
        bus: &BusHandle,
        e3_id: E3id,
        signer: PrivateKeySigner,
        slashing_manager: Address,
        committee: Vec<Address>,
        threshold_m: usize,
        vote_validity_secs: u64,
        params_preset: e3_fhe_params::BfvPreset,
    ) -> Addr<Self> {
        let addr = Self::new(
            bus,
            e3_id,
            signer,
            slashing_manager,
            committee,
            threshold_m,
            vote_validity_secs,
            params_preset,
        )
        .start();
        bus.subscribe(EventType::ProofVerificationFailed, addr.clone().into());
        bus.subscribe(EventType::ProofVerificationPassed, addr.clone().into());
        bus.subscribe(EventType::ProofFailureAccusation, addr.clone().into());
        bus.subscribe(EventType::AccusationVote, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        bus.subscribe(EventType::SlashExecuted, addr.clone().into());
        bus.subscribe(
            EventType::CommitmentConsistencyViolation,
            addr.clone().into(),
        );
        addr
    }

    // ─── Deadline computation ────────────────────────────────────────────

    /// Compute the on-chain vote-validity deadline (Unix seconds) the accuser
    /// stamps on a fresh accusation. Voters then sign this exact value so the
    /// aggregated evidence carries one shared deadline that `SlashingManager`
    /// checks via `block.timestamp <= deadline`.
    ///
    /// `vote_validity_secs` is the registry-wide window fetched from
    /// `CiphernodeRegistry.accusationVoteValidity()` at process startup —
    /// governance can shorten or extend it; live nodes only pick up the new
    /// value on restart.
    ///
    /// `saturating_add` guards against `u64` overflow in the unlikely event
    /// governance sets the validity to a near-`u64::MAX` value.
    fn compute_deadline(&self) -> u64 {
        self.clock
            .unix_now_secs()
            .saturating_add(self.vote_validity_secs)
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

    fn sign_accusation_digest(
        &self,
        accusation: &ProofFailureAccusation,
    ) -> Result<Vec<u8>, alloy::signers::Error> {
        let digest = Self::accusation_digest(accusation);
        let sig = self.signer.sign_message_sync(&digest)?;
        Ok(sig.as_bytes().to_vec())
    }

    /// Structured digest for ECDSA signing of accusations.
    ///
    /// Off-chain only — this digest never reaches the chain. Includes `deadline`
    /// so peers can verify the accuser's chosen on-chain validity window has not
    /// been tampered with in transit:
    /// ```text
    /// keccak256(abi.encode(
    ///     ACCUSATION_TYPEHASH,
    ///     chainId, e3Id, accuser, accused, proofType,
    ///     dataHash, deadline
    /// ))
    /// ```
    fn accusation_digest(accusation: &ProofFailureAccusation) -> [u8; 32] {
        let e3_id_u256: U256 = accusation
            .e3_id
            .clone()
            .try_into()
            .expect("E3id should be valid U256");
        let typehash: [u8; 32] = keccak256(
            "ProofFailureAccusation(uint256 chainId,uint256 e3Id,address accuser,address accused,uint256 proofType,bytes32 dataHash,uint256 deadline)"
        ).into();
        let encoded = (
            typehash,
            U256::from(accusation.e3_id.chain_id()),
            e3_id_u256,
            accusation.accuser,
            accusation.accused,
            U256::from(accusation.proof_type as u8),
            accusation.data_hash,
            U256::from(accusation.deadline),
        )
            .abi_encode();
        keccak256(&encoded).into()
    }

    fn verify_accusation_signature(&self, accusation: &ProofFailureAccusation) -> bool {
        let digest = Self::accusation_digest(accusation);
        let sig = match alloy::primitives::Signature::try_from(
            accusation.signature.extract_bytes().as_ref(),
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        match sig.recover_address_from_msg(&digest) {
            Ok(addr) => addr == accusation.accuser,
            Err(_) => false,
        }
    }

    #[cfg_attr(test, allow(dead_code))]
    fn sign_vote_digest(&self, vote: &AccusationVote) -> Result<Vec<u8>, alloy::signers::Error> {
        let digest = Self::vote_digest(vote, self.slashing_manager);
        // `sign_hash_sync` signs the raw 32-byte hash without EIP-191 wrapping,
        // which is what EIP-712 requires (`digest` is already the
        // `\x19\x01 || domainSeparator || structHash` hash).
        let sig = self.signer.sign_hash_sync(&digest.into())?;
        Ok(sig.as_bytes().to_vec())
    }

    /// Canonical EIP-712 domain separator for vote signatures.
    ///
    /// Must match `SlashingManager`'s domain construction exactly. The `name`
    /// literal is `EIP712_DOMAIN_NAME` in the Solidity contract (see
    /// `packages/enclave-contracts/contracts/slashing/SlashingManager.sol`);
    /// keep these two strings in lockstep — divergence silently breaks
    /// `ECDSA.recover` on chain.
    fn vote_domain_separator(chain_id: u64, verifying_contract: Address) -> [u8; 32] {
        let domain_typehash: [u8; 32] = keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        )
        .into();
        let name_hash: [u8; 32] = keccak256(VOTE_DOMAIN_NAME).into();
        let version_hash: [u8; 32] = keccak256(VOTE_DOMAIN_VERSION).into();
        let encoded = (
            domain_typehash,
            name_hash,
            version_hash,
            U256::from(chain_id),
            verifying_contract,
        )
            .abi_encode();
        keccak256(&encoded).into()
    }

    /// Canonical EIP-712 typed-data hash for a vote.
    ///
    /// `keccak256("\x19\x01" || domainSeparator || structHash)` where the struct
    /// matches `SlashingManager.VOTE_TYPEHASH`:
    /// `AccusationVote(uint256 e3Id,bytes32 accusationId,address voter,bytes32 dataHash,uint256 deadline)`.
    ///
    /// `AccusationVote` no longer carries an `agrees` field. The gossip wire
    /// transmits only agreements; the on-chain verifier treats every submitted
    /// signature as an affirmative vote. See the struct's docstring in
    /// `e3_events::accusation_vote` for rationale.
    ///
    /// Exposed `pub` so the Anvil parity test in
    /// `crates/zk-prover/tests/slashing_integration_tests.rs` can sign votes
    /// through the **same** code path the production actor uses — if the
    /// digest drifts from on-chain `_verifyVotes`, the parity test reverts on
    /// chain immediately rather than allowing the actor to ship broken
    /// signatures.
    pub fn vote_digest(vote: &AccusationVote, verifying_contract: Address) -> [u8; 32] {
        let e3_id_u256: U256 = vote
            .e3_id
            .clone()
            .try_into()
            .expect("E3id should be valid U256");
        let typehash: [u8; 32] = keccak256(VOTE_TYPEHASH_STR).into();
        let struct_hash: [u8; 32] = keccak256(
            &(
                typehash,
                e3_id_u256,
                vote.accusation_id,
                vote.voter,
                vote.data_hash,
                U256::from(vote.deadline),
            )
                .abi_encode(),
        )
        .into();
        let domain = Self::vote_domain_separator(vote.e3_id.chain_id(), verifying_contract);
        let mut buf = Vec::with_capacity(2 + 32 + 32);
        buf.push(0x19);
        buf.push(0x01);
        buf.extend_from_slice(&domain);
        buf.extend_from_slice(&struct_hash);
        keccak256(&buf).into()
    }

    fn verify_vote_signature(&self, vote: &AccusationVote) -> bool {
        let digest = Self::vote_digest(vote, self.slashing_manager);
        let sig =
            match alloy::primitives::Signature::try_from(vote.signature.extract_bytes().as_ref()) {
                Ok(s) => s,
                Err(_) => return false,
            };
        match sig.recover_address_from_prehash(&digest.into()) {
            Ok(addr) => addr == vote.voter,
            Err(_) => false,
        }
    }

    // ─── Core Protocol ───────────────────────────────────────────────────

    /// Called when the local node detects a proof failure.
    ///
    /// Resolves the accused address, caches the failure, extracts C3a/C3b
    /// forwarding payload, then delegates to [`initiate_accusation`].
    fn on_local_proof_failure(
        &mut self,
        event: ProofVerificationFailed,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        if event.e3_id != self.e3_id {
            return;
        }

        let accused_address = if event.accused_address == Address::ZERO {
            if let Some(&addr) = self.committee.get(event.accused_party_id as usize) {
                warn!(
                    "Resolved Address::ZERO for party {} to committee address {}",
                    event.accused_party_id, addr
                );
                addr
            } else {
                error!(
                    "Cannot resolve address for party {} (out of committee bounds) — dropping accusation",
                    event.accused_party_id
                );
                return;
            }
        } else {
            event.accused_address
        };

        if !self.committee.contains(&accused_address) {
            warn!(
                "Ignoring proof failure for {} — not on E3 {} committee",
                accused_address, self.e3_id
            );
            return;
        }

        // Cache the failed verification result.
        // Evidence preimage = `abi.encode(proof.data, public_signals)` — matches
        // the on-chain `keccak256(evidence) == dataHash` check in SlashingManager.
        let evidence = Bytes::from(
            (
                Bytes::copy_from_slice(&event.signed_payload.payload.proof.data),
                Bytes::copy_from_slice(&event.signed_payload.payload.proof.public_signals),
            )
                .abi_encode(),
        );
        self.received_data.insert(
            (accused_address, event.proof_type),
            ReceivedProofData {
                data_hash: event.data_hash,
                verification_passed: false,
                evidence,
            },
        );

        // For C3a/C3b, include the signed payload so other nodes can re-verify
        let forwarded_payload = match event.proof_type {
            ProofType::C3aSkShareEncryption | ProofType::C3bESmShareEncryption => {
                Some(event.signed_payload.clone())
            }
            _ => None,
        };

        self.initiate_accusation(
            accused_address,
            event.accused_party_id,
            event.proof_type,
            event.data_hash,
            forwarded_payload,
            ec,
            ctx,
        );
    }

    /// Called when the `CommitmentConsistencyChecker` detects a cross-circuit
    /// commitment mismatch for a party.
    ///
    /// Caches the failure and delegates to `initiate_accusation` — the same
    /// quorum protocol as ZK proof failures.
    fn on_consistency_violation(
        &mut self,
        data: CommitmentConsistencyViolation,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        if data.e3_id != self.e3_id {
            return;
        }

        if !self.committee.contains(&data.accused_address) {
            warn!(
                "Ignoring commitment violation for {} — not on E3 {} committee",
                data.accused_address, self.e3_id
            );
            return;
        }

        // Cache as a failed verification for voting on future accusations.
        // `data.evidence` carries the raw `abi.encode(proof.data, public_signals)`
        // preimage of `data_hash`, populated by the consistency checker. Slashing
        // via this path now binds voter signatures to evidence bytes on-chain
        // just like the ProofVerificationFailed path.
        self.received_data.insert(
            (data.accused_address, data.proof_type),
            ReceivedProofData {
                data_hash: data.data_hash,
                verification_passed: false,
                evidence: data.evidence.clone(),
            },
        );

        self.initiate_accusation(
            data.accused_address,
            data.accused_party_id,
            data.proof_type,
            data.data_hash,
            None, // No forwarding needed — violations are detected from public signals all nodes have
            ec,
            ctx,
        );
    }

    /// Shared accusation creation and broadcast logic.
    ///
    /// Called by [`on_local_proof_failure`] (ZK verification failure) and
    /// [`on_consistency_violation`] (commitment consistency mismatch).
    /// Deduplicates, creates and signs a [`ProofFailureAccusation`], casts
    /// the node's own vote, and begins vote collection with a timeout.
    fn initiate_accusation(
        &mut self,
        accused_address: Address,
        accused_party_id: u64,
        proof_type: ProofType,
        data_hash: [u8; 32],
        forwarded_payload: Option<SignedProofPayload>,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        if !self.committee.contains(&accused_address) {
            warn!(
                "Refusing accusation against {} — not on E3 {} committee",
                accused_address, self.e3_id
            );
            return;
        }

        let key = (accused_address, proof_type);

        // Dedup: don't create multiple accusations for the same (accused, proof_type)
        if !self.accused_proofs.insert(key) {
            info!(
                "Already accused {:?} for {:?} — skipping duplicate",
                accused_address, proof_type
            );
            return;
        }

        // Pick the on-chain validity deadline once per accusation. Every voter
        // (including ourselves below) signs the same value; otherwise the
        // aggregated evidence cannot be encoded as a single `deadline`.
        let deadline = self.compute_deadline();

        // Create the accusation
        let mut accusation = ProofFailureAccusation {
            e3_id: self.e3_id.clone(),
            accuser: self.my_address,
            accused: accused_address,
            accused_party_id,
            proof_type,
            data_hash,
            deadline,
            signed_payload: forwarded_payload,
            signature: ArcBytes::default(),
        };
        match self.sign_accusation_digest(&accusation) {
            Ok(sig) => accusation.signature = ArcBytes::from_bytes(&sig),
            Err(err) => {
                error!("Failed to sign ProofFailureAccusation: {err}");
                self.accused_proofs.remove(&key);
                return;
            }
        }

        let accusation_id = Self::accusation_id(&accusation);

        info!(
            "Broadcasting accusation against {} for {:?} failure",
            accused_address, proof_type
        );

        // Broadcast accusation via gossip
        if let Err(err) = self.bus.publish(accusation.clone(), ec.clone()) {
            error!("Failed to broadcast ProofFailureAccusation: {err}");
            return;
        }

        // Cast our own agreement vote (we just observed the failure locally).
        let mut own_vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id,
            voter: self.my_address,
            data_hash,
            deadline,
            signature: ArcBytes::default(),
        };
        match self.sign_vote_digest(&own_vote) {
            Ok(sig) => own_vote.signature = ArcBytes::from_bytes(&sig),
            Err(err) => {
                error!("Failed to sign own AccusationVote: {err}");
                return;
            }
        }

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
                timeout_handle: Some(timeout_handle),
                ec: ec.clone(),
            },
        );

        // Replay any votes that arrived before this accusation
        if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
            for vote in buffered {
                self.on_vote_received(vote, ec, ctx);
            }
        }

        // Check quorum immediately (in case threshold_m == 1)
        self.check_quorum(accusation_id, ec, ctx);
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

        // Determine our position based on our local verification state.
        //
        // The gossip wire no longer carries disagreement: if our local check
        // *passed*, we stay silent (no broadcast, no pending state). The
        // accusation will then either reach quorum from other agreeing peers
        // or time out as Inconclusive. Only the "we also saw it fail" branch
        // and the "we don't have local data yet (C3a/C3b)" branch proceed
        // below.
        let key = (accusation.accused, accusation.proof_type);
        let our_data_hash = if let Some(received) = self.received_data.get(&key) {
            if received.verification_passed {
                info!(
                    "Local verification of {:?} from {} passed — abstaining \
                     (no disagreement vote on the wire)",
                    accusation.proof_type, accusation.accused
                );
                return;
            }
            received.data_hash
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
            let evidence: Bytes = (
                Bytes::copy_from_slice(&forwarded.payload.proof.data),
                Bytes::copy_from_slice(&forwarded.payload.proof.public_signals),
            )
                .abi_encode()
                .into();
            let accused_party_id = accusation.accused_party_id;
            let forwarded_clone = forwarded.clone();

            // Create PendingAccusation without our vote — it arrives after ZK completes.
            //
            // NOTE (timeout race): If the async ZK re-verification takes longer than
            // `vote_timeout` (default 5 min), the accusation will time out before this
            // node casts its vote. This is an accepted trade-off: the node's contribution
            // is lost, but the quorum can still be reached by other voters. In small
            // committees near the threshold M, this could cause a valid accusation to
            // become Inconclusive instead of AccusedFaulted. Operators should ensure ZK
            // verification completes well within the vote timeout.
            let timeout_handle = ctx.run_later(self.vote_timeout, move |act, _ctx| {
                act.on_vote_timeout(accusation_id);
            });
            self.pending.insert(
                accusation_id,
                PendingAccusation {
                    accusation,
                    votes_for: Vec::new(),
                    timeout_handle: Some(timeout_handle),
                    ec: ec.clone(),
                },
            );

            // Replay any buffered votes
            if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
                for vote in buffered {
                    self.on_vote_received(vote, ec, ctx);
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
                    evidence,
                },
            );

            let party_proof = PartyProofsToVerify {
                sender_party_id: accused_party_id,
                signed_proofs: vec![forwarded_clone],
            };
            let request = ComputeRequest::zk(
                ZkRequest::VerifyShareProofs(VerifyShareProofsRequest {
                    party_proofs: vec![party_proof],
                    params_preset: self.params_preset,
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

        // We saw the proof fail locally — agree with the accusation. Adopt
        // the accuser's deadline so every voter on this accusation signs the
        // same on-chain validity window.
        let mut vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id,
            voter: self.my_address,
            data_hash: our_data_hash,
            deadline: accusation.deadline,
            signature: ArcBytes::default(),
        };
        match self.sign_vote_digest(&vote) {
            Ok(sig) => vote.signature = ArcBytes::from_bytes(&sig),
            Err(err) => {
                error!("Failed to sign AccusationVote: {err}");
                return;
            }
        }

        info!(
            "Agreeing with accusation against {} for {:?}",
            accusation.accused, accusation.proof_type
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
            votes_for: vec![vote],
            timeout_handle: Some(timeout_handle),
            ec: ec.clone(),
        };
        self.pending.insert(accusation_id, pending);

        // Replay any votes that arrived before this accusation
        if let Some(buffered) = self.buffered_votes.remove(&accusation_id) {
            for vote in buffered {
                self.on_vote_received(vote, ec, ctx);
            }
        }

        // Check quorum
        self.check_quorum(accusation_id, ec, ctx);
    }

    /// Called when we receive a vote from another node via gossip.
    fn on_vote_received(
        &mut self,
        vote: AccusationVote,
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
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
            // Unknown accusation — buffer the vote for replay when the accusation arrives.
            // Cap buffer size to prevent unbounded growth if the accusation never arrives.
            let buf = self.buffered_votes.entry(vote_accusation_id).or_default();
            if buf.len() < self.committee.len() {
                buf.push(vote);
            } else {
                warn!(
                    "Buffered votes for unknown accusation {:?} reached committee-size cap — dropping vote",
                    vote_accusation_id
                );
            }
            return;
        };

        // Reject votes whose deadline disagrees with the accusation's chosen
        // deadline. All voters must sign the same deadline so the aggregated
        // evidence carries a single value for `SlashingManager`'s
        // `block.timestamp <= deadline` check.
        if vote.deadline != pending.accusation.deadline {
            warn!(
                "Ignoring vote from {} — deadline {} does not match accusation deadline {}",
                vote.voter, vote.deadline, pending.accusation.deadline
            );
            return;
        }

        // Reject votes from the accused party — they have a conflict of interest
        if vote.voter == pending.accusation.accused {
            warn!(
                "Ignoring vote from accused party {} on their own accusation",
                vote.voter
            );
            return;
        }

        // Dedup: don't count same voter twice
        let already_voted = pending.votes_for.iter().any(|v| v.voter == vote.voter);
        if already_voted {
            return;
        }

        // If the voter is the original accuser, their vote's data_hash must
        // match the accusation's data_hash. A malicious accuser could otherwise
        // send an accusation with one data_hash and a vote with a different one
        // to create artificial data_hash diversity and trigger false equivocation.
        if vote.voter == pending.accusation.accuser
            && vote.data_hash != pending.accusation.data_hash
        {
            warn!(
                "Accuser {} sent vote with data_hash inconsistent with their accusation — rejecting vote",
                vote.voter
            );
            return;
        }

        // Every received `AccusationVote` is an agreement (the gossip wire
        // carries no disagreement). Append to the agreeing pile and re-check
        // quorum.
        pending.votes_for.push(vote);

        self.check_quorum(vote_accusation_id, ec, ctx);
    }

    /// Evaluate whether we have enough agreeing votes to decide.
    ///
    /// Quorum logic:
    /// - `>= M` agreeing votes → `AccusedFaulted` (or `Equivocation` if those
    ///   votes disagree on `data_hash`, indicating the accused sent different
    ///   bytes to different peers).
    /// - Otherwise → keep waiting; the timeout handler decides
    ///   `Inconclusive` if quorum never arrives.
    ///
    /// The gossip wire no longer carries disagreement, so there is no
    /// fast-fail "quorum unreachable" branch — every silent peer might still
    /// agree in flight. Silence beyond `vote_timeout` ⇒ `Inconclusive`.
    fn check_quorum(
        &mut self,
        accusation_id: [u8; 32],
        ec: &EventContext<Sequenced>,
        ctx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending.get(&accusation_id) else {
            return;
        };

        let agree_count = pending.votes_for.len();
        if agree_count < self.threshold_m {
            // Not yet at quorum — wait for more agreement votes or for the
            // timeout to fire.
            return;
        }

        // Reached `M` — decide between AccusedFaulted and Equivocation by
        // checking whether the agreeing voters all saw the same data_hash.
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
            self.emit_quorum_reached(accusation_id, AccusationOutcome::Equivocation, ec, ctx);
        } else {
            info!(
                "Quorum reached: {} votes confirm {} sent bad {:?} proof — AccusedFaulted",
                agree_count, pending.accusation.accused, pending.accusation.proof_type
            );
            self.emit_quorum_reached(accusation_id, AccusationOutcome::AccusedFaulted, ec, ctx);
        }
    }

    /// Called when the vote timeout expires for an accusation.
    fn on_vote_timeout(&mut self, accusation_id: [u8; 32]) {
        let Some(pending) = self.pending.remove(&accusation_id) else {
            return; // Already resolved
        };

        // All votes received are agreements (the wire carries no
        // disagreement signal). At timeout, decide between AccusedFaulted,
        // Equivocation, or Inconclusive purely from the agreeing pile.
        let outcome = if pending.votes_for.len() >= self.threshold_m {
            let agree_hashes: HashSet<[u8; 32]> =
                pending.votes_for.iter().map(|v| v.data_hash).collect();
            if agree_hashes.len() > 1 {
                AccusationOutcome::Equivocation
            } else {
                AccusationOutcome::AccusedFaulted
            }
        } else {
            // Not enough agreements to convict and no signed disagreements
            // exist; whether that's silence or active disagreement is
            // indistinguishable on the wire. Report Inconclusive.
            AccusationOutcome::Inconclusive
        };

        warn!(
            "Accusation against {} for {:?} timed out with {} agreeing votes — outcome: {:?}",
            pending.accusation.accused,
            pending.accusation.proof_type,
            pending.votes_for.len(),
            outcome
        );

        let evidence = self
            .received_data
            .get(&(pending.accusation.accused, pending.accusation.proof_type))
            .map(|d| d.evidence.clone())
            .unwrap_or_default();
        if let Err(err) = self.bus.publish(
            AccusationQuorumReached {
                e3_id: self.e3_id.clone(),
                accuser: pending.accusation.accuser,
                accused: pending.accusation.accused,
                proof_type: pending.accusation.proof_type,
                votes_for: pending.votes_for,
                outcome,
                evidence,
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
        ctx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending.remove(&accusation_id) else {
            return;
        };

        // Cancel the timeout to avoid unnecessary timer fires
        if let Some(handle) = pending.timeout_handle {
            ctx.cancel_future(handle);
        }

        info!(
            "Accusation quorum reached for {} {:?}: {} agreeing votes — outcome: {}",
            pending.accusation.accused,
            pending.accusation.proof_type,
            pending.votes_for.len(),
            outcome
        );

        let evidence = self
            .received_data
            .get(&(pending.accusation.accused, pending.accusation.proof_type))
            .map(|d| d.evidence.clone())
            .unwrap_or_default();
        if let Err(err) = self.bus.publish(
            AccusationQuorumReached {
                e3_id: self.e3_id.clone(),
                accuser: pending.accusation.accuser,
                accused: pending.accusation.accused,
                proof_type: pending.accusation.proof_type,
                votes_for: pending.votes_for,
                outcome,
                evidence,
            },
            ec.clone(),
        ) {
            error!("Failed to publish AccusationQuorumReached: {err}");
        }
    }

    /// Handle an on-chain SlashExecuted event for this E3.
    fn on_slash_executed(&mut self, data: SlashExecuted) {
        if data.e3_id != self.e3_id {
            return;
        }
        let prev_len = self.committee.len();
        self.committee.retain(|addr| *addr != data.operator);
        if self.committee.len() < prev_len {
            info!(
                "Removed slashed operator {} from committee (now {} members)",
                data.operator,
                self.committee.len()
            );

            // Purge any votes from the expelled node in pending accusations
            for pending in self.pending.values_mut() {
                pending.votes_for.retain(|v| v.voter != data.operator);
            }

            // Purge from buffered votes
            for buf in self.buffered_votes.values_mut() {
                buf.retain(|v| v.voter != data.operator);
            }
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
        evidence: Bytes,
    ) {
        self.received_data.insert(
            (accused, proof_type),
            ReceivedProofData {
                data_hash,
                verification_passed: passed,
                evidence,
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
    fn handle_reverification_response(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        ctx: &mut Context<Self>,
    ) {
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

        // Cache the result for future accusations regardless of outcome.
        self.cache_verification_result(
            reverif.accused,
            reverif.proof_type,
            reverif.data_hash,
            zk_passed,
            reverif.evidence.clone(),
        );

        // ZK re-verification passed ⇒ the proof is actually valid ⇒ we
        // disagree with the accusation. The gossip wire carries no
        // disagreement signal, so just abstain (no broadcast, no pending
        // mutation). Other agreeing peers will or won't reach quorum
        // independently.
        if zk_passed {
            info!(
                "C3a/C3b re-verification passed for {:?} — abstaining from vote",
                reverif.proof_type
            );
            return;
        }

        // ZK re-verification failed ⇒ we agree with the accusation.
        let (ec, deadline) = match self.pending.get(&reverif.accusation_id) {
            Some(pending) => (pending.ec.clone(), pending.accusation.deadline),
            None => {
                // Accusation already resolved (timeout/quorum) before ZK finished
                return;
            }
        };

        let mut vote = AccusationVote {
            e3_id: self.e3_id.clone(),
            accusation_id: reverif.accusation_id,
            voter: self.my_address,
            data_hash: reverif.data_hash,
            deadline,
            signature: ArcBytes::default(),
        };
        match self.sign_vote_digest(&vote) {
            Ok(sig) => vote.signature = ArcBytes::from_bytes(&sig),
            Err(err) => {
                error!("Failed to sign C3a/C3b AccusationVote: {err}");
                return;
            }
        }

        info!(
            "C3a/C3b re-verification confirmed failure for {:?} — agreeing with accusation",
            reverif.proof_type
        );

        // Broadcast vote via gossip
        if let Err(err) = self.bus.publish(vote.clone(), ec.clone()) {
            error!("Failed to broadcast C3a/C3b AccusationVote: {err}");
        }

        // Record in pending
        if let Some(pending) = self.pending.get_mut(&reverif.accusation_id) {
            pending.votes_for.push(vote);
        }

        // Check quorum
        self.check_quorum(reverif.accusation_id, &ec, ctx);
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
            EnclaveEventData::SlashExecuted(data) => {
                self.on_slash_executed(data);
            }
            EnclaveEventData::CommitmentConsistencyViolation(data) => {
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
        if data.e3_id != self.e3_id {
            return;
        }
        if !self.committee.contains(&data.address) {
            return;
        }
        // Cache successful verification for voting on future accusations.
        // Evidence preimage = `abi.encode(proof.data, public_signals)`.
        let evidence: Bytes = (
            Bytes::copy_from_slice(&data.proof_data),
            Bytes::copy_from_slice(&data.public_signals),
        )
            .abi_encode()
            .into();
        self.received_data.insert(
            (data.address, data.proof_type),
            ReceivedProofData {
                data_hash: data.data_hash,
                verification_passed: true,
                evidence,
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

    fn handle(&mut self, msg: TypedEvent<AccusationVote>, ctx: &mut Self::Context) -> Self::Result {
        let (data, ec) = msg.into_components();
        self.on_vote_received(data, &ec, ctx);
    }
}

impl Handler<TypedEvent<ComputeResponse>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_reverification_response(msg, ctx);
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

impl Handler<TypedEvent<CommitmentConsistencyViolation>> for AccusationManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<CommitmentConsistencyViolation>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        let (data, ec) = msg.into_components();
        self.on_consistency_violation(data, &ec, ctx);
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════════════════════════════════════
//
// These tests pin the actor's EIP-712 digest computation to the exact bytes
// that off-chain test helpers (and ultimately the on-chain
// `SlashingManager._verifyVotes`) expect. If anyone tweaks the typehash
// string, the domain name, or the struct field layout on EITHER side without
// updating the other, these tests fail before the broken signatures ever
// reach the chain.

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::FixedBytes;
    use alloy::signers::SignerSync;

    /// Independent re-derivation of the EIP-712 vote digest, mirroring exactly
    /// what `SlashingManager._verifyVotes` computes on chain. Kept here (and
    /// not imported from a helper) so a regression in the actor's `vote_digest`
    /// is caught by a byte-for-byte assertion against a hand-rolled reference.
    fn reference_vote_digest(
        chain_id: u64,
        verifying_contract: Address,
        e3_id: u64,
        accusation_id: [u8; 32],
        voter: Address,
        data_hash: [u8; 32],
        deadline: u64,
    ) -> [u8; 32] {
        let domain_typehash: [u8; 32] = keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        )
        .into();
        let name_hash: [u8; 32] = keccak256(VOTE_DOMAIN_NAME).into();
        let version_hash: [u8; 32] = keccak256(VOTE_DOMAIN_VERSION).into();
        let domain_separator: [u8; 32] = keccak256(
            &(
                domain_typehash,
                name_hash,
                version_hash,
                U256::from(chain_id),
                verifying_contract,
            )
                .abi_encode(),
        )
        .into();

        let typehash: [u8; 32] = keccak256(VOTE_TYPEHASH_STR).into();
        let struct_hash: [u8; 32] = keccak256(
            &(
                typehash,
                U256::from(e3_id),
                FixedBytes::<32>::from(accusation_id),
                voter,
                FixedBytes::<32>::from(data_hash),
                U256::from(deadline),
            )
                .abi_encode(),
        )
        .into();

        let mut buf = Vec::with_capacity(2 + 32 + 32);
        buf.push(0x19);
        buf.push(0x01);
        buf.extend_from_slice(&domain_separator);
        buf.extend_from_slice(&struct_hash);
        keccak256(&buf).into()
    }

    /// The actor's `vote_digest` must equal the reference digest byte-for-byte.
    /// If this fails, the actor's typehash / domain / struct layout has drifted
    /// from what the on-chain verifier expects (or from the constants in
    /// `e3_events::accusation_vote`).
    #[test]
    fn vote_digest_matches_reference() {
        let chain_id = 31337u64;
        let verifying_contract: Address = "0x9999999999999999999999999999999999999999"
            .parse()
            .unwrap();
        let voter: Address = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();
        let accusation_id = [0xab; 32];
        let data_hash = [0xcd; 32];
        let deadline: u64 = 1_700_000_000;

        let vote = AccusationVote {
            e3_id: E3id::new("42", chain_id),
            accusation_id,
            voter,
            data_hash,
            deadline,
            signature: ArcBytes::default(),
        };

        let actor = AccusationManager::vote_digest(&vote, verifying_contract);
        let reference = reference_vote_digest(
            chain_id,
            verifying_contract,
            42,
            accusation_id,
            voter,
            data_hash,
            deadline,
        );

        assert_eq!(
            actor, reference,
            "AccusationManager::vote_digest drifted from the reference EIP-712 \
             computation. Check VOTE_TYPEHASH_STR / VOTE_DOMAIN_NAME against \
             SlashingManager.sol — these MUST stay byte-equal across crates."
        );
    }

    /// Sign-and-recover round-trip using the actor's digest. Since
    /// `vote_digest_matches_reference` already pins the digest bytes, signing
    /// that digest and recovering via `recover_address_from_prehash` must
    /// return the voter — i.e. the actor's signatures will be accepted by the
    /// on-chain `ECDSA.recover` step.
    #[test]
    fn actor_signature_recovers_to_voter() {
        let signer: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse()
                .unwrap();
        let voter = signer.address();
        let verifying_contract: Address = "0x5555555555555555555555555555555555555555"
            .parse()
            .unwrap();
        let chain_id = 31337u64;

        let vote = AccusationVote {
            e3_id: E3id::new("12345", chain_id),
            accusation_id: [0x07; 32],
            voter,
            data_hash: [0x08; 32],
            deadline: 1_700_000_000,
            signature: ArcBytes::default(),
        };

        let digest = AccusationManager::vote_digest(&vote, verifying_contract);
        let sig = signer
            .sign_hash_sync(&FixedBytes::<32>::from(digest))
            .unwrap();
        let recovered = sig
            .recover_address_from_prehash(&FixedBytes::<32>::from(digest))
            .expect("recover");
        assert_eq!(
            recovered, voter,
            "signing the actor's digest and recovering must yield the voter"
        );
    }

    /// The accusation digest must include `deadline`. A malicious peer could
    /// otherwise rewrite the deadline in transit without invalidating the
    /// accuser's signature. Guard: changing only `deadline` must change the
    /// digest.
    #[test]
    fn accusation_digest_binds_deadline() {
        let make = |deadline: u64| ProofFailureAccusation {
            e3_id: E3id::new("9", 31337),
            accuser: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
            accused: "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .parse()
                .unwrap(),
            accused_party_id: 1,
            proof_type: ProofType::C1PkGeneration,
            data_hash: [0x42; 32],
            deadline,
            signed_payload: None,
            signature: ArcBytes::default(),
        };
        let a = AccusationManager::accusation_digest(&make(1_700_000_000));
        let b = AccusationManager::accusation_digest(&make(1_700_000_001));
        assert_ne!(a, b, "deadline must be part of the accusation digest");
    }
}

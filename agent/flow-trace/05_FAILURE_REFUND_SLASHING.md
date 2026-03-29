# Part 5: Failure, Refunds & Slashing

## Overview

An E3 can fail at any stage due to timeouts, insufficient participants, or misbehavior. When failure
is detected, the protocol refunds the requester (proportional to work not completed), compensates
honest nodes, and slashes misbehaving operators.

---

## Failure Detection

### Timeout-Based Failure (Permissionless)

Anyone can call `markE3Failed()` when a deadline is missed:

> **NOTE:** The `gracePeriod` is stored in `_timeoutConfig` and validated on config update, but it
> is **NOT added** to the deadline checks in `_checkFailureCondition()`. The actual checks compare
> `block.timestamp` directly against the raw deadlines (which themselves already incorporate the
> window durations). This may be intentional (grace already baked into the window sizes) or a
> missing feature.

```
Anyone calls: Enclave.markE3Failed(e3Id)
│
├─ Revert if stage == None, Complete, or Failed
│
├─ CHECK 1: Committee Formation Timeout
│   stage == Requested
│   AND block.timestamp > committeeDeadline
│   → Reason: CommitteeFormationTimeout
│
├─ CHECK 2: DKG Timeout
│   stage == CommitteeFinalized
│   AND block.timestamp > dkgDeadline
│   → Reason: DkgTimeout
│
├─ CHECK 3: Compute Timeout
│   stage == KeyPublished
│   AND block.timestamp > computeDeadline
│   → Reason: ComputeTimeout
│
├─ CHECK 4: Decryption Timeout
│   stage == CiphertextReady
│   AND block.timestamp > decryptionDeadline
│   → Reason: DecryptionTimeout
│
└─ If ANY check passes:
    _e3Stages[e3Id] = E3Stage.Failed
    _e3FailureReasons[e3Id] = reason
    Emit E3StageChanged(e3Id, currentStage, E3Stage.Failed)
    Emit E3Failed(e3Id, currentStage, reason)
```

### Contract-Triggered Failure

```
CiphernodeRegistry or SlashingManager calls:
  Enclave.onE3Failed(e3Id, reason)
│
├─ require(caller == ciphernodeRegistry || caller == slashingManager)
├─ _e3Stages[e3Id] = Failed
├─ _e3FailureReasons[e3Id] = reason
└─ Emit E3StageChanged, E3Failed
```

Specific triggers:

- **InsufficientCommitteeMembers**: `finalizeCommittee()` when < N nodes submitted tickets
- **Committee became non-viable**: SlashingManager expelled enough members to drop below threshold M

---

## Refund Processing

### Step 1: Process Failure

```
Anyone calls: Enclave.processE3Failure(e3Id)
│
├─ require(stage == Failed)
├─ require(e3Payments[e3Id] > 0) → payment exists
│
├─ 1. payment = e3Payments[e3Id]
├─ 2. e3Payments[e3Id] = 0  (prevent double-processing)
│
├─ 3. Get honest nodes:
│     (honestNodes, _) = ciphernodeRegistry.getActiveCommitteeNodes(e3Id)
│     → Returns committee members NOT expelled by slashing plus their ticket scores
│
├─ 4. Transfer payment to E3RefundManager:
│     paymentToken = _e3FeeTokens[e3Id]  (per-E3 token, not current global)
│     paymentToken.transfer(e3RefundManager, payment)
│
├─ 5. e3RefundManager.calculateRefund(
│       e3Id, payment, honestNodes, paymentToken
│     )
│     │
│     │  ┌─── E3RefundManager.calculateRefund() ────────────────┐
│     │  │                                                       │
│     │  │  1. Determine work completed based on failure stage:  │
│     │  │                                                       │
│     │  │  Stage at Failure     │ Work Done │ Work Left │Proto  │
│     │  │  ─────────────────────┼───────────┼───────────┼────── │
│     │  │  Requested / None     │    0 BPS  │  9500 BPS │ 500   │
│     │  │  (no committee yet)   │    (0%)   │   (95%)   │ (5%)  │
│     │  │  CommitteeFinalized   │ 1000 BPS  │  8500 BPS │ 500   │
│     │  │  (DKG failed)         │   (10%)   │   (85%)   │ (5%)  │
│     │  │  KeyPublished         │ 4000 BPS  │  5500 BPS │ 500   │
│     │  │  (compute failed)     │   (40%)   │   (55%)   │ (5%)  │
│     │  │  CiphertextReady      │ 4000 BPS  │  5500 BPS │ 500   │
│     │  │  (decryption failed)  │   (40%)   │   (55%)   │ (5%)  │
│     │  │                                                       │
│     │  │  NOTE: KeyPublished and CiphertextReady have the SAME │
│     │  │  work-completed value (4000 BPS). The decryptionBps    │
│     │  │  (5500) is NOT added for CiphertextReady — decryption │
│     │  │  work is not counted as completed until E3 is Complete.│
│     │  │                                                       │
│     │  │  2. Calculate amounts:                                │
│     │  │     honestNodeAmount = payment * workDoneBps / 10000  │
│     │  │     requesterAmount = payment * workLeftBps / 10000   │
│     │  │     protocolAmount = payment - honest - requester     │
│     │  │                                                       │
│     │  │  3. Transfer protocol fee to treasury immediately     │
│     │  │                                                       │
│     │  │  4. Store RefundDistribution {                        │
│     │  │       honestNodeAmount, requesterAmount,              │
│     │  │       protocolAmount, totalSlashed: 0,                │
│     │  │       honestNodeCount, feeToken,                      │
│     │  │       originalPayment                                 │
│     │  │     }                                                 │
│     │  │                                                       │
│     │  │  5. Drain pending slashed funds queue:                │
│     │  │     pending = _pendingSlashedFunds[e3Id]              │
│     │  │     if pending > 0:                                   │
│     │  │       _applySlashedFunds(e3Id, pending)               │
│     │  │       (see "Slashed Funds Routing" section below)     │
│     │  │     → Handles slashes that arrived BEFORE             │
│     │  │       processE3Failure was called                     │
│     │  │                                                       │
│     │  │  6. Emit RefundDistributionCalculated(e3Id,           │
│     │  │       honestNodeAmount, requesterAmount, protocolAmt) │
│     │  └───────────────────────────────────────────────────────┘
│
└─ Emit E3FailureProcessed(e3Id)
```

### Step 2: Claim Refunds

```
REQUESTER claims:
  E3RefundManager.claimRequesterRefund(e3Id)
│
├─ require(distribution calculated)
├─ require(msg.sender == requester from Enclave)
├─ require(!already claimed)
├─ requesterAmount includes BOTH:
│   • Base refund (from work-value BPS allocation)
│   • Slashed funds (requester filled first, up to originalPayment)
├─ Transfer requesterAmount in the per-E3 fee token
└─ Emit RefundClaimed(e3Id, requester, amount)

HONEST NODE claims:
  E3RefundManager.claimHonestNodeReward(e3Id)
│
├─ require(distribution calculated)
├─ require(msg.sender is in honestNodes[e3Id])
├─ require(!already claimed by this node)
├─ honestNodeAmount includes BOTH:
│   • Base compensation (from work-value BPS allocation)
│   • Slashed funds surplus (after requester is made whole)
├─ perNodeAmount = honestNodeAmount / honestNodeCount
├─ Last claimer gets dust (remainder)
├─ Transfer directly to node (not via BondingRegistry)
└─ Emit RefundClaimed(e3Id, node, amount)
```

### Refund Example (Base Only)

```
Scenario: E3 fails at KeyPublished stage (compute timeout)
  Payment: 1,000,000 USDC (1 USDC in base units = 1e6)
  Honest nodes: 3 (out of 5 committee members, 2 were slashed)

  Work completed:  40% → honestNodeAmount = 400,000
  Work remaining:  55% → requesterAmount  = 550,000
  Protocol fee:     5% → protocolAmount   =  50,000

  Each honest node claims: 400,000 / 3 = 133,333
  Last honest node claims: 133,333 + 1 (dust) = 133,334

  Requester claims: 550,000
  Treasury receives: 50,000 (immediately)
```

### Refund Example (With Slashed Funds)

```
Same scenario as above, then 2 nodes are slashed for 300,000 each:

  Before slash:
    requesterAmount  = 550,000
    honestNodeAmount = 400,000
    originalPayment  = 1,000,000

  Slash #1: 300,000 escrowed to refund pool
    Requester gap = 1,000,000 - 550,000 = 450,000
    toRequester   = min(300,000, 450,000) = 300,000
    toHonestNodes = 300,000 - 300,000 = 0
    → requesterAmount = 850,000, honestNodeAmount = 400,000

  Slash #2: 300,000 escrowed to refund pool
    Requester gap = 1,000,000 - 850,000 = 150,000
    toRequester   = min(300,000, 150,000) = 150,000
    toHonestNodes = 300,000 - 150,000 = 150,000
    → requesterAmount = 1,000,000, honestNodeAmount = 550,000

  Final:
    Requester claims:      1,000,000 (fully made whole)
    Each honest node gets:   550,000 / 3 = 183,333
    Treasury received:        50,000 (at processE3Failure time)
```

---

## Slashing Mechanism

### Off-Chain Fault Attribution: AccusationManager

**Actor:** `AccusationManager` (`crates/zk-prover/src/actors/accusation_manager.rs`)

The AccusationManager is a per-E3 ephemeral actor created when `CommitteeFinalized` fires. It
bridges proof verification failures to on-chain slashing through an off-chain committee quorum
protocol.

```
LIFECYCLE:
  Created by AccusationManagerExtension on CommitteeFinalized
  → Stores committee list, threshold_m, this node's address + signer
  → In-memory only (ephemeral — no persistence)
  → Destroyed by E3RequestComplete (Die signal)
```

#### Step 1: Local Proof Failure Detection

```
ProofVerificationFailed OR CommitmentConsistencyViolation event arrives
│
├─ For ProofVerificationFailed:
│   ├─ 1. Resolve accused address:
│   │     If accused_address == 0x0:
│   │       Look up from committee list by party_id
│   │
│   ├─ 2. Cache verification result:
│   │     received_data[(accused, proof_type)] = { data_hash, passed: false }
│   │
│   ├─ 3. For C3a/C3b proofs: attach signed_payload for re-verification
│   │     → Other nodes need the original proof to independently verify
│   │
│   └─ 4. Delegate to initiate_accusation()
│
├─ For CommitmentConsistencyViolation:
│   ├─ 1. Cache verification result:
│   │     received_data[(accused, proof_type)] = { data_hash, passed: false }
│   │
│   └─ 2. Delegate to initiate_accusation() (no forwarded payload)
│
└─ initiate_accusation() — shared logic:
    │
    ├─ 3. Dedup check:
    │     If (accused, proof_type) already in accused_proofs set:
    │       → Return (already accused, skip)
    │     Else: insert into accused_proofs
    │
    ├─ 4. Create and SIGN accusation:
    │     ProofFailureAccusation {
    │       e3_id, accuser: my_address, accused, accused_party_id,
    │       proof_type, data_hash, signed_payload (C3 only),
    │       signature: ecSign(accusation_digest)
    │     }
    │
    ├─ 5. Broadcast accusation via P2P gossip
    │
    ├─ 6. Cast OWN VOTE (agrees = true):
    │     AccusationVote {
    │       e3_id, accusation_id, voter: my_address,
    │       agrees: true, data_hash,
    │       signature: ecSign(vote_digest)
    │     }
    │     → Broadcast via P2P gossip
    │
    ├─ 7. Start vote timeout (300 seconds):
    │     → If quorum not reached by timeout, resolve as Inconclusive
    │
    └─ 8. Check for immediate quorum (if threshold_m == 1)
```

#### Step 2: Incoming Accusation Handling

```
ProofFailureAccusation arrives via P2P from another committee member
│
├─ 1. Verify accuser is a committee member
│
├─ 2. Verify accuser's ECDSA signature on accusation digest
│
├─ 3. Compute accusation_id:
│     keccak256(abi.encodePacked(chainId, e3Id, accused, proofType))
│     → Deterministic: all nodes compute same ID for same accusation
│
├─ 4. Determine own vote based on local verification cache:
│     │
│     ├─ Case A: We already FAILED verification for (accused, proof_type):
│     │   → Vote agrees = true
│     │
│     ├─ Case B: We already PASSED verification for (accused, proof_type):
│     │   → Vote agrees = false
│     │
│     └─ Case C: Unknown (haven't verified yet):
│         ├─ For C3a/C3b: re-verify using signed_payload from accusation
│         │   → Dispatch to ZkActor for local re-verification
│         │   → Vote after re-verification completes
│         └─ For other proofs: vote agrees = false (no local evidence)
│
├─ 5. Create and SIGN vote:
│     AccusationVote {
│       e3_id, accusation_id, voter: my_address,
│       agrees: <determined above>, data_hash,
│       signature: ecSign(vote_digest)
│     }
│     → Broadcast via P2P gossip
│
└─ 6. Check quorum immediately
```

#### Step 3: Vote Digest & Accusation ID (Must Match Solidity)

```
Accusation ID (deterministic, same on Rust + Solidity):
  accusation_id = keccak256(abi.encodePacked(
    chainId, e3Id, accused_address, proofType
  ))

Vote Digest (EIP-191 signed, verified on-chain):
  vote_digest = keccak256(abi.encode(
    VOTE_TYPEHASH,           // "AccusationVote(uint256 chainId,...)"
    chainId,
    e3Id,
    accusation_id,
    voter_address,
    agrees,                  // bool
    data_hash                // keccak256 of the proof data
  ))
  signature = personal_sign(vote_digest, voter_private_key)

CRITICAL: These type hashes MUST match the Solidity constants:
  VOTE_TYPEHASH = keccak256(
    "AccusationVote(uint256 chainId,uint256 e3Id,"
    "bytes32 accusationId,address voter,"
    "bool agrees,bytes32 dataHash)"
  )
```

#### Step 4: Quorum Decision Logic

```
check_quorum(accusation_id):
│
├─ Count: agree_count, disagree_count, total_votes
│
├─ CASE A: agree_count >= threshold_m
│   │
│   ├─ Check for equivocation:
│   │   All agreeing voters have same data_hash?
│   │   ├─ YES → AccusationOutcome::AccusedFaulted (SLASHABLE)
│   │   │   → accused sent the same bad proof to everyone
│   │   └─ NO  → AccusationOutcome::Equivocation (SLASHABLE)
│   │       → accused sent DIFFERENT data to different nodes
│   │
│   └─ Emit AccusationQuorumReached
│
├─ CASE B: agree_count + remaining_voters < threshold_m
│   │   → Mathematically impossible to reach quorum
│   │
│   ├─ Multiple data_hashes across ALL votes?
│   │   └─ YES → AccusationOutcome::Equivocation (SLASHABLE)
│   │
│   ├─ Only accuser says bad, others disagree?
│   │   └─ AccusationOutcome::AccuserLied (NOT slashable)
│   │
│   └─ Otherwise → AccusationOutcome::Inconclusive (NOT slashable)
│
└─ CASE C: Still waiting for more votes
    → Timeout (300s) handles this case → resolves as Inconclusive
```

#### Step 5: On-Chain Slash Submission

```
AccusationQuorumReached event arrives at SlashingManagerSolWriter
│
├─ Only for SLASHABLE outcomes (AccusedFaulted, Equivocation):
│
├─ 1. STAGGERED SUBMISSION (fallback submitters):
│     Rank all agreeing voters by address (sorted ascending)
│     My rank = position in sorted list
│     │
│     ├─ Rank 0 (primary): submit immediately
│     ├─ Rank 1: wait 30 seconds, then submit
│     ├─ Rank 2: wait 60 seconds, then submit
│     └─ ... (each rank waits rank × 30 seconds)
│     → Prevents multiple nodes wasting gas on same slash
│     → Higher-rank submitters expect DuplicateEvidence revert
│
├─ 2. Encode attestation evidence:
│     proof = abi.encode(
│       proofType,       // uint256 — which proof failed (C0-C7)
│       voters[],        // address[] — sorted ascending
│       agrees[],        // bool[] — all true (only agreeing votes submitted)
│       dataHashes[],    // bytes32[] — per-voter data hashes
│       signatures[]     // bytes[] — per-voter ECDSA signatures
│     )
│
├─ 3. Call SlashingManager.proposeSlash(e3Id, accused, proof)
│     → On-chain verification happens (see Lane A below)
│
└─ 4. Handle result:
     ├─ Success: log transaction hash
     └─ DuplicateEvidence: expected for fallback submitters (logged as warning)
```

### Lane A: Attestation-Based Slashing (Permissionless, Atomic)

```
Anyone calls: SlashingManager.proposeSlash(e3Id, operator, proof)
│
├─ 1. Decode proof:
│     (proofType, voters[], agrees[], dataHashes[], signatures[])
│     = abi.decode(proof, (...))
│
├─ 2. Derive slash reason deterministically:
│     reason = keccak256(abi.encodePacked(proofType))
│     → Eliminates cross-reason replay
│     → Each proofType maps to one policy (E3_BAD_DKG_PROOF, etc.)
│
├─ 3. Load policy:
│     policy = slashPolicies[reason]
│     require(policy.enabled)
│     require(policy.requiresProof)  → Lane A only
│
├─ 4. Verify operator is committee member:
│     require(ciphernodeRegistry.isCommitteeMember(e3Id, operator))
│
├─ 5. Replay protection:
│     evidenceKey = keccak256(abi.encodePacked(chainId, e3Id, operator, proofType))
│     require(!evidenceConsumed[evidenceKey])
│     evidenceConsumed[evidenceKey] = true
│
├─ 6. VERIFY ATTESTATION EVIDENCE:
│     _verifyAttestationEvidence(proof, e3Id, operator)
│     │
│     │  ┌─── Attestation Verification ─────────────────────────┐
│     │  │                                                       │
│     │  │  1. Validate array lengths match (voters, agrees,    │
│     │  │     dataHashes, signatures all same length)           │
│     │  │                                                       │
│     │  │  2. Compute accusation_id:                            │
│     │  │     keccak256(abi.encodePacked(                       │
│     │  │       chainId, e3Id, operator, proofType              │
│     │  │     ))                                                │
│     │  │     → SAME formula as Rust AccusationManager          │
│     │  │                                                       │
│     │  │  3. Check quorum: numVotes >= threshold_m             │
│     │  │     → Get threshold from ciphernodeRegistry           │
│     │  │                                                       │
│     │  │  4. For EACH voter:                                   │
│     │  │     ├─ Ascending order check (prevents duplicates):   │
│     │  │     │   require(voter > prevVoter)                    │
│     │  │     ├─ Conflict check (accused can't vote):           │
│     │  │     │   require(voter != operator)                    │
│     │  │     ├─ All votes must agree:                          │
│     │  │     │   require(agrees[i] == true)                    │
│     │  │     ├─ Voter must be active committee member:         │
│     │  │     │   require(isCommitteeMemberActive(e3Id, voter)) │
│     │  │     └─ VERIFY ECDSA SIGNATURE:                        │
│     │  │         hash = toEthSignedMessageHash(                │
│     │  │           keccak256(abi.encode(                       │
│     │  │             VOTE_TYPEHASH, chainId, e3Id,             │
│     │  │             accusationId, voter, agrees[i],           │
│     │  │             dataHashes[i]                              │
│     │  │           ))                                           │
│     │  │         )                                              │
│     │  │         require(ECDSA.recover(hash, sig) == voter)    │
│     │  │         → Proves voter actually signed this vote      │
│     │  │                                                       │
│     │  └───────────────────────────────────────────────────────┘
│
├─ 7. Create proposal with SNAPSHOTTED policy values:
│     proposal = SlashProposal {
│       e3Id, operator, reason,
│       ticketAmount: policy.ticketPenalty,
│       licenseAmount: policy.licensePenalty,
│       proofVerified: true,          // Lane A marker
│       executableAt: block.timestamp, // immediate
│       banNode: policy.banNode,
│       affectsCommittee: policy.affectsCommittee,
│       failureReason: policy.failureReason
│     }
│     → Policy values snapshotted at proposal time
│     → Prevents execution drift if policy changes later
│
└─ 8. IMMEDIATELY execute:
      _executeSlash(proposalId)
      │
      │  (see "Slash Execution" below)
```

### Lane B: Evidence-Based Slashing (Delayed, With Appeals)

```
SLASHER_ROLE calls: SlashingManager.proposeSlashEvidence(
  e3Id, operator, reason, evidence
)
│
├─ 1. Load policy = slashPolicies[reason]
│     require(policy.enabled)
│     require(!policy.requiresProof) → evidence-based only
│     → reason is an explicit bytes32, not derived from proof
│
├─ 2. Replay protection:
│     evidenceHash = keccak256(abi.encode(e3Id, operator, keccak256(evidence)))
│     require(!evidenceConsumed[evidenceHash])
│     evidenceConsumed[evidenceHash] = true
│
├─ 3. Create proposal with SNAPSHOTTED policy values:
│     proposal = SlashProposal {
│       e3Id, operator, reason,
│       ticketAmount: policy.ticketPenalty,
│       licenseAmount: policy.licensePenalty,
│       proofVerified: false,
│       executableAt: block.timestamp + policy.appealWindow,
│       banNode: policy.banNode,
│       affectsCommittee: policy.affectsCommittee,
│       failureReason: policy.failureReason
│     }
│     → NOT executed immediately
│
└─ 4. Emit SlashProposed(proposalId, e3Id, operator, reason)

─── APPEAL WINDOW OPENS ─────────────────────────────────────

Operator (accused) calls: SlashingManager.fileAppeal(proposalId, evidence)
│
├─ require(msg.sender == proposal.operator)
├─ require(block.timestamp < proposal.executableAt)
│   → Must appeal before window closes
├─ require(!proposal.proofVerified)
│   → Cannot appeal proof-based slashes
├─ require(!proposal.appealed)
│   → Only one appeal per proposal
├─ proposal.appealed = true
├─ proposal.appealEvidence = evidence
└─ Emit AppealFiled(proposalId, evidence)

GOVERNANCE_ROLE resolves: SlashingManager.resolveAppeal(
  proposalId, upheld, resolution
)
│
├─ require(proposal.appealed && !proposal.resolved)
├─ proposal.resolved = true
├─ proposal.appealUpheld = upheld
└─ Emit AppealResolved(proposalId, upheld, resolution)

─── AFTER APPEAL WINDOW ──────────────────────────────────────

Anyone calls: SlashingManager.executeSlash(proposalId)
│
├─ require(!proposal.executed && !proposal.proofVerified)
├─ require(block.timestamp >= proposal.executableAt)
├─ If appealed:
│   require(proposal.resolved)
│   require(!proposal.appealUpheld)
│   → If appeal was upheld, slash is cancelled
│
└─ _executeSlash(proposalId, policy)
```

### Slash Execution (Both Lanes)

```
_executeSlash(proposalId):
│
├─ proposal.executed = true
│
├─ 1. SLASH TICKET BALANCE (if ticketAmount > 0):
│     actualTicketSlashed = bondingRegistry.slashTicketBalance(
│       operator, proposal.ticketAmount, reason
│     )
│     → Returns ACTUAL amount slashed (may be less if balance insufficient)
│     │
│     │  ┌─── BondingRegistry.slashTicketBalance() ─────────────┐
│     │  │                                                       │
│     │  │  1. Slash from ACTIVE balance first:                  │
│     │  │     activeBalance = ticketToken.balanceOf(operator)   │
│     │  │     slashFromActive = min(amount, activeBalance)      │
│     │  │     ticketToken.burnTickets(operator, slashFromActive)│
│     │  │     → Burns ETK, underlying stays as payableBalance   │
│     │  │                                                       │
│     │  │  2. Remaining from EXIT QUEUE:                        │
│     │  │     remaining = amount - slashFromActive              │
│     │  │     if remaining > 0:                                 │
│     │  │       _exits.slashPendingAssets(                      │
│     │  │         operator, remaining, 0,                       │
│     │  │         includeLockedAssets=true                      │
│     │  │       )                                               │
│     │  │       → Can slash EVEN LOCKED exit tranches           │
│     │  │       → No escaping via queued exits                  │
│     │  │                                                       │
│     │  │  3. slashedTicketBalance += totalSlashed              │
│     │  │     → Tracked for redirect to refund pool or treasury │
│     │  │                                                       │
│     │  │  4. _updateOperatorStatus(operator)                   │
│     │  │     → May deactivate if below thresholds              │
│     │  └───────────────────────────────────────────────────────┘
│
├─ 2. SLASH LICENSE BOND (if licenseAmount > 0):
│     bondingRegistry.slashLicenseBond(
│       operator, proposal.licenseAmount, reason
│     )
│     │
│     │  ┌─── BondingRegistry.slashLicenseBond() ───────────────┐
│     │  │                                                       │
│     │  │  1. Slash from ACTIVE bond first:                     │
│     │  │     slashFromActive = min(amount, licenseBond)        │
│     │  │     operators[op].licenseBond -= slashFromActive      │
│     │  │                                                       │
│     │  │  2. Remaining from EXIT QUEUE:                        │
│     │  │     _exits.slashPendingAssets(                        │
│     │  │       operator, 0, remaining,                         │
│     │  │       includeLockedAssets=true                        │
│     │  │     )                                                 │
│     │  │                                                       │
│     │  │  3. slashedLicenseBond += totalSlashed                │
│     │  │  4. _updateOperatorStatus(operator)                   │
│     │  └───────────────────────────────────────────────────────┘
│
├─ 3. BAN NODE (if proposal.banNode):
│     banned[operator] = true
│     Emit NodeBanUpdated(operator, true, reason, address(this))
│     → Banned nodes cannot re-register
│     → Only governance can lift ban
│
├─ 4. COMMITTEE EXPULSION (if proposal.affectsCommittee):
│     (activeCount, thresholdM) =
│       ciphernodeRegistry.expelCommitteeMember(
│         e3Id, operator, reason
│       )
│     │
│     │  ┌─── CiphernodeRegistry.expelCommitteeMember() ────────┐
│     │  │                                                       │
│     │  │  1. If already expelled: return (no-op, idempotent)   │
│     │  │  2. committees[e3Id].active[operator] = false         │
│     │  │  3. committees[e3Id].activeCount--                    │
│     │  │  4. Emit CommitteeMemberExpelled(e3Id, operator)      │
│     │  │  5. Return (activeCount, threshold[0])                │
│     │  └───────────────────────────────────────────────────────┘
│     │
│     └─ If activeCount < thresholdM AND proposal.failureReason > 0:
│         try enclave.onE3Failed(e3Id, proposal.failureReason)
│         → Committee can no longer meet threshold
│         → E3 is irrecoverably failed
│         catch: emit RoutingFailed (E3 may already be failed)
│         → Slash itself still proceeds regardless
│
│
├─ 5. SLASHED FUNDS ESCROWING (if actualTicketSlashed > 0):
│     │
│     │  Always escrows — regardless of E3 stage.
│     │  Destination decided later at terminal state.
│     │
│     │  Self-call for atomicity:
│     │  try this.escrowSlashedFundsToRefund(e3Id, actualTicketSlashed)
│     │  │
│     │  │  ┌─── escrowSlashedFundsToRefund() ───────────────────┐
│     │  │  │  require(msg.sender == address(this))              │
│     │  │  │  → Self-call only (for try/catch atomicity)        │
│     │  │  │                                                    │
│     │  │  │  Step A: Move USDC from BondingRegistry            │
│     │  │  │    bondingRegistry.redirectSlashedTicketFunds(      │
│     │  │  │      e3RefundManager, amount                       │
│     │  │  │    )                                               │
│     │  │  │    │                                               │
│     │  │  │    ├─ slashedTicketBalance -= amount                │
│     │  │  │    └─ ticketToken.payout(e3RefundManager, amount)   │
│     │  │  │       → Transfers UNDERLYING USDC (not ticket      │
│     │  │  │         tokens) to the E3RefundManager contract     │
│     │  │  │       → Uses payableBalance incremented by          │
│     │  │  │         burnTickets() during slashTicketBalance     │
│     │  │  │                                                    │
│     │  │  │  Step B: Update escrow accounting                  │
│     │  │  │    enclave.escrowSlashedFunds(e3Id, amount)         │
│     │  │  │    → e3RefundManager.escrowSlashedFunds(e3Id, amt)  │
│     │  │  │      │                                             │
│     │  │  │      ├─ If refund distribution NOT yet calculated:  │
│     │  │  │      │   _pendingSlashedFunds[e3Id] += amount       │
│     │  │  │      │   → Queued until terminal state is reached   │
│     │  │  │      │                                             │
│     │  │  │      └─ If refund distribution IS calculated:       │
│     │  │  │          require(no claims started yet)             │
│     │  │  │          _applySlashedFunds(e3Id, amount)           │
│     │  │  │          (see priority logic below — failure path)  │
│     │  │  │                                                    │
│     │  │  │  If EITHER step reverts → both revert together     │
│     │  │  │  → Funds stay in BondingRegistry for treasury      │
│     │  │  │  → Slash itself still proceeds                     │
│     │  │  └────────────────────────────────────────────────────┘
│     │
│     └─ catch: emit RoutingFailed(e3Id, actualTicketSlashed)
│        → Slash is NOT rolled back, only fund escrowing fails
│
└─ 6. Emit SlashExecuted(proposalId, e3Id, operator, reason,
       ticketSlashed, licenseSlashed, banned)
```

### Slashed Funds Priority Logic (Failure Path): \_applySlashedFunds()

```
_applySlashedFunds(e3Id, amount):
│
├─ Priority: MAKE REQUESTER WHOLE FIRST
│
├─ requesterGap = originalPayment - dist.requesterAmount
│   → How much more the requester needs to reach their original payment
│
├─ toRequester = min(amount, requesterGap)
│   → Fill requester up to originalPayment, no more
│
├─ toHonestNodes = amount - toRequester
│   → Surplus (after requester is whole) goes to honest nodes
│
├─ dist.requesterAmount += toRequester
├─ dist.honestNodeAmount += toHonestNodes
├─ dist.totalSlashed += amount
│
└─ Emit SlashedFundsApplied(e3Id, toRequester, toHonestNodes)

Design rationale:
  The requester PAID for the computation and got nothing. They should
  be made whole before honest nodes receive any slash-based bonus.
  Honest nodes already receive compensation via the base BPS allocation
  for work they completed.
```

### Slashed Funds Distribution (Success Path): distributeSlashedFundsOnSuccess()

```
distributeSlashedFundsOnSuccess(e3Id, activeNodes, paymentToken):
│
├─ Called by Enclave._distributeRewards() when E3 completes successfully
│
├─ escrowed = _pendingSlashedFunds[e3Id]
│   if escrowed == 0: return (nothing to distribute)
│
├─ _pendingSlashedFunds[e3Id] = 0
│
├─ Split using WorkValueAllocation.successSlashedNodeBps (default 5000):
│   toNodes = escrowed * successSlashedNodeBps / 10000
│   toTreasury = escrowed - toNodes
│
├─ Distribute toNodes evenly to activeNodes:
│   perNode = toNodes / activeNodes.length
│   dust = toNodes % activeNodes.length → last node
│   paymentToken.transfer(node, perNode) for each
│
├─ Transfer toTreasury to protocolTreasury
│
└─ Emit SlashedFundsDistributedOnSuccess(e3Id, toNodes, toTreasury)

Design rationale:
  On success the requester got their computation. Slashed funds are
  split between honest committee members (reward for completing despite
  a slashed peer) and the protocol treasury.
```

### Slashed Funds Ordering: Escrow → Terminal State Resolution

```
Slashing always escrows funds in _pendingSlashedFunds[e3Id],
regardless of the current E3 stage. The destination is decided
only when the E3 reaches a terminal state (Complete or Failed).

── FAILURE PATH ──────────────────────────────────────────────

Case 1: Slash happens BEFORE processE3Failure
  → escrowSlashedFunds sees !dist.calculated
  → Funds queued in _pendingSlashedFunds[e3Id]
  → When processE3Failure → calculateRefund runs:
     drains pending queue via _applySlashedFunds
     (requester filled first, surplus to honest nodes)

Case 2: Slash happens AFTER processE3Failure
  → escrowSlashedFunds sees dist.calculated
  → _applySlashedFunds runs immediately
  → require(no claims started) — reverts if too late

Case 3: Multiple slashes on same E3 (failure)
  → Each slash independently escrows funds
  → Priority logic runs per-slash: requester filled first each time
  → totalSlashed accumulates across all slashes

── SUCCESS PATH ──────────────────────────────────────────────

Case 4: E3 completes successfully with escrowed slashed funds
  → _distributeRewards calls distributeSlashedFundsOnSuccess
  → _pendingSlashedFunds[e3Id] split between nodes and treasury
  → Nodes receive successSlashedNodeBps portion (default 50%)
  → Treasury receives the remainder
```

### Slash Policy Configuration

```
SlashPolicy {
  ticketPenalty:    uint256   // tickets to slash (in base units)
  licensePenalty:   uint256   // ENCL to slash
  requiresProof:   bool      // Lane A (true) or Lane B (false)
  proofVerifier:    address   // verifier address (Lane A: used in policy lookup)
  banNode:          bool      // permanently ban operator
  appealWindow:     uint256   // seconds for appeal (Lane B only, 0 for Lane A)
  enabled:          bool      // policy active
  affectsCommittee: bool      // expel from E3 committee
  failureReason:    uint8     // FailureReason enum (0 = no E3 failure)
}

Constraints:
- If requiresProof: appealWindow must be 0 (atomic execution, no appeal)
- If !requiresProof: appealWindow must be > 0 (delayed execution, with appeal)
- At least one penalty must be non-zero

Slash Reasons (derived from ProofType for Lane A):
  reason = keccak256(abi.encodePacked(proofType))
  ┌─────────────────┬──────────────────────────┐
  │ ProofType       │ Slash Reason             │
  ├─────────────────┼──────────────────────────┤
  │ C0, C1-C4       │ E3_BAD_DKG_PROOF         │
  │ C5              │ E3_BAD_PK_AGGREGATION    │
  │ C6              │ E3_BAD_DECRYPTION_PROOF   │
  │ C7              │ E3_BAD_AGGREGATION_PROOF │
  └─────────────────┴──────────────────────────┘
```

### End-to-End: Proof Failure → On-Chain Slash

```
┌─────────────────────────────────────────────────────────────────┐
│              Complete Proof-to-Slash Pipeline                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. PROOF GENERATION (each committee member)                   │
│     ProofRequestActor generates & signs C0-C7 proofs           │
│     → Broadcasts signed proofs via P2P gossip                  │
│                                                                 │
│  2. PROOF VERIFICATION (each receiving committee member)       │
│     ProofVerificationActor (C0) / ShareVerificationActor       │
│     (C2/C3/C4/C6)                                              │
│     ├─ Phase 1: ECDSA signature validation (inline)            │
│     └─ Phase 2: ZK proof verification (multithread)            │
│                                                                 │
│  3. FAILURE DETECTION                                          │
│     If verification fails → SignedProofFailed event            │
│                                                                 │
│  4. ACCUSATION (AccusationManager, per-E3 actor)               │
│     ├─ Create ProofFailureAccusation (signed, broadcast)       │
│     ├─ Cast own vote (agrees=true)                             │
│     └─ Start 300s timeout                                      │
│                                                                 │
│  5. VOTING (all committee members)                             │
│     ├─ Receive accusation via P2P                              │
│     ├─ Check own verification cache                            │
│     ├─ Cast AccusationVote (signed, broadcast)                 │
│     └─ Each vote independently verified by all nodes           │
│                                                                 │
│  6. QUORUM (AccusationManager)                                 │
│     ├─ votes_for >= threshold_m → AccusedFaulted/Equivocation  │
│     └─ AccusationQuorumReached event published                 │
│                                                                 │
│  7. ON-CHAIN SUBMISSION (SlashingManagerSolWriter)             │
│     ├─ Staggered: rank 0 submits immediately                  │
│     │   ranks 1+ wait rank×30s as fallback                     │
│     ├─ Encodes attestation evidence (votes + signatures)       │
│     └─ Calls SlashingManager.proposeSlash(e3Id, operator, proof)│
│                                                                 │
│  8. ON-CHAIN VERIFICATION (Lane A, atomic)                     │
│     ├─ Verify each voter's ECDSA signature                    │
│     ├─ Verify quorum (numVotes >= threshold_m)                 │
│     ├─ Verify voters are active committee members              │
│     └─ Execute slash immediately (no appeal)                   │
│                                                                 │
│  9. PENALTIES                                                  │
│     ├─ Ticket balance slashed (active + exit queue)            │
│     ├─ License bond slashed (active + exit queue)              │
│     ├─ Node banned (if policy requires)                        │
│     ├─ Committee member expelled                               │
│     └─ Slashed USDC escrowed in E3RefundManager                │
│                                                                 │
│  10. FUND DISTRIBUTION (at E3 terminal state)                  │
│      ├─ Failure: requester refunded first, surplus to honest   │
│      └─ Success: nodes + treasury split                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Slashed Funds: Escrow Model & Final Destinations

Slashed ticket funds are always escrowed first. Their final destination depends on the E3's terminal
state:

```
STEP 1: ESCROWING (always, at slash time)
  Triggered by: _executeSlash → escrowSlashedFundsToRefund
  When: Any slash with actualTicketSlashed > 0, regardless of E3 stage
  Flow: BondingRegistry.redirectSlashedTicketFunds(refundManager, amount)
    → ticketToken.payout(refundManager, amount)
    → USDC moves to E3RefundManager
    → _pendingSlashedFunds[e3Id] += amount (if not yet calculated)
  Effect: slashedTicketBalance goes UP (during slash) then DOWN (during redirect)

STEP 2a: E3 FAILS → Requester-first distribution
  Triggered by: processE3Failure → calculateRefund drains pending queue
    OR: escrowSlashedFunds after distribution is calculated
  Flow: _applySlashedFunds(e3Id, amount)
    → Requester filled up to originalPayment first
    → Surplus to honest nodes
  Claims: requester and honest nodes claim via claimRequesterRefund / claimHonestNodeReward

STEP 2b: E3 SUCCEEDS → Nodes + Treasury split
  Triggered by: _distributeRewards → distributeSlashedFundsOnSuccess
  Flow: _pendingSlashedFunds[e3Id] split by successSlashedNodeBps
    → Nodes receive their share evenly (with dust to last)
    → Treasury receives the remainder
  Effect: _pendingSlashedFunds[e3Id] set to 0

FALLBACK: TREASURY WITHDRAWAL
  Triggered by: Owner calls BondingRegistry.withdrawSlashedFunds()
  When: Escrowing failed (catch block) or leftover balance
  Flow: BondingRegistry sends to slashedFundsTreasury
    → ticketToken.payout(treasury, ticketAmount)
    → licenseToken.safeTransfer(treasury, licenseAmount)
  Effect: slashedTicketBalance decremented

License bond slashes always go to treasury (no escrow routing for ENCL).
```

---

## Rust-Side Handling

```
When CommitteeMemberExpelled event arrives from EVM:
│
├─ Event initially has party_id: None (not resolved yet)
│
├─ Sortition actor (party_id enrichment):
│   ├─ Receives raw CommitteeMemberExpelled { party_id: None }
│   ├─ Looks up the expelled node's address in the stored Committee
│   │   → Committee::party_id_for(addr) provides O(1) lookup
│   ├─ Re-publishes enriched CommitteeMemberExpelled { party_id: Some(id) }
│   └─ Ignores already-enriched events (party_id.is_some()) to avoid loops
│
├─ ThresholdKeyshare (receives enriched event):
│   ├─ Ignores raw events (party_id: None) — waits for Sortition enrichment
│   ├─ On enriched event (party_id: Some(id)):
│   │   ├─ Removes party_id from EncryptionKeyCollector
│   │   │   → May trigger aggregation if enough keys remain
│   │   └─ Removes party_id from ThresholdShareCollector
│   │       → May trigger share processing with reduced set
│   └─ Does NOT hold committee state — fully delegated to Sortition
│
├─ PublicKeyAggregator (aggregator, receives raw event):
│   ├─ Only processes raw events (party_id: None)
│   ├─ Ignores enriched events (party_id: Some) to avoid double-processing
│   └─ Reduces threshold_n
│   └─ May trigger aggregation if enough keyshares collected
│
├─ KeyshareCreatedFilterBuffer (aggregator):
│   ├─ Only processes raw events (party_id: None)
│   └─ Removes expelled node from committee filter set
│
└─ When E3Failed / E3StageChanged(Complete|Failed) arrives:
    │
    ├─ E3Router (central cleanup orchestrator):
    │   └─ Converts E3Failed / E3StageChanged(Complete|Failed) → E3RequestComplete
    │       → Single cleanup signal for all per-E3 actors
    │
    ├─ CommitteeFinalizer (direct handler — semantic work):
    │   └─ Cancels any pending committee-finalization timer for this e3_id
    │       → Prevents stale timer from firing after E3 is already terminal
    │
    ├─ Sortition (direct handler — semantic work):
    │   ├─ Decrements active job counts for each committee member
    │   │   → Frees up sortition tickets for future E3s
    │   └─ Removes e3_id from finalized_committees map
    │       → Prevents unbounded memory growth
    │
    └─ E3RequestComplete propagates to all per-E3 actors:
        ├─ ThresholdKeyshare: receives Die → actor stops
        ├─ PublicKeyAggregator: receives Die → actor stops
        ├─ ThresholdPlaintextAggregator: receives Die → actor stops
        ├─ KeyshareCreatedFilterBuffer: receives Die → actor stops
        ├─ CiphernodeSelector: cleans e3_cache entry for this e3_id
        └─ E3Router: removes E3Context for this e3_id
```

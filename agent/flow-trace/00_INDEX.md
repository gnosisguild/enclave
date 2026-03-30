# Enclave Protocol — Complete Flow Trace

## Index

| #   | File                                                                   | Covers                                                                                                                                                                                                                                                                                                                                                                |
| --- | ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | [01_REGISTRATION.md](01_REGISTRATION.md)                               | `setup`, `register`, `activate`, `status` CLI commands. On-chain registration into BondingRegistry → CiphernodeRegistry IMT. Rust-side event detection.                                                                                                                                                                                                               |
| 2   | [02_TOKENS_AND_ACTIVATION.md](02_TOKENS_AND_ACTIVATION.md)             | ENCL license bonding, USDC→ETK ticket purchasing, unbonding, burning, exit queue, claiming. Activation thresholds and the `_updateOperatorStatus` formula.                                                                                                                                                                                                            |
| 3   | [03_E3_REQUEST_AND_COMMITTEE.md](03_E3_REQUEST_AND_COMMITTEE.md)       | E3 request on-chain flow, fee payment, committee request, IMT snapshot. Rust-side sortition (score-based), on-chain ticket submission, committee finalization, `CiphernodeSelected` event.                                                                                                                                                                            |
| 4   | [04_DKG_AND_COMPUTATION.md](04_DKG_AND_COMPUTATION.md)                 | Full DKG with ZK proof pipeline: BFV keygen → C0 proof → encryption key exchange → TrBFV share generation → C1/C2/C3 proofs → share verification → Shamir secret sharing → encrypted share broadcast → C4 proofs → decryption key reconstruction. C5 proof for PK aggregation. Ciphertext output → C6 proof for decryption shares → C7 proof for plaintext → rewards. |
| 5   | [05_FAILURE_REFUND_SLASHING.md](05_FAILURE_REFUND_SLASHING.md)         | Timeout-based failure detection, `markE3Failed`, `processE3Failure`. Refund calculation (work-value allocation). Off-chain AccusationManager quorum protocol (proof failure → accusation → voting → quorum). Lane A (attestation-based, atomic) and Lane B (evidence-based, with appeals) slashing. Ticket/license slashing. Slashed funds escrow and routing.        |
| 6   | [06_DEACTIVATION_AND_COMPLETION.md](06_DEACTIVATION_AND_COMPLETION.md) | Voluntary deactivation (ticket/license withdrawal), full deregistration (IMT removal), E3 happy-path completion, node shutdown, sync/restart, exit queue timing, ban/unban.                                                                                                                                                                                           |

---

## End-to-End Happy Path Summary

```
1. SETUP        enclave ciphernode setup
                  → Config, password, private key stored locally

2. BOND         enclave ciphernode license bond --amount N
                  → ENCL tokens locked in BondingRegistry

3. TICKETS      enclave ciphernode tickets buy --amount N
                  → USDC → EnclaveTicketToken (non-transferable)

4. REGISTER     enclave ciphernode register
                  → BondingRegistry.registerOperator()
                  → CiphernodeRegistry.addCiphernode() (IMT insert)
                  → If bond+tickets meet thresholds → active=true

5. START        enclave start
                  → Node boots, syncs historical events, starts listening

6. E3 REQUEST   Requester calls Enclave.request(params)
                  → Fee paid, committee requested, IMT root snapshot

7. SORTITION    Ciphernodes compute scores, submit tickets on-chain
                  → Top N lowest scores selected

8. FINALIZE     finalizeCommittee() → committee locked in

9. DKG          Selected nodes perform distributed key generation:
                  a. BFV keygen → C0 proof (proves keypair valid)
                  b. Exchange BFV public keys (C0 verified on receipt)
                  c. TrBFV key + Shamir shares → C1/C2a/C2b/C3a/C3b proofs
                  d. Broadcast ThresholdShareCreated (all proofs attached)
                  e. Collect shares → verify C2/C3 proofs (2-phase)
                  f. Decrypt shares → calc decryption key → C4a/C4b proofs
                  g. Exchange DecryptionKeyShared → verify C4 proofs
                  h. Publish KeyshareCreated → aggregator

10. PK AGG      Aggregator aggregates pk_shares → aggregate PK
                  → C5 proof (proves aggregation correct)
                  → publishCommittee() on-chain → KeyPublished stage

11. COMPUTE     Data encrypted with aggregate PK, computation runs
                  → Ciphertext output published on-chain

12. DECRYPT     Committee members produce decryption shares
                  → C6 proof per share (proves share correctly derived)
                  → Broadcast to aggregator

13. AGGREGATE   Aggregator combines M+1 shares → plaintext
                  → C7 proof (proves reconstruction correct)

14. COMPLETE    publishPlaintextOutput() → rewards distributed
                  → Each active committee member gets fee / N
                  → Any escrowed slashed funds split:
                    nodes (successSlashedNodeBps) + treasury

15. DEREGISTER  enclave ciphernode deregister --proof X
                  → All collateral queued for exit
                  → Removed from IMT
                  → After exitDelay: claim USDC + ENCL back
```

## End-to-End Failure Path Summary

```
1-9.  Same as happy path through DKG (proofs generated at each step)

10.   PROOF FAIL  A committee member submits an invalid proof (C0-C7)
                    → ProofVerificationActor / ShareVerificationActor detects
                    → SignedProofFailed triggers AccusationManager
                  OR: Commitment consistency mismatch detected (cross-circuit)
                    → CommitmentConsistencyChecker publishes CommitmentConsistencyViolation
                    → Also triggers AccusationManager

11.   ACCUSATION  AccusationManager creates ProofFailureAccusation
                    → Signed and broadcast via P2P gossip
                    → Accuser casts own vote (agrees=true)

12.   VOTING      Committee members receive accusation
                    → Check own verification cache
                    → Cast signed AccusationVote (agree/disagree)
                    → Broadcast via P2P gossip

13.   QUORUM      AccusationManager detects quorum:
                    → votes_for >= threshold_m → AccusedFaulted/Equivocation
                    → Publishes AccusationQuorumReached

14.   SLASH SUB   SlashingManagerSolWriter submits on-chain:
                    → Staggered: rank 0 immediately, rank N waits N×30s
                    → Calls SlashingManager.proposeSlash(e3Id, operator, proof)

15.   ON-CHAIN    SlashingManager verifies attestation evidence:
                    → ECDSA signature verification per voter
                    → Quorum check (numVotes >= threshold_m)
                    → Atomic execution: slash + ban + expel

──────── OR: TIMEOUT-BASED FAILURE ────────────────────────────

10b.  TIMEOUT     A deadline is missed (committee, DKG, compute, or decryption)
                    → Anyone calls markE3Failed(e3Id)

──────── THEN: REFUND PROCESSING ──────────────────────────────

16.   PROCESS     Anyone calls processE3Failure(e3Id)
                    → Payment transferred to E3RefundManager
                    → Work-value allocation calculated (BPS-based)

17.   REFUND      Requester claims proportional refund
                    Honest nodes claim proportional compensation
                    Protocol treasury gets 5%

18.   SLASHED $   If slashed funds escrowed:
                    → Failure: requester filled FIRST, surplus → honest nodes
                    → Success: nodes + treasury split (successSlashedNodeBps)
```

## Contract Interaction Map

```
┌──────────────┐     ┌──────────────────────┐     ┌─────────────────┐
│   Enclave    │────→│ CiphernodeRegistry   │────→│ BondingRegistry │
│  (orchestr.) │←────│  (IMT, committees)   │←────│ (stakes, exits) │
└──────┬───────┘     └──────────┬───────────┘     └────────┬────────┘
       │                        │                          │
       │              ┌─────────┴──────────┐     ┌─────────┴────────┐
       │              │  SlashingManager   │────→│ EnclaveTicketTkn │
       │              │  (fault, penalties) │     │ (USDC wrapper)   │
       │              └─────────┬──────────┘     └──────────────────┘
       │                        │
       │                        │ escrowSlashedFundsToRefund:
       │                        │   BondingRegistry.redirectSlashedTicketFunds
       │                        │   → ticketToken.payout(refundMgr, USDC)
       │                        │   Enclave.escrowSlashedFunds
       │                        ▼
       ├────→ E3Program (validate, verify computation)
       ├────→ DecryptionVerifier (verify plaintext)
       └────→ E3RefundManager (failure refunds + slashed funds escrow/distribution)
                    │
                    └────→ Requester + Honest Nodes (claim refunds)
                           Active Nodes + Treasury (slashed funds on success)
```

---

## Verified Bugs & Protocol Concerns

_Found during source-code cross-referencing of these trace documents._

### Critical Doc Inaccuracies (now fixed)

| #   | Description                                                                                                                                                                                                                                                    | Where                             | Fix Applied     |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- | --------------- |
| 1   | `addTicketBalance` does NOT multiply by `ticketPrice` — raw stablecoin units are passed directly to `ticketToken.depositFrom()`. `ticketPrice` is only used in the activation check.                                                                           | BondingRegistry.sol:371           | 02_TOKENS       |
| 2   | `removeTicketBalance` does NOT multiply by `ticketPrice` — raw amount passed to `ticketToken.burnTickets()`.                                                                                                                                                   | BondingRegistry.sol:395           | 02_TOKENS       |
| 3   | `gracePeriod` is NOT added to deadline checks in `_checkFailureCondition()`. All timeout checks compare `block.timestamp` directly against the raw deadline. `gracePeriod` is only validated in `_setTimeoutConfig` but never referenced in failure detection. | Enclave.sol:860-887               | 05_FAILURE      |
| 4   | `activate()` calls `register()` → `registerOperator()` which has `require(!registered, AlreadyRegistered())`. So activate **reverts** for already-registered operators. It only works for re-registration after deregistration.                                | BondingRegistry.sol:308           | 01_REGISTRATION |
| 5   | `E3Requested` event is `(uint256 e3Id, E3 e3, IE3Program indexed e3Program)` — seed and params are inside the E3 struct, not separate parameters.                                                                                                              | IEnclave.sol:82                   | 03_E3_REQUEST   |
| 6   | `finalizeCommittee()` checks `>=` deadline, not `>`.                                                                                                                                                                                                           | CiphernodeRegistryOwnable.sol     | 03_E3_REQUEST   |
| 7   | `publishCommittee()` is `onlyOwner` restricted — centralized trust assumption acknowledged in contract TODOs.                                                                                                                                                  | CiphernodeRegistryOwnable.sol     | 04_DKG          |
| 8   | `CommitteePublished` event emits `(e3Id, nodes, publicKey, proof)` — full PK bytes and C5 proof, not just pkHash.                                                                                                                                              | CiphernodeRegistryOwnable.sol     | 04_DKG          |
| 9   | `_validateNodeEligibility` calls `bondingRegistry.getTicketBalanceAtBlock()` (not `ticketToken.getPastVotes()` directly).                                                                                                                                      | CiphernodeRegistryOwnable.sol:668 | 03_E3_REQUEST   |
| 10  | Lane A slashing uses **attestation-based** verification (committee quorum votes), not direct ZK proof re-verification on-chain. `proposeSlash()` decodes voter addresses, agrees, data hashes, and ECDSA signatures — not ZK proofs.                           | SlashingManager.sol               | 05_FAILURE      |

### Protocol Design Concerns

| #   | Concern                                  | Severity | Detail                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --- | ---------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Deregister-before-slash race**         | Accepted | SlashingManager Lane B (evidence+appeal) has a window during which the operator can deregister and claim their exit. If they do, the slash executes against 0 funds. The contract comments acknowledge this as an accepted tradeoff for the appeal window design.                                                                                                                                                                                                                                                                                                                           |
| 2   | **`publishCommittee()` is centralized**  | High     | Only the contract owner can publish the committee public key. A malicious or compromised owner could publish a fake key. The contract has `// TODO` and `// SECURITY` comments acknowledging this.                                                                                                                                                                                                                                                                                                                                                                                          |
| 3   | **`gracePeriod` is dead code**           | Medium   | `gracePeriod` is stored and validated during config updates but never actually used in any timeout check. Either the deadlines already bake in sufficient buffer, or this is a missing feature.                                                                                                                                                                                                                                                                                                                                                                                             |
| 4   | **`activate` CLI command is misleading** | Low      | Named "activate" but actually calls "register" — will fail for already-registered operators. There's no standalone way to trigger re-evaluation of active status; instead, `_updateOperatorStatus()` runs automatically inside `addTicketBalance()`, `bondLicense()`, etc.                                                                                                                                                                                                                                                                                                                  |
| 5   | **Active-job load balancing bug fixed**  | Info     | The Rust `NodeStateStore.available_tickets()` subtracts `active_jobs` from total tickets, reducing the chance of busy nodes being selected for new E3s. Previously, the `Sortition` actor's `Handler<EnclaveEvent>` was missing match arms for `E3Failed` and `E3StageChanged`, causing these events to fall to the default `_ => ()` — the typed handlers for decrementing jobs were dead code. This has been fixed: E3Failed and E3StageChanged are now routed to their handlers, and `finalized_committees` is cleaned up in `decrement_jobs_for_e3` to prevent unbounded memory growth. |
| 6   | **Committee member expulsion**           | Info     | `SlashingManager` can call `expelCommitteeMember()` mid-DKG. The `Sortition` actor enriches the raw `CommitteeMemberExpelled` event with the expelled member's `party_id` (resolved from its stored `Committee` list) and re-publishes it. `ThresholdKeyshare` then uses the enriched `party_id` to update its collectors, potentially completing DKG with fewer parties. `ThresholdKeyshare` itself does not hold committee state.                                                                                                                                                         |

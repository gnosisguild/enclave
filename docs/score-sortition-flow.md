# Complete Score Sortition Flow - End to End

This document explains the entire score sortition process, from when an E3 is requested to when the committee and public key are published.

## Overview

**Score sortition** is an alternative to distance sortition where:
- ALL eligible nodes can participate (not just closest in merkle tree)
- Nodes submit "lottery tickets" with computed scores
- Contract selects top N nodes with lowest scores
- More decentralized and fair

---

## Step-by-Step Flow

### 1. **E3 Requested** (User creates computation request)

**Contract: Enclave.sol**
```solidity
requestCompute() → emits E3Requested(e3Id, threshold, seed, ...)
```

**All Ciphernodes:**
- `EnclaveSolReader` picks up `E3Requested` event
- Converts to `EnclaveEvent::E3Requested` and broadcasts on event bus

**CiphernodeRegistry.sol** (if score sortition enabled):
```solidity
requestCommittee() is called by Enclave
  → Calls CommitteeSortition.initializeSortition(e3Id, threshold, seed, block.number)
  → Sets submission deadline = now + submissionWindow (e.g., 60 seconds)
```

**All Ciphernodes:**
- `CiphernodeSelector` receives `E3Requested`
- Checks eligibility (bonding, ticket balance)
- Performs ticket sortition locally (computes scores for all owned tickets)
- Finds best ticket (lowest score)
- Emits `CiphernodeSelected` event with ticket_id

**Aggregator:**
- `CommitteeSortitionSolWriter` (with `enable_finalizer=true`) receives `E3Requested`
- Calls `schedule_finalization(e3_id)`:
  - Sets deadline = now + submission_window (fetched from contract)
  - Stores in `pending_e3s` HashMap
  - Schedules timer to check after submission_window expires

---

### 2. **Ticket Submission Window** (Selected nodes submit tickets)

**Selected Ciphernodes:**
- `CommitteeSortitionSolWriter` receives `CiphernodeSelected` event
- Calls `submitTicket(e3Id, ticketNumber)` on contract

**Contract: CommitteeSortition.sol**
```solidity
submitTicket(e3Id, ticketNumber)
  1. Validates submission window still open (block.timestamp <= deadline)
  2. Validates node hasn't submitted before
  3. Validates node has ticket balance at snapshot block
  4. Computes score = keccak256(node || ticketNumber || e3Id || seed)
  5. Tries to insert into topNodes sorted array (size = threshold)
  6. Emits TicketSubmitted(e3Id, node, ticketNumber, score, addedToCommittee)
```

**All Ciphernodes:**
- `CommitteeSortitionSolReader` reads `TicketSubmitted` events
- Broadcasts as `EnclaveEvent::TicketSubmitted`
- Nodes can see who submitted and whether they made it into top N

---

### 3. **Submission Window Closes** (After 60 seconds)

**Aggregator:**
- `CommitteeSortitionSolWriter` has a timer running (10-second interval checks)
- `CheckDeadlines` handler finds expired E3s in `pending_e3s`
- Calls `finalize_committee(e3_id)`

**Contract: CommitteeSortition.sol**
```solidity
finalizeCommittee(e3Id)
  1. Validates submission window has closed (block.timestamp > deadline)
  2. Validates not already finalized
  3. Sets finalized = true
  4. Returns topNodes array
  5. Emits CommitteeFinalized(e3Id, topNodes[])
```

**All Ciphernodes:**
- `CommitteeSortitionSolReader` picks up `CommitteeFinalized` event
- Broadcasts as `EnclaveEvent::CommitteeFinalized(e3_id, committee[])`

**Aggregator:**
- `CommitteeSortitionSolWriter` receives `CommitteeFinalized`
- Removes e3_id from `pending_e3s` (cleanup)

---

### 4. **Keygen Starts** (Committee members generate key shares)

**Committee Members Only:**
- Receive `CommitteeFinalized` event
- Check if their address is in the committee array
- `ThresholdKeyshareExtension` starts DKG (Distributed Key Generation):
  - Generates local secret share
  - Broadcasts commitments to other committee members
  - Receives and validates shares from others
  - Computes public key share

**All Committee Members:**
- Complete DKG protocol
- Each member now has:
  - Their secret share (stored locally)
  - The aggregated public key (same for all)

---

### 5. **Public Key Aggregation** (Aggregator collects and publishes)

**Committee Members:**
- `ThresholdKeyshareExtension` emits `PublicKeyGenerated` event locally
- Contains their computed public key

**Aggregator:**
- `PublicKeyAggregatorExtension` receives multiple `PublicKeyGenerated` events
- Waits for threshold M nodes to report same public key
- Once threshold reached, emits `PublicKeyAggregated` event with:
  - `e3_id`
  - `publicKey` (aggregated)

---

### 6. **Committee Published to Registry** (Aggregator writes to blockchain)

**Aggregator:**
- `CiphernodeRegistrySolWriter` receives `PublicKeyAggregated` event
- Calls `publishCommittee(e3Id, nodes[], publicKey)` on contract

> **Note:** For score sortition, this needs an update (still TODO):
> - Should track committee from earlier `CommitteeFinalized` event
> - Use those stored nodes when publishing
> - Current code works for distance sortition

**Contract: CiphernodeRegistryOwnable.sol**
```solidity
publishCommittee(e3Id, nodes[], publicKey)
  1. Validates not already published
  2. Stores committee data
  3. Stores publicKey hash
  4. Emits CommitteePublished(e3Id, nodes[], publicKey)
```

**All Ciphernodes:**
- `CiphernodeRegistrySolReader` picks up `CommitteePublished` event
- Broadcasts as `EnclaveEvent::CommitteePublished`
- Committee is now officially registered and ready for computations

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. E3 REQUESTED                                                  │
│    Enclave.sol → E3Requested event                              │
│         ↓                                                        │
│    All Nodes: CiphernodeSelector checks eligibility             │
│         ↓                                                        │
│    Selected Nodes: Emit CiphernodeSelected(ticket_id)           │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 2. TICKET SUBMISSION (60 second window)                         │
│    Selected Nodes → CommitteeSortition.submitTicket()           │
│         ↓                                                        │
│    Contract: Validates, computes score, inserts into topN       │
│         ↓                                                        │
│    Contract: Emits TicketSubmitted for each submission          │
│                                                                  │
│    [Meanwhile: Aggregator schedules finalization timer]         │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 3. FINALIZATION (after 60s)                                     │
│    Aggregator → CommitteeSortition.finalizeCommittee()          │
│         ↓                                                        │
│    Contract: Returns topN nodes, emits CommitteeFinalized       │
│         ↓                                                        │
│    All Nodes: Receive CommitteeFinalized(e3Id, nodes[])         │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 4. KEYGEN (Committee members only)                              │
│    Committee Members: Run DKG protocol                           │
│         ↓                                                        │
│    Each generates secret share + aggregated public key          │
│         ↓                                                        │
│    Emit PublicKeyGenerated locally                               │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 5. PUBLIC KEY AGGREGATION                                        │
│    Aggregator: Collects PublicKeyGenerated from M+ nodes        │
│         ↓                                                        │
│    Emits PublicKeyAggregated(e3Id, publicKey)                   │
└─────────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────────┐
│ 6. COMMITTEE PUBLISHED                                           │
│    Aggregator → CiphernodeRegistry.publishCommittee()            │
│         ↓                                                        │
│    Contract: Stores committee + pubkey, emits CommitteePublished│
│         ↓                                                        │
│    ✅ Committee is now registered and ready!                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Components & Their Roles

### **CommitteeSortitionSolWriter** (`crates/evm/src/committee_sortition_sol.rs`)
- **All Nodes:** Submits tickets when `CiphernodeSelected`
- **Aggregator Only** (if `enable_finalizer=true`):
  - Tracks submission deadlines
  - Auto-calls `finalizeCommittee()` after window
  - Cleans up after `CommitteeFinalized`

### **CommitteeSortition.sol** (Solidity contract)
- Validates ticket submissions
- Maintains sorted topN array (by score)
- Enforces submission window
- Finalizes committee selection

### **CiphernodeSelector** (`crates/sortition/`)
- Checks if node is eligible (bonding, tickets)
- Computes scores for all owned tickets
- Finds best ticket to submit
- Emits `CiphernodeSelected` if should participate

### **ThresholdKeyshareExtension** (`crates/keyshare/`)
- Waits for `CommitteeFinalized` event
- Runs DKG with other committee members
- Generates secret shares and public key

### **PublicKeyAggregatorExtension** (`crates/aggregator/`)
- Collects public keys from committee members
- Validates threshold reached
- Emits `PublicKeyAggregated`

### **CiphernodeRegistrySolWriter** (`crates/evm/`)
- Receives `PublicKeyAggregated`
- Publishes committee + pubkey to blockchain
- Makes committee official

---

## What Makes Score Sortition Different from Distance Sortition?

| Aspect | Distance Sortition | Score Sortition |
|--------|-------------------|-----------------|
| **Participation** | Only closest N nodes in merkle tree | ALL eligible nodes can try |
| **Selection** | Aggregator computes distances | Nodes self-select via lottery |
| **Submission** | Aggregator publishes immediately | 60s window for submissions |
| **Fairness** | Depends on tree position | Equal chance based on tickets |
| **Finalization** | Immediate | After submission window |
| **Contract** | CiphernodeRegistry only | CiphernodeRegistry + CommitteeSortition |

---

## Event Flow Summary

```
E3Requested
    ↓
CiphernodeSelected (local, selected nodes only)
    ↓
TicketSubmitted (on-chain, for each submission)
    ↓
CommitteeFinalized (on-chain, after window closes)
    ↓
PublicKeyGenerated (local, committee members)
    ↓
PublicKeyAggregated (local, aggregator)
    ↓
CommitteePublished (on-chain, final registration)
```

---

## Configuration

To enable score sortition in your builder:
```rust
CiphernodeBuilder::new()
    .with_contract_committee_sortition()  // ← Enable score sortition
    .with_pubkey_agg()                    // ← Makes this node an aggregator
    .build()
```

- **Regular nodes:** Submit tickets only
- **Aggregator nodes:** Submit tickets + auto-finalize committees

The submission window is automatically fetched from the contract's `submissionWindow` immutable variable (typically 60 seconds).

---

## Implementation Files

### Core Implementation
- `crates/evm/src/committee_sortition_sol.rs` - Contract interaction and finalization logic
- `crates/ciphernode-builder/src/ciphernode_builder.rs:375-410` - Integration and attachment
- `packages/enclave-contracts/contracts/sortition/CommitteeSortition.sol` - On-chain sortition logic

### Events
- `crates/events/src/enclave_event/committee_finalized.rs` - CommitteeFinalized event
- `crates/events/src/enclave_event/committee_published.rs` - CommitteePublished event

### Related Components
- `crates/sortition/` - Node selection and ticket computation
- `crates/keyshare/` - DKG and key generation
- `crates/aggregator/` - Public key aggregation

---

## TODOs for Complete Score Sortition Support

1. **Update CiphernodeRegistrySolWriter** to track finalized committees:
   - Store committee nodes from `CommitteeFinalized` event
   - Use stored nodes when publishing (not aggregated nodes)

2. **Make ThresholdKeyshareExtension wait for CommitteeFinalized**:
   - Currently starts on `CiphernodeSelected`
   - Should wait for `CommitteeFinalized` in score sortition
   - Add mode detection or configuration

3. **Make submission window configurable**:
   - Currently fetched from contract (good!)
   - Consider adding override in config for testing

---

## Testing Score Sortition

1. Deploy contracts with `CommitteeSortition` enabled
2. Start aggregator node with `with_contract_committee_sortition()` and `with_pubkey_agg()`
3. Start multiple regular nodes with `with_contract_committee_sortition()`
4. Request E3 computation
5. Watch logs for:
   - Ticket submissions
   - Finalization after 60s
   - Committee members starting keygen
   - Public key aggregation
   - Final committee publication

---

## Advantages of Score Sortition

1. **Fair Participation**: All bonded nodes have equal chance based on tickets owned
2. **Decentralized Selection**: No single node controls selection
3. **Transparent**: All submissions on-chain, verifiable
4. **Secure**: Uses cryptographic randomness (seed + ticket numbers)
5. **Flexible**: Can adjust committee size via threshold parameter

---

## Security Considerations

1. **Submission Window**: Must be long enough for honest nodes but short enough to prevent attacks
2. **Ticket Balance Snapshot**: Uses block number at E3 request to prevent manipulation
3. **One Submission per Node**: Prevents spam and ensures fair distribution
4. **Score Verification**: Contract recomputes score on-chain (not trusted input)
5. **Finalization Permission**: Anyone can finalize (no central authority)

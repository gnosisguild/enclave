# Part 6: Deactivation, Deregistration & Completion

## Overview

An operator can voluntarily leave the network by deactivating (withdrawing collateral) and
deregistering (removing from the Merkle tree). The exit is time-locked, and pending exits remain
slashable until claimed.

---

## Voluntary Deactivation

### Via Ticket Withdrawal

```
User runs: enclave ciphernode deactivate --tickets 50
│
├─ ChainContext::new() → loads config, decrypts wallet
│
└─ BondingRegistryContract.removeTicketBalance(50).send().await
    │
    │  ┌─── ON-CHAIN (BondingRegistry.sol) ─────────────────────┐
    │  │                                                         │
    │  │  removeTicketBalance(50):                               │
    │  │    1. require(amount != 0, registered, sufficient ETK)  │
    │  │    2. ticketToken.burnTickets(operator, amount)         │
    │  │       → ETK destroyed, underlying becomes claimable      │
    │  │    3. _exits.queueTicketsForExit(                       │
    │  │         operator, exitDelay, amount                      │
    │  │       )                                                  │
    │  │       → Locked in ExitQueue until now + exitDelay        │
    │  │    4. _updateOperatorStatus(operator)                   │
    │  │       → Active iff registered &&                         │
    │  │         licenseBond >= _minLicenseBond() &&              │
    │  │         (ticketBalance / ticketPrice) >= minTicketBalance│
    │  │         active = false, numActiveOperators--             │
    │  │         Emit OperatorActivationChanged(op, false)        │
    │  │    5. Emit TicketBalanceUpdated(op, -amount, newBal,     │
    │  │       "WITHDRAW")                                         │
    │  │  }                                                      │
    │  └─────────────────────────────────────────────────────────┘
```

### Via License Withdrawal

```
User runs: enclave ciphernode deactivate --license 20000
│
└─ BondingRegistryContract.unbondLicense(20000).send().await
    │
    │  ┌─── ON-CHAIN ───────────────────────────────────────────┐
    │  │                                                         │
    │  │  unbondLicense(20000):                                  │
    │  │    1. require(amount != 0, sufficient bonded ENCL)      │
    │  │    2. operators[op].licenseBond -= 20000                │
    │  │    3. _exits.queueLicensesForExit(op, exitDelay, 20000)│
    │  │    4. _updateOperatorStatus(operator)                   │
    │  │       → If licenseBond <                                │
    │  │         (licenseRequiredBond * licenseActiveBps / 10000)│
    │  │         (default: 80% of required bond):                │
    │  │         active = false, numActiveOperators--             │
    │  │    5. Emit LicenseBondUpdated(op, -amount, newBond,      │
    │  │       "UNBOND")                                          │
    │  │  }                                                      │
    │  └─────────────────────────────────────────────────────────┘
```

### Combined Deactivation

```
User runs: enclave ciphernode deactivate --tickets 50 --license 20000
│
├─ Calls removeTicketBalance(50) first
└─ Then calls unbondLicense(20000)
   → Both queued in ExitQueue with same exitDelay
   → May merge into single tranche if same unlock time
```

---

## Full Deregistration

```
User runs: enclave ciphernode deregister
│
├─ ChainContext::new()
│
└─ BondingRegistryContract.deregisterOperator().send().await
    │
    │  ┌─── ON-CHAIN (BondingRegistry.sol) ─────────────────────┐
    │  │                                                         │
    │  │  deregisterOperator() {                                  │
    │  │    1. require(operators[msg.sender].registered)         │
    │  │    2. require(!exitInProgress(msg.sender))              │
    │  │       → Cannot deregister if an exit is already pending │
    │  │                                                         │
    │  │    3. operators[msg.sender].registered = false          │
    │  │    4. operators[msg.sender].exitRequested = true        │
    │  │    5. operators[msg.sender].exitUnlocksAt =             │
    │  │         block.timestamp + exitDelay                      │
    │  │                                                         │
    │  │    6. Burn ALL tickets:                                 │
    │  │       fullTicketBalance = ticketToken.balanceOf(op)     │
    │  │       ticketToken.burnTickets(op, fullTicketBalance)    │
    │  │                                                         │
    │  │    7. Queue ALL collateral for exit:                    │
    │  │       licenseBondAmount = operators[op].licenseBond     │
    │  │       operators[op].licenseBond = 0                     │
    │  │       _exits.queueAssetsForExit(                        │
    │  │         op, exitDelay,                                   │
    │  │         fullTicketBalance,  // tickets                   │
    │  │         licenseBondAmount   // license                   │
    │  │       )                                                  │
    │  │                                                         │
    │  │    8. Remove from Merkle tree:                          │
    │  │       registry.removeCiphernode(msg.sender)             │
    │  │       │                                                  │
    │  │       │  ┌─ CiphernodeRegistryOwnable ──────────────┐  │
    │  │       │  │  removeCiphernode(node):                  │  │
    │  │       │  │    index = ciphernodeTreeIndex[node]      │  │
    │  │       │  │    ciphernodes._update(0, index)          │  │
    │  │       │  │    → Leaf zeroed in Lazy IMT              │  │
    │  │       │  │    numCiphernodes--                       │  │
    │  │       │  │    Emit CiphernodeRemoved(node)           │  │
    │  │       │  └──────────────────────────────────────────┘  │
    │  │                                                         │
    │  │    9. _updateOperatorStatus(msg.sender)                 │
    │  │       → active = false (registered is now false)        │
    │  │       → numActiveOperators--                            │
    │  │       → Emit OperatorActivationChanged(op, false)       │
    │  │                                                         │
    │  │   10. Emit CiphernodeDeregistrationRequested(op)        │
    │  │  }                                                      │
    │  └─────────────────────────────────────────────────────────┘
│
└─ After exitDelay seconds, operator can claim unlocked exits:
    enclave ciphernode license claim
    # optional caps:
    enclave ciphernode license claim --max-ticket X --max-license Y
```

## E3 Completion (Happy Path)

When an E3 completes successfully:

```
publishPlaintextOutput() succeeds
│
├─ ON-CHAIN:
│   ├─ stage = Complete
│   ├─ _distributeRewards(e3Id)
│   │   ├─ (activeNodes, _) = ciphernodeRegistry.getActiveCommitteeNodes(e3Id)
│   │   ├─ perNode = payment / activeNodes.length
│   │   ├─ dust → last member
│   │   ├─ if activeNodes.length == 0: refund payment to requester
│   │   ├─ if payment == 0: only slashed-funds distribution runs
│   │   ├─ bondingRegistry.distributeRewards(token, nodes, amounts)
│   │   │   → Transfers fee tokens to each registered operator
│   │   ├─ e3RefundManager.distributeSlashedFundsOnSuccess(
│   │   │     e3Id, activeNodes, paymentToken
│   │   │   )
│   │   │   → If any escrowed slashed funds exist for this E3:
│   │   │     split by successSlashedNodeBps (default 50%)
│   │   │     nodes portion distributed evenly to activeNodes
│   │   │     remainder sent to protocol treasury
│   │   │   → If no escrowed funds: no-op
│   │   └─ Emit RewardsDistributed(e3Id)
│   └─ Emit PlaintextOutputPublished(e3Id, plaintext, proof), E3StageChanged(Complete)
│
└─ RUST-SIDE (cleanup via E3RequestComplete):
    │
    ├─ E3Router detects PlaintextAggregated (or E3StageChanged(Complete)):
    │   └─ Publishes E3RequestComplete { e3_id }
    │       → Single cleanup signal for all per-E3 actors
    │
    ├─ Sortition: decrements activeJobs for each committee member
    │   → Node becomes available for future E3s
    │   → Removes e3_id from node_state.e3_committees map
    │
    ├─ CiphernodeSelector: removes e3_id from e3_cache, committee, expelled set,
    │  and persisted aggregator designation for the E3
    │
    ├─ Per-E3 actors receive Die / shutdown on completion:
    │   ├─ ThresholdKeyshare: state = Completed, actor stops
    │   ├─ PublicKeyAggregator: actor stops
    │   ├─ ThresholdPlaintextAggregator: actor stops
    │   ├─ KeyshareCreatedFilterBuffer: no new E3 events after context teardown
    │   └─ DecryptionshareCreatedBuffer: no new E3 events after context teardown
    │
    └─ E3Router: removes E3Context for this e3_id
        → All per-E3 state fully cleaned up
```

---

## Rust-Side: Node Shutdown

```
enclave start → running node
│
├─ Ctrl+C / SIGINT / SIGTERM
│
└─ listen_for_shutdown():
    ├─ Signals EventBus to stop
    ├─ Awaits join_handle (main actor system)
    ├─ Persists final state to Sled DB
    └─ Clean exit

On restart:
├─ Sync module replays:
│   1. Load snapshot metadata and hydrate persisted per-E3 state
│      → Extensions must preserve hydrated recipients; replayed committee events
│        must not replace a restored per-E3 actor with a fresh instance
│   2. CiphernodeSelector emits persisted AggregatorChanged state before replay
│   3. Replay EventStore events since last snapshot (effects still disabled)
│   4. Fetch historical EVM events from last known block
│   5. Historical libp2p sync retries failed aggregate fetches after reconnects
│      and also on bounded retry intervals even without a new connection event
│   6. Sort & publish merged events by HLC timestamp
│   7. Enable effects (writers may submit only after this point)
│   8. SyncEnded → live operations begin
└─ Node resumes from where it left off
```

---

## Exit Queue Timing

```
Time ──────────────────────────────────────────────────────►

│ deregister()     │                    │ claimExits()     │
│ or deactivate()  │   EXIT DELAY       │                  │
│                  │  (configured)       │                  │
│ Assets queued    │                    │ Assets claimable │
│ ETK burned       │  Cannot cancel     │ USDC returned    │
│ ENCL locked      │  Can be slashed!   │ ENCL returned    │
│                  │                    │                  │

IMPORTANT: Even during the exit delay, slashing can still
reach into the exit queue and take locked assets. There is
no safe harbor for misbehaving operators.
```

---

## Ban & Unban

```
SLASHING → operator banned:
  banned[operator] = true
    → Cannot call registerOperator() (reverts with CiphernodeBanned)
  → Permanent until governance intervenes

GOVERNANCE lifts ban:
    SlashingManager.updateBanStatus(operator, false, keccak256("reason"))
  → banned[operator] = false
  → Operator can re-register
```

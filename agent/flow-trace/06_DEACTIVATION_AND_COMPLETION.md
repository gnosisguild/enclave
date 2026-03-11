# Part 6: Deactivation, Deregistration & Completion

## Overview

An operator can voluntarily leave the network by deactivating (withdrawing collateral) and
deregistering (removing from the Merkle tree). The exit is time-locked to prevent flash-unstake
attacks.

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
    │  │    1. burnAmount = 50 * ticketPrice                     │
    │  │    2. ticketToken.burnTickets(operator, burnAmount)     │
    │  │       → ETK destroyed, USDC held in payableBalance      │
    │  │    3. _exits.queueTicketsForExit(                       │
    │  │         operator, exitDelay, burnAmount                  │
    │  │       )                                                  │
    │  │       → Locked in ExitQueue until now + exitDelay        │
    │  │    4. _updateOperatorStatus(operator)                   │
    │  │       → If tickets drop below minTicketBalance:          │
    │  │         active = false, numActiveOperators--             │
    │  │         Emit OperatorActivationChanged(op, false)        │
    │  │    5. Emit TicketBalanceUpdated(operator, newBalance)    │
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
    │  │    1. operators[op].licenseBond -= 20000                │
    │  │    2. _exits.queueLicensesForExit(op, exitDelay, 20000)│
    │  │    3. _updateOperatorStatus(operator)                   │
    │  │       → If licenseBond < requiredBond * 80%:            │
    │  │         active = false, numActiveOperators--             │
    │  │    4. Emit LicenseBondUpdated(operator, newBond)        │
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
User runs: enclave ciphernode deregister --proof 123,456,789
│
├─ ChainContext::new()
│
├─ Parse proof: comma-separated IMT sibling node values → Vec<U256>
│
└─ BondingRegistryContract.deregisterOperator(siblingNodes).send().await
    │
    │  ┌─── ON-CHAIN (BondingRegistry.sol) ─────────────────────┐
    │  │                                                         │
    │  │  deregisterOperator(uint256[] siblingNodes) {           │
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
    │  │       registry.removeCiphernode(msg.sender, siblingNodes)│
    │  │       │                                                  │
    │  │       │  ┌─ CiphernodeRegistryOwnable ──────────────┐  │
    │  │       │  │  removeCiphernode(node, siblings):        │  │
    │  │       │  │    _remove(uint160(node), siblings)       │  │
    │  │       │  │    → Node removed from Lean IMT           │  │
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
└─ After exitDelay seconds, operator can claim everything back:
     enclave ciphernode license claim --max-ticket X --max-license Y
```

### Getting the IMT Proof

The `--proof` parameter requires Indexed Merkle Tree sibling nodes. These must be computed off-chain
by traversing the current IMT state. The proof allows the contract to verify and remove the node
from the tree.

---

## E3 Completion (Happy Path)

When an E3 completes successfully:

```
publishPlaintextOutput() succeeds
│
├─ ON-CHAIN:
│   ├─ stage = Complete
│   ├─ _distributeRewards(e3Id)
│   │   ├─ activeNodes = ciphernodeRegistry.getActiveCommitteeNodes(e3Id)
│   │   ├─ perNode = payment / activeNodes.length
│   │   ├─ dust → last member
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
│   └─ Emit PlaintextOutputPublished, E3StageChanged(Complete)
│
└─ RUST-SIDE (cleanup via E3RequestComplete):
    │
    ├─ E3Router detects PlaintextAggregated (or E3StageChanged(Complete)):
    │   └─ Publishes E3RequestComplete { e3_id }
    │       → Single cleanup signal for all per-E3 actors
    │
    ├─ Sortition: decrements activeJobs for each committee member
    │   → Node becomes available for future E3s
    │   → Removes e3_id from finalized_committees map
    │
    ├─ CiphernodeSelector: removes e3_id from e3_cache
    │
    ├─ Per-E3 actors receive Die via E3RequestComplete:
    │   ├─ ThresholdKeyshare: state = Completed, actor stops
    │   ├─ PublicKeyAggregator: actor stops
    │   ├─ ThresholdPlaintextAggregator: actor stops
    │   └─ KeyshareCreatedFilterBuffer: actor stops
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
│   1. Load snapshot metadata
│   2. Replay EventStore events since last snapshot
│   3. Fetch historical EVM events from last known block
│   4. Sort & publish merged events by HLC timestamp
│   5. Enable effects (writers activated)
│   6. SyncEnded → live operations begin
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
  → Cannot call registerOperator() (reverts with "banned")
  → Permanent until governance intervenes

GOVERNANCE lifts ban:
  SlashingManager.updateBanStatus(operator, false, "reason")
  → banned[operator] = false
  → Operator can re-register
```

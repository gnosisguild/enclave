# Part 2: Tokens, Bonding & Activation

## Overview

Before a node can register, it must stake two types of collateral:

1. **ENCL tokens** (license bond) — governance/utility token, staked directly
2. **Stablecoin via ETK tickets** (ticket collateral) — USDC wrapped into non-transferable
   EnclaveTicketToken

---

## Token Architecture

```
┌───────────────────────────────────────────────────────────┐
│                    EnclaveToken (ENCL)                     │
│  ERC20 + ERC20Permit + ERC20Votes + AccessControl         │
│                                                           │
│  MAX_SUPPLY: 1,200,000,000 (1.2B with 18 decimals)       │
│  Roles: MINTER_ROLE can mint via mintAllocation()         │
│  Transfer restrictions: when transfersRestricted=true,    │
│    only whitelisted addresses can transfer                │
│  Used as: LICENSE BOND token                              │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│              EnclaveTicketToken (ETK)                      │
│  ERC20Wrapper over stablecoin (e.g. USDC)                 │
│                                                           │
│  NON-TRANSFERABLE: _update() reverts on transfer          │
│  NO DELEGATION: delegate() reverts                        │
│  NO APPROVALS: approve() reverts                          │
│                                                           │
│  Only BondingRegistry (registry role) can:                │
│    depositFor()  → wrap USDC, mint ETK to operator        │
│    depositFrom() → pull USDC from X, mint ETK to Y       │
│    burnTickets() → burn ETK, NO underlying returned       │
│    withdrawTo()  → burn ETK, return underlying USDC       │
│    payout()      → send underlying from payableBalance    │
│                                                           │
│  Used as: TICKET COLLATERAL token                         │
└───────────────────────────────────────────────────────────┘
```

---

## Step 1: Bond License (`enclave ciphernode license bond`)

**File:** `crates/cli/src/ciphernode/license.rs`

```
User runs: enclave ciphernode license bond --amount 50000
│
├─ 1. ChainContext::new()
│     └─ Loads config, decrypts wallet, connects to BondingRegistry
│
├─ 2. Approve ENCL spend:
│     └─ EnclaveToken.approve(bondingRegistry, 50000)
│        → Allows BondingRegistry to pull ENCL tokens
│
├─ 3. BondingRegistryContract.bondLicense(50000).send().await
│     │
│     │  ┌─── ON-CHAIN (BondingRegistry.sol) ──────────────────┐
│     │  │                                                      │
│     │  │  bondLicense(uint256 amount) {                       │
│     │  │    1. require(amount > 0)                            │
│     │  │    2. licenseToken.safeTransferFrom(                 │
│     │  │         msg.sender,   // from operator               │
│     │  │         address(this), // to BondingRegistry         │
│     │  │         amount                                       │
│     │  │       )                                              │
│     │  │       → ENCL tokens move from operator → contract    │
│     │  │    3. operators[msg.sender].licenseBond += amount    │
│     │  │    4. _updateOperatorStatus(msg.sender)              │
│     │  │       → May activate if all conditions now met       │
│     │  │    5. Emit LicenseBondUpdated(msg.sender, newBond)   │
│     │  │  }                                                   │
│     │  └──────────────────────────────────────────────────────┘
│     │
└─ OUTPUT: "Transaction hash: 0x..."
```

### Activation check after bonding:

```
_updateOperatorStatus(operator):
  wasActive = operators[operator].active

  isNowActive = (
    operators[operator].registered == true
    AND operators[operator].licenseBond >= (licenseRequiredBond * licenseActiveBps / 10000)
        // Default: licenseActiveBps = 8000 (80%)
        // So if licenseRequiredBond = 50000, need >= 40000 ENCL
    AND ticketToken.balanceOf(operator) / ticketPrice >= minTicketBalance
  )

  if (wasActive && !isNowActive):
    operators[operator].active = false
    numActiveOperators--
    emit OperatorActivationChanged(operator, false)

  if (!wasActive && isNowActive):
    operators[operator].active = true
    numActiveOperators++
    emit OperatorActivationChanged(operator, true)
```

---

## Step 2: Buy Tickets (`enclave ciphernode tickets buy`)

**File:** `crates/cli/src/ciphernode/tickets.rs`

> **IMPORTANT:** The `amount` parameter to `addTicketBalance` is in **underlying stablecoin base
> units** (e.g., USDC wei), NOT in ticket count. The CLI parses the user's input using the
> underlying token's decimals. `ticketPrice` is only used in the activation check
> (`balanceOf / ticketPrice >= minTicketBalance`) and in sortition eligibility — it is NOT used to
> multiply the deposit amount.

```
User runs: enclave ciphernode tickets buy --amount 100
│
├─ 1. ChainContext::new()
│
├─ 2. CLI resolves the ticket token's underlying stablecoin address
│     and its decimals, then parses "100" → 100_000_000 (raw units)
│
├─ 3. Approve stablecoin spend:
│     └─ USDC.approve(ticketTokenAddress, 100_000_000)
│        → Note: approval is to the TicketToken contract (not BondingRegistry)
│        → because depositFrom pulls USDC into the TicketToken wrapper
│
├─ 4. BondingRegistryContract.addTicketBalance(100_000_000).send().await
│     │
│     │  ┌─── ON-CHAIN (BondingRegistry.sol) ──────────────────┐
│     │  │                                                      │
│     │  │  addTicketBalance(uint256 amount) {                  │
│     │  │    1. require(amount > 0)                            │
│     │  │    2. require(operators[msg.sender].registered)      │
│     │  │    3. modifier: require(!exitInProgress(msg.sender)) │
│     │  │    4. ticketToken.depositFrom(                       │
│     │  │         msg.sender,  // pull USDC from operator      │
│     │  │         msg.sender,  // mint ETK to operator         │
│     │  │         amount       // RAW stablecoin units         │
│     │  │       )              // NO ticketPrice multiplication│
│     │  │       │                                              │
│     │  │       │  ┌─ EnclaveTicketToken.depositFrom() ────┐  │
│     │  │       │  │  1. underlying.transferFrom(           │  │
│     │  │       │  │       from, address(this), amount)     │  │
│     │  │       │  │     → USDC moves: operator → ETK       │  │
│     │  │       │  │  2. _mint(to, amount)                  │  │
│     │  │       │  │     → ETK minted 1:1 with USDC         │  │
│     │  │       │  │  3. Auto-delegate to self on first     │  │
│     │  │       │  │     deposit (for voting power tracking)│  │
│     │  │       │  └────────────────────────────────────────┘  │
│     │  │    5. _updateOperatorStatus(msg.sender)              │
│     │  │    6. Emit TicketBalanceUpdated(msg.sender,          │
│     │  │         +amount, newBalance, "DEPOSIT")              │
│     │  │  }                                                   │
│     │  └──────────────────────────────────────────────────────┘
│     │
└─ OUTPUT: "Purchased 100 tickets (tx: 0x...)"
```

### Why tickets are non-transferable:

ETK tokens cannot be transferred between addresses. This ensures:

- An operator's collateral can't be moved to avoid slashing
- The ticket balance is always attributable to the specific operator
- Snapshot-based committee eligibility (checking balance at `requestBlock - 1`) is reliable

---

## Step 3: Unbond License (`enclave ciphernode license unbond`)

```
User runs: enclave ciphernode license unbond --amount 10000
│
├─ BondingRegistryContract.unbondLicense(10000).send().await
│     │
│     │  ┌─── ON-CHAIN ─────────────────────────────────────────┐
│     │  │                                                       │
│     │  │  unbondLicense(uint256 amount) {                      │
│     │  │    1. require(amount > 0)                             │
│     │  │    2. require(operators[msg.sender].licenseBond       │
│     │  │              >= amount)                               │
│     │  │    3. operators[msg.sender].licenseBond -= amount     │
│     │  │    4. _exits.queueLicensesForExit(                   │
│     │  │         msg.sender, exitDelay, amount                 │
│     │  │       )                                               │
│     │  │       │                                               │
│     │  │       │  ┌─ ExitQueueLib ─────────────────────────┐  │
│     │  │       │  │  Creates ExitTranche {                 │  │
│     │  │       │  │    unlockTimestamp: now + exitDelay,    │  │
│     │  │       │  │    ticketAmount: 0,                    │  │
│     │  │       │  │    licenseAmount: 10000                │  │
│     │  │       │  │  }                                     │  │
│     │  │       │  │  Merges into last tranche if same      │  │
│     │  │       │  │  unlock time, else appends new tranche │  │
│     │  │       │  │  Updates pendingTotals                 │  │
│     │  │       │  └────────────────────────────────────────┘  │
│     │  │    5. _updateOperatorStatus(msg.sender)               │
│     │  │       → May DEACTIVATE if bond drops below threshold  │
│     │  │    6. Emit LicenseBondUpdated(msg.sender, newBond)    │
│     │  │  }                                                    │
│     │  └───────────────────────────────────────────────────────┘
│
└─ Funds are now LOCKED for exitDelay seconds (time-locked exit)
```

---

## Step 4: Burn Tickets (`enclave ciphernode tickets burn`)

> **IMPORTANT:** Like `addTicketBalance`, the `amount` here is in **raw stablecoin base units** (ETK
> units, which are 1:1 with underlying). There is NO `ticketPrice` multiplication. The CLI parses
> the user's amount using the ticket token's decimals.

```
User runs: enclave ciphernode tickets burn --amount 50
│
├─ CLI parses "50" using ticket token decimals → raw units
│
├─ BondingRegistryContract.removeTicketBalance(rawAmount).send().await
│     │
│     │  ┌─── ON-CHAIN ─────────────────────────────────────────┐
│     │  │                                                       │
│     │  │  removeTicketBalance(uint256 amount) {                │
│     │  │    1. require(amount > 0)                             │
│     │  │    2. require(operators[msg.sender].registered)       │
│     │  │    3. require(ticketToken.balanceOf(msg.sender)       │
│     │  │              >= amount)                               │
│     │  │    4. ticketToken.burnTickets(msg.sender, amount)     │
│     │  │       │  (NO ticketPrice multiplication — raw units)  │
│     │  │       │                                               │
│     │  │       │  ┌─ EnclaveTicketToken ───────────────────┐  │
│     │  │       │  │  burnTickets(operator, amount):        │  │
│     │  │       │  │    payableBalance += amount             │  │
│     │  │       │  │    _burn(operator, amount)             │  │
│     │  │       │  │    → ETK destroyed                     │  │
│     │  │       │  │    → Underlying USDC NOT returned yet  │  │
│     │  │       │  │    → Tracked in payableBalance for     │  │
│     │  │       │  │      later payout()                    │  │
│     │  │       │  └────────────────────────────────────────┘  │
│     │  │    5. _exits.queueTicketsForExit(                    │
│     │  │         msg.sender, exitDelay, amount)                │
│     │  │    6. _updateOperatorStatus(msg.sender)               │
│     │  │       → May DEACTIVATE if tickets drop below minimum  │
│     │  │    7. Emit TicketBalanceUpdated(msg.sender,           │
│     │  │         -amount, newBalance, "WITHDRAW")              │
│     │  │  }                                                    │
│     │  └───────────────────────────────────────────────────────┘
│
└─ Tickets burned, USDC queued for exit after delay
```

---

## Step 5: Claim Exits (`enclave ciphernode license claim`)

```
User runs: enclave ciphernode license claim [--max-ticket 50] [--max-license 10000]
│
├─ BondingRegistryContract.claimExits(50, 10000).send().await
│     │
│     │  ┌─── ON-CHAIN ─────────────────────────────────────────┐
│     │  │                                                       │
│     │  │  claimExits(maxTicket, maxLicense) {                  │
│     │  │    1. (ticketAmount, licenseAmount) =                 │
│     │  │       _exits.claimAssets(                             │
│     │  │         msg.sender, maxTicket, maxLicense             │
│     │  │       )                                               │
│     │  │       │                                               │
│     │  │       │  ┌─ ExitQueueLib.claimAssets() ───────────┐  │
│     │  │       │  │  Iterates tranches from head:          │  │
│     │  │       │  │  for each tranche where                │  │
│     │  │       │  │    block.timestamp >= unlockTimestamp:  │  │
│     │  │       │  │      take min(wanted, available)       │  │
│     │  │       │  │      from ticketAmount & licenseAmount  │  │
│     │  │       │  │  Skip locked tranches (future unlock)  │  │
│     │  │       │  │  Clean up empty tranches               │  │
│     │  │       │  │  Update pendingTotals                  │  │
│     │  │       │  └────────────────────────────────────────┘  │
│     │  │                                                       │
│     │  │    2. if ticketAmount > 0:                            │
│     │  │       ticketToken.payout(msg.sender, ticketAmount)    │
│     │  │       │                                               │
│     │  │       │  ┌─ EnclaveTicketToken.payout() ──────────┐  │
│     │  │       │  │  Transfers underlying USDC from        │  │
│     │  │       │  │  payableBalance to operator             │  │
│     │  │       │  │  payableBalance -= amount               │  │
│     │  │       │  │  underlying.safeTransfer(to, amount)    │  │
│     │  │       │  └────────────────────────────────────────┘  │
│     │  │                                                       │
│     │  │    3. if licenseAmount > 0:                           │
│     │  │       licenseToken.safeTransfer(                      │
│     │  │         msg.sender, licenseAmount                     │
│     │  │       )                                               │
│     │  │       → ENCL tokens returned to operator              │
│     │  │  }                                                    │
│     │  └───────────────────────────────────────────────────────┘
│
└─ Operator receives back their USDC and/or ENCL tokens
```

---

## Activation Thresholds Summary

| Requirement           | Default             | Description                                |
| --------------------- | ------------------- | ------------------------------------------ |
| `licenseRequiredBond` | Configured by owner | Min ENCL to register                       |
| `licenseActiveBps`    | 8000 (80%)          | % of required bond to stay active          |
| `minTicketBalance`    | Configured by owner | Min tickets for active status              |
| `ticketPrice`         | Configured by owner | Stablecoin cost per ticket (in base units) |
| `exitDelay`           | Configured by owner | Seconds before exits can be claimed        |

### Activation formula:

```
active = registered
  AND licenseBond >= (licenseRequiredBond * licenseActiveBps / 10000)
  AND (ticketToken.balanceOf(operator) / ticketPrice) >= minTicketBalance
```

---

## Token Flow Diagram

```
                BOND LICENSE                          BUY TICKETS
                ────────────                          ───────────
  Operator                                 Operator
  ENCL wallet ──→ BondingRegistry          USDC wallet ──→ EnclaveTicketToken
                  (licenseBond++)                          (wraps USDC → mints ETK)
                                                           ETK → Operator balance

               UNBOND LICENSE                         BURN TICKETS
               ──────────────                         ────────────
  licenseBond -= amount                    ETK burned from operator
  amount → ExitQueue (locked)              USDC stays in ETK contract (payableBalance)
                                           amount → ExitQueue (locked)

                              CLAIM EXITS
                              ───────────
                   After exitDelay seconds:
                   ENCL → returned from ExitQueue
                   USDC → paid out from ETK.payableBalance
```

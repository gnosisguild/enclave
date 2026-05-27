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
│     │  │    3. Record LicenseBondSource {                     │
│     │  │         amount, withdrawalAddress: msg.sender,        │
│     │  │         sourceId: 0, sequence                         │
│     │  │       }                                               │
│     │  │    4. operators[msg.sender].licenseBond += amount    │
│     │  │    5. _updateOperatorStatus(msg.sender)              │
│     │  │       → May activate if all conditions now met       │
│     │  │    6. Emit LicenseBondSourceAdded and                │
│     │  │       LicenseBondUpdated(msg.sender, newBond)        │
│     │  │  }                                                   │
│     │  └──────────────────────────────────────────────────────┘
│     │
└─ OUTPUT: "Transaction hash: 0x..."
```

### Delegated / locked ENCL bonding

`BondingRegistry.bondLicenseFor(operator, amount, withdrawalAddress, sourceId)` lets a funder supply
ENCL while crediting another operator. `InterfoldVestingEscrow` uses this path so locked allocations
can run nodes without first transferring unrestricted ENCL to the beneficiary. Each ENCL bond source
keeps its own withdrawal address and LIFO sequence. The legacy `bondLicense(amount)` path is
equivalent to `bondLicenseFor(msg.sender, amount, msg.sender, 0)`.

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
│     │  │    4. _queueLicenseExitFromSources(                  │
│     │  │         msg.sender, amount                            │
│     │  │       )                                               │
│     │  │       → Pops active LicenseBondSource entries LIFO    │
│     │  │       → Queues PendingLicenseBondSource entries       │
│     │  │         preserving withdrawalAddress + sourceId       │
│     │  │    5. _updateOperatorStatus(msg.sender)               │
│     │  │       → May DEACTIVATE if bond drops below threshold  │
│     │  │    6. Emit LicenseBondSourceQueuedForExit and         │
│     │  │       LicenseBondUpdated(msg.sender, newBond)         │
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
│     │  │    1. (ticketAmount, _) =                             │
│     │  │       _exits.claimAssets(                             │
│     │  │         msg.sender, maxTicket, 0                      │
│     │  │       )                                               │
│     │  │       │                                               │
│     │  │       │  ┌─ ExitQueueLib.claimAssets() ───────────┐  │
│     │  │       │  │  Iterates tranches from head:          │  │
│     │  │       │  │  for each tranche where                │  │
│     │  │       │  │    block.timestamp >= unlockTimestamp:  │  │
│     │  │       │  │      take min(wanted, available)       │  │
│     │  │       │  │      from ticketAmount                  │  │
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
│     │  │    3. licenseAmount = _claimLicenseExits(             │
│     │  │         msg.sender, maxLicense                        │
│     │  │       )                                               │
│     │  │       → Each ENCL source pays its withdrawalAddress   │
│     │  │       → Receiver callback gets (operator, amount,     │
│     │  │         sourceId) when supported                      │
│     │  │  }                                                    │
│     │  └───────────────────────────────────────────────────────┘
│
└─ Operator receives back USDC; ENCL goes to each source's withdrawal address
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
                   ENCL → returned to source withdrawal address
                   USDC → paid out from ETK.payableBalance
```

---

## Audit Cluster 2 Changes (Tokens)

The token contracts were hardened against the following audit findings. All changes are covered by
`packages/enclave-contracts/test/Token/` and have no runtime impact outside the touched contracts.

### EnclaveTicketToken (ETK)

- **H-02 — registry initialization.** The constructor now takes
  `(IERC20 baseToken, address registry_, address initialOwner_)` and assigns `registry = registry_`
  directly (emitting `RegistryChanged(0, registry_)`) instead of requiring the deployer to call
  `setRegistry()` later. Reverts `ZeroAddress` if `registry_ == 0`.
- **H-03 — fee-on-transfer safe deposits.** `depositFor` and `depositFrom` measure the underlying
  balance before/after `safeTransferFrom` and mint the _actual_ amount received. Operators auto
  self-delegate on first deposit.
- **H-16 / H-20 / M-22 — registry swap timelock.** Once `lockRegistry()` is called (one-way,
  `RegistryLockAlreadySet` on repeat) further registry swaps must go through
  `requestRegistryChange(addr)` → wait `REGISTRY_CHANGE_DELAY = 1 day` → `activateRegistryChange()`.
  Errors: `RegistryNotLocked`, `RegistryChangeNotReady`, `NoPendingRegistry`,
  `RegistryAlreadyLocked`. `cancelRegistryChange()` clears the pending swap.
- **M-11 — permit disabled.** `permit()` always reverts `PermitDisabled` so non-transferable tickets
  cannot be moved via off-chain signatures.
- **M-12 — rescueERC20.** `rescueERC20(token, to, amount)` lets the owner recover stray ERC-20s but
  refuses the underlying asset (`CannotRescueUnderlying`).
- **M-25 — delegation locked to self.** `delegate()` only accepts the caller's own address (else
  `DelegationLocked`); `delegateBySig` always reverts.
- **M-29 — EIP-6372 timestamp clock.** `clock() = uint48(block.timestamp)`,
  `CLOCK_MODE() = "mode=timestamp"`.

### EnclaveToken (ENCL)

- **H-15 — WHITELIST_ROLE separation + one-way disable.** New `WHITELIST_ROLE` gates
  `toggleTransferWhitelist` and `whitelistContracts`, decoupling whitelist edits from `MINTER_ROLE`.
  `disableTransferRestrictions` is `DEFAULT_ADMIN_ROLE` only and idempotent (silent no-op when
  already disabled) so deployment/setup scripts can call it unconditionally.
- **M-21 — per-epoch mint cap.** New rolling cap configured via
  `setMintCap(epochLength, capPerEpoch)` (`ZeroEpochLength` on zero length). Both `mintAllocation`
  and `batchMintAllocations` route through `_accountForMintAgainstCap`, which rolls the epoch
  (`MintEpochRolled(newStart)`) and reverts `ExceedsMintCap` on overflow. Constructor defaults to a
  30-day epoch with `cap = MAX_SUPPLY` so bootstrap deployments keep working; governance is expected
  to tighten this before broad distribution.
- **M-29 — EIP-6372 timestamp clock.** Same timestamp clock as ETK, aligning ENCL voting checkpoints
  with timepoints used elsewhere.

### Registry coordination

- `CiphernodeRegistryOwnable.requestBlock` now stores `block.timestamp` (the storage slot and event
  field names are preserved for backwards compatibility). All callers — including
  `BondingRegistry.getTicketBalanceAtBlock(node, c.requestBlock - 1)` — pass the value through
  unchanged; the parameter is now a timepoint per EIP-6372 rather than a block number, which is
  required for the ETK timestamp clock to be valid.

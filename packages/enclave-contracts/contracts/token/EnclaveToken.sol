// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    ERC20Permit,
    Nonces
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {
    ERC20Votes
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Votes.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";

import {IBondingRegistry} from "../interfaces/IBondingRegistry.sol";

/**
 * @title EnclaveToken
 * @notice The governance and utility token for the Enclave protocol, with
 *         wallet-level lock enforcement designed around the Uniswap CCA
 *         distribution flow.
 *
 */
contract EnclaveToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable2Step,
    AccessControl
{
    // ─────────────────────────────────────────────────────────────────────────
    // Types
    // ─────────────────────────────────────────────────────────────────────────

    /// @notice Global token lifecycle phase, derived from the immutable CCA
    ///         window and the TGE: Virtual (pre-sale), PublicSale (CCA
    ///         bidding window), Cooldown (sale ended, TGE not yet fired),
    ///         Live (TGE fired).
    enum Phase {
        Virtual,
        PublicSale,
        Cooldown,
        Live
    }

    enum Anchor {
        Absolute,
        Tge
    }

    /// @param anchor How the curve's start resolves (Absolute or Tge).
    /// @param start Anchor timestamp when {anchor} is Absolute; must be zero
    ///        when {anchor} is Tge.
    /// @param cliffDuration Seconds after the anchor before anything releases.
    /// @param vestDuration Total linear release duration measured from the
    ///        anchor; zero means everything releases at the cliff.
    struct Curve {
        Anchor anchor;
        uint64 start;
        uint64 cliffDuration;
        uint64 vestDuration;
    }

    /// @param holdUntil Optional absolute timestamp before which nothing is
    ///        transferable, whatever the unlock curve has accrued;
    struct LockPolicy {
        uint64 holdUntil;
        Curve unlock;
    }

    struct Lock {
        bytes32 policyId;
        uint256 amount;
    }

    struct MintAllocation {
        address recipient;
        uint256 amount;
        bytes32 policyId;
        bytes32 label;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Errors
    // ─────────────────────────────────────────────────────────────────────────

    /// @notice A zero address was provided where a valid address is required.
    error ZeroAddress();

    /// @notice A zero amount or zero timestamp was provided where a non-zero
    ///         value is required.
    error ZeroAmount();

    /// @notice Minting would exceed {MAX_SUPPLY}.
    error MaxSupplyExceeded();

    /// @notice The transfer is not one of the movements allowed pre-TGE:
    ///         bonding (the registry on either side, any phase) or a CCA
    ///         claim ({CLAIM_SOURCE} sending while PublicSale).
    error TransferRestricted(address from, address to);

    /// @notice {mint} or {mintAllocations} was called after the Virtual phase; the
    ///         full supply is distributed before {CCA_START}.
    error MintingClosed();

    /// @notice {tge} was called but the token is already live.
    error AlreadyLive();

    /// @notice {tge} was called before {CCA_END} + {TGE_COOLDOWN}.
    error TgeTooEarly(uint64 current, uint64 notBefore);

    /// @notice The CCA window is empty, inverted, or does not start in the
    ///         future.
    error InvalidCcaWindow(uint64 ccaStart, uint64 ccaEnd);

    /// @notice Policy parameters are internally inconsistent.
    error InvalidPolicy();

    /// @notice The policy id is already defined; policies are write-once.
    error PolicyAlreadyDefined(bytes32 policyId);

    /// @notice The referenced policy id has not been defined.
    error PolicyNotDefined(bytes32 policyId);

    /// @notice A transfer of `value` exceeds the sender's spendable balance
    ///         (balance + bonded − locked balance).
    error InsufficientUnlockedBalance(
        address account,
        uint256 spendable,
        uint256 value
    );

    /// @notice The bonding registry address has no deployed code.
    error InvalidBondingRegistry(address registry);

    /// @notice Thrown when {renounceOwnership} is called. Ownership is
    ///         critical for protocol governance; renouncing would permanently
    ///         freeze admin functions and is disallowed.
    error RenounceOwnershipDisabled();

    // ─────────────────────────────────────────────────────────────────────────
    // Constants and immutables
    // ─────────────────────────────────────────────────────────────────────────

    uint256 public constant MAX_SUPPLY = 1_200_000_000e18;

    /// @notice Role authorized to mint allocations, while Virtual.
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    /// @notice Role authorized to manage the pre-TGE transfer whitelist.
    bytes32 public constant WHITELIST_ROLE = keccak256("WHITELIST_ROLE");

    /// @notice Role authorized to create lock policies, manage the lock whitelist
    bytes32 public constant LOCK_MANAGER_ROLE = keccak256("LOCK_MANAGER_ROLE");

    /// @notice Minimum time between {CCA_END} and {tge}.
    uint64 public constant TGE_COOLDOWN = 45 days;

    bytes32 public constant PENDING_LOCK_POLICY_ID = "PENDING";

    /// @notice Start of the CCA auction window, fixed at deployment.
    uint64 public immutable CCA_START;

    /// @notice End of the CCA auction window, fixed at deployment
    uint64 public immutable CCA_END;

    /// @notice The CCA auction contract
    address public immutable CLAIM_SOURCE;

    /// @notice Registry whose bonded ENCL counts toward locked balances.
    IBondingRegistry public immutable BONDING_REGISTRY;

    // ─────────────────────────────────────────────────────────────────────────
    // Storage
    // ─────────────────────────────────────────────────────────────────────────

    /// @notice TGE timestamp; zero until {tge} is called, then immutable.
    uint64 public tgeTimestamp;

    /// @notice Addresses allowed to transfer before TGE.
    mapping(address account => bool whitelisted) public transferWhitelist;

    /// @notice Addresses exempt from automatic claim-source lock creation.
    mapping(address account => bool whitelisted) public lockWhitelist;

    /// @notice Write-once lock policies by id.
    mapping(bytes32 policyId => LockPolicy policy) public lockPolicies;

    /// @notice Active locks by account.
    mapping(address account => Lock[] entries) public locks;

    /// @notice Links that arrived before their exact-size claim.
    mapping(address account => Lock[] entries) public queuedLocks;

    // ─────────────────────────────────────────────────────────────────────────
    // Events
    // ─────────────────────────────────────────────────────────────────────────

    /// @notice Emitted for every mint instruction.
    event AllocationMinted(
        address indexed recipient,
        uint256 amount,
        bytes32 indexed policyId,
        bytes32 indexed label
    );

    /// @notice Emitted when a lock policy is defined (write-once).
    event PolicyDefined(bytes32 indexed policyId, LockPolicy policy);

    /// @notice Emitted when an account's transfer whitelist status changes.
    event TransferWhitelistUpdated(address indexed account, bool whitelisted);

    /// @notice Emitted when an account's lock whitelist status changes.
    event LockWhitelistUpdated(address indexed account, bool whitelisted);

    /// @notice Emitted whenever the amount `account` holds locked under
    ///         `policyId` changes; `amount` is the new total.
    event LockUpdated(
        address indexed account,
        bytes32 indexed policyId,
        uint256 amount
    );

    /// @notice Emitted once, when {tge} fires.
    event TgeTriggered(uint64 timestamp);

    // ─────────────────────────────────────────────────────────────────────────
    // Constructor
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * @notice Deploys ENCL with no TGE set.
     * @dev The initial owner receives every role via the {_transferOwnership}
     *      sync. Operational roles can additionally be granted to dedicated
     *      keys post-deployment.
     * @param initialOwner_ Initial owner; receives all roles.
     * @param ccaStart_ CCA auction window start;
     * @param ccaEnd_ CCA auction window end; after `ccaStart_`.
     * @param claimSource_ The CCA auction contract
     * @param bondingRegistry_ Registry whose bonded ENCL
     */
    constructor(
        address initialOwner_,
        uint64 ccaStart_,
        uint64 ccaEnd_,
        address claimSource_,
        IBondingRegistry bondingRegistry_
    ) ERC20("Enclave", "ENCL") ERC20Permit("Enclave") Ownable(initialOwner_) {
        if (ccaStart_ <= block.timestamp) {
            revert InvalidCcaWindow(ccaStart_, ccaEnd_);
        }
        if (ccaEnd_ <= ccaStart_) revert InvalidCcaWindow(ccaStart_, ccaEnd_);
        if (claimSource_ == address(0)) revert ZeroAddress();
        address registry = address(bondingRegistry_);
        if (registry == address(0)) revert ZeroAddress();
        if (registry.code.length == 0) {
            revert InvalidBondingRegistry(registry);
        }
        CCA_START = ccaStart_;
        CCA_END = ccaEnd_;
        CLAIM_SOURCE = claimSource_;
        BONDING_REGISTRY = bondingRegistry_;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Minting
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * @notice Plain vanilla mint: ENCL with no lock attached. Callable only
     *         while Virtual
     */
    function mint(
        address recipient,
        uint256 amount,
        bytes32 label
    ) external onlyRole(MINTER_ROLE) {
        if (phase() != Phase.Virtual) revert MintingClosed();
        _mintTokens(recipient, amount);
        emit AllocationMinted(recipient, amount, bytes32(0), label);
    }

    function mintAllocations(
        MintAllocation[] calldata allocations
    ) external onlyRole(MINTER_ROLE) {
        if (phase() != Phase.Virtual) revert MintingClosed();
        uint256 len = allocations.length;
        for (uint256 i = 0; i < len; i++) {
            _mintAllocation(allocations[i]);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Launch lifecycle
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * @notice Sets {tgeTimestamp} to the current block timestamp, exactly once.
     * @dev Permissionless: anyone may trigger the TGE once {CCA_END} +
     *      {TGE_COOLDOWN} has passed, so launch cannot be stalled by an idle
     *      operator.
     */
    function tge() external {
        if (tgeTimestamp != 0) revert AlreadyLive();
        uint64 current = uint64(block.timestamp);
        uint64 earliest = CCA_END + TGE_COOLDOWN;
        if (current < earliest) revert TgeTooEarly(current, earliest);
        tgeTimestamp = current;
        emit TgeTriggered(current);
    }

    /// @notice Current lifecycle phase. Live is event-driven ({tge}); the
    ///         earlier phases derive from the immutable CCA window.
    function phase() public view returns (Phase) {
        if (tgeTimestamp != 0) return Phase.Live;
        uint64 current = uint64(block.timestamp);
        if (current < CCA_START) return Phase.Virtual;
        if (current < CCA_END) return Phase.PublicSale;
        return Phase.Cooldown;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Whitelistig
    // ─────────────────────────────────────────────────────────────────────────

    function setTransferWhitelisted(
        address account,
        bool whitelisted
    ) external onlyRole(WHITELIST_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        transferWhitelist[account] = whitelisted;
        emit TransferWhitelistUpdated(account, whitelisted);
    }

    function setLockWhitelisted(
        address account,
        bool whitelisted
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        lockWhitelist[account] = whitelisted;
        emit LockWhitelistUpdated(account, whitelisted);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lock configuration
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * @notice Creates a lock policy. Policies are write-once: once created,
     *         the terms backing existing locks can never be changed, by anyone.
     * @param policyId Non-zero identifier, e.g. bytes32("CCA_REG_S").
     * @param policy Lock terms. The unlock curve is required, must lock
     *        something, and must be consistent with its anchor mode;
     *        `holdUntil` is optional (zero = none).
     */
    function createLockPolicy(
        bytes32 policyId,
        LockPolicy calldata policy
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (policyId == bytes32(0) || policyId == PENDING_LOCK_POLICY_ID) {
            revert InvalidPolicy();
        }

        if (_policyDefined(policyId)) {
            revert PolicyAlreadyDefined(policyId);
        }
        _validateCurve(policy.unlock);

        lockPolicies[policyId] = policy;
        emit PolicyDefined(policyId, policy);
    }

    /**
     * @notice Links `amount` of `account`'s claims to `policyId` — the
     *         Predicate/KYC bucket import.
     *
     *         Claims from the CCA can come before or after the operator's
     *         linkClaim. {_claim} and {_linkClaim} manipulate {locks} and
     *         {queuedLocks} to link balances to policies in a resilient
     *         way: it doesn't matter who calls what first.
     */
    function linkClaim(
        address account,
        uint256 amount,
        bytes32 policyId
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        if (!_policyDefined(policyId)) revert PolicyNotDefined(policyId);

        _linkClaim(account, amount, policyId);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lock views
    // ─────────────────────────────────────────────────────────────────────────

    /// @notice Current locked balance of `account`: the amount that must remain
    ///         controlled (wallet balance + bonded) by the account.
    function lockedBalanceOf(address account) public view returns (uint256) {
        return lockedBalanceAt(account, uint64(block.timestamp));
    }

    /// @notice Locked balance of `account` at `timestamp`, evaluated against the
    ///         current configuration (an unset TGE keeps {Anchor.Tge} policies
    ///         fully locked for any timestamp).
    function lockedBalanceAt(
        address account,
        uint64 timestamp
    ) public view returns (uint256 lockedBalance) {
        Lock[] storage accountLocks = locks[account];
        for (uint256 i = 0; i < accountLocks.length; i++) {
            Lock storage accountLock = accountLocks[i];
            bytes32 policyId = accountLock.policyId;
            uint256 amount = accountLock.amount;
            if (policyId == PENDING_LOCK_POLICY_ID) {
                // Unclassified claims are fully locked, immune to time.
                lockedBalance += amount;
            } else {
                lockedBalance += _lockedAmount(
                    lockPolicies[policyId],
                    amount,
                    timestamp
                );
            }
        }
    }

    /// @notice Wallet balance `account` can transfer right now: the wallet
    ///         must retain whatever part of its locked balance its bond does
    ///         not already cover.
    /// @dev Never consults the registry for accounts with nothing locked.
    function transferableBalanceOf(
        address account
    ) public view returns (uint256) {
        uint256 balance = balanceOf(account);
        uint256 lockedBalance = lockedBalanceOf(account);
        if (lockedBalance == 0) return balance;

        uint256 bondedBalance = BONDING_REGISTRY.totalBonded(account);
        uint256 mustRetain = lockedBalance > bondedBalance
            ? lockedBalance - bondedBalance
            : 0;
        return balance > mustRetain ? balance - mustRetain : 0;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Transfer hook
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * @dev Applies, in order:
     *      1. The pre-TGE transfer gate (_isTransferRestricted)
     *      2. The lock check, the sender can move at most its transferable
     *         balance
     *      3. The transfer itself, via parent contract
     *      4. Claim-lock creation, unless the recipient is in lockWhitelist.
     */
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override(ERC20, ERC20Votes) {
        bool isMint = from == address(0);
        bool isBurn = to == address(0);

        if (_isTransferRestricted(from, to)) {
            revert TransferRestricted(from, to);
        }

        if (!isMint) {
            uint256 transferable = transferableBalanceOf(from);
            if (value > transferable) {
                revert InsufficientUnlockedBalance(from, transferable, value);
            }
        }

        super._update(from, to, value);

        // from == CLAIM_SOURCE implies neither mint nor an unset claim source.
        if (
            !isBurn && value != 0 && from == CLAIM_SOURCE && !lockWhitelist[to]
        ) {
            _claim(to, value);
        }
    }

    /// @dev Whether a transfer from `from` to `to` is blocked by the
    ///      pre-TGE gate. Always false once {tge} fires; mints and burns
    ///      are never gated. The locked-balance check
    ///      ({transferableBalanceOf}) applies independently.
    function _isTransferRestricted(
        address from,
        address to
    ) internal view returns (bool) {
        if (tgeTimestamp != 0) return false;
        if (from == address(0) || to == address(0)) return false;

        address registry = address(BONDING_REGISTRY);
        bool isBonding = from == registry || to == registry;
        uint64 current = uint64(block.timestamp);
        bool isCcaClaim = from == CLAIM_SOURCE &&
            current >= CCA_START &&
            current < CCA_END;
        bool isWhitelisted = transferWhitelist[from] || transferWhitelist[to];
        return !isBonding && !isCcaClaim && !isWhitelisted;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Internals
    // ─────────────────────────────────────────────────────────────────────────

    function _mintAllocation(MintAllocation calldata allocation) internal {
        // Every batch allocation is locked under a policy; unlocked supply
        // goes through {mint}.
        if (allocation.policyId == bytes32(0)) {
            revert InvalidPolicy();
        }

        if (!_policyDefined(allocation.policyId)) {
            revert PolicyNotDefined(allocation.policyId);
        }

        _mintTokens(allocation.recipient, allocation.amount);
        uint256 total = _addOrIncrementLock(
            locks[allocation.recipient],
            allocation.policyId,
            allocation.amount
        );
        emit LockUpdated(allocation.recipient, allocation.policyId, total);
        emit AllocationMinted(
            allocation.recipient,
            allocation.amount,
            allocation.policyId,
            allocation.label
        );
    }

    /// @dev Shared mint path: amount and supply-cap validation plus the
    ///      ERC20 mint itself.
    function _mintTokens(address recipient, uint256 amount) internal {
        if (amount == 0) revert ZeroAmount();
        if (totalSupply() + amount > MAX_SUPPLY) {
            revert MaxSupplyExceeded();
        }
        _mint(recipient, amount);
    }

    /// @dev Claim behavior:
    ///
    ///      - When a claim arrives, scan queuedLocks[account].
    ///      - If there is an entry with exactly the same amount, remove it
    ///        from queuedLocks and add/increment that { policyId, amount }
    ///        in locks.
    ///      - If no exact queued match exists, add/increment
    ///        { PENDING_LOCK_POLICY_ID, amount } in locks.
    ///
    ///      Note: no order guarantees on claims and links.
    function _claim(address account, uint256 amount) private {
        bytes32 policyId = PENDING_LOCK_POLICY_ID;

        Lock[] storage queued = queuedLocks[account];
        (bool found, uint256 index) = _indexOfLockByAmount(queued, amount);
        if (found) {
            policyId = queued[index].policyId;
            _removeLockAt(queued, index);
        }

        uint256 total = _addOrIncrementLock(locks[account], policyId, amount);
        emit LockUpdated(account, policyId, total);
    }

    /// @dev Link behavior:
    ///
    ///      - When linking a claim amount to a real policyId, scan
    ///        locks[account] for PENDING_LOCK_POLICY_ID.
    ///      - If pending has enough amount, deduct the linked amount from
    ///        pending (dropping the entry at zero) and add/increment
    ///        { policyId, amount } in locks.
    ///      - If there is not enough pending, add/increment
    ///        { policyId, amount } in queuedLocks.
    ///
    ///      Note: no order guarantees on claims and links.
    function _linkClaim(
        address account,
        uint256 amount,
        bytes32 policyId
    ) private {
        Lock[] storage active = locks[account];
        (bool found, uint256 index) = _indexOfLockByPolicyId(
            active,
            PENDING_LOCK_POLICY_ID
        );
        if (found && active[index].amount >= amount) {
            uint256 pendingLeft = _decrementLockAt(active, index, amount);
            emit LockUpdated(account, PENDING_LOCK_POLICY_ID, pendingLeft);
            uint256 total = _addOrIncrementLock(active, policyId, amount);
            emit LockUpdated(account, policyId, total);
        } else {
            _addOrIncrementLock(queuedLocks[account], policyId, amount);
        }
    }

    /// @dev Index of the first entry holding exactly `amount`.
    function _indexOfLockByAmount(
        Lock[] storage entries,
        uint256 amount
    ) private view returns (bool found, uint256 index) {
        uint256 len = entries.length;
        for (uint256 i = 0; i < len; i++) {
            if (entries[i].amount == amount) return (true, i);
        }
    }

    /// @dev Index of the first entry under `policyId`.
    function _indexOfLockByPolicyId(
        Lock[] storage entries,
        bytes32 policyId
    ) private view returns (bool found, uint256 index) {
        uint256 len = entries.length;
        for (uint256 i = 0; i < len; i++) {
            if (entries[i].policyId == policyId) return (true, i);
        }
    }

    function _decrementLockAt(
        Lock[] storage entries,
        uint256 index,
        uint256 amount
    ) private returns (uint256 remaining) {
        remaining = entries[index].amount - amount;
        if (remaining == 0) {
            _removeLockAt(entries, index);
        } else {
            entries[index].amount = remaining;
        }
    }

    function _addOrIncrementLock(
        Lock[] storage entries,
        bytes32 policyId,
        uint256 amount
    ) internal returns (uint256 newAmount) {
        uint256 len = entries.length;
        for (uint256 i = 0; i < len; i++) {
            if (entries[i].policyId == policyId) {
                entries[i].amount += amount;
                return entries[i].amount;
            }
        }
        entries.push(Lock({policyId: policyId, amount: amount}));
        return amount;
    }

    function _removeLockAt(Lock[] storage entries, uint256 index) internal {
        entries[index] = entries[entries.length - 1];
        entries.pop();
    }

    /// @dev A defined policy always has a present unlock curve ({createLockPolicy}
    ///      enforces it), so this doubles as existence check.
    function _policyDefined(bytes32 policyId) internal view returns (bool) {
        Curve storage unlock = lockPolicies[policyId].unlock;
        return unlock.cliffDuration != 0 || unlock.vestDuration != 0;
    }

    /// @dev Validates the unlock curve: it must lock something and be
    ///      consistent with its anchor mode.
    function _validateCurve(Curve calldata curve) internal pure {
        if (curve.cliffDuration == 0 && curve.vestDuration == 0) {
            revert InvalidPolicy();
        }
        if (curve.anchor == Anchor.Absolute && curve.start == 0) {
            revert InvalidPolicy();
        }
        if (curve.anchor == Anchor.Tge && curve.start != 0) {
            revert InvalidPolicy();
        }
        // A cliff past the vest end would be a disguised step function.
        if (
            curve.vestDuration != 0 && curve.cliffDuration > curve.vestDuration
        ) {
            revert InvalidPolicy();
        }
    }

    /// @dev Still-locked amount under `policy` at `timestamp`: everything
    ///      before {LockPolicy.holdUntil}; the unlock curve's remainder
    ///      after (the curve accrues through the hold, so the accrued
    ///      portion catches up the moment the hold lapses). Fails closed:
    ///      TGE-anchored curves release nothing while TGE is unset.
    function _lockedAmount(
        LockPolicy storage policy,
        uint256 amount,
        uint64 timestamp
    ) internal view returns (uint256) {
        if (timestamp < policy.holdUntil) return amount;

        Curve storage curve = policy.unlock;
        uint256 anchor;
        if (curve.anchor == Anchor.Tge) {
            anchor = tgeTimestamp;
        } else {
            anchor = curve.start;
        }

        if (anchor == 0 || timestamp < anchor + curve.cliffDuration) {
            return amount;
        }

        uint256 vestDuration = curve.vestDuration;
        if (vestDuration == 0 || timestamp >= anchor + vestDuration) {
            return 0;
        }
        return amount - (amount * (timestamp - anchor)) / vestDuration;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Required overrides
    // ─────────────────────────────────────────────────────────────────────────

    /// @inheritdoc ERC20Permit
    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }

    /// @notice EIP-6372 clock — block.timestamp, aligned with other Enclave
    ///         contracts.
    function clock() public view override returns (uint48) {
        return uint48(block.timestamp);
    }

    /// @notice EIP-6372 clock mode.
    // solhint-disable-next-line func-name-mixedcase
    function CLOCK_MODE() public pure override returns (string memory) {
        return "mode=timestamp";
    }

    /// @notice Disabled. Reverts unconditionally.
    function renounceOwnership() public view override onlyOwner {
        revert RenounceOwnershipDisabled();
    }

    /**
     * @notice Synchronises AccessControl roles whenever Ownable2Step completes a
     *         transfer (i.e. when {acceptOwnership} is called by the pending owner).
     */
    function _transferOwnership(address newOwner) internal override {
        address previousOwner = owner();
        super._transferOwnership(newOwner);
        if (previousOwner != address(0)) {
            _revokeRole(DEFAULT_ADMIN_ROLE, previousOwner);
            _revokeRole(MINTER_ROLE, previousOwner);
            _revokeRole(WHITELIST_ROLE, previousOwner);
            _revokeRole(LOCK_MANAGER_ROLE, previousOwner);
        }
        if (newOwner != address(0)) {
            _grantRole(DEFAULT_ADMIN_ROLE, newOwner);
            _grantRole(MINTER_ROLE, newOwner);
            _grantRole(WHITELIST_ROLE, newOwner);
            _grantRole(LOCK_MANAGER_ROLE, newOwner);
        }
    }
}

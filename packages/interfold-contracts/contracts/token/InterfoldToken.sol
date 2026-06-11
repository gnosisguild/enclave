// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    ERC20Permit,
    Nonces
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {
    ERC20Votes
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Votes.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { Ownable2Step } from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {
    AccessControl
} from "@openzeppelin/contracts/access/AccessControl.sol";

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";

/**
 * @title InterfoldToken
 * @notice The governance and utility token for the Interfold protocol
 * @dev ERC20 token with voting capabilities, permit functionality, and controlled minting.
 *
 *      Lifecycle — Virtual → Live (TGE):
 *      The token starts in {TokenMode.Virtual}. Once the configured {tgeEarliest}
 *      timestamp has passed, LOCK_MANAGER_ROLE calls {tge} to transition to
 *      {TokenMode.Live}. At that point {tgeTimestamp} is set to block.timestamp and
 *      all TGE-anchored lock schedules (those with tokenUnlockStart = 0) can be
 *      created and resolve.
 *
 *      Roles:
 *      - DEFAULT_ADMIN_ROLE manages role assignments and can {disableTransferRestrictions}
 *        (only after TGE — requires {TokenMode.Live}).
 *      - MINTER_ROLE can call {mintAllocation} / {batchMintAllocations} up to MAX_SUPPLY.
 *      - WHITELIST_ROLE can manage the transfer whitelist independently from minting so
 *        the same account is not required to control both surfaces.
 *      - LOCK_MANAGER_ROLE can configure token-level lock schedules, CCA claim sources,
 *        buyer claim profiles, the bonding registry, and the TGE transition.
 *
 *      Transfer restrictions are a one-way switch: once {disableTransferRestrictions} is called
 *      they cannot be re-enabled. The TGE transition ({TokenMode.Virtual} → {TokenMode.Live})
 *      is also one-way.
 *
 *      Token-level locks are pooled per account. For every non-mint/non-burn transfer, the sender
 *      must satisfy: balanceOf(sender) + BondingRegistry.totalBonded(sender) >= lockedFloorOf(sender).
 *      This lets locked holders use same-account ENCL as operator bond collateral while preserving
 *      the explicit product constraint that all ENCL in the same wallet is pooled for locks/slashing.
 *
 *      Voting uses {block.timestamp} (EIP-6372 "mode=timestamp") so timepoints align with other
 *      Interfold contracts.
 */
contract InterfoldToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable2Step,
    AccessControl
{
    /// @notice Thrown when {renounceOwnership} is called. Ownership is
    ///         critical for protocol governance; renouncing would permanently
    ///         freeze admin functions and is disallowed.
    error RenounceOwnershipDisabled();
    // Custom errors

    /// @notice Thrown when a zero address is provided where a valid address is required
    error ZeroAddress();

    /// @notice Thrown when attempting to mint zero tokens
    error ZeroAmount();

    /// @notice Thrown when minting would exceed the maximum token supply
    error ExceedsTotalSupply();

    /// @notice Thrown when array parameters have mismatched lengths
    error ArrayLengthMismatch();

    /// @notice Thrown when a transfer is attempted while restrictions are active and neither party is whitelisted
    error TransferNotAllowed();

    /// @notice Thrown when lock schedule parameters are internally inconsistent.
    error InvalidLockSchedule();

    /// @notice Thrown when an account already has the maximum supported number of lock schedules.
    error MaxLockSchedulesExceeded();

    /// @notice Thrown when a locked account attempts to move below its current locked floor.
    error LockedBalanceInvariant(
        address account,
        uint256 balance,
        uint256 bonded,
        uint256 lockedFloor
    );

    /// @notice Thrown when a claim source sends locked CCA tokens to an account without an active profile.
    error ClaimLockProfileMissing(address account);

    /// @notice Thrown when a schedule requests the default TGE timestamp before it has been configured.
    error TgeTimestampUnset();

    /// @notice Thrown when {tge} is called before the earliest allowed timestamp.
    error TgeTooEarly(uint64 current, uint64 earliest);

    /// @notice Thrown when {tge} is called but the token is already live.
    error TgeAlreadyLive();

    /// @notice Thrown when an operation requires the token to be in Live mode.
    error TokenNotLive();

    /// @notice Token lifecycle mode.
    /// @dev Virtual: pre-TGE phase. Live: TGE has occurred, {tgeTimestamp} is set,
    ///      and TGE-anchored lock schedules can be created and resolved.
    enum TokenMode {
        Virtual,
        Live
    }

    /// @notice Maximum supply of the token: 1.2 billion tokens with 18 decimals
    /// @dev Hard cap on total token supply that cannot be exceeded through minting
    uint256 public constant MAX_SUPPLY = 1_200_000_000e18;

    /// @notice Maximum lock schedules retained for a single account.
    /// @dev Bounds the per-transfer loop in {lockedFloorOf}.
    uint256 public constant MAX_LOCK_SCHEDULES = 64;

    /// @notice Role identifier for accounts authorized to mint new tokens
    /// @dev Keccak256 hash of "MINTER_ROLE" used in AccessControl
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    /// @notice Role identifier for accounts authorized to manage the transfer whitelist.
    /// @dev Separated from MINTER_ROLE so mint authority does not also control transferability.
    bytes32 public constant WHITELIST_ROLE = keccak256("WHITELIST_ROLE");

    /// @notice Role identifier for accounts authorized to manage token-level locks.
    bytes32 public constant LOCK_MANAGER_ROLE = keccak256("LOCK_MANAGER_ROLE");

    /// @notice Token-lock schedule recorded against one account.
    /// @param amount Original amount subject to this schedule.
    /// @param tokenHoldUntil Absolute timestamp before which no amount is transferable.
    /// @param tokenUnlockStart Absolute linear token unlock start timestamp.
    /// @param tokenUnlockEnd Absolute linear token unlock end timestamp.
    /// @param serviceStart Optional service vesting start timestamp.
    /// @param serviceCliff Optional service vesting cliff timestamp.
    /// @param serviceEnd Optional service vesting end timestamp.
    /// @param group Schedule group marker for indexers and operations.
    struct LockSchedule {
        uint128 amount;
        uint64 tokenHoldUntil;
        uint64 tokenUnlockStart;
        uint64 tokenUnlockEnd;
        uint64 serviceStart;
        uint64 serviceCliff;
        uint64 serviceEnd;
        bytes32 group;
    }

    /// @notice Input used to create an absolute lock schedule.
    /// @param account Account whose wallet-level locked floor increases.
    /// @param amount Amount subject to the schedule.
    /// @param tokenHoldUntil Absolute timestamp before which no amount is transferable.
    /// @param tokenUnlockStart Absolute linear unlock start. Zero resolves to {tgeTimestamp}.
    /// @param tokenUnlockEnd Absolute linear unlock end.
    /// @param serviceStart Optional service vesting start timestamp.
    /// @param serviceCliff Optional service vesting cliff timestamp.
    /// @param serviceEnd Optional service vesting end timestamp.
    /// @param group Schedule group marker for indexers and operations.
    struct LockScheduleInput {
        address account;
        uint256 amount;
        uint64 tokenHoldUntil;
        uint64 tokenUnlockStart;
        uint64 tokenUnlockEnd;
        uint64 serviceStart;
        uint64 serviceCliff;
        uint64 serviceEnd;
        bytes32 group;
    }

    /// @notice Relative lock profile applied to transfers from approved CCA claim sources.
    /// @param active Whether the account can receive locked claim-source transfers.
    /// @param lockStart Absolute timestamp (e.g. CCA auction end) that anchors the
    ///        hold and unlock durations. All buyers in the same route share the same
    ///        lock start regardless of when they claim.
    /// @param holdDuration Seconds after {lockStart} before any amount is transferable.
    /// @param unlockDuration Optional linear unlock duration after the hold ends. Zero is a cliff unlock.
    /// @param group Schedule group marker, e.g. REG_S_CCA or REG_D_CCA.
    struct ClaimLockProfile {
        bool active;
        uint64 lockStart;
        uint64 holdDuration;
        uint64 unlockDuration;
        bytes32 group;
    }

    /// @notice Input used to batch update CCA claim lock profiles.
    struct ClaimLockProfileInput {
        address account;
        ClaimLockProfile profile;
    }

    /// @notice Tracks the cumulative amount of tokens minted since deployment
    uint256 public totalMinted;

    /// @notice Optional default TGE timestamp used when lock schedule inputs pass tokenUnlockStart = 0.
    uint64 public tgeTimestamp;

    /// @notice Current token lifecycle mode. Starts as {TokenMode.Virtual}.
    TokenMode public mode;

    /// @notice Earliest timestamp at which {tge} may be called.
    uint64 public tgeEarliest;

    /// @notice Registry queried for ENCL that still counts toward an account's locked floor.
    IBondingRegistry public bondingRegistry;

    /// @notice Mapping of addresses permitted to transfer tokens when restrictions are active
    /// @dev When transfersRestricted is true, only whitelisted addresses can send or receive tokens
    mapping(address account => bool allowed) public transferWhitelisted;

    /// @notice Indicates whether token transfers are currently restricted
    /// @dev When true, only whitelisted addresses can transfer tokens
    bool public transfersRestricted;

    /// @notice Approved CCA/auction claim sources whose outbound ENCL creates wallet-level locks.
    mapping(address source => bool approved) public approvedClaimSources;

    /// @notice Relative CCA claim lock profile for each buyer/recipient.
    mapping(address account => ClaimLockProfile profile)
        public claimLockProfiles;

    /// @notice Recipients that do NOT receive a claim-lock schedule when they
    ///         receive tokens from an approved claim source. Used for CCA sweeps,
    ///         treasury returns, and other system recipients that are not buyers.
    mapping(address account => bool exempt) public claimLockExemptRecipients;

    /// @dev Lock schedules by account. The array length is bounded by {MAX_LOCK_SCHEDULES}.
    mapping(address account => LockSchedule[] schedules) private _lockSchedules;

    /// @notice Emitted when tokens are minted as part of a named allocation
    /// @param recipient Address receiving the minted tokens
    /// @param amount Number of tokens minted (18 decimals)
    /// @param allocation Description of the allocation for tracking purposes
    event AllocationMinted(
        address indexed recipient,
        uint256 amount,
        string allocation
    );

    /// @notice Emitted when the transfer restriction setting is changed
    /// @param restricted New state of transfer restrictions (true = restricted, false = unrestricted)
    event TransferRestrictionUpdated(bool restricted);

    /// @notice Emitted when an address is added to or removed from the transfer whitelist
    /// @param account Address whose whitelist status changed
    /// @param whitelisted New whitelist status (true = whitelisted, false = not whitelisted)
    event TransferWhitelistUpdated(address indexed account, bool whitelisted);

    /// @notice Emitted when the default TGE timestamp changes.
    event TgeTimestampUpdated(uint64 previous, uint64 next);

    /// @notice Emitted when the bonding registry used for locked-floor accounting changes.
    event BondingRegistryUpdated(
        address indexed previous,
        address indexed next
    );

    /// @notice Emitted when a lock schedule is created for an account.
    event LockScheduleCreated(
        address indexed account,
        uint256 indexed scheduleId,
        bytes32 indexed group,
        uint256 amount,
        uint64 tokenHoldUntil,
        uint64 tokenUnlockStart,
        uint64 tokenUnlockEnd,
        uint64 serviceStart,
        uint64 serviceCliff,
        uint64 serviceEnd
    );

    /// @notice Emitted when a CCA/auction claim source is approved or revoked.
    event ClaimSourceUpdated(address indexed source, bool approved);

    /// @notice Emitted when a relative CCA claim lock profile changes.
    event ClaimLockProfileUpdated(
        address indexed account,
        bool active,
        uint64 lockStart,
        uint64 holdDuration,
        uint64 unlockDuration,
        bytes32 indexed group
    );

    /// @notice Emitted when the token transitions from Virtual to Live mode.
    event TgeTriggered(uint64 timestamp);

    /// @notice Emitted when the earliest TGE timestamp is updated.
    event TgeEarliestUpdated(uint64 previous, uint64 next);

    /// @notice Emitted when a claim-lock exemption is set or cleared.
    event ClaimLockExemptionUpdated(address indexed account, bool exempt);

    /**
     * @notice Initializes the Interfold token with name "Interfold" and symbol "INTF"
     * @dev Sets up the token with voting and permit functionality. Grants admin, minter, and
     *      whitelist roles to the owner; enables transfer restrictions; whitelists the owner.
     * @param initialOwner_ Address that will own the contract and receive admin, minter, whitelist, and lock roles.
     */
    constructor(
        address initialOwner_
    )
        ERC20("Interfold", "INTF")
        ERC20Permit("Interfold")
        Ownable(initialOwner_)
    {
        // Grant the deployer all admin roles.
        _grantRole(DEFAULT_ADMIN_ROLE, initialOwner_);
        _grantRole(MINTER_ROLE, initialOwner_);
        _grantRole(WHITELIST_ROLE, initialOwner_);
        _grantRole(LOCK_MANAGER_ROLE, initialOwner_);

        // Initialise state variables.
        mode = TokenMode.Virtual;
        transfersRestricted = true;
        transferWhitelisted[initialOwner_] = true;

        emit TransferRestrictionUpdated(true);
        emit TransferWhitelistUpdated(initialOwner_, true);
    }

    /**
     * @notice Mints a named allocation of tokens to a specified recipient
     * @dev Only callable by accounts with MINTER_ROLE. Reverts if recipient is zero address,
     *      amount is zero, or minting would exceed MAX_SUPPLY.
     * @param recipient Address to receive the minted tokens (cannot be zero address)
     * @param amount Number of tokens to mint in wei (18 decimals, must be greater than zero)
     * @param allocation Human-readable description of this allocation for tracking and auditing purposes
     */
    function mintAllocation(
        address recipient,
        uint256 amount,
        string memory allocation
    ) external onlyRole(MINTER_ROLE) {
        if (recipient == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        // Ensure we do not exceed the total supply.
        if (totalMinted + amount > MAX_SUPPLY) revert ExceedsTotalSupply();

        _mint(recipient, amount);
        totalMinted += amount;
        emit AllocationMinted(recipient, amount, allocation);
    }

    /**
     * @notice Mints multiple named allocations to different recipients in a single transaction
     * @dev Only callable by accounts with MINTER_ROLE. All arrays must have the same length.
     *      Reverts if any amount is zero, or if cumulative minting would exceed MAX_SUPPLY.
     * @param recipients Array of addresses to receive minted tokens
     * @param amounts Array of token amounts to mint (18 decimals, must match recipients length)
     * @param allocations Array of allocation descriptions (must match recipients length)
     */
    function batchMintAllocations(
        address[] calldata recipients,
        uint256[] calldata amounts,
        string[] calldata allocations
    ) external onlyRole(MINTER_ROLE) {
        uint256 len = recipients.length;
        if (amounts.length != len || allocations.length != len) {
            revert ArrayLengthMismatch();
        }

        uint256 minted = totalMinted;

        for (uint256 i = 0; i < len; i++) {
            address recipient = recipients[i];
            uint256 amount = amounts[i];
            if (recipient == address(0)) revert ZeroAddress();
            if (amount == 0) revert ZeroAmount();

            if (amount > MAX_SUPPLY - minted) revert ExceedsTotalSupply();
            minted += amount;

            _mint(recipient, amount);
            emit AllocationMinted(recipient, amount, allocations[i]);
        }

        totalMinted = minted;
    }

    /**
     * @notice Permanently disables transfer restrictions.
     * @dev Once disabled, restrictions cannot be re-enabled (one-way switch).
     *      Only callable by DEFAULT_ADMIN_ROLE, and only after the token is Live
     *      ({tge} has been called). Idempotent: a no-op when already disabled.
     */
    function disableTransferRestrictions()
        external
        onlyRole(DEFAULT_ADMIN_ROLE)
    {
        if (mode != TokenMode.Live) revert TokenNotLive();
        if (!transfersRestricted) return;
        transfersRestricted = false;
        emit TransferRestrictionUpdated(false);
    }

    /**
     * @notice Toggles an account's transfer whitelist status between enabled and disabled
     * @dev Only callable by accounts holding WHITELIST_ROLE. Flips the current whitelist
     *      state for the given account. Whitelisted accounts can send and receive tokens even
     *      when transfer restrictions are active.
     * @param account Address whose whitelist status will be toggled
     */
    function toggleTransferWhitelist(
        address account
    ) external onlyRole(WHITELIST_ROLE) {
        bool newStatus = !transferWhitelisted[account];
        transferWhitelisted[account] = newStatus;
        emit TransferWhitelistUpdated(account, newStatus);
    }

    /**
     * @notice Whitelists key protocol contracts to allow them to transfer tokens during restricted periods
     * @dev Only callable by accounts holding WHITELIST_ROLE. Zero addresses are safely ignored.
     * @param bondingManager Address of the BondingManager contract (zero address skipped)
     * @param claimSource Address of a claim source contract (zero address skipped)
     */
    function whitelistContracts(
        address bondingManager,
        address claimSource
    ) external onlyRole(WHITELIST_ROLE) {
        if (bondingManager != address(0)) {
            transferWhitelisted[bondingManager] = true;
            emit TransferWhitelistUpdated(bondingManager, true);
        }
        if (claimSource != address(0)) {
            transferWhitelisted[claimSource] = true;
            emit TransferWhitelistUpdated(claimSource, true);
        }
    }

    /// @notice Sets the default TGE timestamp used by schedule inputs with tokenUnlockStart = 0.
    /// @dev Callable in both Virtual and Live mode. Before TGE this allows pre-configuring
    ///      the timestamp that schedules will resolve against. After TGE, {tge} sets the
    ///      timestamp to block.timestamp; this setter exists for administrative correction.
    ///      Existing schedules store resolved timestamps and are not changed by this setter.
    function setTgeTimestamp(
        uint64 newTgeTimestamp
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (newTgeTimestamp == 0) revert InvalidLockSchedule();
        uint64 previous = tgeTimestamp;
        tgeTimestamp = newTgeTimestamp;
        emit TgeTimestampUpdated(previous, newTgeTimestamp);
    }

    /// @notice Sets the bonding registry queried by locked-floor transfer checks.
    /// @dev Passing zero disables bonded-credit accounting. Non-zero values must be deployed code.
    function setBondingRegistry(
        IBondingRegistry newBondingRegistry
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        address newRegistryAddress = address(newBondingRegistry);
        if (
            newRegistryAddress != address(0) &&
            newRegistryAddress.code.length == 0
        ) revert ZeroAddress();

        address previous = address(bondingRegistry);
        bondingRegistry = newBondingRegistry;
        emit BondingRegistryUpdated(previous, newRegistryAddress);
    }

    /// @notice Sets the earliest timestamp at which {tge} may be called.
    /// @dev Must be set before {tge} is called. Value is a Unix timestamp in seconds.
    function setTgeEarliest(
        uint64 newTgeEarliest
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (newTgeEarliest == 0) revert InvalidLockSchedule();
        if (mode == TokenMode.Live) revert TgeAlreadyLive();
        uint64 previous = tgeEarliest;
        tgeEarliest = newTgeEarliest;
        emit TgeEarliestUpdated(previous, newTgeEarliest);
    }

    /// @notice Transitions the token from Virtual to Live mode.
    /// @dev One-way switch. Requires {tgeEarliest} to be set and the current
    ///      block timestamp to be >= {tgeEarliest}. Sets {tgeTimestamp} to
    ///      block.timestamp unless a future timestamp was pre-configured via
    ///      {setTgeTimestamp} during Virtual mode.
    function tge() external onlyRole(LOCK_MANAGER_ROLE) {
        if (mode == TokenMode.Live) revert TgeAlreadyLive();
        uint64 earliest = tgeEarliest;
        if (earliest == 0) revert TgeTimestampUnset();
        uint64 current = _currentTimestamp();
        if (current < earliest) revert TgeTooEarly(current, earliest);

        mode = TokenMode.Live;
        if (tgeTimestamp == 0) {
            tgeTimestamp = current;
        }
        emit TgeTriggered(current);
    }

    /// @notice Returns true when the token is in Live (post-TGE) mode.
    function isLive() external view returns (bool) {
        return mode == TokenMode.Live;
    }

    /// @notice Creates a wallet-level lock schedule for an account.
    function createLockSchedule(
        LockScheduleInput calldata input
    ) external onlyRole(LOCK_MANAGER_ROLE) returns (uint256 scheduleId) {
        scheduleId = _addLockSchedule(input);
        _enforceLockedFloor(input.account);
    }

    /// @notice Creates many wallet-level lock schedules.
    function batchCreateLockSchedules(
        LockScheduleInput[] calldata inputs
    )
        external
        onlyRole(LOCK_MANAGER_ROLE)
        returns (uint256[] memory scheduleIds)
    {
        uint256 len = inputs.length;
        scheduleIds = new uint256[](len);

        for (uint256 i = 0; i < len; i++) {
            scheduleIds[i] = _addLockSchedule(inputs[i]);
            _enforceLockedFloor(inputs[i].account);
        }
    }

    /// @notice Approves or revokes a CCA/auction claim source.
    function setClaimSource(
        address source,
        bool approved
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (source == address(0)) revert ZeroAddress();
        approvedClaimSources[source] = approved;
        emit ClaimSourceUpdated(source, approved);
    }

    /// @notice Sets the relative claim lock profile for a buyer/recipient.
    function setClaimLockProfile(
        address account,
        ClaimLockProfile calldata profile
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        _setClaimLockProfile(account, profile);
    }

    /// @notice Batch sets relative claim lock profiles.
    function batchSetClaimLockProfiles(
        ClaimLockProfileInput[] calldata inputs
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        uint256 len = inputs.length;
        for (uint256 i = 0; i < len; i++) {
            _setClaimLockProfile(inputs[i].account, inputs[i].profile);
        }
    }

    /// @notice Marks an address as exempt from automatic claim-lock schedule creation.
    /// @dev Used for CCA sweep/treasury recipients that receive tokens from an approved
    ///      claim source but are not buyers. Exempt recipients skip {_addClaimLock}.
    function setClaimLockExemption(
        address account,
        bool exempt
    ) external onlyRole(LOCK_MANAGER_ROLE) {
        if (account == address(0)) revert ZeroAddress();
        claimLockExemptRecipients[account] = exempt;
        emit ClaimLockExemptionUpdated(account, exempt);
    }

    /// @notice Number of lock schedules recorded for an account.
    function lockScheduleCount(
        address account
    ) external view returns (uint256) {
        return _lockSchedules[account].length;
    }

    /// @notice Returns a lock schedule by account and index.
    function lockScheduleOf(
        address account,
        uint256 scheduleId
    ) external view returns (LockSchedule memory) {
        return _lockSchedules[account][scheduleId];
    }

    /// @notice Current amount that must remain controlled by an account's wallet plus bonded ENCL.
    function lockedFloorOf(address account) public view returns (uint256) {
        return lockedFloorAt(account, uint64(block.timestamp));
    }

    /// @notice Amount that must remain controlled by an account at a given timestamp.
    function lockedFloorAt(
        address account,
        uint64 timestamp
    ) public view returns (uint256 lockedFloor) {
        LockSchedule[] storage schedules = _lockSchedules[account];
        uint256 len = schedules.length;
        for (uint256 i = 0; i < len; i++) {
            lockedFloor += _lockedAmount(schedules[i], timestamp);
        }
    }

    /// @notice ENCL bonded by an account that still counts toward its locked floor.
    function totalBondedOf(address account) public view returns (uint256) {
        IBondingRegistry registry = bondingRegistry;
        if (address(registry) == address(0)) return 0;
        return registry.totalBonded(account);
    }

    /// @notice Current wallet balance that can be transferred without violating the locked floor.
    function transferableBalanceOf(
        address account
    ) external view returns (uint256) {
        uint256 balance = balanceOf(account);
        uint256 bonded = totalBondedOf(account);
        uint256 lockedFloor = lockedFloorOf(account);
        uint256 controlled = balance + bonded;
        if (controlled <= lockedFloor) return 0;

        uint256 transferable = controlled - lockedFloor;
        return transferable < balance ? transferable : balance;
    }

    /**
     * @notice Internal hook that enforces transfer restrictions and updates voting power
     * @dev Overrides ERC20 and ERC20Votes to add transfer restriction logic. Reverts if transfers
     *      are restricted and neither sender nor receiver is whitelisted. Minting (from == 0) and
     *      burning (to == 0) are always allowed regardless of restrictions.
     *
     * @param from Address sending tokens (zero address for minting)
     * @param to Address receiving tokens (zero address for burning)
     * @param value Amount of tokens being transferred
     */
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override(ERC20, ERC20Votes) {
        // When transfers are restricted, only whitelisted addresses can send or receive.
        if (from != address(0) && to != address(0) && transfersRestricted) {
            if (!transferWhitelisted[from] && !transferWhitelisted[to]) {
                revert TransferNotAllowed();
            }
        }
        super._update(from, to, value);

        if (from != address(0) && to != address(0)) {
            if (
                approvedClaimSources[from] &&
                value != 0 &&
                from != address(bondingRegistry) &&
                !claimLockExemptRecipients[to]
            ) {
                _addClaimLock(to, value);
            }
            _enforceLockedFloor(from);
        }
    }

    function _addLockSchedule(
        LockScheduleInput calldata input
    ) internal returns (uint256 scheduleId) {
        if (input.account == address(0)) revert ZeroAddress();
        if (input.amount == 0) revert ZeroAmount();
        if (input.amount > type(uint128).max) revert InvalidLockSchedule();

        uint64 tokenUnlockStart = _resolveTokenUnlockStart(
            input.tokenUnlockStart
        );
        _validateLockSchedule(
            tokenUnlockStart,
            input.tokenUnlockEnd,
            input.serviceStart,
            input.serviceCliff,
            input.serviceEnd
        );

        scheduleId = _pushLockSchedule(
            input.account,
            LockSchedule({
                amount: uint128(input.amount),
                tokenHoldUntil: input.tokenHoldUntil,
                tokenUnlockStart: tokenUnlockStart,
                tokenUnlockEnd: input.tokenUnlockEnd,
                serviceStart: input.serviceStart,
                serviceCliff: input.serviceCliff,
                serviceEnd: input.serviceEnd,
                group: input.group
            })
        );
    }

    function _addClaimLock(address account, uint256 amount) internal {
        ClaimLockProfile memory profile = claimLockProfiles[account];
        if (!profile.active) revert ClaimLockProfileMissing(account);
        if (profile.holdDuration == 0 && profile.unlockDuration == 0) {
            revert InvalidLockSchedule();
        }
        if (amount > type(uint128).max) revert InvalidLockSchedule();

        uint256 holdUntil = uint256(profile.lockStart) + profile.holdDuration;
        uint256 unlockEnd = holdUntil + profile.unlockDuration;
        if (unlockEnd > type(uint64).max) revert InvalidLockSchedule();

        uint64 tokenHoldUntil = uint64(holdUntil);
        uint64 tokenUnlockEnd = uint64(unlockEnd);

        _pushLockSchedule(
            account,
            LockSchedule({
                amount: uint128(amount),
                tokenHoldUntil: tokenHoldUntil,
                tokenUnlockStart: tokenHoldUntil,
                tokenUnlockEnd: tokenUnlockEnd,
                serviceStart: 0,
                serviceCliff: 0,
                serviceEnd: 0,
                group: profile.group
            })
        );
    }

    function _pushLockSchedule(
        address account,
        LockSchedule memory schedule
    ) internal returns (uint256 scheduleId) {
        LockSchedule[] storage schedules = _lockSchedules[account];
        uint256 len = schedules.length;
        if (len < MAX_LOCK_SCHEDULES) {
            schedules.push(schedule);
            scheduleId = len;
        } else {
            scheduleId = _reclaimUnlockedScheduleSlot(schedules);
            schedules[scheduleId] = schedule;
        }

        emit LockScheduleCreated(
            account,
            scheduleId,
            schedule.group,
            uint256(schedule.amount),
            schedule.tokenHoldUntil,
            schedule.tokenUnlockStart,
            schedule.tokenUnlockEnd,
            schedule.serviceStart,
            schedule.serviceCliff,
            schedule.serviceEnd
        );
    }

    function _reclaimUnlockedScheduleSlot(
        LockSchedule[] storage schedules
    ) internal view returns (uint256 scheduleId) {
        uint64 currentTimestamp = _currentTimestamp();
        uint256 len = schedules.length;
        for (uint256 i = 0; i < len; i++) {
            if (_lockedAmount(schedules[i], currentTimestamp) == 0) return i;
        }

        revert MaxLockSchedulesExceeded();
    }

    function _setClaimLockProfile(
        address account,
        ClaimLockProfile calldata profile
    ) internal {
        if (account == address(0)) revert ZeroAddress();
        if (profile.active) {
            if (profile.lockStart == 0) revert InvalidLockSchedule();
            if (profile.holdDuration == 0 && profile.unlockDuration == 0)
                revert InvalidLockSchedule();
        }

        claimLockProfiles[account] = profile;
        emit ClaimLockProfileUpdated(
            account,
            profile.active,
            profile.lockStart,
            profile.holdDuration,
            profile.unlockDuration,
            profile.group
        );
    }

    function _resolveTokenUnlockStart(
        uint64 tokenUnlockStart
    ) internal view returns (uint64) {
        if (tokenUnlockStart != 0) return tokenUnlockStart;

        uint64 configuredTgeTimestamp = tgeTimestamp;
        if (configuredTgeTimestamp == 0) revert TgeTimestampUnset();
        return configuredTgeTimestamp;
    }

    function _currentTimestamp() internal view returns (uint64) {
        if (block.timestamp > type(uint64).max) revert InvalidLockSchedule();
        return uint64(block.timestamp);
    }

    function _validateLockSchedule(
        uint64 tokenUnlockStart,
        uint64 tokenUnlockEnd,
        uint64 serviceStart,
        uint64 serviceCliff,
        uint64 serviceEnd
    ) internal pure {
        if (tokenUnlockStart > tokenUnlockEnd) revert InvalidLockSchedule();

        bool hasServiceCurve = serviceStart != 0 ||
            serviceCliff != 0 ||
            serviceEnd != 0;
        if (!hasServiceCurve) return;

        if (
            serviceStart == 0 ||
            serviceEnd <= serviceStart ||
            serviceCliff < serviceStart ||
            serviceCliff > serviceEnd
        ) revert InvalidLockSchedule();
    }

    function _lockedAmount(
        LockSchedule storage schedule,
        uint64 timestamp
    ) internal view returns (uint256) {
        uint256 amount = uint256(schedule.amount);
        uint256 tokenUnlocked = _tokenUnlockedAmount(schedule, timestamp);
        uint256 serviceVested = _serviceVestedAmount(schedule, timestamp);
        uint256 released = tokenUnlocked < serviceVested
            ? tokenUnlocked
            : serviceVested;
        return amount - released;
    }

    function _tokenUnlockedAmount(
        LockSchedule storage schedule,
        uint64 timestamp
    ) internal view returns (uint256) {
        uint256 amount = uint256(schedule.amount);
        if (timestamp < schedule.tokenHoldUntil) return 0;
        if (schedule.tokenUnlockStart == schedule.tokenUnlockEnd) return amount;
        if (timestamp <= schedule.tokenUnlockStart) return 0;
        if (timestamp >= schedule.tokenUnlockEnd) return amount;

        uint256 elapsed = uint256(timestamp - schedule.tokenUnlockStart);
        uint256 duration = uint256(
            schedule.tokenUnlockEnd - schedule.tokenUnlockStart
        );
        return (amount * elapsed) / duration;
    }

    function _serviceVestedAmount(
        LockSchedule storage schedule,
        uint64 timestamp
    ) internal view returns (uint256) {
        uint256 amount = uint256(schedule.amount);
        if (schedule.serviceStart == 0 && schedule.serviceEnd == 0) {
            return amount;
        }
        if (timestamp < schedule.serviceCliff) return 0;
        if (timestamp >= schedule.serviceEnd) return amount;

        uint256 elapsed = uint256(timestamp - schedule.serviceStart);
        uint256 duration = uint256(schedule.serviceEnd - schedule.serviceStart);
        return (amount * elapsed) / duration;
    }

    function _enforceLockedFloor(address account) internal view {
        uint256 lockedFloor = lockedFloorOf(account);
        if (lockedFloor == 0) return;

        uint256 balance = balanceOf(account);
        uint256 bonded = totalBondedOf(account);
        if (balance + bonded < lockedFloor) {
            revert LockedBalanceInvariant(
                account,
                balance,
                bonded,
                lockedFloor
            );
        }
    }

    /**
     * @notice Checks if this contract implements a given interface
     * @dev Implements ERC165 interface detection via AccessControl
     * @param interfaceId The interface identifier to check, as specified in ERC-165
     * @return bool True if the contract implements the interface, false otherwise
     */
    function supportsInterface(
        bytes4 interfaceId
    ) public view override(AccessControl) returns (bool) {
        return super.supportsInterface(interfaceId);
    }

    /**
     * @notice Returns the current nonce for an address, used for permit signatures
     * @dev Resolves the override conflict between ERC20Permit and Nonces by calling the parent
     *      implementation. Nonces are incremented with each permit to prevent replay attacks.
     * @param owner Address to query the nonce for
     * @return uint256 The current nonce value for the given address
     */
    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }

    // ── EIP-6372 clock (timestamp mode) ───────────────────────────────────────

    /// @notice EIP-6372 clock — uses {block.timestamp}.
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
     * @dev Without this override, the new `owner()` would have no roles: the previous
     *      owner would silently retain DEFAULT_ADMIN_ROLE, MINTER_ROLE, and WHITELIST_ROLE.
     *      Called internally by {Ownable._transferOwnership}; never call directly.
     *
     *      Roles are also granted during construction (previousOwner == address(0)),
     *      but the constructor body already calls `_grantRole` explicitly, so the
     *      grant here is idempotent for the deployment case and adds no overhead.
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

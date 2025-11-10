// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {
    ERC20Permit,
    Nonces
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import {
    ERC20Votes
} from "@openzeppelin/contracts/token/ERC20/extensions/ERC20Votes.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    AccessControl
} from "@openzeppelin/contracts/access/AccessControl.sol";

/**
 * @title EnclaveToken
 * @notice The governance and utility token for the Enclave protocol
 * @dev ERC20 token with voting capabilities, permit functionality, and controlled minting.
 *      Implements transfer restrictions that can be toggled by the owner to control token
 *      transferability during early phases. Supports a maximum supply cap and role-based
 *      minting through the MINTER_ROLE.
 */
contract EnclaveToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable,
    AccessControl
{
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

    /// @notice Maximum supply of the token: 1.2 billion tokens with 18 decimals
    /// @dev Hard cap on total token supply that cannot be exceeded through minting
    uint256 public constant MAX_SUPPLY = 1_200_000_000e18;

    /// @notice Role identifier for accounts authorized to mint new tokens
    /// @dev Keccak256 hash of "MINTER_ROLE" used in AccessControl
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    /// @notice Tracks the cumulative amount of tokens minted since deployment
    /// @dev Incremented with each mint operation to enforce MAX_SUPPLY cap
    uint256 public totalMinted;

    /// @notice Mapping of addresses permitted to transfer tokens when restrictions are active
    /// @dev When transfersRestricted is true, only whitelisted addresses can send or receive tokens
    mapping(address account => bool allowed) public transferWhitelisted;

    /// @notice Indicates whether token transfers are currently restricted
    /// @dev When true, only whitelisted addresses can transfer tokens
    bool public transfersRestricted;

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

    /**
     * @notice Initializes the Enclave token with name "Enclave" and symbol "ENCL"
     * @dev Sets up the token with voting and permit functionality. Grants admin and minter
     *      roles to the owner, enables transfer restrictions, and whitelists the owner.
     * @param _owner Address that will own the contract and receive DEFAULT_ADMIN_ROLE and MINTER_ROLE
     */
    constructor(
        address _owner
    ) ERC20("Enclave", "ENCL") ERC20Permit("Enclave") Ownable(_owner) {
        // Grant the deployer all admin roles.
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(MINTER_ROLE, _owner);

        // Initialise state variables.
        transfersRestricted = true;
        transferWhitelisted[_owner] = true;

        emit TransferRestrictionUpdated(true);
        emit TransferWhitelistUpdated(_owner, true);
    }

    /**
     * @notice Mints a named allocation of tokens to a specified recipient
     * @dev Only callable by accounts with MINTER_ROLE. Reverts if recipient is zero address,
     *      amount is zero, or minting would exceed MAX_SUPPLY. Updates totalMinted tracker.
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
     *      Reverts if any amount is zero or if the cumulative minting would exceed MAX_SUPPLY.
     *      Gas-efficient for distributing tokens to multiple addresses.
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
            if (amount == 0) revert ZeroAmount();

            if (amount > MAX_SUPPLY - minted) revert ExceedsTotalSupply();
            minted += amount;

            _mint(recipient, amount);
            emit AllocationMinted(recipient, amount, allocations[i]);
        }

        totalMinted = minted;
    }

    /**
     * @notice Enables or disables transfer restrictions for the token
     * @dev Only callable by the contract owner. When restrictions are enabled, only whitelisted
     *      addresses can send or receive tokens. Useful for controlling token circulation during
     *      early phases before public trading.
     * @param restricted True to enable restrictions, false to allow unrestricted transfers
     */
    function setTransferRestriction(bool restricted) external onlyOwner {
        transfersRestricted = restricted;
        emit TransferRestrictionUpdated(restricted);
    }

    /**
     * @notice Toggles an account's transfer whitelist status between enabled and disabled
     * @dev Only callable by the contract owner. Flips the current whitelist state for the given
     *      account. Whitelisted accounts can send and receive tokens even when transfer restrictions
     *      are active.
     * @param account Address whose whitelist status will be toggled
     */
    function toggleTransferWhitelist(address account) external onlyOwner {
        bool newStatus = !transferWhitelisted[account];
        transferWhitelisted[account] = newStatus;
        emit TransferWhitelistUpdated(account, newStatus);
    }

    /**
     * @notice Whitelists key protocol contracts to allow them to transfer tokens during restricted periods
     * @dev Only callable by the contract owner. Convenience function for whitelisting multiple protocol
     *      contracts in a single transaction. Zero addresses are safely ignored. Typically used to whitelist
     *      contracts like bonding managers and vesting escrows that need to handle tokens on behalf of users.
     * @param bondingManager Address of the BondingManager contract (zero address skipped)
     * @param vestingEscrow Address of the VestingEscrow contract (zero address skipped)
     */
    function whitelistContracts(
        address bondingManager,
        address vestingEscrow
    ) external onlyOwner {
        if (bondingManager != address(0)) {
            transferWhitelisted[bondingManager] = true;
            emit TransferWhitelistUpdated(bondingManager, true);
        }
        if (vestingEscrow != address(0)) {
            transferWhitelisted[vestingEscrow] = true;
            emit TransferWhitelistUpdated(vestingEscrow, true);
        }
    }

    /**
     * @notice Internal hook that enforces transfer restrictions and updates voting power
     * @dev Overrides ERC20 and ERC20Votes to add transfer restriction logic. Reverts if transfers
     *      are restricted and neither sender nor receiver is whitelisted. Minting (from == 0) and
     *      burning (to == 0) are always allowed regardless of restrictions.
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
}

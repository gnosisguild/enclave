// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { ERC20 } from "@oz/token/ERC20/ERC20.sol";
import {
    ERC20Permit,
    Nonces
} from "@oz/token/ERC20/extensions/ERC20Permit.sol";
import { ERC20Votes } from "@oz/token/ERC20/extensions/ERC20Votes.sol";
import { Ownable } from "@oz/access/Ownable.sol";
import { AccessControl } from "@oz/access/AccessControl.sol";

/**
 * @title EnclaveToken
 */
contract EnclaveToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable,
    AccessControl
{
    // Custom errors to reduce deployment cost and provide descriptive reverts.
    error ZeroAddress();
    error ZeroAmount();
    error ExceedsTotalSupply();
    error ArrayLengthMismatch();
    error TransferNotAllowed();

    /// @dev Maximum supply of the token (18 decimals).
    uint256 public constant TOTAL_SUPPLY = 1_200_000_000e18;

    /// @dev Role allowing accounts to mint new tokens.
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    /// @dev Tracks the amount of tokens minted so far.
    uint256 public totalMinted;

    /// @dev Mapping of addresses allowed to transfer when restrictions are active.
    mapping(address => bool) public transferWhitelisted;

    /// @dev Whether transfers are currently restricted.
    bool public transfersRestricted;

    /// @dev Emitted when tokens are minted as part of an allocation.
    event AllocationMinted(
        address indexed recipient,
        uint256 amount,
        string allocation
    );

    /// @dev Emitted whenever the transfer restriction flag is updated.
    event TransferRestrictionUpdated(bool restricted);

    /// @dev Emitted when an address is added to or removed from the whitelist.
    event TransferWhitelistUpdated(address indexed account, bool whitelisted);

    /**
     * @notice Deploy the Enclave token.
     * @param _owner Address that will initially own the contract and have admin rights.
     */
    constructor(
        address _owner
    ) ERC20("Enclave", "ENCL") ERC20Permit("Enclave") Ownable(_owner) {
        // Grant the deployer all admin roles.
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(MINTER_ROLE, _owner);

        // Initialise state variables.
        totalMinted = 0;
        transfersRestricted = true;
        transferWhitelisted[_owner] = true;

        emit TransferRestrictionUpdated(true);
        emit TransferWhitelistUpdated(_owner, true);
    }

    /**
     * @notice Mint an allocation of tokens to a recipient.
     * @dev Only accounts with the MINTER_ROLE may call this function.
     * @param recipient Address to receive the minted tokens.
     * @param amount Amount of tokens to mint (18 decimals).
     * @param allocation Description of the allocation for off-chain bookkeeping.
     */
    function mintAllocation(
        address recipient,
        uint256 amount,
        string memory allocation
    ) external onlyRole(MINTER_ROLE) {
        if (recipient == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();
        // Ensure we do not exceed the total supply.
        if (totalMinted + amount > TOTAL_SUPPLY) revert ExceedsTotalSupply();

        _mint(recipient, amount);
        totalMinted += amount;
        emit AllocationMinted(recipient, amount, allocation);
    }

    /**
     * @notice Mint multiple allocations in a batch.
     * @dev Only accounts with the MINTER_ROLE may call this function.
     * @param recipients Array of addresses to receive tokens.
     * @param amounts Corresponding array of amounts to mint.
     * @param allocations Array of allocation descriptions.
     */
    function batchMintAllocations(
        address[] memory recipients,
        uint256[] memory amounts,
        string[] memory allocations
    ) external onlyRole(MINTER_ROLE) {
        if (
            recipients.length != amounts.length ||
            amounts.length != allocations.length
        ) revert ArrayLengthMismatch();

        uint256 totalAmount = 0;
        for (uint256 i = 0; i < amounts.length; i++) {
            totalAmount += amounts[i];
        }
        if (totalMinted + totalAmount > TOTAL_SUPPLY)
            revert ExceedsTotalSupply();

        for (uint256 i = 0; i < recipients.length; i++) {
            address rec = recipients[i];
            uint256 amt = amounts[i];
            if (rec == address(0)) revert ZeroAddress();
            if (amt == 0) revert ZeroAmount();

            _mint(rec, amt);
            emit AllocationMinted(rec, amt, allocations[i]);
        }
        totalMinted += totalAmount;
    }

    /**
     * @notice Enable or disable transfer restrictions.
     * @dev Only the owner can toggle this flag.
     * @param restricted Whether transfers should be restricted.
     */
    function setTransferRestriction(bool restricted) external onlyOwner {
        transfersRestricted = restricted;
        emit TransferRestrictionUpdated(restricted);
    }

    /**
     * @notice Add or remove an address from the transfer whitelist.
     * @dev Only the owner may call this.
     * @param account Address whose whitelist status is to be updated.
     * @param whitelisted Whether the address should be whitelisted.
     */
    function setTransferWhitelist(
        address account,
        bool whitelisted
    ) external onlyOwner {
        transferWhitelisted[account] = whitelisted;
        emit TransferWhitelistUpdated(account, whitelisted);
    }

    /**
     * @notice Toggle an account's whitelist status.
     * @dev Only the owner may call this.
     * @param account Address whose whitelist status should be toggled.
     */
    function toggleTransferWhitelist(address account) external onlyOwner {
        bool newStatus = !transferWhitelisted[account];
        transferWhitelisted[account] = newStatus;
        emit TransferWhitelistUpdated(account, newStatus);
    }

    /**
     * @notice Whitelist contracts that are allowed to transfer while restricted.
     * @dev Convenience function for whitelisting middleware contracts.
     * @param bondingManager BondingManager contract to whitelist.
     * @param vestingEscrow VestingEscrow contract to whitelist.
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
     * @dev Override ERC20Votes update hook to enforce transfer restrictions.
     */
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override(ERC20, ERC20Votes) {
        // When transfers are restricted, only whitelisted addresses can send or receive.
        if (from != address(0) && to != address(0) && transfersRestricted) {
            if (!transferWhitelisted[from] && !transferWhitelisted[to])
                revert TransferNotAllowed();
        }
        super._update(from, to, value);
    }

    /**
     * @dev Expose ERC165 interface support via AccessControl.
     */
    function supportsInterface(
        bytes4 interfaceId
    ) public view override(AccessControl) returns (bool) {
        return super.supportsInterface(interfaceId);
    }

    /**
     * @dev Expose permit nonces via both ERC20Permit and OpenZeppelin Nonces.
     */
    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }
}

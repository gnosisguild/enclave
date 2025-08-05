// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Votes.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

contract EnclaveToken is
    ERC20,
    ERC20Permit,
    ERC20Votes,
    Ownable,
    AccessControl
{
    uint256 public constant TOTAL_SUPPLY = 1_200_000_000e18;
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    uint256 public totalMinted;
    mapping(address => bool) public transferWhitelisted;
    bool public transfersRestricted;

    event AllocationMinted(
        address indexed recipient,
        uint256 amount,
        string allocation
    );
    event TransferRestrictionUpdated(bool restricted);
    event TransferWhitelistUpdated(address indexed account, bool whitelisted);

    /**
     * @param _owner Contract owner
     */
    constructor(
        address _owner
    ) ERC20("Enclave", "ENCL") ERC20Permit("Enclave") Ownable(_owner) {
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(MINTER_ROLE, _owner);

        totalMinted = 0;
        transfersRestricted = true;
        transferWhitelisted[_owner] = true;

        emit TransferRestrictionUpdated(true);
        emit TransferWhitelistUpdated(_owner, true);
    }

    /**
     * @param recipient Address to receive tokens
     * @param amount Amount to mint
     * @param allocation Allocation description
     */
    function mintAllocation(
        address recipient,
        uint256 amount,
        string memory allocation
    ) external onlyRole(MINTER_ROLE) {
        require(recipient != address(0), "EnclaveToken: zero address");
        require(amount > 0, "EnclaveToken: zero amount");
        require(
            totalMinted + amount <= TOTAL_SUPPLY,
            "EnclaveToken: exceeds total supply"
        );

        _mint(recipient, amount);
        totalMinted += amount;
        emit AllocationMinted(recipient, amount, allocation);
    }

    /**
     * @param recipients Addresses to receive tokens
     * @param amounts Amounts to mint
     * @param allocations Allocation descriptions
     */
    function batchMintAllocations(
        address[] memory recipients,
        uint256[] memory amounts,
        string[] memory allocations
    ) external onlyRole(MINTER_ROLE) {
        require(
            recipients.length == amounts.length &&
                amounts.length == allocations.length,
            "EnclaveToken: array length mismatch"
        );

        uint256 totalAmount = 0;
        for (uint256 i = 0; i < amounts.length; i++) {
            totalAmount += amounts[i];
        }
        require(
            totalMinted + totalAmount <= TOTAL_SUPPLY,
            "EnclaveToken: exceeds total supply"
        );

        for (uint256 i = 0; i < recipients.length; i++) {
            require(recipients[i] != address(0), "EnclaveToken: zero address");
            require(amounts[i] > 0, "EnclaveToken: zero amount");

            _mint(recipients[i], amounts[i]);
            emit AllocationMinted(recipients[i], amounts[i], allocations[i]);
        }

        totalMinted += totalAmount;
    }

    /**
     * @param restricted Enable/disable transfer restrictions
     */
    function setTransferRestriction(bool restricted) external onlyOwner {
        transfersRestricted = restricted;
        emit TransferRestrictionUpdated(restricted);
    }

    /**
     * @param account Address to whitelist
     * @param whitelisted Whitelist status
     */
    function setTransferWhitelist(
        address account,
        bool whitelisted
    ) external onlyOwner {
        transferWhitelisted[account] = whitelisted;
        emit TransferWhitelistUpdated(account, whitelisted);
    }

    /**
     * @param account Address to toggle whitelist
     */
    function toggleTransferWhitelist(address account) external onlyOwner {
        transferWhitelisted[account] = !transferWhitelisted[account];
        emit TransferWhitelistUpdated(account, transferWhitelisted[account]);
    }

    /**
     * @param bondingManager BondingManager address
     * @param vestingEscrow VestingEscrow address
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
     * @dev Transfer restriction enforcement
     */
    function _update(
        address from,
        address to,
        uint256 value
    ) internal override(ERC20, ERC20Votes) {
        if (from != address(0) && to != address(0) && transfersRestricted) {
            require(
                transferWhitelisted[from] || transferWhitelisted[to],
                "EnclaveToken: transfer not allowed"
            );
        }

        super._update(from, to, value);
    }

    function supportsInterface(
        bytes4 interfaceId
    ) public view override(AccessControl) returns (bool) {
        return super.supportsInterface(interfaceId);
    }

    function nonces(
        address owner
    ) public view override(ERC20Permit, Nonces) returns (uint256) {
        return super.nonces(owner);
    }
}

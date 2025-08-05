// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract EnclaveToken is ERC20, AccessControl, ReentrancyGuard, Ownable {
    uint256 public constant TOTAL_SUPPLY = 1_200_000_000e18;
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    uint256 public totalMinted;
    event AllocationMinted(
        address indexed recipient,
        uint256 amount,
        string allocation
    );
    mapping(address => bool) public transferWhitelisted;
    bool public transfersRestricted;

    event TransferRestrictionUpdated(bool restricted);
    event TransferWhitelistUpdated(address indexed account, bool whitelisted);

    constructor(address _owner) ERC20("Enclave", "ENCL") Ownable(_owner) {
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(MINTER_ROLE, _owner);
        totalMinted = 0;

        transfersRestricted = true;
        transferWhitelisted[_owner] = true;

        emit TransferRestrictionUpdated(true);
        emit TransferWhitelistUpdated(_owner, true);
    }

    function mint(
        address recipient,
        uint256 amount
    ) external onlyRole(MINTER_ROLE) {
        require(totalMinted + amount <= TOTAL_SUPPLY, "Exceeds cap");
        _mint(recipient, amount);
        totalMinted += amount;
    }

    function mintAllocation(
        address recipient,
        uint256 amount,
        string memory allocation
    ) external onlyRole(MINTER_ROLE) {
        require(recipient != address(0), "Zero address");
        require(amount > 0, "Zero amount");
        require(totalMinted + amount <= TOTAL_SUPPLY, "Exceeds supply");

        _mint(recipient, amount);
        totalMinted += amount;

        emit AllocationMinted(recipient, amount, allocation);
    }

    function batchMintAllocations(
        address[] memory recipients,
        uint256[] memory amounts,
        string[] memory allocations
    ) external onlyRole(MINTER_ROLE) {
        require(
            recipients.length == amounts.length &&
                amounts.length == allocations.length,
            "Length mismatch"
        );

        uint256 totalAmount = 0;
        for (uint256 i = 0; i < amounts.length; i++) {
            totalAmount += amounts[i];
        }
        require(totalMinted + totalAmount <= TOTAL_SUPPLY, "Exceeds supply");

        for (uint256 i = 0; i < recipients.length; i++) {
            require(recipients[i] != address(0), "Zero address");
            require(amounts[i] > 0, "Zero amount");
            _mint(recipients[i], amounts[i]);
            emit AllocationMinted(recipients[i], amounts[i], allocations[i]);
        }

        totalMinted += totalAmount;
    }

    function setTransferRestriction(bool restricted) external onlyOwner {
        transfersRestricted = restricted;
        emit TransferRestrictionUpdated(restricted);
    }

    function setTransferWhitelist(
        address account,
        bool whitelisted
    ) external onlyOwner {
        transferWhitelisted[account] = whitelisted;
        emit TransferWhitelistUpdated(account, whitelisted);
    }

    function toggleTransferWhitelist(address account) external onlyOwner {
        transferWhitelisted[account] = !transferWhitelisted[account];
        emit TransferWhitelistUpdated(account, transferWhitelisted[account]);
    }

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

    function _update(
        address from,
        address to,
        uint256 value
    ) internal override {
        if (from != address(0) && to != address(0) && transfersRestricted) {
            require(
                transferWhitelisted[from] || transferWhitelisted[to],
                "Transfer not allowed"
            );
        }

        super._update(from, to, value);
    }
}

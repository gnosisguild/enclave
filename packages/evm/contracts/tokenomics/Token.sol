// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/AccessControl.sol";

contract EnclaveToken is ERC20, AccessControl {
    uint256 public constant TOTAL_SUPPLY = 1_200_000_000e18;
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    uint256 public totalMinted;

    constructor(address _owner) ERC20("Enclave", "ENCL") {
        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(MINTER_ROLE, _owner);
        totalMinted = 0;
    }

    function mint(
        address recipient,
        uint256 amount
    ) external onlyRole(MINTER_ROLE) {
        require(totalMinted + amount <= TOTAL_SUPPLY, "Exceeds cap");
        _mint(recipient, amount);
        totalMinted += amount;
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract EnclaveToken is ERC20 {
    uint256 public constant TOTAL_SUPPLY = 1_200_000_000e18;
    uint256 public totalMinted;

    constructor() ERC20("Enclave", "ENCL") {
        totalMinted = 0;
    }

    function mint(address recipient, uint256 amount) external {
        require(totalMinted + amount <= TOTAL_SUPPLY, "Exceeds cap");
        _mint(recipient, amount);
        totalMinted += amount;
    }
}

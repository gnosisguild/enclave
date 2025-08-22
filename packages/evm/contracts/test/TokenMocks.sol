// SPDX-License-Identifier: MIT
pragma solidity ^0.8.12;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract ENCLToken is ERC20 {
    constructor() ERC20("Enclave Token", "ENCL") {}

    function mint(address account, uint256 amount) public {
        _mint(account, amount);
    }
}

contract USDCToken is ERC20 {
    uint8 private _decimals = 6;
    
    constructor() ERC20("USD Coin", "USDC") {}
    
    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

    function mint(address account, uint256 amount) public {
        _mint(account, amount);
    }
}

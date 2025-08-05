// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract EnclaveToken is ERC20 {
    constructor() ERC20("Enclave", "ENCL") {}
}

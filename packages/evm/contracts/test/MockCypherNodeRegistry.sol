// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICypherNodeRegistry } from "../interfaces/ICypherNodeRegistry.sol";

contract MockCypherNodeRegistry is ICypherNodeRegistry {
    function selectCommittee(uint256, address pool, uint32[2] calldata) external pure returns (bool success) {
        if (pool == address(2)) {
            success = false;
        } else {
            success = true;
        }
    }

    function getCommitteePublicKey(uint256 e3Id) external pure returns (bytes memory) {
        return abi.encodePacked(keccak256(abi.encode(e3Id)));
    }
}

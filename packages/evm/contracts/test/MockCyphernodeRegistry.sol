// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";

contract MockCyphernodeRegistry is ICyphernodeRegistry {
    function selectCommittee(
        uint256,
        address[] memory pools,
        uint32[2] calldata
    ) external pure returns (bool success) {
        if (pools[0] == address(2)) {
            success = false;
        } else {
            success = true;
        }
    }

    function getCommitteePublicKey(uint256 e3Id) external pure returns (bytes memory) {
        return abi.encodePacked(keccak256(abi.encode(e3Id)));
    }
}

contract MockCyphernodeRegistryEmptyKey is ICyphernodeRegistry {
    function selectCommittee(
        uint256,
        address[] memory pools,
        uint32[2] calldata
    ) external pure returns (bool success) {
        if (pools[0] == address(2)) {
            success = false;
        } else {
            success = true;
        }
    }

    function getCommitteePublicKey(uint256) external pure returns (bytes memory) {
        return hex"";
    }
}

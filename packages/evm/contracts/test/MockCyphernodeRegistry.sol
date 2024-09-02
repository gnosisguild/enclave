// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICyphernodeRegistry } from "../interfaces/ICyphernodeRegistry.sol";

contract MockCyphernodeRegistry is ICyphernodeRegistry {
    function requestCommittee(
        uint256,
        address filter,
        uint32[2] calldata
    ) external pure returns (bool success) {
        if (filter == address(2)) {
            success = false;
        } else {
            success = true;
        }
    }

    // solhint-disable no-empty-blocks
    function publishCommittee(
        uint256,
        bytes calldata,
        bytes calldata
    ) external {}

    function committeePublicKey(
        uint256 e3Id
    ) external pure returns (bytes memory) {
        if (e3Id == type(uint256).max) {
            return hex"";
        } else {
            return abi.encodePacked(keccak256(abi.encode(e3Id)));
        }
    }

    function isCyphernodeEligible(address) external pure returns (bool) {
        return false;
    }
}

contract MockCyphernodeRegistryEmptyKey is ICyphernodeRegistry {
    function requestCommittee(
        uint256,
        address filter,
        uint32[2] calldata
    ) external pure returns (bool success) {
        if (filter == address(2)) {
            success = false;
        } else {
            success = true;
        }
    }

    // solhint-disable no-empty-blocks
    function publishCommittee(
        uint256,
        bytes calldata,
        bytes calldata
    ) external {}

    function committeePublicKey(uint256) external pure returns (bytes memory) {
        return hex"";
    }

    function isCyphernodeEligible(address) external pure returns (bool) {
        return false;
    }
}

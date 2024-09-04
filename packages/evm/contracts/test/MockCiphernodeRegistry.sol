// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";

contract MockCiphernodeRegistry is ICiphernodeRegistry {
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

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }
}

contract MockCiphernodeRegistryEmptyKey is ICiphernodeRegistry {
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

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }
}

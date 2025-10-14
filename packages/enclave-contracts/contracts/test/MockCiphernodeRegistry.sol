// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";

contract MockCiphernodeRegistry is ICiphernodeRegistry {
    function requestCommittee(
        uint256,
        uint32[2] calldata
    ) external pure returns (bool success) {
        success = true;
    }

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    function committeePublicKey(uint256 e3Id) external pure returns (bytes32) {
        if (e3Id == type(uint256).max) {
            return bytes32(0);
        } else {
            return keccak256(abi.encode(e3Id));
        }
    }

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address, uint256[] calldata) external pure {}

    function publishCommittee(
        uint256,
        address[] calldata,
        bytes calldata
    ) external pure {} // solhint-disable-line no-empty-blocks

    function getCommittee(
        uint256
    ) external pure returns (ICiphernodeRegistry.Committee memory) {
        address[] memory nodes = new address[](0);
        uint32[2] memory threshold = [uint32(0), uint32(0)];
        return ICiphernodeRegistry.Committee(nodes, threshold, bytes32(0));
    }

    function root() external pure returns (uint256) {
        return 0;
    }

    function rootAt(uint256) external pure returns (uint256) {
        return 0;
    }

    function treeSize() external pure returns (uint256) {
        return 0;
    }

    function getBondingRegistry() external pure returns (address) {
        return address(0);
    }

    // solhint-disable-next-line no-empty-blocks
    function setEnclave(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function setBondingRegistry(address) external pure {}
}

contract MockCiphernodeRegistryEmptyKey is ICiphernodeRegistry {
    function requestCommittee(
        uint256,
        uint32[2] calldata
    ) external pure returns (bool success) {
        success = true;
    }

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    function committeePublicKey(uint256) external pure returns (bytes32) {
        return bytes32(0);
    }

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address, uint256[] calldata) external pure {}

    function publishCommittee(
        uint256,
        address[] calldata,
        bytes calldata
    ) external pure {} // solhint-disable-line no-empty-blocks

    function getCommittee(
        uint256
    ) external pure returns (ICiphernodeRegistry.Committee memory) {
        address[] memory nodes = new address[](0);
        uint32[2] memory threshold = [uint32(0), uint32(0)];
        return ICiphernodeRegistry.Committee(nodes, threshold, bytes32(0));
    }

    function root() external pure returns (uint256) {
        return 0;
    }

    function rootAt(uint256) external pure returns (uint256) {
        return 0;
    }

    function treeSize() external pure returns (uint256) {
        return 0;
    }

    function getBondingRegistry() external pure returns (address) {
        return address(0);
    }

    // solhint-disable-next-line no-empty-blocks
    function setEnclave(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function setBondingRegistry(address) external pure {}
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IRegistryFilter } from "../interfaces/IRegistryFilter.sol";

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

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external {}

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address, uint256[] calldata) external {}

    // solhint-disable no-empty-blocks
    function publishCommittee(
        uint256,
        bytes calldata,
        bytes calldata
    ) external {} // solhint-disable-line no-empty-blocks

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

    function getFilter(uint256) external pure returns (address) {
        return address(0);
    }

    function getCommittee(
        uint256
    ) external pure returns (IRegistryFilter.Committee memory) {
        return
            IRegistryFilter.Committee(
                new address[](0),
                [uint32(0), uint32(0)],
                bytes32(0)
            );
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

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external {}

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address, uint256[] calldata) external {}

    // solhint-disable no-empty-blocks
    function publishCommittee(
        uint256,
        bytes calldata,
        bytes calldata
    ) external {} // solhint-disable-line no-empty-blocks

    function getFilter(uint256) external pure returns (address) {
        return address(0);
    }

    function getCommittee(
        uint256
    ) external pure returns (IRegistryFilter.Committee memory) {
        return
            IRegistryFilter.Committee(
                new address[](0),
                [uint32(0), uint32(0)],
                bytes32(0)
            );
    }

    function committeePublicKey(uint256) external pure returns (bytes32) {
        return bytes32(0);
    }

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IEnclave } from "../interfaces/IEnclave.sol";
import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";

contract MockCiphernodeRegistry is ICiphernodeRegistry {
    /// @notice Configurable committee members per E3 for testing
    mapping(uint256 => address[]) private _committeeNodes;

    /// @notice Set committee members for an E3 (test helper)
    function setCommitteeNodes(
        uint256 e3Id,
        address[] calldata nodes
    ) external {
        delete _committeeNodes[e3Id];
        for (uint256 i = 0; i < nodes.length; i++) {
            _committeeNodes[e3Id].push(nodes[i]);
        }
    }

    function requestCommittee(
        uint256,
        uint256,
        uint32[2] calldata
    ) external pure returns (bool success) {
        success = true;
    }

    function getCommitteeDeadline(uint256) external view returns (uint256) {
        return block.timestamp + 10;
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
        bytes calldata,
        bytes32
    ) external pure {} // solhint-disable-line no-empty-blocks

    function getCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory) {
        return _committeeNodes[e3Id];
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
    function setEnclave(IEnclave) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function setBondingRegistry(IBondingRegistry) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function submitTicket(uint256, uint256) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function finalizeCommittee(uint256) external pure returns (bool) {
        return true;
    }

    // solhint-disable-next-line no-empty-blocks
    function setSortitionSubmissionWindow(uint256) external pure {}

    function isOpen(uint256) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function expelCommitteeMember(
        uint256,
        address,
        bytes32
    ) external pure returns (uint256, uint32) {
        return (0, 0);
    }

    function isCommitteeMemberActive(
        uint256,
        address
    ) external pure returns (bool) {
        return true;
    }

    function getActiveCommitteeNodes(
        uint256
    ) external pure returns (address[] memory) {
        return new address[](0);
    }

    function getActiveCommitteeCount(uint256) external pure returns (uint256) {
        return 0;
    }

    function getCommitteeThreshold(
        uint256
    ) external pure returns (uint32[2] memory) {
        return [uint32(0), uint32(0)];
    }
}

contract MockCiphernodeRegistryEmptyKey is ICiphernodeRegistry {
    error CommitteeNotPublished();

    function requestCommittee(
        uint256,
        uint256,
        uint32[2] calldata
    ) external pure returns (bool success) {
        success = true;
    }

    function getCommitteeDeadline(uint256) external view returns (uint256) {
        return block.timestamp + 10;
    }

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    function committeePublicKey(uint256) external pure returns (bytes32) {
        revert CommitteeNotPublished();
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
        bytes calldata,
        bytes32
    ) external pure {} // solhint-disable-line no-empty-blocks

    function getCommitteeNodes(
        uint256
    ) external pure returns (address[] memory) {
        address[] memory nodes = new address[](0);
        return nodes;
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
    function setEnclave(IEnclave) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function setBondingRegistry(IBondingRegistry) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function setSortitionSubmissionWindow(uint256) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function submitTicket(uint256, uint256) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function finalizeCommittee(uint256) external pure returns (bool) {
        return true;
    }

    function isOpen(uint256) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function expelCommitteeMember(
        uint256,
        address,
        bytes32
    ) external pure returns (uint256, uint32) {
        return (0, 0);
    }

    function isCommitteeMemberActive(
        uint256,
        address
    ) external pure returns (bool) {
        return true;
    }

    function getActiveCommitteeNodes(
        uint256
    ) external pure returns (address[] memory) {
        return new address[](0);
    }

    function getActiveCommitteeCount(uint256) external pure returns (uint256) {
        return 0;
    }

    function getCommitteeThreshold(
        uint256
    ) external pure returns (uint32[2] memory) {
        return [uint32(0), uint32(0)];
    }
}

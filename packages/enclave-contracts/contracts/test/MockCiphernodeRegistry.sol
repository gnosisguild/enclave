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
    mapping(uint256 e3Id => address[] nodes) private _committeeNodes;

    /// @notice Configurable threshold M per E3 for testing
    mapping(uint256 e3Id => uint32 threshold) private _thresholdM;

    /// @notice Configurable lifecycle stage per E3 for testing
    mapping(uint256 e3Id => CommitteeStage stage) private _committeeStage;

    /// @notice Tracks whether a stage was explicitly configured for an E3
    mapping(uint256 e3Id => bool configured) private _stageConfigured;

    /// @notice Configurable committee deadline per E3 for testing
    mapping(uint256 e3Id => uint256 deadline) private _committeeDeadline;

    /// @notice Configurable public key hash per E3 for testing
    mapping(uint256 e3Id => bytes32 hash) private _publicKeyHash;

    /// @notice Tracks whether a public key hash was explicitly configured for an E3
    mapping(uint256 e3Id => bool configured) private _publicKeyHashConfigured;

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

    /// @notice Set the threshold M for an E3 (test helper)
    function setThreshold(uint256 e3Id, uint32 m) external {
        _thresholdM[e3Id] = m;
    }

    /// @notice Set the committee stage for an E3 (test helper)
    function setCommitteeStage(uint256 e3Id, CommitteeStage stage) external {
        _committeeStage[e3Id] = stage;
        _stageConfigured[e3Id] = true;
    }

    /// @notice Set the committee deadline for an E3 (test helper)
    function setCommitteeDeadline(uint256 e3Id, uint256 deadline) external {
        _committeeDeadline[e3Id] = deadline;
    }

    /// @notice Set the published public key hash for an E3 (test helper)
    function setPublicKeyHash(uint256 e3Id, bytes32 publicKeyHash) external {
        _publicKeyHash[e3Id] = publicKeyHash;
        _publicKeyHashConfigured[e3Id] = true;
    }

    function requestCommittee(
        uint256 e3Id,
        uint256,
        uint32[2] calldata threshold
    ) external returns (bool success) {
        _committeeStage[e3Id] = CommitteeStage.Requested;
        _stageConfigured[e3Id] = true;
        _committeeDeadline[e3Id] = block.timestamp + 10;
        _thresholdM[e3Id] = threshold[0];
        _publicKeyHash[e3Id] = bytes32(0);
        _publicKeyHashConfigured[e3Id] = true;
        success = true;
    }

    function getCommitteeDeadline(
        uint256 e3Id
    ) external view returns (uint256) {
        uint256 deadline = _committeeDeadline[e3Id];
        return deadline == 0 ? block.timestamp + 10 : deadline;
    }

    function isEnabled(address) external pure returns (bool) {
        return true;
    }

    function committeePublicKey(uint256 e3Id) external view returns (bytes32) {
        bytes32 publicKeyHash = _publicKeyHashConfigured[e3Id]
            ? _publicKeyHash[e3Id]
            : bytes32(0);
        if (publicKeyHash == bytes32(0)) {
            revert CommitteeNotPublished();
        }
        return publicKeyHash;
    }

    function publicKeyHashes(uint256 e3Id) external view returns (bytes32) {
        return
            _publicKeyHashConfigured[e3Id] ? _publicKeyHash[e3Id] : bytes32(0);
    }

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address) external pure {}

    function publishCommittee(
        uint256 e3Id,
        address[] calldata,
        bytes calldata publicKey,
        bytes calldata,
        bytes calldata
    ) external {
        _committeeStage[e3Id] = CommitteeStage.Finalized;
        _stageConfigured[e3Id] = true;
        _publicKeyHash[e3Id] = keccak256(publicKey);
        _publicKeyHashConfigured[e3Id] = true;
    }

    function getCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory nodes, uint256[] memory scores) {
        nodes = _committeeNodes[e3Id];
        scores = new uint256[](nodes.length);
        for (uint256 i = 0; i < nodes.length; i++) {
            scores[i] = uint256(keccak256(abi.encode(nodes[i])));
        }
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
    function finalizeCommittee(uint256 e3Id) external returns (bool) {
        _committeeStage[e3Id] = CommitteeStage.Finalized;
        _stageConfigured[e3Id] = true;
        return true;
    }

    function getCommitteeStage(
        uint256 e3Id
    ) external view returns (ICiphernodeRegistry.CommitteeStage) {
        return
            _stageConfigured[e3Id]
                ? _committeeStage[e3Id]
                : ICiphernodeRegistry.CommitteeStage.None;
    }

    function sortitionSubmissionWindow() external pure returns (uint256) {
        return 0;
    }

    // solhint-disable-next-line no-empty-blocks
    function setSortitionSubmissionWindow(uint256) external pure {}

    function isOpen(uint256) external pure returns (bool) {
        return false;
    }

    function expelCommitteeMember(
        uint256 e3Id,
        address member,
        bytes32
    ) external returns (uint256, uint32) {
        address[] storage nodes = _committeeNodes[e3Id];
        for (uint256 i = 0; i < nodes.length; i++) {
            if (nodes[i] == member) {
                nodes[i] = nodes[nodes.length - 1];
                nodes.pop();
                break;
            }
        }
        uint32 m = _thresholdM[e3Id];
        return (nodes.length, m);
    }

    function isCommitteeMemberActive(
        uint256 e3Id,
        address node
    ) external view returns (bool) {
        address[] storage nodes = _committeeNodes[e3Id];
        for (uint256 i = 0; i < nodes.length; i++) {
            if (nodes[i] == node) return true;
        }
        return false;
    }

    function isCommitteeMember(
        uint256 e3Id,
        address node
    ) external view returns (bool) {
        address[] storage nodes = _committeeNodes[e3Id];
        for (uint256 i = 0; i < nodes.length; i++) {
            if (nodes[i] == node) return true;
        }
        return false;
    }

    function getActiveCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory nodes) {
        nodes = _committeeNodes[e3Id];
    }

    function getCommitteeViability(
        uint256 e3Id
    ) external view returns (uint256, uint32, uint32, bool) {
        uint32 m = _thresholdM[e3Id];
        uint32 n = uint32(_committeeNodes[e3Id].length);
        return (n, m, n, n >= m && m > 0);
    }
}

contract MockCiphernodeRegistryEmptyKey is ICiphernodeRegistry {
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

    function publicKeyHashes(uint256) external pure returns (bytes32) {
        return bytes32(0);
    }

    function isCiphernodeEligible(address) external pure returns (bool) {
        return false;
    }

    // solhint-disable-next-line no-empty-blocks
    function addCiphernode(address) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function removeCiphernode(address) external pure {}

    function publishCommittee(
        uint256,
        address[] calldata,
        bytes calldata,
        bytes calldata,
        bytes calldata
    ) external pure {} // solhint-disable-line no-empty-blocks

    function getCommitteeNodes(
        uint256
    ) external pure returns (address[] memory nodes, uint256[] memory scores) {
        nodes = new address[](0);
        scores = new uint256[](0);
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

    function sortitionSubmissionWindow() external pure returns (uint256) {
        return 0;
    }

    // solhint-disable-next-line no-empty-blocks
    function setSortitionSubmissionWindow(uint256) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function submitTicket(uint256, uint256) external pure {}

    // solhint-disable-next-line no-empty-blocks
    function finalizeCommittee(uint256) external pure returns (bool) {
        return true;
    }

    function getCommitteeStage(
        uint256
    ) external pure returns (ICiphernodeRegistry.CommitteeStage) {
        return ICiphernodeRegistry.CommitteeStage.Finalized;
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
        return false;
    }

    function isCommitteeMember(uint256, address) external pure returns (bool) {
        return false;
    }

    function getActiveCommitteeNodes(
        uint256
    ) external pure returns (address[] memory) {
        return new address[](0);
    }

    function getCommitteeViability(
        uint256
    ) external pure returns (uint256, uint32, uint32, bool) {
        return (0, 0, 0, false);
    }
}

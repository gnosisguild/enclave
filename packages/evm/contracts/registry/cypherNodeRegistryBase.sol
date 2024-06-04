// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { ICypherNodeRegistry } from "../interfaces/ICypherNodeRegistry.sol";

abstract contract CypherNodeRegistryBase is ICypherNodeRegistry {
    function selectCommittee(
        uint256 e3Id,
        uint256 poolId,
        uint32[2] calldata threshold
    ) external returns (bool success) {}

    function getCommitteePublicKey(uint256 e3Id) external view returns (bytes memory publicKey) {}
}

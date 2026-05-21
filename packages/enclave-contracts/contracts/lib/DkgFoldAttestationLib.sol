// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import { ECDSA } from "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";
import {
    MessageHashUtils
} from "@openzeppelin/contracts/utils/cryptography/MessageHashUtils.sol";

/**
 * @title DkgFoldAttestationLib
 * @notice EIP-712-style digests for per-node DKG fold attestations.
 * @dev Must stay aligned with `DkgFoldAttestationPayload::typehash()` in
 *      `crates/events/src/enclave_event/dkg_fold_attestation.rs`.
 */
library DkgFoldAttestationLib {
    /// @dev `keccak256("DkgFoldAttestation(uint256 chainId,uint256 e3Id,uint256 partyId,bytes32 skAggCommit,bytes32 esmAggCommit)")`
    bytes32 public constant TYPEHASH =
        keccak256(
            "DkgFoldAttestation(uint256 chainId,uint256 e3Id,uint256 partyId,bytes32 skAggCommit,bytes32 esmAggCommit)"
        );

    /// @notice One node's signed fold output.
    struct Attestation {
        uint256 partyId;
        bytes32 skAggCommit;
        bytes32 esmAggCommit;
        bytes signature;
    }

    /// @notice Maps sortition `partyId` to the operator that produced the fold.
    struct PartySlotBinding {
        uint256 partyId;
        address node;
    }

    /// @notice `personal_sign` digest for a fold attestation (EIP-191 applied by callers).
    function digest(
        uint256 chainId,
        uint256 e3Id,
        uint256 partyId,
        bytes32 skAggCommit,
        bytes32 esmAggCommit
    ) internal pure returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    TYPEHASH,
                    chainId,
                    e3Id,
                    partyId,
                    skAggCommit,
                    esmAggCommit
                )
            );
    }

    /// @notice Recover the signer for a fold attestation.
    function recoverSigner(
        uint256 chainId,
        uint256 e3Id,
        Attestation memory attestation
    ) internal pure returns (address) {
        bytes32 structHash = digest(
            chainId,
            e3Id,
            attestation.partyId,
            attestation.skAggCommit,
            attestation.esmAggCommit
        );
        bytes32 ethSignedHash = MessageHashUtils.toEthSignedMessageHash(
            structHash
        );
        return ECDSA.recover(ethSignedHash, attestation.signature);
    }
}

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import {
    IDkgFoldAttestationVerifier
} from "../interfaces/IDkgFoldAttestationVerifier.sol";
import { DkgFoldAttestationLib } from "../lib/DkgFoldAttestationLib.sol";

/**
 * @title DkgFoldAttestationVerifier
 * @notice Stateless verifier for DKG fold attestations at committee publication.
 */
contract DkgFoldAttestationVerifier is IDkgFoldAttestationVerifier {
    struct BundleData {
        bytes32[] publicInputs;
        DkgFoldAttestationLib.Attestation[] attestations;
        DkgFoldAttestationLib.PartySlotBinding[] bindings;
        uint256 h;
    }

    /// @inheritdoc IDkgFoldAttestationVerifier
    function verify(
        address registry,
        uint256 chainId,
        uint256 e3Id,
        bytes calldata proof,
        bytes calldata dkgAttestationBundle
    )
        external
        view
        returns (
            uint256[] memory partyIds,
            bytes32[] memory skAggCommits,
            bytes32[] memory esmAggCommits
        )
    {
        BundleData memory data = _loadBundle(proof, dkgAttestationBundle);
        return _fillAnchors(registry, chainId, e3Id, data);
    }

    function _loadBundle(
        bytes calldata proof,
        bytes calldata dkgAttestationBundle
    ) private pure returns (BundleData memory data) {
        (, data.publicInputs) = abi.decode(proof, (bytes, bytes32[]));
        data.h = _honestPartyCount(data.publicInputs);
        (data.attestations, data.bindings) = abi.decode(
            dkgAttestationBundle,
            (
                DkgFoldAttestationLib.Attestation[],
                DkgFoldAttestationLib.PartySlotBinding[]
            )
        );
        if (
            data.attestations.length != data.bindings.length ||
            data.bindings.length != data.h
        ) {
            revert ICiphernodeRegistry.AttestationBindingCountMismatch();
        }
    }

    function _fillAnchors(
        address registry,
        uint256 chainId,
        uint256 e3Id,
        BundleData memory data
    )
        private
        view
        returns (
            uint256[] memory partyIdsOut,
            bytes32[] memory skAggOut,
            bytes32[] memory esmAggOut
        )
    {
        partyIdsOut = new uint256[](data.h);
        skAggOut = new bytes32[](data.h);
        esmAggOut = new bytes32[](data.h);

        for (uint256 i = 0; i < data.h; i++) {
            _applyBinding(
                registry,
                chainId,
                e3Id,
                data,
                i,
                partyIdsOut,
                skAggOut,
                esmAggOut
            );
        }
    }

    function _applyBinding(
        address registry,
        uint256 chainId,
        uint256 e3Id,
        BundleData memory data,
        uint256 i,
        uint256[] memory partyIdsOut,
        bytes32[] memory skAggOut,
        bytes32[] memory esmAggOut
    ) private view {
        if (i > 0) {
            require(
                data.bindings[i].partyId > data.bindings[i - 1].partyId,
                ICiphernodeRegistry.InvalidFoldAttestation()
            );
        }

        (uint256 slot, bytes32 sk, bytes32 esm) = _verifyBinding(
            registry,
            chainId,
            e3Id,
            data,
            data.bindings[i]
        );

        partyIdsOut[slot] = data.bindings[i].partyId;
        skAggOut[slot] = sk;
        esmAggOut[slot] = esm;
    }

    function _honestPartyCount(
        bytes32[] memory publicInputs
    ) private pure returns (uint256 h) {
        require(
            publicInputs.length >= 6 && (publicInputs.length - 6) % 3 == 0,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );
        h = (publicInputs.length - 6) / 3;
    }

    function _verifyBinding(
        address registry,
        uint256 chainId,
        uint256 e3Id,
        BundleData memory data,
        DkgFoldAttestationLib.PartySlotBinding memory binding
    ) private view returns (uint256 slot, bytes32 skCommit, bytes32 esmCommit) {
        // Structural check: the binding's `node` must be the operator
        // registered at `topNodes[partyId]` on chain. This prevents an
        // aggregator from reassigning a node's signed attestation to a
        // different slot (e.g. claiming the operator at `topNodes[0]` is
        // party 1) even if the operator cooperated by signing with the
        // wrong `partyId`. Combined with the `ecrecover` check below, the
        // attestation is bound to *both* the right address and the right slot.
        require(
            ICiphernodeRegistry(registry).getCommitteeNodeAt(
                e3Id,
                binding.partyId
            ) == binding.node,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );
        require(
            ICiphernodeRegistry(registry).isCommitteeMemberActive(
                e3Id,
                binding.node
            ),
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        DkgFoldAttestationLib.Attestation memory att = _findAttestation(
            data.attestations,
            binding.partyId
        );

        address signer = DkgFoldAttestationLib.recoverSigner(
            chainId,
            address(this),
            e3Id,
            att
        );
        require(
            signer == binding.node,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        uint256 partyIdOffset = 2;
        uint256 skOffset = 5 + data.h;
        uint256 esmOffset = 5 + (2 * data.h);

        slot = _partySlot(
            data.publicInputs,
            partyIdOffset,
            data.h,
            binding.partyId
        );

        require(
            data.publicInputs[skOffset + slot] == att.skAggCommit &&
                data.publicInputs[esmOffset + slot] == att.esmAggCommit,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        return (slot, att.skAggCommit, att.esmAggCommit);
    }

    function _findAttestation(
        DkgFoldAttestationLib.Attestation[] memory attestations,
        uint256 partyId
    ) private pure returns (DkgFoldAttestationLib.Attestation memory att) {
        for (uint256 j = 0; j < attestations.length; j++) {
            if (attestations[j].partyId == partyId) {
                return attestations[j];
            }
        }
        revert ICiphernodeRegistry.InvalidFoldAttestation();
    }

    function _partySlot(
        bytes32[] memory publicInputs,
        uint256 partyIdOffset,
        uint256 h,
        uint256 partyId
    ) private pure returns (uint256 slot) {
        for (uint256 k = 0; k < h; k++) {
            if (uint256(publicInputs[partyIdOffset + k]) == partyId) {
                return k;
            }
        }
        revert ICiphernodeRegistry.PartyIdNotInProof();
    }
}

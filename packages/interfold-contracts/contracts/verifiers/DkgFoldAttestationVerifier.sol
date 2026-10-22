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
        uint256 n;
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
            bytes32[] memory prfKeyCommits
        )
    {
        BundleData memory data = _loadBundle(
            registry,
            e3Id,
            proof,
            dkgAttestationBundle
        );
        return _fillAnchors(registry, chainId, e3Id, data);
    }

    function _loadBundle(
        address registry,
        uint256 e3Id,
        bytes calldata proof,
        bytes calldata dkgAttestationBundle
    ) private view returns (BundleData memory data) {
        try this.decodeProofPublicInputs(proof) returns (
            bytes32[] memory publicInputs
        ) {
            data.publicInputs = publicInputs;
        } catch {
            revert ICiphernodeRegistry.InvalidFoldAttestation();
        }
        data.n = ICiphernodeRegistry(registry).getCommitteeNodes(e3Id).length;
        data.h = _honestPartyCount(data.publicInputs, data.n);
        // Defense in depth: require the public-inputs `partyId` slots
        // (`publicInputs[2..2+h]`) to be strictly ascending. The zk circuit
        // already enforces this, but rejecting duplicates here prevents two
        // bindings from resolving to the same slot in `_partySlot` (which
        // would silently overwrite each other in `partyIdsOut` etc.).
        for (uint256 k = 1; k < data.h; k++) {
            require(
                uint256(data.publicInputs[2 + k]) >
                    uint256(data.publicInputs[2 + k - 1]),
                ICiphernodeRegistry.InvalidFoldAttestation()
            );
        }
        try this.decodeAttestationBundle(dkgAttestationBundle) returns (
            DkgFoldAttestationLib.Attestation[] memory attestations,
            DkgFoldAttestationLib.PartySlotBinding[] memory bindings
        ) {
            data.attestations = attestations;
            data.bindings = bindings;
        } catch {
            revert ICiphernodeRegistry.InvalidFoldAttestation();
        }
        if (
            data.attestations.length != data.bindings.length ||
            data.bindings.length != data.h
        ) {
            revert ICiphernodeRegistry.AttestationBindingCountMismatch();
        }
    }

    /// @notice Exposed only for guarded decode in `_loadBundle`.
    function decodeProofPublicInputs(
        bytes calldata proof
    ) external pure returns (bytes32[] memory publicInputs) {
        (, publicInputs) = abi.decode(proof, (bytes, bytes32[]));
    }

    /// @notice Exposed only for guarded decode in `_loadBundle`.
    function decodeAttestationBundle(
        bytes calldata dkgAttestationBundle
    )
        external
        pure
        returns (
            DkgFoldAttestationLib.Attestation[] memory attestations,
            DkgFoldAttestationLib.PartySlotBinding[] memory bindings
        )
    {
        (attestations, bindings) = abi.decode(
            dkgAttestationBundle,
            (
                DkgFoldAttestationLib.Attestation[],
                DkgFoldAttestationLib.PartySlotBinding[]
            )
        );
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
            bytes32[] memory prfKeysOut
        )
    {
        partyIdsOut = new uint256[](data.h);
        skAggOut = new bytes32[](data.h);
        prfKeysOut = new bytes32[](data.h * data.n);

        for (uint256 i = 0; i < data.h; i++) {
            _applyBinding(
                registry,
                chainId,
                e3Id,
                data,
                i,
                partyIdsOut,
                skAggOut,
                prfKeysOut
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
        bytes32[] memory prfKeysOut
    ) private view {
        if (i > 0) {
            require(
                data.bindings[i].partyId > data.bindings[i - 1].partyId,
                ICiphernodeRegistry.InvalidFoldAttestation()
            );
            require(
                data.attestations[i].partyId > data.attestations[i - 1].partyId,
                ICiphernodeRegistry.InvalidFoldAttestation()
            );
        }

        (uint256 slot, bytes32 sk) = _verifyBinding(
            registry,
            chainId,
            e3Id,
            data,
            data.bindings[i],
            data.attestations[i]
        );

        partyIdsOut[slot] = data.bindings[i].partyId;
        skAggOut[slot] = sk;
        for (uint256 peer; peer < data.n; ++peer) {
            prfKeysOut[slot * data.n + peer] = data.publicInputs[
                6 + 2 * data.h + slot * data.n + peer
            ];
        }
    }

    function _honestPartyCount(
        bytes32[] memory publicInputs,
        uint256 n
    ) private pure returns (uint256 h) {
        require(
            n > 0 &&
                publicInputs.length >= 6 &&
                (publicInputs.length - 6) % (n + 2) == 0,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );
        h = (publicInputs.length - 6) / (n + 2);
        // Defense in depth: the BFV pk-verifier already rejects `h == 0`, but
        // a zero-honest-party proof would otherwise pass this verifier with no
        // attestations to check and write empty anchors to the registry.
        require(h > 0 && h <= n, ICiphernodeRegistry.InvalidFoldAttestation());
    }

    function _verifyBinding(
        address registry,
        uint256 chainId,
        uint256 e3Id,
        BundleData memory data,
        DkgFoldAttestationLib.PartySlotBinding memory binding,
        DkgFoldAttestationLib.Attestation memory att
    ) private view returns (uint256 slot, bytes32 skCommit) {
        // Canonical-slot binding: `binding.node` must equal the canonical
        // operator at index `partyId` of the finalized committee — i.e.
        // `canonicalCommitteeNodeAt(e3Id, partyId)`, which returns
        // `topNodes[partyId]` in address-ascending order. `partyId` is the
        // canonical sortition slot id, so this rejects any binding pointing
        // to an operator who is not the canonical occupant of that slot,
        // even one who is otherwise an active committee member. Combined
        // with the EIP-712 `ecrecover` check below, the attestation is
        // bound to *both* the right address and the right canonical slot.
        require(
            ICiphernodeRegistry(registry).canonicalCommitteeNodeAt(
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

        require(
            att.partyId == binding.partyId,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        address signer = DkgFoldAttestationLib.recoverSigner(
            chainId,
            address(this),
            registry,
            e3Id,
            att
        );
        require(
            signer == binding.node,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        uint256 partyIdOffset = 2;
        uint256 skOffset = 5 + data.h;

        slot = _partySlot(
            data.publicInputs,
            partyIdOffset,
            data.h,
            binding.partyId
        );

        bytes32[] memory keys = new bytes32[](data.n);
        for (uint256 peer; peer < data.n; ++peer) {
            keys[peer] = data.publicInputs[
                6 + 2 * data.h + slot * data.n + peer
            ];
        }
        require(
            data.publicInputs[skOffset + slot] == att.skAggCommit &&
                keccak256(abi.encodePacked(keys)) == att.prfKeyCommitsHash,
            ICiphernodeRegistry.InvalidFoldAttestation()
        );

        return (slot, att.skAggCommit);
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

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave } from "../interfaces/IEnclave.sol";

/**
 * @title EnclavePricing
 * @notice External library extracted from {Enclave} to keep its deployed
 *         runtime bytecode under the EIP-170 24,576-byte cap.
 *
 *         All functions are pure validation / fee-quote math. They are
 *         declared `external` so Solidity emits a linked library DELEGATECALL
 *         site at each call instead of inlining the bytes into Enclave.
 *
 *         Behaviour and revert selectors match the inlined originals: typed
 *         errors are imported from {IEnclave} so off-chain
 *         `revertedWithCustomError` lookups against the Enclave ABI continue
 *         to resolve.
 */
library EnclavePricing {
    uint16 internal constant BPS_BASE = 10000;
    uint16 internal constant MAX_PROTOCOL_SHARE_BPS = 5_000;
    uint16 internal constant MAX_MARGIN_BPS = 5_000;
    uint32 internal constant MAX_COMMITTEE_SIZE = 256;

    /// @notice Writes the default {IEnclave.PricingConfig} directly to
    ///         the linked {Enclave} storage starting at slot 24. Called
    ///         via DELEGATECALL from {Enclave.initialize}, so SSTORE
    ///         targets the caller's storage. Hosted in the library so
    ///         the 15-field literal stays out of Enclave runtime bytecode
    ///         (EIP-170 24,576-byte cap).
    /// @dev    Slot map for `_pricingConfig` (struct field order in
    ///         {IEnclave.PricingConfig}):
    ///           24: keyGenFixedPerNode
    ///           25: keyGenPerEncryptionProof
    ///           26: coordinationPerPair
    ///           27: availabilityPerNodePerSec
    ///           28: decryptionPerNode
    ///           29: publicationBase
    ///           30: verificationPerProof
    ///           31: packed { protocolTreasury(20) | marginBps(2) |
    ///               protocolShareBps(2) | dkgUtilizationBps(2) |
    ///               computeUtilizationBps(2) | decryptUtilizationBps(2) }
    ///           32: packed { minCommitteeSize(4) | minThreshold(4) }
    ///         The contract storage layout snapshot in
    ///         `audits/storage-layouts/Enclave-v1.json` MUST keep these
    ///         slots stable; any storage reordering requires updating the
    ///         constants below.
    function applyDefaultPricingConfig() external {
        // Packed slot 31:
        //   marginBps           = 1500  << 160
        //   protocolShareBps    =    0  << 176
        //   dkgUtilizationBps   = 2500  << 192
        //   computeUtilizationBps = 5000 << 208
        //   decryptUtilizationBps = 2500 << 224
        // protocolTreasury (low 160 bits) and trailing padding are zero.
        uint256 slot31 = (uint256(1500) << 160) |
            (uint256(2500) << 192) |
            (uint256(5000) << 208) |
            (uint256(2500) << 224);
        assembly {
            sstore(24, 100000) // keyGenFixedPerNode      = 0.10 USDC
            sstore(25, 50000) // keyGenPerEncryptionProof = 0.05 USDC
            sstore(26, 10000) // coordinationPerPair      = 0.01 USDC
            sstore(27, 50) // availabilityPerNodePerSec   = 0.00005 USDC
            sstore(28, 300000) // decryptionPerNode       = 0.30 USDC
            sstore(29, 1000000) // publicationBase        = 1.00 USDC
            sstore(30, 5000) // verificationPerProof      = 0.005 USDC
            sstore(31, slot31)
            // slot 32 (minCommitteeSize | minThreshold) stays zero.
        }
    }

    /// @notice Returns the default {IEnclave.PricingConfig} applied by
    ///         {Enclave.initialize}. Hosted in the external library so the
    ///         15-field literal stays out of the Enclave runtime bytecode
    ///         (EIP-170 24,576-byte cap).
    function defaultPricingConfig()
        external
        pure
        returns (IEnclave.PricingConfig memory cfg)
    {
        cfg.keyGenFixedPerNode = 100000; // 0.10 USDC
        cfg.keyGenPerEncryptionProof = 50000; // 0.05 USDC
        cfg.coordinationPerPair = 10000; // 0.01 USDC
        cfg.availabilityPerNodePerSec = 50; // 0.00005 USDC
        cfg.decryptionPerNode = 300000; // 0.30 USDC
        cfg.publicationBase = 1000000; // 1.00 USDC
        cfg.verificationPerProof = 5000; // 0.005 USDC
        cfg.marginBps = 1500; // 15%
        cfg.dkgUtilizationBps = 2500; // 25%
        cfg.computeUtilizationBps = 5000; // 50%
        cfg.decryptUtilizationBps = 2500; // 25%
        // protocolTreasury, protocolShareBps, minCommitteeSize, minThreshold
        // remain zero by default and use the struct zero-initialization.
    }

    /// @notice Mirrors the four validation gates at the top of
    ///         {Enclave.publishCiphertextOutput}.
    /// @param current  ABI-encoded as `uint8` to avoid qualified enum names in
    ///                 the library ABI (ethers v6 rejects `IEnclave.E3Stage`).
    function validatePublishCiphertext(
        uint256 e3Id,
        uint8 current,
        uint256 computeDeadline,
        uint256 inputWindowEnd,
        bytes32 ciphertextOutput,
        uint256 nowTs
    ) external pure {
        IEnclave.E3Stage stage = IEnclave.E3Stage(current);
        if (stage != IEnclave.E3Stage.KeyPublished)
            revert IEnclave.InvalidStage(
                e3Id,
                IEnclave.E3Stage.KeyPublished,
                stage
            );
        if (computeDeadline < nowTs)
            revert IEnclave.CommitteeDutiesCompleted(e3Id, computeDeadline);
        if (nowTs < inputWindowEnd)
            revert IEnclave.InputDeadlineNotReached(e3Id, inputWindowEnd);
        if (ciphertextOutput != bytes32(0))
            revert IEnclave.CiphertextOutputAlreadyPublished(e3Id);
    }

    /// @notice Mirrors the three stage-precondition reverts at the top of
    ///         {Enclave.markE3Failed} and {Enclave._markE3FailedWithReason}.
    /// @param current  ABI-encoded as `uint8` to avoid qualified enum names in
    ///                 the library ABI (ethers v6 rejects `IEnclave.E3Stage`).
    function validateMarkFailedStage(
        uint256 e3Id,
        uint8 current
    ) external pure {
        IEnclave.E3Stage stage = IEnclave.E3Stage(current);
        if (stage == IEnclave.E3Stage.None)
            revert IEnclave.InvalidStage(
                e3Id,
                IEnclave.E3Stage.Requested,
                stage
            );
        if (stage == IEnclave.E3Stage.Complete)
            revert IEnclave.E3AlreadyComplete(e3Id);
        if (stage == IEnclave.E3Stage.Failed)
            revert IEnclave.E3AlreadyFailed(e3Id);
    }

    /// @notice Mirrors the threshold / min-size gates at the top of
    ///         {Enclave.getE3Quote} (post param-set existence check).
    /// @param committeeSize  ABI-encoded as `uint8` to avoid qualified enum
    ///                       names in the library ABI (ethers v6 rejects
    ///                       `IEnclave.CommitteeSize`).
    function validateQuoteThresholds(
        uint32[2] memory threshold,
        uint8 committeeSize,
        uint32 minCommitteeSize,
        uint32 minThreshold
    ) external pure {
        IEnclave.CommitteeSize size = IEnclave.CommitteeSize(committeeSize);
        if (threshold[1] == 0) revert IEnclave.CommitteeSizeNotConfigured(size);
        if (minCommitteeSize > 0 && threshold[1] < minCommitteeSize)
            revert IEnclave.CommitteeSizeTooSmall(size);
        if (minThreshold > 0 && threshold[0] < minThreshold)
            revert IEnclave.ThresholdTooSmall(threshold[0]);
    }

    /// @notice Mirrors {Enclave._setTimeoutConfig} validation.
    function validateTimeoutConfig(
        IEnclave.E3TimeoutConfig calldata config,
        uint256 maxTimeoutWindow
    ) external pure {
        if (config.dkgWindow == 0 || config.dkgWindow > maxTimeoutWindow)
            revert IEnclave.InvalidTimeoutWindow();
        if (
            config.computeWindow == 0 || config.computeWindow > maxTimeoutWindow
        ) revert IEnclave.InvalidTimeoutWindow();
        if (
            config.decryptionWindow == 0 ||
            config.decryptionWindow > maxTimeoutWindow
        ) revert IEnclave.InvalidTimeoutWindow();
    }

    /// @notice Mirrors {Enclave.setCommitteeThresholds} validation. The
    ///         caller still writes the mapping to preserve storage layout.
    function validateCommitteeThresholds(
        uint32[2] calldata threshold,
        uint32 minCommitteeSize,
        uint32 minThreshold
    ) external pure {
        if (threshold[0] == 0 || threshold[1] < threshold[0])
            revert IEnclave.InvalidThresholdValues();
        // Hard cap on configured committee size to bound on-chain loops
        // (sortition, reward distribution) against governance misconfiguration.
        if (threshold[1] > MAX_COMMITTEE_SIZE)
            revert IEnclave.InvalidThresholdValues();
        if (minCommitteeSize > 0 && threshold[1] < minCommitteeSize)
            revert IEnclave.BelowMinCommitteeSize(
                threshold[1],
                minCommitteeSize
            );
        if (minThreshold > 0 && threshold[0] < minThreshold)
            revert IEnclave.BelowMinThreshold(threshold[0], minThreshold);
    }

    /// @notice Mirrors the input-window / duration gates at the top
    ///         of {Enclave.request}. Reverts with the same selectors so off-
    ///         chain `revertedWithCustomError(enclave, ...)` lookups keep
    ///         working.
    /// @param inputWindow      `requestParams.inputWindow` ([start, end]).
    /// @param nowTs            `block.timestamp` from the caller.
    /// @param computeWindow    `_timeoutConfig.computeWindow`.
    /// @param decryptionWindow `_timeoutConfig.decryptionWindow`.
    /// @param maxDuration      The Enclave-wide upper bound.
    /// @param quotedFee        Fee returned by {EnclavePricing.quote}.
    function validateRequest(
        uint256[2] calldata inputWindow,
        uint256 nowTs,
        uint256 computeWindow,
        uint256 decryptionWindow,
        uint256 maxDuration,
        uint256 quotedFee
    ) external pure {
        if (inputWindow[0] < nowTs)
            revert IEnclave.InvalidInputDeadlineStart(inputWindow[0]);
        if (inputWindow[1] < inputWindow[0])
            revert IEnclave.InvalidInputDeadlineEnd(inputWindow[1]);
        uint256 totalDuration = inputWindow[1] -
            nowTs +
            computeWindow +
            decryptionWindow;
        if (totalDuration >= maxDuration)
            revert IEnclave.InvalidDuration(totalDuration);
    }

    /// @notice Mirrors {Enclave.setPricingConfig} validation.
    function validatePricingConfig(
        IEnclave.PricingConfig calldata config
    ) external pure {
        if (config.marginBps > MAX_MARGIN_BPS)
            revert IEnclave.BpsExceedsMax(config.marginBps);
        if (config.protocolShareBps > MAX_PROTOCOL_SHARE_BPS)
            revert IEnclave.BpsExceedsMax(config.protocolShareBps);
        if (config.dkgUtilizationBps > BPS_BASE)
            revert IEnclave.UtilizationBpsExceedsMax(config.dkgUtilizationBps);
        if (config.computeUtilizationBps > BPS_BASE)
            revert IEnclave.UtilizationBpsExceedsMax(
                config.computeUtilizationBps
            );
        if (config.decryptUtilizationBps > BPS_BASE)
            revert IEnclave.UtilizationBpsExceedsMax(
                config.decryptUtilizationBps
            );
        if (
            config.protocolShareBps != 0 &&
            config.protocolTreasury == address(0)
        ) revert IEnclave.TreasuryRequired();
        if (config.minCommitteeSize < config.minThreshold)
            revert IEnclave.MinSizeBelowMinThreshold();
    }

    /// @notice Splits `cnAmount` equally across `n` slots, sweeping any
    ///         integer-division dust into a slot chosen by `e3Id % n`.
    ///         Matches the original {Enclave._computeNodeAmounts}.
    function computeNodeAmounts(
        uint256 cnAmount,
        uint256 n,
        uint256 e3Id
    ) external pure returns (uint256[] memory amounts) {
        amounts = new uint256[](n);
        uint256 per = cnAmount / n;
        for (uint256 i = 0; i < n; i++) amounts[i] = per;
        uint256 dust = cnAmount - per * n;
        if (dust > 0) amounts[e3Id % n] += dust;
    }

    /// @notice Pure fee quote math. The caller (Enclave) is responsible for
    ///         loading the per-call inputs and gating on min-committee / min-
    ///         threshold (so we keep the original {CommitteeSize} discriminator
    ///         in revert data).
    /// @param pc                  Snapshot of `_pricingConfig`.
    /// @param tc                  Snapshot of `_timeoutConfig`.
    /// @param sortitionWindow     Result of `ciphernodeRegistry.sortitionSubmissionWindow()`.
    /// @param threshold           `[quorum, total]` resolved from `committeeThresholds`.
    /// @param inputWindowStart    `requestParams.inputWindow[0]`.
    /// @param inputWindowEnd      `requestParams.inputWindow[1]`.
    function quote(
        IEnclave.PricingConfig calldata pc,
        IEnclave.E3TimeoutConfig calldata tc,
        uint256 sortitionWindow,
        uint32[2] calldata threshold,
        uint256 inputWindowStart,
        uint256 inputWindowEnd
    ) external pure returns (uint256 fee) {
        if (inputWindowEnd < inputWindowStart)
            revert IEnclave.InvalidInputDeadlineEnd(inputWindowEnd);

        uint256 n = uint256(threshold[1]); // total committee size
        uint256 m = uint256(threshold[0]); // quorum/decryption threshold

        // Duration covers the full availability period, using expected-case
        // utilization fractions for protocol-controlled timeout windows.
        // Sum the BPS-weighted windows first then divide once so the
        // duration does not lose up to ~3 seconds of weight to per-term
        // integer-division truncation.
        uint256 weightedTimeoutsBps = tc.dkgWindow *
            uint256(pc.dkgUtilizationBps) +
            tc.computeWindow *
            uint256(pc.computeUtilizationBps) +
            tc.decryptionWindow *
            uint256(pc.decryptUtilizationBps);
        uint256 duration = sortitionWindow +
            inputWindowEnd -
            inputWindowStart +
            weightedTimeoutsBps /
            uint256(BPS_BASE);

        // ZK proof count per node: 14 fixed + 4 × (N-1) scaling.
        uint256 proofsPerNode = 14 + 4 * (n - 1);

        // Key generation cost: fixed per-node + per-proof (quadratic in n)
        uint256 baseFee = pc.keyGenFixedPerNode * n;
        baseFee += pc.keyGenPerEncryptionProof * n * proofsPerNode;

        // Key generation coordination cost (quadratic in n)
        if (n > 1) {
            baseFee += (pc.coordinationPerPair * (n * (n - 1))) / 2;
        }

        // Proof verification cost: each node verifies all others' proofs.
        baseFee += pc.verificationPerProof * n * proofsPerNode;

        // Availability cost (linear in n × duration)
        baseFee += pc.availabilityPerNodePerSec * n * duration;

        // Decryption cost (linear in m)
        baseFee += pc.decryptionPerNode * m;
        // Decryption coordination cost (quadratic in m)
        if (m > 1) {
            baseFee += (pc.coordinationPerPair * (m * (m - 1))) / 2;
        }

        // Publication base cost
        baseFee += pc.publicationBase;

        // Apply margin markup
        fee =
            (baseFee * (uint256(BPS_BASE) + uint256(pc.marginBps))) /
            uint256(BPS_BASE);

        if (fee == 0) revert IEnclave.PaymentRequired(fee);
    }
}

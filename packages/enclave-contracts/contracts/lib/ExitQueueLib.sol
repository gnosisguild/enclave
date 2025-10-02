// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

library ExitQueueLib {
    struct ExitTranche {
        uint64 unlockTimestamp;
        uint256 ticketAmount;
        uint256 licenseAmount;
    }

    struct PendingAmounts {
        uint256 ticketAmount;
        uint256 licenseAmount;
    }

    struct ExitQueueState {
        mapping(address operator => ExitTranche[] operatorQueues) operatorQueues;
        mapping(address operator => uint256 queueHeadIndex) queueHeadIndex;
        mapping(address operator => PendingAmounts operatorPendings) pendingTotals;
    }

    enum AssetType {
        Ticket,
        License
    }

    event AssetsQueuedForExit(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount,
        uint64 unlockTimestamp
    );

    event AssetsClaimed(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount
    );

    event PendingAssetsSlashed(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount,
        bool includedLockedAssets
    );

    error ZeroAmountNotAllowed();
    error TimestampOverflow();
    error IndexOutOfBounds();

    function queueAssetsForExit(
        ExitQueueState storage state,
        address operator,
        uint64 exitDelaySeconds,
        uint256 ticketAmount,
        uint256 licenseAmount
    ) internal {
        if (ticketAmount == 0 && licenseAmount == 0) {
            return;
        }

        uint64 currentTimestamp = uint64(block.timestamp);
        require(
            currentTimestamp <= (type(uint64).max - exitDelaySeconds),
            TimestampOverflow()
        );
        uint64 unlockTimestamp = currentTimestamp + exitDelaySeconds;

        ExitTranche[] storage operatorQueue = state.operatorQueues[operator];

        uint256 len = operatorQueue.length;
        bool merged;
        if (len != 0) {
            ExitTranche storage lastTranche = operatorQueue[len - 1];
            if (lastTranche.unlockTimestamp == unlockTimestamp) {
                if (ticketAmount != 0) lastTranche.ticketAmount += ticketAmount;
                if (licenseAmount != 0)
                    lastTranche.licenseAmount += licenseAmount;
                merged = true;
            }
        }

        if (!merged) {
            ExitTranche storage t = operatorQueue.push();
            t.unlockTimestamp = unlockTimestamp;
            t.ticketAmount = ticketAmount;
            t.licenseAmount = licenseAmount;
        }

        _updatePendingTotals(
            state,
            operator,
            ticketAmount,
            licenseAmount,
            true
        );

        emit AssetsQueuedForExit(
            operator,
            ticketAmount,
            licenseAmount,
            unlockTimestamp
        );
    }

    function queueTicketsForExit(
        ExitQueueState storage state,
        address operator,
        uint64 exitDelaySeconds,
        uint256 ticketAmount
    ) internal {
        queueAssetsForExit(state, operator, exitDelaySeconds, ticketAmount, 0);
    }

    function queueLicensesForExit(
        ExitQueueState storage state,
        address operator,
        uint64 exitDelaySeconds,
        uint256 licenseAmount
    ) internal {
        queueAssetsForExit(state, operator, exitDelaySeconds, 0, licenseAmount);
    }

    function getPendingAmounts(
        ExitQueueState storage state,
        address operator
    ) internal view returns (uint256 ticketAmount, uint256 licenseAmount) {
        PendingAmounts storage pending = state.pendingTotals[operator];
        return (pending.ticketAmount, pending.licenseAmount);
    }

    function previewClaimableAmounts(
        ExitQueueState storage state,
        address operator
    ) internal view returns (uint256 ticketAmount, uint256 licenseAmount) {
        ExitTranche[] storage operatorQueue = state.operatorQueues[operator];
        uint256 currentIndex = state.queueHeadIndex[operator];

        for (uint256 i = currentIndex; i < operatorQueue.length; i++) {
            ExitTranche storage tranche = operatorQueue[i];

            if (block.timestamp < tranche.unlockTimestamp) {
                break;
            }

            ticketAmount += tranche.ticketAmount;
            licenseAmount += tranche.licenseAmount;
        }
    }

    function claimAssets(
        ExitQueueState storage state,
        address operator,
        uint256 maxTicketAmount,
        uint256 maxLicenseAmount
    ) internal returns (uint256 ticketsClaimed, uint256 licensesClaimed) {
        if (maxTicketAmount > 0) {
            ticketsClaimed = _takeAssetsFromQueue(
                state,
                operator,
                maxTicketAmount,
                AssetType.Ticket,
                false
            );
            if (ticketsClaimed > 0) {
                state.pendingTotals[operator].ticketAmount -= ticketsClaimed;
            }
        }

        if (maxLicenseAmount > 0) {
            licensesClaimed = _takeAssetsFromQueue(
                state,
                operator,
                maxLicenseAmount,
                AssetType.License,
                false
            );
            if (licensesClaimed > 0) {
                state.pendingTotals[operator].licenseAmount -= licensesClaimed;
            }
        }

        if (ticketsClaimed > 0 || licensesClaimed > 0) {
            _cleanupEmptyTranches(state, operator);
            emit AssetsClaimed(operator, ticketsClaimed, licensesClaimed);
        }
    }

    function slashPendingAssets(
        ExitQueueState storage state,
        address operator,
        uint256 ticketAmountToSlash,
        uint256 licenseAmountToSlash,
        bool includeLockedAssets
    ) internal returns (uint256 ticketsSlashed, uint256 licensesSlashed) {
        if (ticketAmountToSlash > 0) {
            ticketsSlashed = _takeAssetsFromQueue(
                state,
                operator,
                ticketAmountToSlash,
                AssetType.Ticket,
                includeLockedAssets
            );
            if (ticketsSlashed > 0) {
                state.pendingTotals[operator].ticketAmount -= ticketsSlashed;
            }
        }

        if (licenseAmountToSlash > 0) {
            licensesSlashed = _takeAssetsFromQueue(
                state,
                operator,
                licenseAmountToSlash,
                AssetType.License,
                includeLockedAssets
            );
            if (licensesSlashed > 0) {
                state.pendingTotals[operator].licenseAmount -= licensesSlashed;
            }
        }

        if (ticketsSlashed > 0 || licensesSlashed > 0) {
            _cleanupEmptyTranches(state, operator);
            emit PendingAssetsSlashed(
                operator,
                ticketsSlashed,
                licensesSlashed,
                includeLockedAssets
            );
        }
    }

    function _updatePendingTotals(
        ExitQueueState storage state,
        address operator,
        uint256 ticketAmountDelta,
        uint256 licenseAmountDelta,
        bool isIncrease
    ) private {
        if ((ticketAmountDelta | licenseAmountDelta) == 0) return;

        PendingAmounts storage pending = state.pendingTotals[operator];

        if (isIncrease) {
            if (ticketAmountDelta != 0)
                pending.ticketAmount += ticketAmountDelta;
            if (licenseAmountDelta != 0)
                pending.licenseAmount += licenseAmountDelta;
        } else {
            if (ticketAmountDelta != 0)
                pending.ticketAmount -= ticketAmountDelta;
            if (licenseAmountDelta != 0)
                pending.licenseAmount -= licenseAmountDelta;
        }
    }

    function _cleanupEmptyTranches(
        ExitQueueState storage state,
        address operator
    ) private {
        ExitTranche[] storage operatorQueue = state.operatorQueues[operator];
        uint256 currentIndex = state.queueHeadIndex[operator];

        while (currentIndex < operatorQueue.length) {
            ExitTranche storage tranche = operatorQueue[currentIndex];
            if (tranche.ticketAmount == 0 && tranche.licenseAmount == 0) {
                currentIndex++;
            } else {
                break;
            }
        }

        state.queueHeadIndex[operator] = currentIndex;
    }

    function _takeAssetsFromQueue(
        ExitQueueState storage state,
        address operator,
        uint256 wantedAmount,
        AssetType assetType,
        bool includeLockedAssets
    ) private returns (uint256 takenAmount) {
        if (wantedAmount == 0) {
            return 0;
        }

        ExitTranche[] storage operatorQueue = state.operatorQueues[operator];
        uint256 currentIndex = state.queueHeadIndex[operator];
        uint256 queueLength = operatorQueue.length;
        uint256 remainingWanted = wantedAmount;

        while (remainingWanted > 0 && currentIndex < queueLength) {
            ExitTranche storage tranche = operatorQueue[currentIndex];

            if (
                !includeLockedAssets &&
                block.timestamp < tranche.unlockTimestamp
            ) {
                break;
            }

            uint256 availableAmount;
            if (assetType == AssetType.Ticket) {
                availableAmount = tranche.ticketAmount;
            } else {
                availableAmount = tranche.licenseAmount;
            }

            if (availableAmount == 0) {
                currentIndex++;
                continue;
            }

            uint256 amountToTake = remainingWanted < availableAmount
                ? remainingWanted
                : availableAmount;

            if (assetType == AssetType.Ticket) {
                tranche.ticketAmount -= amountToTake;
            } else {
                tranche.licenseAmount -= amountToTake;
            }

            remainingWanted -= amountToTake;
            takenAmount += amountToTake;

            if (tranche.ticketAmount == 0 && tranche.licenseAmount == 0) {
                currentIndex++;
            }
        }

        state.queueHeadIndex[operator] = currentIndex;
    }
}

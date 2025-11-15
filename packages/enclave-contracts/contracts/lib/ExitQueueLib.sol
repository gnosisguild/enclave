// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

/**
 * @title ExitQueueLib
 * @notice Library for managing time-locked exit queues for tickets and licenses
 * @dev Implements a queue system where assets are locked for a delay period before they can be claimed or slashed.
 *      Assets are organized into tranches based on unlock timestamps, allowing efficient batch operations.
 */
library ExitQueueLib {
    /**
     * @notice Represents a single tranche of assets with a specific unlock timestamp
     * @dev Multiple assets queued at the same time are merged into the same tranche for efficiency
     * @param unlockTimestamp The timestamp when assets in this tranche become claimable
     * @param ticketAmount The amount of tickets in this tranche
     * @param licenseAmount The amount of licenses in this tranche
     */
    struct ExitTranche {
        uint64 unlockTimestamp;
        uint256 ticketAmount;
        uint256 licenseAmount;
    }

    /**
     * @notice Tracks total pending amounts for an operator across all tranches
     * @param ticketAmount Total pending tickets waiting in the exit queue
     * @param licenseAmount Total pending licenses waiting in the exit queue
     */
    struct PendingAmounts {
        uint256 ticketAmount;
        uint256 licenseAmount;
    }

    /**
     * @notice Main state structure for the exit queue system
     * @dev Contains all per-operator queue data and pending totals
     * @param operatorQueues Maps operator addresses to their arrays of exit tranches
     * @param queueHeadIndex Maps operator addresses to the current head index (for efficient cleanup)
     * @param pendingTotals Maps operator addresses to their total pending amounts
     */
    struct ExitQueueState {
        mapping(address operator => ExitTranche[] operatorQueues) operatorQueues;
        mapping(address operator => uint256 queueHeadIndex) queueHeadIndex;
        mapping(address operator => PendingAmounts operatorPendings) pendingTotals;
    }

    /**
     * @notice Types of assets that can be queued for exit
     * @dev Used internally to differentiate between ticket and license operations
     */
    enum AssetType {
        Ticket,
        License
    }

    /**
     * @notice Emitted when assets are queued for exit
     * @param operator The operator whose assets were queued
     * @param ticketAmount The amount of tickets queued
     * @param licenseAmount The amount of licenses queued
     * @param unlockTimestamp The timestamp when these assets will become claimable
     */
    event AssetsQueuedForExit(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount,
        uint64 unlockTimestamp
    );

    /**
     * @notice Emitted when assets are claimed from the exit queue
     * @param operator The operator who claimed the assets
     * @param ticketAmount The amount of tickets claimed
     * @param licenseAmount The amount of licenses claimed
     */
    event AssetsClaimed(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount
    );

    /**
     * @notice Emitted when pending assets are slashed
     * @param operator The operator whose assets were slashed
     * @param ticketAmount The amount of tickets slashed
     * @param licenseAmount The amount of licenses slashed
     * @param includedLockedAssets Whether locked (not yet unlocked) assets were included in the slash
     */
    event PendingAssetsSlashed(
        address indexed operator,
        uint256 ticketAmount,
        uint256 licenseAmount,
        bool includedLockedAssets
    );

    /// @notice Thrown when attempting to queue zero amount of both asset types
    error ZeroAmountNotAllowed();

    /// @notice Thrown when timestamp calculation would overflow uint64
    error TimestampOverflow();

    /// @notice Thrown when accessing an invalid queue index
    error IndexOutOfBounds();

    /**
     * @notice Queues both tickets and licenses for exit with a time delay
     * @dev Assets are added to the operator's queue and will be claimable after exitDelaySeconds.
     *      If a tranche with the same unlock timestamp already exists, amounts are merged into it.
     * @param state The exit queue state storage
     * @param operator The operator whose assets are being queued
     * @param exitDelaySeconds The number of seconds until assets become claimable
     * @param ticketAmount The amount of tickets to queue (can be 0)
     * @param licenseAmount The amount of licenses to queue (can be 0)
     */
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
                if (licenseAmount != 0) {
                    lastTranche.licenseAmount += licenseAmount;
                }
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

    /**
     * @notice Queues only tickets for exit with a time delay
     * @dev Convenience function that calls queueAssetsForExit with licenseAmount = 0
     * @param state The exit queue state storage
     * @param operator The operator whose tickets are being queued
     * @param exitDelaySeconds The number of seconds until tickets become claimable
     * @param ticketAmount The amount of tickets to queue
     */
    function queueTicketsForExit(
        ExitQueueState storage state,
        address operator,
        uint64 exitDelaySeconds,
        uint256 ticketAmount
    ) internal {
        queueAssetsForExit(state, operator, exitDelaySeconds, ticketAmount, 0);
    }

    /**
     * @notice Queues only licenses for exit with a time delay
     * @dev Convenience function that calls queueAssetsForExit with ticketAmount = 0
     * @param state The exit queue state storage
     * @param operator The operator whose licenses are being queued
     * @param exitDelaySeconds The number of seconds until licenses become claimable
     * @param licenseAmount The amount of licenses to queue
     */
    function queueLicensesForExit(
        ExitQueueState storage state,
        address operator,
        uint64 exitDelaySeconds,
        uint256 licenseAmount
    ) internal {
        queueAssetsForExit(state, operator, exitDelaySeconds, 0, licenseAmount);
    }

    /**
     * @notice Gets the total pending amounts for an operator across all tranches
     * @dev Returns both locked (not yet claimable) and unlocked (claimable) amounts
     * @param state The exit queue state storage
     * @param operator The operator to query
     * @return ticketAmount Total pending tickets in the exit queue
     * @return licenseAmount Total pending licenses in the exit queue
     */
    function getPendingAmounts(
        ExitQueueState storage state,
        address operator
    ) internal view returns (uint256 ticketAmount, uint256 licenseAmount) {
        PendingAmounts storage pending = state.pendingTotals[operator];
        return (pending.ticketAmount, pending.licenseAmount);
    }

    /**
     * @notice Previews the amounts that can be claimed at the current block timestamp
     * @dev Iterates through tranches and sums up amounts where unlock timestamp has passed
     * @param state The exit queue state storage
     * @param operator The operator to query
     * @return ticketAmount Total claimable tickets at current timestamp
     * @return licenseAmount Total claimable licenses at current timestamp
     */
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

    /**
     * @notice Claims unlocked assets from the exit queue
     * @dev Only processes tranches where unlock timestamp has passed. Updates pending totals
     *      and cleans up empty tranches.
     * @param state The exit queue state storage
     * @param operator The operator claiming assets
     * @param maxTicketAmount Maximum tickets to claim (actual claimed may be less if queue has fewer)
     * @param maxLicenseAmount Maximum licenses to claim (actual claimed may be less if queue has fewer)
     * @return ticketsClaimed Actual amount of tickets claimed
     * @return licensesClaimed Actual amount of licenses claimed
     */
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

    /**
     * @notice Slashes pending assets from the exit queue
     * @dev Can optionally include locked (not yet unlocked) assets. Updates pending totals
     *      and cleans up empty tranches.
     * @param state The exit queue state storage
     * @param operator The operator whose assets are being slashed
     * @param ticketAmountToSlash Maximum tickets to slash
     * @param licenseAmountToSlash Maximum licenses to slash
     * @param includeLockedAssets If true, slashes locked assets; if false, only slashes unlocked assets
     * @return ticketsSlashed Actual amount of tickets slashed
     * @return licensesSlashed Actual amount of licenses slashed
     */
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

    /**
     * @notice Updates the pending totals for an operator
     * @dev Internal helper to increase or decrease pending amounts. Uses bitwise OR for efficient zero check.
     * @param state The exit queue state storage
     * @param operator The operator whose pending totals are being updated
     * @param ticketAmountDelta The change in ticket amount
     * @param licenseAmountDelta The change in license amount
     * @param isIncrease If true, increases totals; if false, decreases totals
     */
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
            if (ticketAmountDelta != 0) {
                pending.ticketAmount += ticketAmountDelta;
            }
            if (licenseAmountDelta != 0) {
                pending.licenseAmount += licenseAmountDelta;
            }
        } else {
            if (ticketAmountDelta != 0) {
                pending.ticketAmount -= ticketAmountDelta;
            }
            if (licenseAmountDelta != 0) {
                pending.licenseAmount -= licenseAmountDelta;
            }
        }
    }

    /**
     * @notice Cleans up empty tranches from the head of the queue
     * @dev Advances the queue head index past all tranches with zero tickets and licenses.
     *      This prevents the queue from growing unbounded and reduces gas costs for future operations.
     * @param state The exit queue state storage
     * @param operator The operator whose queue is being cleaned up
     */
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

    /**
     * @notice Takes assets from the queue, either for claiming or slashing
     * @dev Iterates through tranches from head to tail, taking assets up to wantedAmount.
     *      Respects unlock timestamps unless includeLockedAssets is true.
     * @param state The exit queue state storage
     * @param operator The operator whose assets are being taken
     * @param wantedAmount The maximum amount to take
     * @param assetType Whether to take tickets or licenses
     * @param includeLockedAssets If true, takes locked assets; if false, only takes unlocked assets
     * @return takenAmount The actual amount taken (may be less than wantedAmount if queue has fewer assets)
     */
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

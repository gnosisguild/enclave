// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { Ownable2Step } from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {
    IERC165
} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import {
    IInterfoldVestingEscrow
} from "../interfaces/IInterfoldVestingEscrow.sol";
import { ILicenseBondReceiver } from "../interfaces/ILicenseBondReceiver.sol";

/**
 * @title InterfoldVestingEscrow
 * @notice Multi-schedule protocol token vesting escrow for Interfold TGE allocations.
 * @dev Schedules combine a token transfer lock and an optional service vesting curve. The
 *      releasable amount is the stricter of the two curves, less claimed and bonded amounts.
 */
contract InterfoldVestingEscrow is
    IInterfoldVestingEscrow,
    ILicenseBondReceiver,
    Ownable2Step,
    ReentrancyGuard
{
    using SafeERC20 for IERC20;

    error ZeroAddress();
    error ZeroAmount();
    error InvalidSchedule();
    error UnknownSchedule();
    error UnauthorizedBeneficiary();
    error NothingClaimable();
    error InsufficientUnbondedAllocation();
    error InsufficientEscrowBacking();
    error UnauthorizedBondingRegistry();
    error RenounceOwnershipDisabled();

    IERC20 public immutable TOKEN;
    IBondingRegistry public immutable BONDING_REGISTRY;
    uint64 public immutable TGE_TIMESTAMP;

    uint256 public nextScheduleId;
    uint256 public totalScheduled;
    uint256 public totalClaimed;
    uint256 public totalBonded;
    uint256 public totalSlashed;

    mapping(uint256 scheduleId => ScheduleView schedule) private _schedules;

    constructor(
        IERC20 token_,
        IBondingRegistry bondingRegistry_,
        uint64 tgeTimestamp_,
        address initialOwner_
    ) Ownable(initialOwner_) {
        if (
            address(token_) == address(0) ||
            address(bondingRegistry_) == address(0) ||
            initialOwner_ == address(0)
        ) {
            revert ZeroAddress();
        }
        TOKEN = token_;
        BONDING_REGISTRY = bondingRegistry_;
        TGE_TIMESTAMP = tgeTimestamp_;
        nextScheduleId = 1;
    }

    function createSchedule(
        ScheduleInput calldata input
    ) external onlyOwner returns (uint256 scheduleId) {
        scheduleId = _createSchedule(input);
    }

    function batchCreateSchedules(
        ScheduleInput[] calldata inputs
    ) external onlyOwner returns (uint256[] memory scheduleIds) {
        uint256 len = inputs.length;
        scheduleIds = new uint256[](len);
        for (uint256 i = 0; i < len; i++) {
            scheduleIds[i] = _createSchedule(inputs[i]);
        }
    }

    function claim(
        uint256 scheduleId,
        uint256 maxAmount
    ) external nonReentrant {
        ScheduleView storage schedule = _schedule(scheduleId);
        if (msg.sender != schedule.beneficiary) {
            revert UnauthorizedBeneficiary();
        }

        uint256 amount = claimableAmount(scheduleId);
        if (maxAmount < amount) amount = maxAmount;
        if (amount == 0) revert NothingClaimable();

        schedule.claimedAmount += amount;
        totalClaimed += amount;

        TOKEN.safeTransfer(schedule.beneficiary, amount);
        emit TokensClaimed(scheduleId, schedule.beneficiary, amount);
    }

    function bondLockedTokens(
        uint256 scheduleId,
        address operator,
        uint256 amount
    ) external nonReentrant {
        if (operator == address(0)) revert ZeroAddress();
        if (amount == 0) revert ZeroAmount();

        ScheduleView storage schedule = _schedule(scheduleId);
        if (msg.sender != schedule.beneficiary) {
            revert UnauthorizedBeneficiary();
        }
        if (amount > bondableAmount(scheduleId)) {
            revert InsufficientUnbondedAllocation();
        }

        schedule.bondedAmount += amount;
        totalBonded += amount;

        TOKEN.safeIncreaseAllowance(address(BONDING_REGISTRY), amount);
        BONDING_REGISTRY.bondLicenseFor(
            operator,
            amount,
            address(this),
            bytes32(scheduleId)
        );

        emit LockedTokensBonded(
            scheduleId,
            schedule.beneficiary,
            operator,
            amount
        );
    }

    function onLicenseBondReturned(
        address operator,
        uint256 amount,
        bytes32 sourceId
    ) external returns (bytes4 selector) {
        if (msg.sender != address(BONDING_REGISTRY)) {
            revert UnauthorizedBondingRegistry();
        }

        uint256 scheduleId = uint256(sourceId);
        ScheduleView storage schedule = _schedule(scheduleId);
        schedule.bondedAmount -= amount;
        totalBonded -= amount;

        emit BondedTokensReturned(scheduleId, operator, amount);
        return ILicenseBondReceiver.onLicenseBondReturned.selector;
    }

    function onLicenseBondSlashed(
        address operator,
        uint256 amount,
        bytes32 sourceId
    ) external returns (bytes4 selector) {
        if (msg.sender != address(BONDING_REGISTRY)) {
            revert UnauthorizedBondingRegistry();
        }

        uint256 scheduleId = uint256(sourceId);
        ScheduleView storage schedule = _schedule(scheduleId);
        schedule.bondedAmount -= amount;
        schedule.slashedAmount += amount;
        totalBonded -= amount;
        totalSlashed += amount;

        emit BondedTokensSlashed(scheduleId, operator, amount);
        return ILicenseBondReceiver.onLicenseBondSlashed.selector;
    }

    function vestedAmount(
        uint256 scheduleId,
        uint64 timestamp
    ) public view returns (uint256) {
        ScheduleView storage schedule = _schedule(scheduleId);
        uint256 effectiveTotal = schedule.totalAmount - schedule.slashedAmount;
        uint256 tokenUnlocked = _tokenUnlocked(
            schedule,
            effectiveTotal,
            timestamp
        );
        uint256 serviceVested = _serviceVested(
            schedule,
            effectiveTotal,
            timestamp
        );
        return tokenUnlocked < serviceVested ? tokenUnlocked : serviceVested;
    }

    function claimableAmount(uint256 scheduleId) public view returns (uint256) {
        ScheduleView storage schedule = _schedule(scheduleId);
        uint256 vested = vestedAmount(scheduleId, uint64(block.timestamp));
        uint256 unavailable = schedule.claimedAmount + schedule.bondedAmount;
        if (vested <= unavailable) return 0;
        return vested - unavailable;
    }

    function bondableAmount(uint256 scheduleId) public view returns (uint256) {
        ScheduleView storage schedule = _schedule(scheduleId);
        uint256 unavailable = schedule.claimedAmount +
            schedule.bondedAmount +
            schedule.slashedAmount;
        return schedule.totalAmount - unavailable;
    }

    function getSchedule(
        uint256 scheduleId
    ) external view returns (ScheduleView memory) {
        return _schedule(scheduleId);
    }

    function supportsInterface(bytes4 interfaceId) public pure returns (bool) {
        return
            interfaceId == type(IInterfoldVestingEscrow).interfaceId ||
            interfaceId == type(ILicenseBondReceiver).interfaceId ||
            interfaceId == type(IERC165).interfaceId;
    }

    function renounceOwnership() public view override onlyOwner {
        revert RenounceOwnershipDisabled();
    }

    function _createSchedule(
        ScheduleInput calldata input
    ) internal returns (uint256 scheduleId) {
        _validateSchedule(input);

        uint256 newTotalScheduled = totalScheduled + input.totalAmount;
        if (
            newTotalScheduled >
            TOKEN.balanceOf(address(this)) +
                totalClaimed +
                totalBonded +
                totalSlashed
        ) {
            revert InsufficientEscrowBacking();
        }

        scheduleId = nextScheduleId++;
        totalScheduled = newTotalScheduled;
        _schedules[scheduleId] = ScheduleView({
            beneficiary: input.beneficiary,
            totalAmount: input.totalAmount,
            claimedAmount: 0,
            bondedAmount: 0,
            slashedAmount: 0,
            tokenHoldUntil: input.tokenHoldUntil,
            tokenUnlockStart: input.tokenUnlockStart,
            tokenUnlockEnd: input.tokenUnlockEnd,
            serviceStart: input.serviceStart,
            serviceCliff: input.serviceCliff,
            serviceEnd: input.serviceEnd,
            group: input.group
        });

        emit ScheduleCreated(
            scheduleId,
            input.beneficiary,
            input.group,
            input.totalAmount,
            input.tokenHoldUntil,
            input.tokenUnlockStart,
            input.tokenUnlockEnd,
            input.serviceStart,
            input.serviceCliff,
            input.serviceEnd
        );
    }

    function _validateSchedule(ScheduleInput calldata input) internal pure {
        if (input.beneficiary == address(0)) revert ZeroAddress();
        if (input.totalAmount == 0) revert ZeroAmount();
        if (input.tokenUnlockEnd < input.tokenUnlockStart) {
            revert InvalidSchedule();
        }
        if (input.serviceEnd != 0) {
            if (input.serviceEnd <= input.serviceStart) {
                revert InvalidSchedule();
            }
            if (
                input.serviceCliff < input.serviceStart ||
                input.serviceCliff > input.serviceEnd
            ) {
                revert InvalidSchedule();
            }
        } else if (input.serviceStart != 0 || input.serviceCliff != 0) {
            revert InvalidSchedule();
        }
    }

    function _schedule(
        uint256 scheduleId
    ) internal view returns (ScheduleView storage schedule) {
        schedule = _schedules[scheduleId];
        if (schedule.beneficiary == address(0)) revert UnknownSchedule();
    }

    function _tokenUnlocked(
        ScheduleView storage schedule,
        uint256 total,
        uint64 timestamp
    ) internal view returns (uint256) {
        uint64 unlockStart = schedule.tokenUnlockStart == 0
            ? TGE_TIMESTAMP
            : schedule.tokenUnlockStart;

        if (timestamp < schedule.tokenHoldUntil) return 0;
        if (schedule.tokenUnlockEnd <= unlockStart) {
            return timestamp >= unlockStart ? total : 0;
        }
        if (timestamp < unlockStart) return 0;
        if (timestamp >= schedule.tokenUnlockEnd) return total;

        return
            (total * (uint256(timestamp) - uint256(unlockStart))) /
            (uint256(schedule.tokenUnlockEnd) - uint256(unlockStart));
    }

    function _serviceVested(
        ScheduleView storage schedule,
        uint256 total,
        uint64 timestamp
    ) internal view returns (uint256) {
        if (schedule.serviceEnd == 0) return total;
        if (timestamp < schedule.serviceCliff) return 0;
        if (timestamp >= schedule.serviceEnd) return total;

        return
            (total * (uint256(timestamp) - uint256(schedule.serviceStart))) /
            (uint256(schedule.serviceEnd) - uint256(schedule.serviceStart));
    }
}

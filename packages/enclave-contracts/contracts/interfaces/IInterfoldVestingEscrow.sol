// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity 0.8.28;

import {
    IERC165
} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";

interface IInterfoldVestingEscrow is IERC165 {
    struct ScheduleInput {
        address beneficiary;
        uint256 totalAmount;
        uint64 tokenHoldUntil;
        uint64 tokenUnlockStart;
        uint64 tokenUnlockEnd;
        uint64 serviceStart;
        uint64 serviceCliff;
        uint64 serviceEnd;
        bytes32 group;
    }

    struct ScheduleView {
        address beneficiary;
        uint256 totalAmount;
        uint256 claimedAmount;
        uint256 bondedAmount;
        uint256 slashedAmount;
        uint64 tokenHoldUntil;
        uint64 tokenUnlockStart;
        uint64 tokenUnlockEnd;
        uint64 serviceStart;
        uint64 serviceCliff;
        uint64 serviceEnd;
        bytes32 group;
    }

    event ScheduleCreated(
        uint256 indexed scheduleId,
        address indexed beneficiary,
        bytes32 indexed group,
        uint256 totalAmount,
        uint64 tokenHoldUntil,
        uint64 tokenUnlockStart,
        uint64 tokenUnlockEnd,
        uint64 serviceStart,
        uint64 serviceCliff,
        uint64 serviceEnd
    );

    event TokensClaimed(
        uint256 indexed scheduleId,
        address indexed beneficiary,
        uint256 amount
    );

    event LockedTokensBonded(
        uint256 indexed scheduleId,
        address indexed beneficiary,
        address indexed operator,
        uint256 amount
    );

    event BondedTokensReturned(
        uint256 indexed scheduleId,
        address indexed operator,
        uint256 amount
    );

    event BondedTokensSlashed(
        uint256 indexed scheduleId,
        address indexed operator,
        uint256 amount
    );

    function createSchedule(
        ScheduleInput calldata input
    ) external returns (uint256 scheduleId);

    function batchCreateSchedules(
        ScheduleInput[] calldata inputs
    ) external returns (uint256[] memory scheduleIds);

    function claim(uint256 scheduleId, uint256 maxAmount) external;

    function bondLockedTokens(
        uint256 scheduleId,
        address operator,
        uint256 amount
    ) external;

    function vestedAmount(
        uint256 scheduleId,
        uint64 timestamp
    ) external view returns (uint256);

    function claimableAmount(
        uint256 scheduleId
    ) external view returns (uint256);

    function bondableAmount(uint256 scheduleId) external view returns (uint256);

    function getSchedule(
        uint256 scheduleId
    ) external view returns (ScheduleView memory);
}

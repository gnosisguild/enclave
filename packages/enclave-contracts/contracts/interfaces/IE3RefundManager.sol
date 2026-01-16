// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;
import { IEnclave } from "./IEnclave.sol";

/**
 * @title IE3RefundManager
 * @notice Interface for E3 refund distribution mechanism
 * @dev Handles refund calculation and claiming for failed E3s
 */
interface IE3RefundManager {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Structs                         //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Work value allocation in basis points (10000 = 100%)
    struct WorkValueAllocation {
        uint16 committeeFormationBps;
        uint16 dkgBps;
        uint16 decryptionBps;
        uint16 protocolBps;
    }
    /// @notice Refund distribution for a failed E3
    struct RefundDistribution {
        uint256 requesterAmount; // Amount for requester
        uint256 honestNodeAmount; // Total amount for honest nodes
        uint256 protocolAmount; // Amount for protocol treasury
        uint256 totalSlashed; // Slashed funds added
        uint256 honestNodeCount; // Number of honest nodes
        bool calculated; // Whether distribution is calculated
    }
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Events                          //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Emitted when refund distribution is calculated
    event RefundDistributionCalculated(
        uint256 indexed e3Id,
        uint256 requesterAmount,
        uint256 honestNodeAmount,
        uint256 protocolAmount,
        uint256 totalSlashed
    );
    /// @notice Emitted when a refund is claimed
    event RefundClaimed(
        uint256 indexed e3Id,
        address indexed claimant,
        uint256 amount,
        bytes32 claimType
    );
    /// @notice Emitted when slashed funds are routed to E3
    event SlashedFundsRouted(uint256 indexed e3Id, uint256 amount);
    /// @notice Emitted when work allocation is updated
    event WorkAllocationUpdated(WorkValueAllocation allocation);
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice E3 is not in failed state
    error E3NotFailed(uint256 e3Id);
    /// @notice Refund already claimed
    error AlreadyClaimed(uint256 e3Id, address claimant);
    /// @notice Not the requester
    error NotRequester(uint256 e3Id, address caller);
    /// @notice Not an honest node
    error NotHonestNode(uint256 e3Id, address caller);
    /// @notice Refund not calculated yet
    error RefundNotCalculated(uint256 e3Id);
    /// @notice No refund available
    error NoRefundAvailable(uint256 e3Id);
    /// @notice Caller not authorized
    error Unauthorized();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                      Functions                         //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Calculate refund distribution for a failed E3
    /// @param e3Id The failed E3 ID
    /// @param originalPayment The original payment amount
    /// @param honestNodes Array of honest node addresses
    function calculateRefund(
        uint256 e3Id,
        uint256 originalPayment,
        address[] calldata honestNodes
    ) external;

    /// @notice Requester claims their refund
    /// @param e3Id The failed E3 ID
    /// @return amount The amount claimed
    function claimRequesterRefund(
        uint256 e3Id
    ) external returns (uint256 amount);

    /// @notice Honest node claims their reward
    /// @param e3Id The failed E3 ID
    /// @return amount The amount claimed
    function claimHonestNodeReward(
        uint256 e3Id
    ) external returns (uint256 amount);

    /// @notice Route slashed funds to E3 refund pool
    /// @param e3Id The E3 ID
    /// @param amount The slashed amount
    function routeSlashedFunds(uint256 e3Id, uint256 amount) external;

    /// @notice Get refund distribution for an E3
    /// @param e3Id The E3 ID
    /// @return distribution The refund distribution
    function getRefundDistribution(
        uint256 e3Id
    ) external view returns (RefundDistribution memory distribution);

    /// @notice Check if address has claimed refund
    /// @param e3Id The E3 ID
    /// @param claimant The address to check
    /// @return claimed Whether the address has claimed
    function hasClaimed(
        uint256 e3Id,
        address claimant
    ) external view returns (bool claimed);

    /// @notice Calculate work value for a given stage
    /// @param stage The stage when E3 failed
    /// @return workCompletedBps Work completed in basis points
    /// @return workRemainingBps Work remaining in basis points
    function calculateWorkValue(
        IEnclave.E3Stage stage
    ) external view returns (uint16 workCompletedBps, uint16 workRemainingBps);

    /// @notice Set work value allocation
    /// @param allocation The new work allocation
    function setWorkAllocation(
        WorkValueAllocation calldata allocation
    ) external;

    /// @notice Get current work allocation
    /// @return allocation The current work allocation
    function getWorkAllocation()
        external
        view
        returns (WorkValueAllocation memory allocation);
}

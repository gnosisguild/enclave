// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { IE3RefundManager } from "./interfaces/IE3RefundManager.sol";
import { IEnclave } from "./interfaces/IEnclave.sol";
import { IBondingRegistry } from "./interfaces/IBondingRegistry.sol";

/**
 * @title E3RefundManager
 * @notice Manages refund distribution for failed E3 computations
 * @dev Implements fault-attribution based refund system
 *
 */
contract E3RefundManager is IE3RefundManager, OwnableUpgradeable {
    using SafeERC20 for IERC20;
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice The Enclave contract (contains lifecycle functionality)
    IEnclave public enclave;
    /// @notice The fee token used for payments
    IERC20 public feeToken;
    /// @notice The bonding registry for node rewards
    IBondingRegistry public bondingRegistry;
    /// @notice Protocol treasury for protocol fee collection
    address public treasury;
    /// @notice Work value allocation configuration
    WorkValueAllocation internal _workAllocation;
    /// @notice Maps E3 ID to refund distribution
    mapping(uint256 e3Id => RefundDistribution) internal _distributions;
    /// @notice Tracks claims per E3 per address
    mapping(uint256 e3Id => mapping(address => bool)) internal _claimed;
    /// @notice Tracks number of claims made per E3 (for routeSlashedFunds guard)
    mapping(uint256 e3Id => uint256) internal _claimCount;
    /// @notice Maps E3 ID to honest node addresses
    mapping(uint256 e3Id => address[]) internal _honestNodes;
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                       Modifiers                        //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Restricts function to Enclave contract only
    modifier onlyEnclave() {
        if (msg.sender != address(enclave)) revert Unauthorized();
        _;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Constructor that disables initializers
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the E3RefundManager contract
    /// @param _owner The owner address
    /// @param _enclave The Enclave contract address
    /// @param _treasury The protocol treasury address
    function initialize(
        address _owner,
        address _enclave,
        address _treasury
    ) public initializer {
        __Ownable_init(msg.sender);

        require(_enclave != address(0), "Invalid enclave");
        require(_treasury != address(0), "Invalid treasury");

        enclave = IEnclave(_enclave);
        feeToken = enclave.feeToken();
        bondingRegistry = enclave.bondingRegistry();
        treasury = _treasury;

        _workAllocation = WorkValueAllocation({
            committeeFormationBps: 1000,
            dkgBps: 3000,
            decryptionBps: 5500,
            protocolBps: 500
        });

        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //               Refund Calculation                       //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3RefundManager
    function calculateRefund(
        uint256 e3Id,
        uint256 originalPayment,
        address[] calldata honestNodes,
        IERC20 paymentToken
    ) external onlyEnclave {
        require(!_distributions[e3Id].calculated, "Already calculated");
        require(originalPayment > 0, "No payment");
        require(address(paymentToken) != address(0), "Invalid fee token");

        // Calculate work value based on stage
        IEnclave.E3Stage failedAt = _getFailedAtStage(e3Id);
        (uint16 workCompletedBps, uint16 workRemainingBps) = calculateWorkValue(
            failedAt
        );

        // Calculate base distribution
        uint256 honestNodeAmount = (originalPayment * workCompletedBps) / 10000;
        uint256 requesterAmount = (originalPayment * workRemainingBps) / 10000;
        uint256 protocolAmount = originalPayment -
            honestNodeAmount -
            requesterAmount;

        // Store distribution with the actual token used for this E3
        _distributions[e3Id] = RefundDistribution({
            requesterAmount: requesterAmount,
            honestNodeAmount: honestNodeAmount,
            protocolAmount: protocolAmount,
            totalSlashed: 0,
            honestNodeCount: honestNodes.length,
            calculated: true,
            feeToken: paymentToken
        });

        // Store honest nodes
        for (uint256 i = 0; i < honestNodes.length; i++) {
            _honestNodes[e3Id].push(honestNodes[i]);
        }

        // Transfer protocol fee to treasury immediately
        if (protocolAmount > 0) {
            paymentToken.safeTransfer(treasury, protocolAmount);
        }

        emit RefundDistributionCalculated(
            e3Id,
            requesterAmount,
            honestNodeAmount,
            protocolAmount,
            0
        );
    }

    /// @notice Get the stage at which E3 failed (for work calculation)
    function _getFailedAtStage(
        uint256 e3Id
    ) internal view returns (IEnclave.E3Stage) {
        IEnclave.FailureReason reason = enclave.getFailureReason(e3Id);

        // Map failure reason to stage
        if (
            reason == IEnclave.FailureReason.CommitteeFormationTimeout ||
            reason == IEnclave.FailureReason.InsufficientCommitteeMembers
        ) {
            return IEnclave.E3Stage.Requested;
        }
        if (
            reason == IEnclave.FailureReason.DKGTimeout ||
            reason == IEnclave.FailureReason.DKGInvalidShares
        ) {
            return IEnclave.E3Stage.CommitteeFinalized;
        }
        if (reason == IEnclave.FailureReason.NoInputsReceived) {
            return IEnclave.E3Stage.KeyPublished;
        }
        if (
            reason == IEnclave.FailureReason.ComputeTimeout ||
            reason == IEnclave.FailureReason.ComputeProviderExpired ||
            reason == IEnclave.FailureReason.ComputeProviderFailed ||
            reason == IEnclave.FailureReason.RequesterCancelled
        ) {
            return IEnclave.E3Stage.KeyPublished;
        }
        if (
            reason == IEnclave.FailureReason.DecryptionTimeout ||
            reason == IEnclave.FailureReason.DecryptionInvalidShares ||
            reason == IEnclave.FailureReason.VerificationFailed
        ) {
            return IEnclave.E3Stage.CiphertextReady;
        }

        return IEnclave.E3Stage.None;
    }

    /// @inheritdoc IE3RefundManager
    function calculateWorkValue(
        IEnclave.E3Stage stage
    ) public view returns (uint16 workCompletedBps, uint16 workRemainingBps) {
        WorkValueAllocation memory alloc = _workAllocation;

        if (
            stage == IEnclave.E3Stage.Requested ||
            stage == IEnclave.E3Stage.None
        ) {
            // Failed at Requested = no work done
            workCompletedBps = 0;
        } else if (stage == IEnclave.E3Stage.CommitteeFinalized) {
            // Failed during DKG = sortition work done
            workCompletedBps = alloc.committeeFormationBps;
        } else if (stage == IEnclave.E3Stage.KeyPublished) {
            // Failed during input phase = sortition + DKG done (no additional work)
            workCompletedBps = alloc.committeeFormationBps + alloc.dkgBps;
        } else if (stage == IEnclave.E3Stage.CiphertextReady) {
            // Failed during decryption = sortition + DKG done (awaiting decryption shares)
            workCompletedBps = alloc.committeeFormationBps + alloc.dkgBps;
        }

        workRemainingBps = 10000 - workCompletedBps - alloc.protocolBps;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Claiming Functions                   //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3RefundManager
    function claimRequesterRefund(
        uint256 e3Id
    ) external returns (uint256 amount) {
        RefundDistribution storage dist = _distributions[e3Id];
        if (!dist.calculated) revert RefundNotCalculated(e3Id);

        address requester = enclave.getRequester(e3Id);
        if (msg.sender != requester) revert NotRequester(e3Id, msg.sender);

        if (_claimed[e3Id][msg.sender]) revert AlreadyClaimed(e3Id, msg.sender);

        amount = dist.requesterAmount;
        if (amount == 0) revert NoRefundAvailable(e3Id);

        _claimed[e3Id][msg.sender] = true;
        _claimCount[e3Id]++;

        // Use the per-E3 fee token (not the global one, which may have been rotated)
        dist.feeToken.safeTransfer(msg.sender, amount);

        emit RefundClaimed(e3Id, msg.sender, amount, "REQUESTER");
    }

    /// @inheritdoc IE3RefundManager
    function claimHonestNodeReward(
        uint256 e3Id
    ) external returns (uint256 amount) {
        RefundDistribution storage dist = _distributions[e3Id];
        require(dist.calculated, RefundNotCalculated(e3Id));
        require(!_claimed[e3Id][msg.sender], AlreadyClaimed(e3Id, msg.sender));

        // Check if caller is honest node
        address[] memory nodes = _honestNodes[e3Id];
        bool isHonest = false;
        for (uint256 i = 0; i < nodes.length && !isHonest; i++) {
            isHonest = (nodes[i] == msg.sender);
        }
        require(isHonest, NotHonestNode(e3Id, msg.sender));

        require(dist.honestNodeCount > 0, NoRefundAvailable(e3Id));
        amount = dist.honestNodeAmount / dist.honestNodeCount;
        require(amount > 0, NoRefundAvailable(e3Id));

        _claimed[e3Id][msg.sender] = true;
        _claimCount[e3Id]++;

        // Transfer directly to the honest node. Using distributeRewards would require
        // this contract to be an authorized distributor in BondingRegistry, and the node
        // must be registered. Direct transfer is simpler and more reliable for refunds.
        IERC20 token = dist.feeToken;
        token.safeTransfer(msg.sender, amount);

        emit RefundClaimed(e3Id, msg.sender, amount, "HONEST_NODE");
    }

    /// @inheritdoc IE3RefundManager
    function routeSlashedFunds(
        uint256 e3Id,
        uint256 amount
    ) external onlyEnclave {
        RefundDistribution storage dist = _distributions[e3Id];
        require(dist.calculated, "Not calculated");
        require(_claimCount[e3Id] == 0, "Claims already started");
        require(amount > 0, "Zero amount");

        // Add slashed funds to distribution
        // 50% to requester, 50% to honest nodes for non-participation
        uint256 toRequester = amount / 2;
        uint256 toHonestNodes = amount - toRequester;

        dist.requesterAmount += toRequester;
        dist.honestNodeAmount += toHonestNodes;
        dist.totalSlashed += amount;

        emit SlashedFundsRouted(e3Id, amount);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                    View Functions                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3RefundManager
    function getRefundDistribution(
        uint256 e3Id
    ) external view returns (RefundDistribution memory) {
        return _distributions[e3Id];
    }

    /// @inheritdoc IE3RefundManager
    function hasClaimed(
        uint256 e3Id,
        address claimant
    ) external view returns (bool) {
        return _claimed[e3Id][claimant];
    }

    /// @inheritdoc IE3RefundManager
    function getWorkAllocation()
        external
        view
        returns (WorkValueAllocation memory)
    {
        return _workAllocation;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Admin Functions                      //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @inheritdoc IE3RefundManager
    function setWorkAllocation(
        WorkValueAllocation calldata allocation
    ) external onlyOwner {
        uint256 total = uint256(allocation.committeeFormationBps) +
            uint256(allocation.dkgBps) +
            uint256(allocation.decryptionBps) +
            uint256(allocation.protocolBps);
        require(total == 10000, "Must sum to 10000");

        _workAllocation = allocation;

        emit WorkAllocationUpdated(allocation);
    }

    /// @notice Set the Enclave contract address
    /// @param _enclave New Enclave address
    function setEnclave(address _enclave) external onlyOwner {
        require(_enclave != address(0), "Invalid enclave");
        enclave = IEnclave(_enclave);
    }

    /// @notice Set the treasury address
    /// @param _treasury New treasury address
    function setTreasury(address _treasury) external onlyOwner {
        require(_treasury != address(0), "Invalid treasury");
        treasury = _treasury;
    }
}

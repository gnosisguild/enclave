// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import {
    UUPSUpgradeable
} from "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import {
    Initializable
} from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    PausableUpgradeable
} from "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { Math } from "@openzeppelin/contracts/utils/math/Math.sol";
import { ExitQueueLib } from "../lib/ExitQueueLib.sol";

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { ISlashingManager } from "../interfaces/ISlashingManager.sol";
import { EnclaveTicketToken } from "../token/EnclaveTicket.sol";

/**
 * @title BondingRegistry
 * @notice Main registry for operator balance and license bonds
 */
contract BondingRegistry is
    Initializable,
    UUPSUpgradeable,
    OwnableUpgradeable,
    PausableUpgradeable,
    IBondingRegistry
{
    using SafeERC20 for IERC20;
    using ExitQueueLib for ExitQueueLib.ExitQueueState;

    // ======================
    // Constants
    // ======================

    bytes32 private constant REASON_DEPOSIT = bytes32("DEPOSIT");
    bytes32 private constant REASON_WITHDRAW = bytes32("WITHDRAW");
    bytes32 private constant REASON_BOND = bytes32("BOND");
    bytes32 private constant REASON_UNBOND = bytes32("UNBOND");

    // ======================
    // Storage
    // ======================

    /// @notice ticket token (ETK (Underlying USDC))
    EnclaveTicketToken public ticketToken;

    /// @notice License token (ENCL)
    IERC20 public licenseToken;

    /// @notice Registry contract for committee membership checks
    ICiphernodeRegistry public registry;

    /// @notice Authorized slashing manager
    address public slashingManager;

    /// @notice Authorized reward distributor
    address public rewardDistributor;

    /// @notice Treasury address for slashed funds
    address public slashedFundsTreasury;

    // Configuration
    uint256 public ticketPrice;
    uint256 public licenseRequiredBond;
    uint256 public minTicketBalance;
    uint64 public exitDelay;
    uint256 public licenseActiveBps = 8_000; // 80%

    // Operator data structure
    struct Operator {
        uint256 licenseBond;
        uint64 exitUnlocksAt;
        bool registered;
        bool exitRequested;
        bool active;
    }

    // Operator data
    mapping(address operator => Operator data) internal operators;

    // Total slashed funds available for treasury withdrawal
    uint256 public slashedTicketBalance;
    uint256 public slashedLicenseBond;

    // ======================
    // Exit Queue library state
    // ======================
    ExitQueueLib.ExitQueueState private _exits;

    // ======================
    // Storage Gaps for Upgrades
    // ======================

    uint256[50] private __gap;

    // ======================
    // Modifiers
    // ======================

    modifier onlySlashingManager() {
        if (msg.sender != slashingManager) revert Unauthorized();
        _;
    }

    modifier noExitInProgress(address operator) {
        Operator storage op = operators[operator];
        if (op.exitRequested && block.timestamp < op.exitUnlocksAt)
            revert ExitInProgress();
        _;
    }

    // ======================
    // Initialization
    // ======================

    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /**
     * @notice Initialize the contract
     * @param owner Contract owner
     * @param _ticketToken Ticket token contract
     * @param _licenseToken License token contract
     * @param _registry Registry contract
     * @param _slashedFundsTreasury Slashed funds treasury address
     * @param _ticketPrice Initial ticket price
     * @param _licenseRequiredBond Initial license bond price
     * @param _minTicketBalance Initial minimum ticket balance for activation
     * @param _exitDelay Initial exit delay period
     */
    function initialize(
        address owner,
        EnclaveTicketToken _ticketToken,
        IERC20 _licenseToken,
        address _registry,
        address _slashedFundsTreasury,
        uint256 _ticketPrice,
        uint256 _licenseRequiredBond,
        uint256 _minTicketBalance,
        uint64 _exitDelay
    ) external initializer {
        __Ownable_init(owner);
        __Pausable_init();
        __UUPSUpgradeable_init();

        require(address(_ticketToken) != address(0), ZeroAddress());
        require(address(_licenseToken) != address(0), ZeroAddress());
        require(_slashedFundsTreasury != address(0), ZeroAddress());
        require(_ticketPrice != 0, InvalidConfiguration());
        require(_licenseRequiredBond != 0, InvalidConfiguration());

        ticketToken = _ticketToken;
        licenseToken = _licenseToken;
        registry = ICiphernodeRegistry(_registry);
        slashedFundsTreasury = _slashedFundsTreasury;
        ticketPrice = _ticketPrice;
        licenseRequiredBond = _licenseRequiredBond;
        minTicketBalance = _minTicketBalance;
        exitDelay = _exitDelay;
    }

    // ======================
    // View Functions
    // ======================

    function getTicketBalance(
        address operator
    ) external view returns (uint256) {
        return ticketToken.balanceOf(operator);
    }

    function getLicenseBond(address operator) external view returns (uint256) {
        return operators[operator].licenseBond;
    }

    function availableTickets(
        address operator
    ) external view returns (uint256) {
        if (ticketPrice == 0) return 0;
        return ticketToken.balanceOf(operator) / ticketPrice;
    }

    function getTicketBalanceAtBlock(
        address operator,
        uint256 blockNumber
    ) external view returns (uint256) {
        return ticketToken.getPastVotes(operator, blockNumber);
    }

    function pendingExits(
        address operator
    ) external view returns (uint256 ticket, uint256 license) {
        return _exits.getPendingAmounts(operator);
    }

    function previewClaimable(
        address operator
    ) external view returns (uint256 ticket, uint256 license) {
        return _exits.previewClaimableAmounts(operator);
    }

    function isLicensed(address operator) external view returns (bool) {
        return operators[operator].licenseBond >= _minLicenseBond();
    }

    function isRegistered(address operator) external view returns (bool) {
        return operators[operator].registered;
    }

    function isActive(address operator) external view returns (bool) {
        Operator storage op = operators[operator];
        return
            op.registered &&
            op.licenseBond >= _minLicenseBond() &&
            (ticketPrice == 0 ||
                ticketToken.balanceOf(operator) / ticketPrice >=
                minTicketBalance);
    }

    function hasExitInProgress(address operator) external view returns (bool) {
        Operator storage op = operators[operator];
        return op.exitRequested && block.timestamp < op.exitUnlocksAt;
    }

    // ======================
    // Operator Functions
    // ======================

    function registerOperator()
        external
        whenNotPaused
        noExitInProgress(msg.sender)
    {
        // Clear previous exit request
        if (operators[msg.sender].exitRequested) {
            operators[msg.sender].exitRequested = false;
            operators[msg.sender].exitUnlocksAt = 0;
        }

        require(
            !ISlashingManager(slashingManager).isBanned(msg.sender),
            CiphernodeBanned()
        );
        require(!operators[msg.sender].registered, AlreadyRegistered());
        require(
            operators[msg.sender].licenseBond >= licenseRequiredBond,
            NotLicensed()
        );

        operators[msg.sender].registered = true;

        if (address(registry) != address(0)) {
            // CiphernodeRegistry already emits an event when a ciphernode is added
            registry.addCiphernode(msg.sender);
        }

        _updateOperatorStatus(msg.sender);
    }

    function deregisterOperator(
        uint256[] calldata siblingNodes
    ) external whenNotPaused noExitInProgress(msg.sender) {
        Operator storage op = operators[msg.sender];
        require(op.registered, NotRegistered());

        op.registered = false;
        op.exitRequested = true;
        op.exitUnlocksAt = uint64(block.timestamp) + exitDelay;

        uint256 ticketOut = ticketToken.balanceOf(msg.sender);
        uint256 licenseOut = op.licenseBond;
        if (ticketOut != 0) {
            ticketToken.burnTickets(msg.sender, ticketOut);
            emit TicketBalanceUpdated(
                msg.sender,
                -int256(ticketOut),
                0,
                REASON_WITHDRAW
            );
        }
        if (licenseOut != 0) {
            op.licenseBond = 0;
            emit LicenseBondUpdated(
                msg.sender,
                -int256(licenseOut),
                0,
                REASON_UNBOND
            );
        }

        if (ticketOut != 0 || licenseOut != 0) {
            _exits.queueAssetsForExit(
                msg.sender,
                exitDelay,
                ticketOut,
                licenseOut
            );
        }

        if (address(registry) != address(0)) {
            // CiphernodeRegistry already emits an event when a ciphernode is removed
            registry.removeCiphernode(msg.sender, siblingNodes);
        }

        emit CiphernodeDeregistrationRequested(msg.sender, op.exitUnlocksAt);
        _updateOperatorStatus(msg.sender);
    }

    function addTicketBalance(
        uint256 amount
    ) external whenNotPaused noExitInProgress(msg.sender) {
        require(amount != 0, ZeroAmount());
        require(operators[msg.sender].registered, NotRegistered());

        ticketToken.depositFrom(msg.sender, msg.sender, amount);

        emit TicketBalanceUpdated(
            msg.sender,
            int256(amount),
            ticketToken.balanceOf(msg.sender),
            REASON_DEPOSIT
        );

        _updateOperatorStatus(msg.sender);
    }

    function removeTicketBalance(
        uint256 amount
    ) external whenNotPaused noExitInProgress(msg.sender) {
        require(amount != 0, ZeroAmount());
        require(operators[msg.sender].registered, NotRegistered());
        require(
            ticketToken.balanceOf(msg.sender) >= amount,
            InsufficientBalance()
        );

        ticketToken.burnTickets(msg.sender, amount);
        _exits.queueTicketsForExit(msg.sender, exitDelay, amount);

        emit TicketBalanceUpdated(
            msg.sender,
            -int256(amount),
            ticketToken.balanceOf(msg.sender),
            REASON_WITHDRAW
        );

        _updateOperatorStatus(msg.sender);
    }

    function bondLicense(
        uint256 amount
    ) external whenNotPaused noExitInProgress(msg.sender) {
        require(amount != 0, ZeroAmount());

        uint256 balanceBefore = licenseToken.balanceOf(address(this));
        licenseToken.safeTransferFrom(msg.sender, address(this), amount);
        uint256 actualReceived = licenseToken.balanceOf(address(this)) -
            balanceBefore;

        operators[msg.sender].licenseBond += actualReceived;

        emit LicenseBondUpdated(
            msg.sender,
            int256(actualReceived),
            operators[msg.sender].licenseBond,
            REASON_BOND
        );

        _updateOperatorStatus(msg.sender);
    }

    function unbondLicense(
        uint256 amount
    ) external whenNotPaused noExitInProgress(msg.sender) {
        require(amount != 0, ZeroAmount());
        require(
            operators[msg.sender].licenseBond >= amount,
            InsufficientBalance()
        );

        operators[msg.sender].licenseBond -= amount;
        _exits.queueLicensesForExit(msg.sender, exitDelay, amount);

        emit LicenseBondUpdated(
            msg.sender,
            -int256(amount),
            operators[msg.sender].licenseBond,
            REASON_UNBOND
        );

        _updateOperatorStatus(msg.sender);
    }

    // ======================
    // Claim Functions
    // ======================

    function claimExits(
        uint256 maxTicketAmount,
        uint256 maxLicenseAmount
    ) external whenNotPaused {
        (uint256 ticketClaim, uint256 licenseClaim) = _exits.claimAssets(
            msg.sender,
            maxTicketAmount,
            maxLicenseAmount
        );
        require(ticketClaim > 0 || licenseClaim > 0, ExitNotReady());

        if (ticketClaim > 0) ticketToken.payout(msg.sender, ticketClaim);
        if (licenseClaim > 0)
            licenseToken.safeTransfer(msg.sender, licenseClaim);
    }

    // ======================
    // Slashing Functions
    // ======================

    function slashTicketBalance(
        address operator,
        uint256 requestedSlashAmount,
        bytes32 slashReason
    ) external onlySlashingManager {
        require(requestedSlashAmount != 0, ZeroAmount());

        (uint256 pendingTicketBalance, ) = _exits.getPendingAmounts(operator);
        uint256 activeBalance = ticketToken.balanceOf(operator);
        uint256 totalAvailableBalance = activeBalance + pendingTicketBalance;

        uint256 actualSlashAmount = Math.min(
            requestedSlashAmount,
            totalAvailableBalance
        );

        if (actualSlashAmount == 0) {
            return;
        }

        // Slash from active balance first
        uint256 slashedFromActiveBalance = Math.min(
            actualSlashAmount,
            activeBalance
        );
        if (slashedFromActiveBalance > 0) {
            ticketToken.burnTickets(operator, slashedFromActiveBalance);
        }

        // Slash remaining amount from pending queue
        uint256 remainingToSlash = actualSlashAmount - slashedFromActiveBalance;
        if (remainingToSlash > 0) {
            _exits.slashPendingAssets(
                operator,
                remainingToSlash,
                0, // licenseAmount
                true
            );
        }

        slashedTicketBalance += actualSlashAmount;
        emit TicketBalanceUpdated(
            operator,
            -int256(actualSlashAmount),
            ticketToken.balanceOf(operator),
            slashReason
        );

        _updateOperatorStatus(operator);
    }

    function slashLicenseBond(
        address operator,
        uint256 requestedSlashAmount,
        bytes32 slashReason
    ) external onlySlashingManager {
        require(requestedSlashAmount != 0, ZeroAmount());

        Operator storage operatorData = operators[operator];
        (, uint256 pendingLicenseBalance) = _exits.getPendingAmounts(operator);
        uint256 totalAvailableBalance = operatorData.licenseBond +
            pendingLicenseBalance;
        uint256 actualSlashAmount = Math.min(
            requestedSlashAmount,
            totalAvailableBalance
        );

        if (actualSlashAmount == 0) return;

        // Slash from active balance first
        uint256 slashedFromActiveBalance = Math.min(
            actualSlashAmount,
            operatorData.licenseBond
        );
        if (slashedFromActiveBalance > 0) {
            operatorData.licenseBond -= slashedFromActiveBalance;
        }

        // Slash remaining amount from pending queue
        uint256 remainingToSlash = actualSlashAmount - slashedFromActiveBalance;
        if (remainingToSlash > 0) {
            _exits.slashPendingAssets(
                operator,
                0, // ticketAmount
                remainingToSlash,
                true
            );
        }

        slashedLicenseBond += actualSlashAmount;
        emit LicenseBondUpdated(
            operator,
            -int256(actualSlashAmount),
            operatorData.licenseBond,
            slashReason
        );

        _updateOperatorStatus(operator);
    }

    // ======================
    // Reward Distribution Functions
    // ======================

    function distributeRewards(
        IERC20 rewardToken,
        address[] calldata recipients,
        uint256[] calldata amounts
    ) external {
        require(msg.sender == rewardDistributor, OnlyRewardDistributor());
        require(recipients.length == amounts.length, ArrayLengthMismatch());

        for (uint256 i = 0; i < recipients.length; i++) {
            if (amounts[i] > 0 && operators[recipients[i]].registered) {
                rewardToken.safeTransferFrom(
                    rewardDistributor,
                    recipients[i],
                    amounts[i]
                );
            }
        }
    }

    // ======================
    // Admin Functions
    // ======================

    function setTicketPrice(uint256 newTicketPrice) external onlyOwner {
        require(newTicketPrice != 0, InvalidConfiguration());

        uint256 oldValue = ticketPrice;
        ticketPrice = newTicketPrice;

        emit ConfigurationUpdated("ticketPrice", oldValue, newTicketPrice);
    }

    function setLicenseRequiredBond(
        uint256 newLicenseRequiredBond
    ) external onlyOwner {
        require(newLicenseRequiredBond != 0, InvalidConfiguration());

        uint256 oldValue = licenseRequiredBond;
        licenseRequiredBond = newLicenseRequiredBond;

        emit ConfigurationUpdated(
            "licenseRequiredBond",
            oldValue,
            newLicenseRequiredBond
        );
    }

    function setLicenseActiveBps(uint256 newBps) external onlyOwner {
        require(newBps > 0 && newBps <= 10_000, InvalidConfiguration());

        uint256 oldValue = licenseActiveBps;
        licenseActiveBps = newBps;

        emit ConfigurationUpdated("licenseActiveBps", oldValue, newBps);
    }

    function setMinTicketBalance(
        uint256 newMinTicketBalance
    ) external onlyOwner {
        uint256 oldValue = minTicketBalance;
        minTicketBalance = newMinTicketBalance;

        emit ConfigurationUpdated(
            "minTicketBalance",
            oldValue,
            newMinTicketBalance
        );
    }

    function setExitDelay(uint64 newExitDelay) external onlyOwner {
        uint256 oldValue = uint256(exitDelay);
        exitDelay = newExitDelay;

        emit ConfigurationUpdated("exitDelay", oldValue, uint256(newExitDelay));
    }

    function setSlashedFundsTreasury(
        address newSlashedFundsTreasury
    ) external onlyOwner {
        require(newSlashedFundsTreasury != address(0), ZeroAddress());
        slashedFundsTreasury = newSlashedFundsTreasury;
    }

    function setRegistry(address newRegistry) external onlyOwner {
        registry = ICiphernodeRegistry(newRegistry);
    }

    function setSlashingManager(address newSlashingManager) external onlyOwner {
        slashingManager = newSlashingManager;
    }

    function setRewardDistributor(
        address newRewardDistributor
    ) external onlyOwner {
        rewardDistributor = newRewardDistributor;
    }

    function withdrawSlashedFunds(
        uint256 ticketAmount,
        uint256 licenseAmount
    ) external onlyOwner {
        require(ticketAmount <= slashedTicketBalance, InsufficientBalance());
        require(licenseAmount <= slashedLicenseBond, InsufficientBalance());

        if (ticketAmount > 0) {
            slashedTicketBalance -= ticketAmount;
            ticketToken.payout(slashedFundsTreasury, ticketAmount);
        }

        if (licenseAmount > 0) {
            slashedLicenseBond -= licenseAmount;
            licenseToken.safeTransfer(slashedFundsTreasury, licenseAmount);
        }

        emit SlashedFundsWithdrawn(
            slashedFundsTreasury,
            ticketAmount,
            licenseAmount
        );
    }

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }

    // ======================
    // Internal Functions
    // ======================

    function _updateOperatorStatus(address operator) internal {
        Operator storage op = operators[operator];
        bool newActiveStatus = op.registered &&
            op.licenseBond >= _minLicenseBond() &&
            (ticketPrice == 0 ||
                ticketToken.balanceOf(operator) / ticketPrice >=
                minTicketBalance);

        if (op.active != newActiveStatus) {
            op.active = newActiveStatus;
            emit OperatorActivationChanged(operator, newActiveStatus);
        }
    }

    function _minLicenseBond() internal view returns (uint256) {
        return (licenseRequiredBond * licenseActiveBps) / 10_000;
    }

    function _authorizeUpgrade(
        address newImplementation
    ) internal override onlyOwner {}
}

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

import { IBondingRegistry } from "../interfaces/IBondingRegistry.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { ISlashingManager } from "../interfaces/ISlashingManager.sol";

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

    /// @notice Ticket token (USDC)
    IERC20 public ticketToken;

    /// @notice License token (ENCL)
    IERC20 public licenseToken;

    /// @notice Registry contract for committee membership checks
    ICiphernodeRegistry public registry;

    /// @notice Authorized slashing manager
    address public slashingManager;

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
        uint256 ticketBalance;
        uint256 licenseBond;
        uint64 exitUnlocksAt;
        bool registered;
        bool exitRequested;
        bool active;
    }

    mapping(address operator => Operator data) internal operators;

    // Total slashed funds available for treasury withdrawal
    uint256 public slashedTicketBalance;
    uint256 public slashedLicenseBond;

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

    modifier notInActiveCommittee(address operator) {
        if (
            address(registry) != address(0) &&
            registry.isNodeActiveInAnyCommittee(operator)
        ) {
            revert ActiveCommittee();
        }
        _;
    }

    modifier noExitInProgress(address operator) {
        if (operators[operator].exitRequested) revert ExitInProgress();
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
        IERC20 _ticketToken,
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
        return operators[operator].ticketBalance;
    }

    function getLicenseBond(address operator) external view returns (uint256) {
        return operators[operator].licenseBond;
    }

    function availableTickets(
        address operator
    ) external view returns (uint256) {
        if (ticketPrice == 0) return 0;
        return operators[operator].ticketBalance / ticketPrice;
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
                op.ticketBalance / ticketPrice >= minTicketBalance);
    }

    function hasExitInProgress(address operator) external view returns (bool) {
        return operators[operator].exitRequested;
    }

    // ======================
    // Operator Functions
    // ======================

    function addTicketBalance(
        uint256 amount
    ) external whenNotPaused noExitInProgress(msg.sender) {
        require(amount != 0, ZeroAmount());
        require(operators[msg.sender].registered, NotRegistered());

        uint256 balanceBefore = ticketToken.balanceOf(address(this));
        ticketToken.safeTransferFrom(msg.sender, address(this), amount);
        uint256 actualReceived = ticketToken.balanceOf(address(this)) -
            balanceBefore;

        operators[msg.sender].ticketBalance += actualReceived;

        emit TicketBalanceUpdated(
            msg.sender,
            int256(actualReceived),
            operators[msg.sender].ticketBalance,
            REASON_DEPOSIT
        );

        _updateOperatorStatus(msg.sender);
    }

    function removeTicketBalance(
        uint256 amount
    )
        external
        whenNotPaused
        noExitInProgress(msg.sender)
        notInActiveCommittee(msg.sender)
    {
        require(amount != 0, ZeroAmount());
        require(operators[msg.sender].registered, NotRegistered());
        require(
            operators[msg.sender].ticketBalance >= amount,
            InsufficientBalance()
        );

        operators[msg.sender].ticketBalance -= amount;
        ticketToken.safeTransfer(msg.sender, amount);

        emit TicketBalanceUpdated(
            msg.sender,
            -int256(amount),
            operators[msg.sender].ticketBalance,
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
    )
        external
        whenNotPaused
        noExitInProgress(msg.sender)
        notInActiveCommittee(msg.sender)
    {
        require(amount != 0, ZeroAmount());
        require(
            operators[msg.sender].licenseBond >= amount,
            InsufficientBalance()
        );

        operators[msg.sender].licenseBond -= amount;
        licenseToken.safeTransfer(msg.sender, amount);

        emit LicenseBondUpdated(
            msg.sender,
            -int256(amount),
            operators[msg.sender].licenseBond,
            REASON_UNBOND
        );

        _updateOperatorStatus(msg.sender);
    }

    function registerOperator()
        external
        whenNotPaused
        noExitInProgress(msg.sender)
    {
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

    /**
     * @notice Deregister as an operator and remove from IMT
     * @param siblingNodes Sibling node proofs for IMT removal
     * @dev Requires operator to provide sibling nodes for immediate IMT removal
     */
    function deregisterOperator(
        uint256[] calldata siblingNodes
    )
        external
        whenNotPaused
        noExitInProgress(msg.sender)
        notInActiveCommittee(msg.sender)
    {
        require(operators[msg.sender].registered, NotRegistered());

        operators[msg.sender].registered = false;
        operators[msg.sender].exitRequested = true;
        operators[msg.sender].exitUnlocksAt =
            uint64(block.timestamp) +
            exitDelay;

        if (address(registry) != address(0)) {
            // CiphernodeRegistry already emits an event when a ciphernode is removed
            registry.removeCiphernode(msg.sender, siblingNodes);
        }

        emit CiphernodeDeregistrationRequested(
            msg.sender,
            operators[msg.sender].exitUnlocksAt
        );
        _updateOperatorStatus(msg.sender);
    }

    function finalizeDeregistration() external {
        Operator storage op = operators[msg.sender];
        require(op.exitRequested, ExitInProgress());
        require(block.timestamp >= op.exitUnlocksAt, ExitNotReady());

        uint256 ticketRefund = op.ticketBalance;
        uint256 licenseRefund = op.licenseBond;

        op.ticketBalance = 0;
        op.licenseBond = 0;
        op.exitRequested = false;
        op.exitUnlocksAt = 0;

        if (ticketRefund > 0) {
            ticketToken.safeTransfer(msg.sender, ticketRefund);
        }
        if (licenseRefund > 0) {
            licenseToken.safeTransfer(msg.sender, licenseRefund);
        }

        emit TicketBalanceUpdated(
            msg.sender,
            -int256(ticketRefund),
            0,
            REASON_WITHDRAW
        );
        emit LicenseBondUpdated(
            msg.sender,
            -int256(licenseRefund),
            0,
            REASON_UNBOND
        );
        emit DeregistrationFinalized(msg.sender, ticketRefund, licenseRefund);
        _updateOperatorStatus(msg.sender);
    }

    // ======================
    // Slashing Functions
    // ======================

    function slashTicketBalance(
        address operator,
        uint256 amount,
        bytes32 reason
    ) external onlySlashingManager {
        require(amount != 0, ZeroAmount());

        Operator storage op = operators[operator];
        uint256 currentBalance = op.ticketBalance;
        uint256 slashAmount = Math.min(amount, currentBalance);

        if (slashAmount > 0) {
            op.ticketBalance -= slashAmount;
            slashedTicketBalance += slashAmount;

            emit TicketBalanceUpdated(
                operator,
                -int256(slashAmount),
                op.ticketBalance,
                reason
            );

            _updateOperatorStatus(operator);
        }
    }

    function slashLicenseBond(
        address operator,
        uint256 amount,
        bytes32 reason
    ) external onlySlashingManager {
        require(amount != 0, ZeroAmount());

        Operator storage op = operators[operator];
        uint256 currentBond = op.licenseBond;
        uint256 slashAmount = Math.min(amount, currentBond);

        if (slashAmount > 0) {
            op.licenseBond -= slashAmount;
            slashedLicenseBond += slashAmount;

            emit LicenseBondUpdated(
                operator,
                -int256(slashAmount),
                op.licenseBond,
                reason
            );

            _updateOperatorStatus(operator);
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

    function withdrawSlashedFunds(
        uint256 ticketAmount,
        uint256 licenseAmount
    ) external onlyOwner {
        require(ticketAmount <= slashedTicketBalance, InsufficientBalance());
        require(licenseAmount <= slashedLicenseBond, InsufficientBalance());

        if (ticketAmount > 0) {
            slashedTicketBalance -= ticketAmount;
            ticketToken.safeTransfer(slashedFundsTreasury, ticketAmount);
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
                op.ticketBalance / ticketPrice >= minTicketBalance);

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

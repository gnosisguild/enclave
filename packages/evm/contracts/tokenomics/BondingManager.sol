// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { ReentrancyGuard } from "@oz/utils/ReentrancyGuard.sol";
import { IERC20Metadata } from "@oz/token/ERC20/extensions/IERC20Metadata.sol";
import { SafeERC20 } from "@oz/token/ERC20/utils/SafeERC20.sol";
import { Ownable } from "@oz/access/Ownable.sol";
import { Math } from "@oz/utils/math/Math.sol";
import {
    IStrategy
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";
import {
    IDelegationManager
} from "eigenlayer-contracts/src/contracts/interfaces/IDelegationManager.sol";
import {
    IAllocationManager
} from "eigenlayer-contracts/src/contracts/interfaces/IAllocationManager.sol";
import {
    IStrategyManager
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategyManager.sol";
import {
    OperatorSet
} from "eigenlayer-contracts/src/contracts/libraries/OperatorSetLib.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IServiceManager } from "../interfaces/IServiceManager.sol";

contract BondingManager is Ownable, ReentrancyGuard, IBondingManager {
    using SafeERC20 for IERC20Metadata;

    uint256 public minTicketBalance;
    IStrategy public enclStrategy;
    IStrategy public usdcStrategy;
    uint256 public licenseStake;
    uint256 public ticketPrice;
    uint32 public operatorSetId;

    mapping(address => OperatorInfo) public operators;
    mapping(address => bool) public registeredOperators;
    mapping(address => uint256) public ticketBudgetSpent;

    ICiphernodeRegistry public ciphernodeRegistry;
    IDelegationManager public delegationManager;
    IAllocationManager public allocationManager;
    address public serviceManager;

    modifier onlyServiceManager() {
        if (msg.sender != serviceManager) revert OnlyServiceManager();
        _;
    }

    constructor(
        address _owner,
        address _serviceManager,
        IDelegationManager _delegationManager,
        IAllocationManager _allocationManager,
        address _ciphernodeRegistry,
        IStrategy _enclStrategy,
        IStrategy _usdcStrategy,
        uint256 _licenseStake,
        uint256 _ticketPrice,
        uint32 _operatorSetId
    ) Ownable(_owner) {
        if (_serviceManager == address(0)) revert ZeroAddress();
        if (_ciphernodeRegistry == address(0)) revert ZeroAddress();
        if (address(_delegationManager) == address(0)) revert ZeroAddress();
        if (address(_allocationManager) == address(0)) revert ZeroAddress();
        if (address(_enclStrategy) == address(0)) revert ZeroAddress();
        if (address(_usdcStrategy) == address(0)) revert ZeroAddress();
        if (_licenseStake == 0) revert InsufficientLicenseStake();
        if (_ticketPrice == 0) revert InvalidTicketAmount();

        serviceManager = _serviceManager;
        delegationManager = _delegationManager;
        allocationManager = _allocationManager;
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        enclStrategy = _enclStrategy;
        usdcStrategy = _usdcStrategy;
        licenseStake = _licenseStake;
        ticketPrice = _ticketPrice;
        operatorSetId = _operatorSetId;
        minTicketBalance = 5;
    }

    function acquireLicense() external nonReentrant {
        if (!delegationManager.isOperator(msg.sender))
            revert OperatorNotRegistered();
        if (operators[msg.sender].isLicensed) revert AlreadyLicensed();

        uint256 enclShares = _getOperatorShares(msg.sender, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);
        if (enclAmount < licenseStake) revert InsufficientLicenseStake();

        _requireAllocatedAtLeastUnderlying(
            msg.sender,
            enclStrategy,
            licenseStake
        );

        operators[msg.sender] = OperatorInfo({
            isLicensed: true,
            licenseStake: enclAmount,
            ticketBalance: 0,
            registeredAt: block.timestamp,
            isActive: false,
            collateralUsd: 0
        });

        emit LicenseAcquired(msg.sender, enclAmount);
    }

    function purchaseTickets(uint256 ticketCount) external nonReentrant {
        if (ticketCount == 0) revert InvalidTicketAmount();
        if (!operators[msg.sender].isLicensed) revert NotLicensed();

        uint256 totalCost = ticketCount * ticketPrice;
        if (totalCost / ticketPrice != ticketCount) revert CostOverflow();

        uint256 allocatedUsdc = _allocatedUnderlyingToAVS(
            msg.sender,
            usdcStrategy
        );
        uint256 alreadySpent = ticketBudgetSpent[msg.sender];
        if (allocatedUsdc < alreadySpent + totalCost)
            revert InsufficientTicketBudget();

        ticketBudgetSpent[msg.sender] = alreadySpent + totalCost;
        operators[msg.sender].ticketBalance += ticketCount;

        if (
            registeredOperators[msg.sender] &&
            !operators[msg.sender].isActive &&
            operators[msg.sender].ticketBalance >= minTicketBalance
        ) {
            operators[msg.sender].isActive = true;
            emit CiphernodeActivated(msg.sender);
        }

        emit TicketsPurchased(msg.sender, totalCost, ticketCount);
    }

    function registerCiphernode() external nonReentrant {
        if (registeredOperators[msg.sender]) revert AlreadyRegistered();
        if (!operators[msg.sender].isLicensed) revert NotLicensed();
        if (!delegationManager.isOperator(msg.sender))
            revert OperatorNotRegistered();

        uint256 encl = enclStrategy.sharesToUnderlyingView(
            _getOperatorShares(msg.sender, enclStrategy)
        );
        if (encl < licenseStake) revert InsufficientLicenseStake();
        _requireAllocatedAtLeastUnderlying(
            msg.sender,
            enclStrategy,
            licenseStake
        );
        _requireAllocatedAtLeastUnderlying(
            msg.sender,
            usdcStrategy,
            ticketBudgetSpent[msg.sender]
        );

        registeredOperators[msg.sender] = true;
        ciphernodeRegistry.addCiphernode(msg.sender);

        bool activeNow = (operators[msg.sender].ticketBalance >=
            minTicketBalance);
        operators[msg.sender].isActive = activeNow;
        if (activeNow) emit CiphernodeActivated(msg.sender);

        emit CiphernodeRegistered(msg.sender, 0);
    }

    function deregisterCiphernode(
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        if (!registeredOperators[msg.sender]) revert OperatorNotRegistered();

        if (operators[msg.sender].isActive) {
            operators[msg.sender].isActive = false;
            emit CiphernodeDeactivated(msg.sender);
        }

        registeredOperators[msg.sender] = false;
        ciphernodeRegistry.removeCiphernode(msg.sender, siblingNodes);
        emit CiphernodeDeregistered(msg.sender);
    }

    function useTickets(address operator, uint256 ticketCount) external {
        if (msg.sender != address(ciphernodeRegistry)) revert OnlyRegistry();
        if (operators[operator].ticketBalance < ticketCount)
            revert InsufficientTicketBalance();

        operators[operator].ticketBalance -= ticketCount;
        emit TicketsUsed(operator, ticketCount);
    }

    function slashTickets(
        address operator,
        uint256 wadToSlash
    ) external onlyServiceManager {
        uint256 oldTickets = operators[operator].ticketBalance;
        uint256 ticketsLost = (oldTickets * wadToSlash) / 1e18;
        if (ticketsLost > 0 && ticketsLost <= oldTickets) {
            operators[operator].ticketBalance -= ticketsLost;
            emit TicketsSlashed(operator, ticketsLost);
        }

        if (
            registeredOperators[operator] &&
            operators[operator].isActive &&
            operators[operator].ticketBalance < minTicketBalance
        ) {
            operators[operator].isActive = false;
            emit CiphernodeDeactivated(operator);
        }
    }

    function updateLicenseStatus(address operator) external onlyServiceManager {
        if (!operators[operator].isLicensed) return;

        uint256 enclShares = _getOperatorShares(operator, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);

        bool belowAbsolute = (enclAmount < licenseStake) ||
            (enclAmount < (operators[operator].licenseStake / 2));
        bool belowAllocated = _allocatedUnderlyingToAVS(
            operator,
            enclStrategy
        ) < licenseStake;

        if (belowAbsolute || belowAllocated) {
            operators[operator].isLicensed = false;
            if (registeredOperators[operator]) {
                registeredOperators[operator] = false;
                emit CiphernodeDeregistered(operator);
            }
            emit LicenseRevoked(operator);
        }
    }

    function syncTicketHealth(address operator) external onlyServiceManager {
        uint256 allocatedUsdc = _allocatedUnderlyingToAVS(
            operator,
            usdcStrategy
        );
        if (allocatedUsdc < ticketBudgetSpent[operator]) {
            if (registeredOperators[operator] && operators[operator].isActive) {
                operators[operator].isActive = false;
                emit CiphernodeDeactivated(operator);
            }
        }
    }

    function setMinTicketBalance(uint256 _minTicketBalance) external onlyOwner {
        if (_minTicketBalance == 0) revert InvalidMinTicketBalance();
        minTicketBalance = _minTicketBalance;
        emit MinTicketBalanceUpdated(_minTicketBalance);
    }

    function setLicenseStake(uint256 _licenseStake) external onlyOwner {
        if (_licenseStake == 0) revert InsufficientLicenseStake();
        licenseStake = _licenseStake;
        emit LicenseStakeUpdated(_licenseStake);
    }

    function setTicketPrice(uint256 _ticketPrice) external onlyOwner {
        if (_ticketPrice == 0) revert InvalidTicketAmount();
        ticketPrice = _ticketPrice;
        emit TicketPriceUpdated(_ticketPrice);
    }

    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory) {
        return operators[operator];
    }

    function getAvailableTicketBudget(
        address operator
    ) external view returns (uint256) {
        uint256 allocatedUsdc = _allocatedUnderlyingToAVS(
            operator,
            usdcStrategy
        );
        uint256 alreadySpent = ticketBudgetSpent[operator];
        return allocatedUsdc > alreadySpent ? allocatedUsdc - alreadySpent : 0;
    }

    function getLicenseStake() external view returns (uint256) {
        return licenseStake;
    }

    function getTicketPrice() external view returns (uint256) {
        return ticketPrice;
    }

    function isRegisteredOperator(
        address operator
    ) external view returns (bool) {
        return registeredOperators[operator];
    }

    function isActive(address operator) external view returns (bool) {
        return operators[operator].isActive;
    }

    function _getOperatorShares(
        address operator,
        IStrategy strategy
    ) internal view returns (uint256 shares) {
        IServiceManager sm = IServiceManager(serviceManager);
        IStrategyManager strategyManager = sm.strategyManager();
        return strategyManager.stakerDepositShares(operator, strategy);
    }

    function _getTotalMagnitude(
        IStrategy strategy,
        address operator
    ) internal view returns (uint256) {
        return
            uint256(
                allocationManager.getEncumberedMagnitude(operator, strategy)
            ) +
            uint256(
                allocationManager.getAllocatableMagnitude(operator, strategy)
            );
    }

    function _getCurrentMagnitudeForAVS(
        IStrategy strategy,
        address operator
    ) internal view returns (uint256) {
        OperatorSet memory set_ = OperatorSet({
            avs: serviceManager,
            id: operatorSetId
        });
        return
            uint256(
                allocationManager
                    .getAllocation(operator, set_, strategy)
                    .currentMagnitude
            );
    }

    function _allocatedUnderlyingToAVS(
        address operator,
        IStrategy strategy
    ) internal view returns (uint256) {
        uint256 curMag = _getCurrentMagnitudeForAVS(strategy, operator);
        if (curMag == 0) return 0;

        uint256 totalShares = _getOperatorShares(operator, strategy);
        uint256 allocatedShares = Math.mulDiv(totalShares, curMag, 1e9);
        return strategy.sharesToUnderlyingView(allocatedShares);
    }

    function _requireAllocatedAtLeastUnderlying(
        address operator,
        IStrategy strategy,
        uint256 requiredUnderlying
    ) internal view {
        uint256 allocatedUnderlying = _allocatedUnderlyingToAVS(
            operator,
            strategy
        );
        if (allocatedUnderlying < requiredUnderlying)
            revert InsufficientAllocatedMagnitude();
    }

    function getCiphernodeState(
        address operator
    ) external view returns (IBondingManager.CiphernodeState) {
        if (!registeredOperators[operator])
            return IBondingManager.CiphernodeState.REMOVED;
        return
            operators[operator].isActive
                ? IBondingManager.CiphernodeState.ACTIVE
                : IBondingManager.CiphernodeState.REGISTERED_INACTIVE;
    }
}

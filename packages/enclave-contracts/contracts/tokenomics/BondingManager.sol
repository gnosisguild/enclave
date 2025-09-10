// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { ReentrancyGuard } from "@oz/utils/ReentrancyGuard.sol";
import { Ownable } from "@oz/access/Ownable.sol";
import { Math } from "@oz/utils/math/Math.sol";
import { IERC20 } from "@oz/token/ERC20/IERC20.sol";
import { SafeERC20 } from "@oz/token/ERC20/utils/SafeERC20.sol";
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
    OperatorSet
} from "eigenlayer-contracts/src/contracts/libraries/OperatorSetLib.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";

contract BondingManager is Ownable, ReentrancyGuard, IBondingManager {
    using SafeERC20 for IERC20;

    uint256 private constant BPS_DENOM = 10_000;
    uint256 private constant INACTIVE_AT_BPS = 9_500; // 95%

    uint256 public minTicketBalance;
    uint256 public ticketPrice;
    IStrategy public enclStrategy;
    IERC20 public usdcToken;
    uint256 public licenseStake;
    uint32 public operatorSetId;

    uint32 private nextTicketId = 1;
    mapping(uint32 ticketId => Ticket ticket) public tickets;
    mapping(address operator => uint32[] ticketIds) public operatorTickets;
    mapping(address operator => OperatorInfo info) public operators;
    mapping(address operator => uint256 availableTickets)
        public availableTicketCount;

    ICiphernodeRegistry public ciphernodeRegistry;
    IDelegationManager public delegationManager;
    IAllocationManager public allocationManager;
    address public serviceManager;

    modifier onlyServiceManager() {
        require(msg.sender == serviceManager, OnlyServiceManager());
        _;
    }

    constructor(
        address _owner,
        address _serviceManager,
        IDelegationManager _delegationManager,
        IAllocationManager _allocationManager,
        address _ciphernodeRegistry,
        IStrategy _enclStrategy,
        IERC20 _usdcToken,
        uint256 _licenseStake,
        uint256 _ticketPrice,
        uint32 _operatorSetId
    ) Ownable(_owner) {
        require(_serviceManager != address(0), ZeroAddress());
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        require(address(_delegationManager) != address(0), ZeroAddress());
        require(address(_allocationManager) != address(0), ZeroAddress());
        require(address(_enclStrategy) != address(0), ZeroAddress());
        require(address(_usdcToken) != address(0), ZeroAddress());
        require(_licenseStake != 0, InsufficientLicenseStake());
        require(_ticketPrice != 0, InvalidTicketPrice());

        serviceManager = _serviceManager;
        delegationManager = _delegationManager;
        allocationManager = _allocationManager;
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        enclStrategy = _enclStrategy;
        usdcToken = _usdcToken;
        licenseStake = _licenseStake;
        ticketPrice = _ticketPrice;
        operatorSetId = _operatorSetId;
        minTicketBalance = 1;
    }

    function acquireLicense() external nonReentrant {
        require(
            delegationManager.isOperator(msg.sender),
            OperatorNotRegistered()
        );
        require(!operators[msg.sender].isLicensed, AlreadyLicensed());

        uint256 enclShares = _getOperatorShares(msg.sender, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);
        require(enclAmount >= licenseStake, InsufficientLicenseStake());

        _requireAllocatedAtLeastUnderlying(
            msg.sender,
            enclStrategy,
            licenseStake
        );

        operators[msg.sender] = OperatorInfo({
            isLicensed: true,
            licenseStake: enclAmount,
            registeredAt: block.timestamp,
            isActive: false,
            isRegistered: false
        });

        emit LicenseAcquired(msg.sender, enclAmount);
    }

    function purchaseTickets(uint256 ticketCount) external nonReentrant {
        require(ticketCount != 0, InvalidTicketAmount());
        require(operators[msg.sender].isLicensed, NotLicensed());

        uint256 totalCost = ticketCount * ticketPrice;
        require(ticketPrice <= type(uint96).max, InvalidTicketPrice());

        usdcToken.safeTransferFrom(msg.sender, address(this), totalCost);

        for (uint256 i = 0; i < ticketCount; i++) {
            uint32 ticketId = nextTicketId++;
            tickets[ticketId] = Ticket({
                createdAt: uint64(block.timestamp),
                originalValue: uint96(ticketPrice),
                slashedAmount: 0,
                operator: msg.sender,
                id: ticketId,
                isUsed: false,
                status: TicketStatus.Active
            });
            operatorTickets[msg.sender].push(ticketId);
        }

        availableTicketCount[msg.sender] += ticketCount;

        uint32 firstTicketId = nextTicketId - uint32(ticketCount);

        if (
            operators[msg.sender].isRegistered &&
            !operators[msg.sender].isActive &&
            availableTicketCount[msg.sender] >= minTicketBalance
        ) {
            operators[msg.sender].isActive = true;
            emit CiphernodeActivated(msg.sender);
        }

        emit TicketsPurchased(
            msg.sender,
            firstTicketId,
            ticketCount,
            totalCost
        );
    }

    function topUpTicket(
        uint32 ticketId,
        uint256 usdcAmount
    ) external nonReentrant {
        require(usdcAmount != 0, InvalidTicketAmount());
        Ticket storage t = tickets[ticketId];
        require(t.operator == msg.sender, TicketNotFound());
        require(t.status != TicketStatus.Burned, TicketAlreadyInactive());
        require(!t.isUsed, InvalidTicketAmount());

        usdcToken.safeTransferFrom(msg.sender, address(this), usdcAmount);

        require(usdcAmount <= type(uint96).max, InvalidTicketAmount());
        t.originalValue += uint96(usdcAmount);

        if (t.status == TicketStatus.Inactive) {
            if (
                uint256(t.slashedAmount) * BPS_DENOM <
                uint256(t.originalValue) * INACTIVE_AT_BPS
            ) {
                t.status = TicketStatus.Active;
                availableTicketCount[msg.sender]++;
                if (
                    operators[msg.sender].isRegistered &&
                    !operators[msg.sender].isActive &&
                    availableTicketCount[msg.sender] >= 1
                ) {
                    operators[msg.sender].isActive = true;
                    emit CiphernodeActivated(msg.sender);
                }
                emit TicketStatusChanged(
                    msg.sender,
                    ticketId,
                    TicketStatus.Active
                );
            }
        }

        emit TicketToppedUp(msg.sender, ticketId, usdcAmount);
    }

    function registerCiphernode() external nonReentrant {
        require(!operators[msg.sender].isRegistered, AlreadyRegistered());
        require(operators[msg.sender].isLicensed, NotLicensed());
        require(
            delegationManager.isOperator(msg.sender),
            OperatorNotRegistered()
        );

        uint256 encl = enclStrategy.sharesToUnderlyingView(
            _getOperatorShares(msg.sender, enclStrategy)
        );
        require(encl >= licenseStake, InsufficientLicenseStake());
        _requireAllocatedAtLeastUnderlying(
            msg.sender,
            enclStrategy,
            licenseStake
        );

        operators[msg.sender].isRegistered = true;
        ciphernodeRegistry.addCiphernode(msg.sender);

        bool activeNow = availableTicketCount[msg.sender] >= 1;
        operators[msg.sender].isActive = activeNow;
        if (activeNow) emit CiphernodeActivated(msg.sender);

        emit CiphernodeRegistered(msg.sender);
    }

    function deregisterCiphernode(
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        require(operators[msg.sender].isRegistered, OperatorNotRegistered());

        if (operators[msg.sender].isActive) {
            operators[msg.sender].isActive = false;
            emit CiphernodeDeactivated(msg.sender);
        }

        operators[msg.sender].isRegistered = false;
        ciphernodeRegistry.removeCiphernode(msg.sender, siblingNodes);
        emit CiphernodeDeregistered(msg.sender);
    }

    function useTicket(address operator, uint32 ticketId) external {
        require(msg.sender == address(ciphernodeRegistry), OnlyRegistry());
        require(tickets[ticketId].operator == operator, TicketNotFound());
        require(
            tickets[ticketId].status == TicketStatus.Active,
            TicketAlreadyInactive()
        );
        require(!tickets[ticketId].isUsed, InvalidTicketAmount());

        tickets[ticketId].isUsed = true;
        tickets[ticketId].status = TicketStatus.Burned;
        availableTicketCount[operator]--;

        emit TicketUsed(operator, ticketId);
        emit TicketStatusChanged(operator, ticketId, TicketStatus.Burned);

        if (availableTicketCount[operator] == 0) {
            operators[operator].isActive = false;
            emit CiphernodeDeactivated(operator);
        }
    }

    function slashTicket(
        address operator,
        uint32 ticketId,
        uint256 wadToSlash
    ) external onlyServiceManager {
        require(wadToSlash <= 1e18 && wadToSlash > 0, InvalidTicketAmount());
        Ticket storage t = tickets[ticketId];
        require(t.operator == operator, TicketNotFound());
        require(t.status == TicketStatus.Active, TicketAlreadyInactive());

        uint256 slashAmount = Math.mulDiv(t.originalValue, wadToSlash, 1e18);
        if (slashAmount == 0) return;

        uint256 remaining = t.originalValue - t.slashedAmount;
        if (slashAmount > remaining) slashAmount = remaining;

        t.slashedAmount += uint96(slashAmount);
        emit TicketSlashed(operator, ticketId, slashAmount);

        if (t.slashedAmount == t.originalValue) {
            t.isUsed = true;
            t.status = TicketStatus.Burned;
            availableTicketCount[operator]--;
            emit TicketStatusChanged(operator, ticketId, TicketStatus.Burned);
        } else if (
            uint256(t.slashedAmount) * BPS_DENOM >=
            uint256(t.originalValue) * INACTIVE_AT_BPS
        ) {
            t.status = TicketStatus.Inactive;
            availableTicketCount[operator]--;
            emit TicketStatusChanged(operator, ticketId, TicketStatus.Inactive);
        }

        if (
            operators[operator].isRegistered &&
            operators[operator].isActive &&
            availableTicketCount[operator] == 0
        ) {
            operators[operator].isActive = false;
            emit CiphernodeDeactivated(operator);
        }
    }

    // Legacy function for ServiceManager compatibility - slashes all active tickets
    function slashTickets(
        address operator,
        uint256 wadToSlash
    ) external onlyServiceManager {
        require(wadToSlash <= 1e18 && wadToSlash > 0, InvalidTicketAmount());

        uint32[] memory ticketIds = operatorTickets[operator];

        for (uint256 i = 0; i < ticketIds.length; i++) {
            _processTicketSlashing(operator, ticketIds[i], wadToSlash);
        }

        availableTicketCount[operator] = _countAvailableTickets(operator);
        _checkAndDeactivateOperator(operator);
    }

    function _processTicketSlashing(
        address operator,
        uint32 ticketId,
        uint256 wadToSlash
    ) internal {
        Ticket storage ticket = tickets[ticketId];

        if (ticket.status != TicketStatus.Active || ticket.isUsed) return;

        uint256 slashAmount = Math.mulDiv(
            ticket.originalValue,
            wadToSlash,
            1e18
        );
        uint256 remaining = ticket.originalValue - ticket.slashedAmount;
        if (slashAmount > remaining) slashAmount = remaining;
        ticket.slashedAmount += uint96(slashAmount);
        emit TicketSlashed(operator, ticketId, slashAmount);

        _updateTicketStatusAfterSlashing(operator, ticketId, ticket);
    }

    function _updateTicketStatusAfterSlashing(
        address operator,
        uint32 ticketId,
        Ticket storage ticket
    ) internal {
        if (ticket.slashedAmount == ticket.originalValue) {
            ticket.isUsed = true;
            ticket.status = TicketStatus.Burned;
            emit TicketStatusChanged(operator, ticketId, TicketStatus.Burned);
        } else if (
            uint256(ticket.slashedAmount) * BPS_DENOM >=
            uint256(ticket.originalValue) * INACTIVE_AT_BPS
        ) {
            ticket.status = TicketStatus.Inactive;
            emit TicketStatusChanged(operator, ticketId, TicketStatus.Inactive);
        }
    }

    function _countAvailableTickets(
        address operator
    ) internal view returns (uint256) {
        uint32[] memory ticketIds = operatorTickets[operator];
        uint256 newCount = 0;
        for (uint256 i = 0; i < ticketIds.length; i++) {
            Ticket storage ticket = tickets[ticketIds[i]];
            if (
                ticket.status == TicketStatus.Active &&
                !ticket.isUsed &&
                uint256(ticket.slashedAmount) * BPS_DENOM <
                uint256(ticket.originalValue) * INACTIVE_AT_BPS
            ) {
                newCount++;
            }
        }
        return newCount;
    }

    function _checkAndDeactivateOperator(address operator) internal {
        if (
            operators[operator].isRegistered &&
            operators[operator].isActive &&
            availableTicketCount[operator] == 0
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
            if (operators[operator].isRegistered) {
                operators[operator].isRegistered = false;
                emit CiphernodeDeregistered(operator);
            }
            emit LicenseRevoked(operator);
        }
    }

    function setMinTicketBalance(uint256 _minTicketBalance) external onlyOwner {
        require(_minTicketBalance != 0, InvalidMinTicketBalance());
        minTicketBalance = _minTicketBalance;
        emit MinTicketBalanceUpdated(_minTicketBalance);
    }

    function setLicenseStake(uint256 _licenseStake) external onlyOwner {
        require(_licenseStake != 0, InsufficientLicenseStake());
        licenseStake = _licenseStake;
        emit LicenseStakeUpdated(_licenseStake);
    }

    function setTicketPrice(uint256 _ticketPrice) external onlyOwner {
        require(_ticketPrice != 0, InvalidTicketPrice());
        ticketPrice = _ticketPrice;
        emit TicketPriceUpdated(_ticketPrice);
    }

    function withdrawUSDC(address to, uint256 amount) external onlyOwner {
        require(to != address(0), ZeroAddress());
        usdcToken.safeTransfer(to, amount);
        emit USDCWithdrawn(to, amount);
    }

    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory) {
        return operators[operator];
    }

    function getTotalActiveTicketBalance(
        address operator
    ) public view returns (uint256) {
        return availableTicketCount[operator];
    }

    function getAvailableTicketCount(
        address operator
    ) public view returns (uint256) {
        return availableTicketCount[operator];
    }

    function getAvailableTickets(
        address operator
    ) external view returns (uint32[] memory) {
        uint32[] memory ticketIds = operatorTickets[operator];
        uint32[] memory tempAvailable = new uint32[](ticketIds.length);
        uint256 count = 0;

        for (uint256 i = 0; i < ticketIds.length; i++) {
            Ticket memory ticket = tickets[ticketIds[i]];
            if (
                ticket.status == TicketStatus.Active &&
                !ticket.isUsed &&
                uint256(ticket.slashedAmount) * BPS_DENOM <
                uint256(ticket.originalValue) * INACTIVE_AT_BPS
            ) {
                tempAvailable[count] = ticketIds[i];
                count++;
            }
        }

        uint32[] memory availableTickets = new uint32[](count);
        for (uint256 i = 0; i < count; i++) {
            availableTickets[i] = tempAvailable[i];
        }

        return availableTickets;
    }

    function getTicket(uint32 ticketId) external view returns (Ticket memory) {
        return tickets[ticketId];
    }

    function getOperatorTickets(
        address operator
    ) external view returns (uint32[] memory) {
        return operatorTickets[operator];
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
        return operators[operator].isRegistered;
    }

    function isActive(address operator) external view returns (bool) {
        return operators[operator].isActive;
    }

    function _getOperatorShares(
        address operator,
        IStrategy strategy
    ) internal view returns (uint256 shares) {
        IStrategy[] memory arr = new IStrategy[](1);
        arr[0] = strategy;
        uint256[] memory ops = delegationManager.getOperatorShares(
            operator,
            arr
        );
        return ops[0];
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
        require(
            allocatedUnderlying >= requiredUnderlying,
            InsufficientAllocatedMagnitude()
        );
    }

    function getCiphernodeState(
        address operator
    ) external view returns (IBondingManager.CiphernodeState) {
        if (!operators[operator].isRegistered)
            return IBondingManager.CiphernodeState.REMOVED;
        return
            operators[operator].isActive
                ? IBondingManager.CiphernodeState.ACTIVE
                : IBondingManager.CiphernodeState.REGISTERED_INACTIVE;
    }
}

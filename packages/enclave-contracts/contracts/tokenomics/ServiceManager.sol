// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import {
    ServiceManagerBase
} from "@eigenlayer-middleware/src/ServiceManagerBase.sol";
import {
    IStakeRegistry
} from "@eigenlayer-middleware/src/interfaces/IStakeRegistry.sol";
import {
    IRewardsCoordinator
} from "eigenlayer-contracts/src/contracts/interfaces/IRewardsCoordinator.sol";
import {
    IPermissionController
} from "eigenlayer-contracts/src/contracts/interfaces/IPermissionController.sol";
import {
    ISlashingRegistryCoordinator
} from "@eigenlayer-middleware/src/interfaces/ISlashingRegistryCoordinator.sol";
import {
    IStrategy
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";
import {
    IAllocationManager,
    IAllocationManagerTypes
} from "eigenlayer-contracts/src/contracts/interfaces/IAllocationManager.sol";
import {
    IAVSRegistrar
} from "eigenlayer-contracts/src/contracts/interfaces/IAVSRegistrar.sol";
import {
    IDelegationManager
} from "eigenlayer-contracts/src/contracts/interfaces/IDelegationManager.sol";
import {
    IAVSDirectory
} from "eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {
    IStrategyManager
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategyManager.sol";
import {
    OperatorSet
} from "eigenlayer-contracts/src/contracts/libraries/OperatorSetLib.sol";
import {
    AggregatorV3Interface
} from "@chainlink/contracts/src/v0.8/shared/interfaces/AggregatorV3Interface.sol";
// Need this for deployment of service manager proxy
/* solhint-disable no-unused-import */
import {
    TransparentUpgradeableProxy
} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
/* solhint-enable no-unused-import */
import { ReentrancyGuard } from "@oz/utils/ReentrancyGuard.sol";
import { IERC20Metadata } from "@oz/token/ERC20/extensions/IERC20Metadata.sol";
import { SafeERC20 } from "@oz/token/ERC20/utils/SafeERC20.sol";
import { Math } from "@oz/utils/math/Math.sol";
import { IServiceManager } from "../interfaces/IServiceManager.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";

contract ServiceManager is
    ServiceManagerBase,
    IServiceManager,
    ReentrancyGuard,
    IAVSRegistrar
{
    using SafeERC20 for IERC20Metadata;

    uint256 public constant PRICE_STALENESS_THRESHOLD = 86400;
    uint32 public operatorSetId;
    IStrategy public enclStrategy;

    mapping(IStrategy strategy => StrategyConfig config) public strategyConfigs;
    mapping(IStrategy strategy => uint256 index) private strategyToIndex;
    mapping(address slasher => bool isSlasher) public slashers;

    IStrategy[] public allowedStrategies;
    IStrategyManager public strategyManager;
    IDelegationManager public delegationManager;
    IBondingManager public bondingManager;
    address public rewardDistributor;

    modifier onlyAllocationManager() {
        require(
            msg.sender == address(_allocationManager),
            OnlyAllocationManager()
        );
        _;
    }

    constructor(
        IAVSDirectory _avsDirectory,
        IRewardsCoordinator _rewardsCoordinator,
        ISlashingRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        IPermissionController _permissionController,
        IAllocationManager _allocationManager
    )
        ServiceManagerBase(
            _avsDirectory,
            _rewardsCoordinator,
            _registryCoordinator,
            _stakeRegistry,
            _permissionController,
            _allocationManager
        )
    {
        _disableInitializers();
    }

    function initialize(
        address _owner,
        address _rewardsInitiator,
        IStrategyManager _strategyManager,
        IDelegationManager _delegationManager,
        address _bondingManager,
        IStrategy _enclStrategy,
        uint32 _operatorSetId
    ) external reinitializer(1) {
        require(address(_strategyManager) != address(0), ZeroAddress());
        require(address(_delegationManager) != address(0), ZeroAddress());
        require(address(_enclStrategy) != address(0), ZeroAddress());
        require(_operatorSetId != 0, InvalidOperatorSetId());

        __ServiceManagerBase_init(_owner, _rewardsInitiator);
        strategyManager = _strategyManager;
        delegationManager = _delegationManager;
        bondingManager = IBondingManager(_bondingManager);
        enclStrategy = _enclStrategy;
        operatorSetId = _operatorSetId;

        _addStrategyInternal(_enclStrategy, address(0));
    }

    function addStrategy(
        IStrategy strategy,
        address priceFeed
    ) external onlyOwner {
        _addStrategyInternal(strategy, priceFeed);
    }

    function _addStrategyInternal(
        IStrategy strategy,
        address priceFeed
    ) internal {
        require(address(strategy) != address(0), ZeroAddress());
        require(!strategyConfigs[strategy].isAllowed, StrategyAlreadyAllowed());

        uint8 decimals = IERC20Metadata(address(strategy.underlyingToken()))
            .decimals();

        strategyConfigs[strategy] = StrategyConfig({
            isAllowed: true,
            priceFeed: priceFeed,
            decimals: decimals
        });
        strategyToIndex[strategy] = allowedStrategies.length;
        allowedStrategies.push(strategy);
        emit StrategyAdded(address(strategy), priceFeed);
    }

    function removeStrategy(IStrategy strategy) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());
        require(strategy != enclStrategy, CannotRemoveCoreStrategies());

        uint256 index = strategyToIndex[strategy];
        uint256 lastIndex = allowedStrategies.length - 1;

        if (index != lastIndex) {
            IStrategy lastStrategy = allowedStrategies[lastIndex];
            allowedStrategies[index] = lastStrategy;
            strategyToIndex[lastStrategy] = index;
        }

        allowedStrategies.pop();
        delete strategyToIndex[strategy];
        delete strategyConfigs[strategy];
        emit StrategyRemoved(address(strategy));
    }

    function updateStrategy(
        IStrategy strategy,
        address newPriceFeed
    ) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());
        strategyConfigs[strategy].priceFeed = newPriceFeed;
        emit StrategyUpdated(address(strategy), newPriceFeed);
    }

    function setOperatorSetId(uint32 _operatorSetId) external onlyOwner {
        require(_operatorSetId != 0, InvalidOperatorSetId());
        require(
            operatorSetId == 0 || operatorSetId != _operatorSetId,
            AlreadySetToThisId()
        );
        uint32 previousId = operatorSetId;
        operatorSetId = _operatorSetId;
        emit OperatorSetIdUpdated(previousId, _operatorSetId);
    }

    function setAVSRegistrar(IAVSRegistrar registrar) external onlyOwner {
        _allocationManager.setAVSRegistrar(address(this), registrar);
        emit AVSRegistrarSet(address(registrar));
    }

    function setRewardDistributor(
        address _rewardDistributor
    ) external onlyOwner {
        require(_rewardDistributor != address(0), ZeroAddress());
        rewardDistributor = _rewardDistributor;
    }

    function setBondingManager(address _bondingManager) external onlyOwner {
        require(_bondingManager != address(0), ZeroAddress());
        bondingManager = IBondingManager(_bondingManager);
        emit BondingManagerSet(_bondingManager);
    }

    function publishAVSMetadata(string calldata uri) external onlyOwner {
        _allocationManager.updateAVSMetadataURI(address(this), uri);
    }

    function createOperatorSet(
        uint32 id,
        IStrategy[] calldata strategies
    ) external onlyOwner {
        IAllocationManagerTypes.CreateSetParams[]
            memory params = new IAllocationManagerTypes.CreateSetParams[](1);
        params[0] = IAllocationManagerTypes.CreateSetParams({
            operatorSetId: id,
            strategies: strategies
        });
        _allocationManager.createOperatorSets(address(this), params);
    }

    function addStrategies(
        uint32 id,
        IStrategy[] calldata strategies
    ) external onlyOwner {
        _allocationManager.addStrategiesToOperatorSet(
            address(this),
            id,
            strategies
        );
    }

    function registerOperator(
        address operator,
        address avs,
        uint32[] calldata operatorSetIds,
        bytes calldata
    ) external override onlyAllocationManager {
        require(avs == address(this), InvalidOperatorSet());

        bool ours;
        for (uint256 i = 0; i < operatorSetIds.length; ++i) {
            if (operatorSetIds[i] == operatorSetId) {
                ours = true;
                break;
            }
        }
        require(ours, WrongOperatorSet());
        require(
            delegationManager.isOperator(operator),
            OperatorNotRegistered()
        );

        IBondingManager.OperatorInfo memory bondingInfo = bondingManager
            .getOperatorInfo(operator);
        require(bondingInfo.isLicensed, NotLicensed());

        emit OperatorRegisteredToAVS(operator);
    }

    function deregisterOperator(
        address operator,
        address avs,
        uint32[] calldata operatorSetIds
    ) external override onlyAllocationManager {
        require(avs == address(this), InvalidAVS());

        bool ours;
        for (uint256 i = 0; i < operatorSetIds.length; ++i) {
            if (operatorSetIds[i] == operatorSetId) {
                ours = true;
                break;
            }
        }
        if (!ours) return;

        require(
            !bondingManager.isRegisteredOperator(operator),
            MustDeregisterCiphernodeFirst()
        );

        emit OperatorDeregisteredFromAVS(operator);
    }

    function supportsAVS(address avs) external view returns (bool) {
        return avs == address(this);
    }

    function addSlasher(address slasher) external onlyOwner {
        require(slasher != address(0), ZeroAddress());
        slashers[slasher] = true;
        emit SlasherAdded(slasher);
    }

    function removeSlasher(address slasher) external onlyOwner {
        slashers[slasher] = false;
        emit SlasherRemoved(slasher);
    }

    function slashTicket(
        address operator,
        uint32 ticketId,
        uint256 wadToSlash
    ) external nonReentrant {
        require(slashers[msg.sender], NotAuthorizedSlasher());
        bondingManager.slashTicket(operator, ticketId, wadToSlash);
    }

    function slashOperator(
        address operator,
        uint256 wadToSlash,
        string calldata description
    ) external nonReentrant {
        require(slashers[msg.sender], NotAuthorizedSlasher());
        require(
            bondingManager.isRegisteredOperator(operator),
            OperatorNotRegistered()
        );
        require(wadToSlash <= 1e18 && wadToSlash != 0, InvalidWadSlashing());
        require(allowedStrategies.length != 0, NoStrategiesConfigured());

        IStrategy[] memory strategies = new IStrategy[](1);
        strategies[0] = enclStrategy;
        uint256[] memory wads = new uint256[](1);
        wads[0] = wadToSlash;

        IAllocationManagerTypes.SlashingParams
            memory params = IAllocationManagerTypes.SlashingParams({
                operator: operator,
                operatorSetId: operatorSetId,
                strategies: strategies,
                wadsToSlash: wads,
                description: description
            });

        try _allocationManager.slashOperator(address(this), params) returns (
            uint256,
            uint256[] memory slashedShares
        ) {
            emit OperatorSlashed(
                operator,
                wadToSlash,
                description,
                strategies,
                slashedShares
            );
            bondingManager.updateLicenseStatus(operator);
        } catch Error(string memory errorMsg) {
            revert SlashingFailed(errorMsg);
        }
    }

    // TODO: do we need a fee for this?
    // TODO: do we need to provide the strategies for the rewards? i.e multiple strategies or just the encl strategy?
    function distributeRewards(
        address[] calldata recipients,
        uint256[] calldata amounts
    ) external {
        require(msg.sender == rewardDistributor, OnlyRewardDistributor());
        require(recipients.length == amounts.length, ArrayLengthMismatch());

        IERC20Metadata enclToken = IERC20Metadata(
            address(enclStrategy.underlyingToken())
        );
        for (uint256 i = 0; i < recipients.length; i++) {
            if (
                amounts[i] > 0 &&
                bondingManager.isRegisteredOperator(recipients[i])
            ) {
                enclToken.safeTransferFrom(
                    rewardDistributor,
                    recipients[i],
                    amounts[i]
                );
            }
        }
    }

    function getOperatorShares(
        address operator,
        IStrategy strategy
    ) public view returns (uint256 shares) {
        IStrategy[] memory strategies = new IStrategy[](1);
        strategies[0] = strategy;
        uint256[] memory operatorShares = delegationManager.getOperatorShares(
            operator,
            strategies
        );
        return operatorShares[0];
    }

    function getAllocatedMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        OperatorSet memory operatorSet = OperatorSet({
            avs: address(this),
            id: operatorSetId
        });
        return
            uint256(
                _allocationManager
                    .getAllocation(operator, operatorSet, strategy)
                    .currentMagnitude
            );
    }

    function getTotalMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        return
            uint256(
                _allocationManager.getEncumberedMagnitude(operator, strategy)
            ) +
            uint256(
                _allocationManager.getAllocatableMagnitude(operator, strategy)
            );
    }

    function isStrategyAllowed(
        IStrategy strategy
    ) external view returns (bool) {
        return strategyConfigs[strategy].isAllowed;
    }

    function getAllowedStrategies() external view returns (IStrategy[] memory) {
        return allowedStrategies;
    }

    function getStrategyConfig(
        IStrategy strategy
    ) external view returns (address) {
        return strategyConfigs[strategy].priceFeed;
    }

    function getOperatorSetInfo() external view returns (uint32, address) {
        return (operatorSetId, address(this));
    }

    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory) {
        bool active = bondingManager.isActive(operator);
        return
            OperatorInfo({
                isActive: active,
                registeredAt: 0,
                collateralUsd: 0
            });
    }

    function _requireAllocatedAtLeast(
        address operator,
        IStrategy strategy,
        uint256 requiredUnderlying
    ) internal view {
        OperatorSet memory set_ = OperatorSet({
            avs: address(this),
            id: operatorSetId
        });

        uint256 totalMag = uint256(
            _allocationManager.getEncumberedMagnitude(operator, strategy)
        ) +
            uint256(
                _allocationManager.getAllocatableMagnitude(operator, strategy)
            );
        require(totalMag != 0, InsufficientAllocatedMagnitude());

        uint256 curMag = uint256(
            _allocationManager
                .getAllocation(operator, set_, strategy)
                .currentMagnitude
        );
        uint256 totalShares = getOperatorShares(operator, strategy);
        uint256 allocatedShares = Math.mulDiv(totalShares, curMag, totalMag);
        uint256 allocatedUnderlying = strategy.sharesToUnderlyingView(
            allocatedShares
        );

        require(
            allocatedUnderlying >= requiredUnderlying,
            InsufficientAllocatedMagnitude()
        );
    }

    function _convertSharesToUsd(
        IStrategy strategy,
        uint256 shares
    ) internal view returns (uint256 usdValue) {
        StrategyConfig memory config = strategyConfigs[strategy];
        uint256 underlyingAmount = strategy.sharesToUnderlyingView(shares);

        if (config.priceFeed == address(0)) {
            usdValue = Math.mulDiv(
                underlyingAmount,
                1e18,
                10 ** config.decimals
            );
        } else {
            uint256 price = _getTokenPrice(config.priceFeed);
            usdValue = Math.mulDiv(
                underlyingAmount,
                price,
                10 ** config.decimals
            );
        }
    }

    function _getTokenPrice(
        address priceFeed
    ) internal view returns (uint256 price) {
        AggregatorV3Interface feed = AggregatorV3Interface(priceFeed);

        try feed.latestRoundData() returns (
            uint80,
            int256 answer,
            uint256,
            uint256 updatedAt,
            uint80
        ) {
            require(
                answer > 0 &&
                    block.timestamp - updatedAt <= PRICE_STALENESS_THRESHOLD,
                InvalidPriceOrStale()
            );

            uint8 feedDecimals = feed.decimals();
            uint256 rawPrice = uint256(answer);

            if (feedDecimals < 18) {
                return rawPrice * (10 ** (18 - feedDecimals));
            } else if (feedDecimals > 18) {
                return rawPrice / (10 ** (feedDecimals - 18));
            } else {
                return rawPrice;
            }
        } catch {
            revert PriceFeedError();
        }
    }
}

// SPDX-License-Identifier: BUSL-1.1
pragma solidity ^0.8.12;

import { Script } from "forge-std/Script.sol";

import {
    CoreDeployLib,
    CoreDeploymentParsingLib
} from "./utils/CoreDeploymentParsingLib.sol";
import { UpgradeableProxyLib } from "./utils/UpgradeableProxyLib.sol";
import {
    MiddlewareDeployLib
} from "../../lib/eigenlayer-middleware/test/utils/MiddlewareDeployLib.sol";

import {
    IRewardsCoordinator
} from "eigenlayer-contracts/src/contracts/interfaces/IRewardsCoordinator.sol";
import {
    StrategyManager
} from "eigenlayer-contracts/src/contracts/core/StrategyManager.sol";

import "forge-std/Test.sol";

contract DeployEigenLayerCore is Script, Test {
    using CoreDeployLib for *;
    using UpgradeableProxyLib for address;

    address internal deployer;
    address internal proxyAdmin;
    CoreDeployLib.DeploymentData internal deploymentData;
    CoreDeployLib.DeploymentConfigData internal configData;
    MiddlewareDeployLib.MiddlewareDeployData internal middlewareData;

    function setUp() public virtual {
        deployer = vm.rememberKey(vm.envUint("PRIVATE_KEY"));
        vm.label(deployer, "Deployer");
    }

    function run() external {
        vm.startBroadcast(deployer);
        //set the rewards updater to the deployer address for payment flow
        configData = CoreDeploymentParsingLib.readDeploymentConfigValues(
            "deploy/eigenlayer/utils/config/",
            block.chainid
        );
        configData.rewardsCoordinator.rewardsUpdater = deployer;
        proxyAdmin = UpgradeableProxyLib.deployProxyAdmin();
        deploymentData = CoreDeployLib.deployContracts(proxyAdmin, configData);

        // TODO: the deployer lib should probably do this
        StrategyManager(deploymentData.strategyManager).setStrategyWhitelister(
            deploymentData.strategyFactory
        );

        // Deploy middleware components
        MiddlewareDeployLib.MiddlewareDeployConfig
            memory middlewareConfig = _getMiddlewareConfig();
        middlewareData = MiddlewareDeployLib.deployMiddleware(
            proxyAdmin,
            deploymentData.allocationManager,
            deploymentData.strategyManager,
            deploymentData.pauserRegistry,
            middlewareConfig
        );

        vm.stopBroadcast();
        string memory deploymentPath = "deployments/core/";
        CoreDeploymentParsingLib.writeDeploymentJson(
            deploymentPath,
            block.chainid,
            deploymentData,
            middlewareData
        );
    }

    function _getMiddlewareConfig()
        internal
        view
        returns (MiddlewareDeployLib.MiddlewareDeployConfig memory)
    {
        // Create minimal middleware config for local testing
        MiddlewareDeployLib.MiddlewareDeployConfig memory config;

        config.instantSlasher.initialOwner = deployer;
        config.instantSlasher.slasher = deployer;

        config.slashingRegistryCoordinator.initialOwner = deployer;
        config.slashingRegistryCoordinator.churnApprover = deployer;
        config.slashingRegistryCoordinator.ejector = deployer;
        config.slashingRegistryCoordinator.initPausedStatus = 0;
        config.slashingRegistryCoordinator.serviceManager = address(0); // Will be set later

        config.socketRegistry.initialOwner = deployer;
        config.indexRegistry.initialOwner = deployer;

        config.stakeRegistry.initialOwner = deployer;
        config.stakeRegistry.minimumStake = 1e18;
        config.stakeRegistry.strategyParams = 0;
        config.stakeRegistry.delegationManager = deploymentData
            .delegationManager;
        config.stakeRegistry.avsDirectory = deploymentData.avsDirectory;
        config.stakeRegistry.lookAheadPeriod = 1;

        config.blsApkRegistry.initialOwner = deployer;

        return config;
    }
}

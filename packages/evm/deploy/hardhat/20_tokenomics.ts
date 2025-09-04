// SPDX-License-Identifier: LGPL-3.0-only
import "@nomicfoundation/hardhat-ethers";
import { ethers } from "ethers";
import "hardhat-deploy";
import { DeployFunction } from "hardhat-deploy/types";
import type { HardhatRuntimeEnvironment } from "hardhat/types";

import { CONFIG, loadEigenLayerDeployment } from "./_helpers";

const ENV = {
  ENCL_TOKEN: process.env.ENCL_TOKEN,
  USDC_TOKEN: process.env.USDC_TOKEN,
  ENCL_STRATEGY: process.env.ENCL_STRATEGY,
  USDC_STRATEGY: process.env.USDC_STRATEGY,
};

const func: DeployFunction = async (hre: HardhatRuntimeEnvironment) => {
  const { deploy } = hre.deployments;
  const { deployer } = await hre.getNamedAccounts();
  const chainId = parseInt(await hre.getChainId());
  const eigen = loadEigenLayerDeployment(chainId);

  // 1) Tokens
  let enclToken =
    ENV.ENCL_TOKEN ?? (await hre.deployments.getOrNull("EnclToken"))?.address;
  let usdcToken =
    ENV.USDC_TOKEN ?? (await hre.deployments.getOrNull("UsdcToken"))?.address;

  if (!hre.network.live) {
    // local mocks
    if (!enclToken) {
      enclToken = (
        await deploy("EnclaveToken", {
          from: deployer,
          args: [deployer],
          log: true,
          contract: "EnclaveToken",
        })
      ).address;
    }
    if (!usdcToken) {
      usdcToken = (
        await deploy("UsdcToken", {
          from: deployer,
          args: ["USD Coin", "USDC", 6],
          log: true,
          contract: "contracts/test/TestTokens.sol:MockERC20",
        })
      ).address;
    }
  }

  if (!enclToken || !usdcToken) {
    throw new Error(
      "Token addresses missing. Provide ENCL_TOKEN/USDC_TOKEN or run on local.",
    );
  }

  // 2) Strategies
  let enclStrategy =
    ENV.ENCL_STRATEGY ??
    (await hre.deployments.getOrNull("EnclStrategy"))?.address;
  let usdcStrategy =
    ENV.USDC_STRATEGY ??
    (await hre.deployments.getOrNull("UsdcStrategy"))?.address;

  if (!enclStrategy || !usdcStrategy) {
    const strategyFactory = await hre.ethers.getContractAt(
      [
        "function deployNewStrategy(address token) external returns (address)",
        "function deployedStrategies(address token) external view returns (address)",
      ],
      eigen.strategyFactory,
    );

    const ensureStrategy = async (token: string) => {
      let s = await strategyFactory.deployedStrategies(token);
      if (s === ethers.ZeroAddress) {
        await (
          await strategyFactory.deployNewStrategy(token, {
            gasLimit: 3_000_000,
          })
        ).wait();
        s = await strategyFactory.deployedStrategies(token);
      }
      return s;
    };

    if (!enclStrategy) enclStrategy = await ensureStrategy(enclToken);
    if (!usdcStrategy) usdcStrategy = await ensureStrategy(usdcToken);

    await hre.deployments.save("EnclStrategy", {
      address: enclStrategy!,
      abi: [],
    });
    await hre.deployments.save("UsdcStrategy", {
      address: usdcStrategy!,
      abi: [],
    });
  }

  // 3) ServiceManager & BondingManager
  const registryAddr = (await hre.deployments.get("CiphernodeRegistryOwnable"))
    .address;

  const impl = await deploy("ServiceManagerImplementation", {
    from: deployer,
    contract: "ServiceManager",
    args: [
      eigen.avsDirectory,
      eigen.rewardsCoordinator,
      eigen.slashingRegistryCoordinator,
      eigen.stakeRegistry,
      eigen.permissionController,
      eigen.allocationManager,
    ],
    log: true,
  });

  const iface = (await hre.ethers.getContractFactory("ServiceManager"))
    .interface;
  const initData = iface.encodeFunctionData("initialize", [
    deployer,
    deployer,
    eigen.strategyManager,
    eigen.delegationManager,
    CONFIG.addresses.addressOne, // BondingManager needs to be updated
    enclStrategy,
    usdcStrategy,
    CONFIG.tokenomics.minCollateralUsd,
    CONFIG.tokenomics.operatorSetId,
  ]);

  const smProxy = await deploy("ServiceManager_Proxy", {
    contract: "TransparentUpgradeableProxy",
    from: deployer,
    args: [impl.address, eigen.proxyAdmin, initData],
    log: true,
  });

  const bonding = await deploy("BondingManager", {
    from: deployer,
    contract: "BondingManager",
    args: [
      deployer,
      smProxy.address,
      eigen.delegationManager,
      eigen.allocationManager,
      registryAddr,
      enclStrategy,
      usdcStrategy,
      CONFIG.tokenomics.licenseStake,
      CONFIG.tokenomics.ticketPrice,
      CONFIG.tokenomics.operatorSetId,
    ],
    log: true,
  });

  // Save canonical names for post step
  await hre.deployments.save("ServiceManager", {
    address: smProxy.address,
    abi: (await hre.artifacts.readArtifact("ServiceManager")).abi,
  });
  await hre.deployments.save("BondingManager", {
    address: bonding.address,
    abi: (await hre.artifacts.readArtifact("BondingManager")).abi,
  });

  // Deploy Vesting Contract?
  if (hre.network.live && process.env.DEPLOY_VESTING) {
    const vestingEscrow = await deploy("VestingEscrow", {
      from: deployer,
      args: [enclToken, deployer],
      log: true,
    });
    await hre.deployments.save("VestingEscrow_gov", {
      address: vestingEscrow.address,
      abi: [],
    });
  }

  // Save Addresses
  if (enclToken)
    await hre.deployments.save("EnclToken", { address: enclToken, abi: [] });
  if (usdcToken)
    await hre.deployments.save("UsdcToken", { address: usdcToken, abi: [] });

  console.log("Tokenomics bundle deployed:", {
    serviceManager: smProxy.address,
    bondingManager: bonding.address,
    enclStrategy,
    usdcStrategy,
    enclToken,
    usdcToken,
  });
};
export default func;
func.tags = ["tokenomics"];
func.dependencies = ["enclave"];

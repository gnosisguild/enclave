// SPDX-License-Identifier: LGPL-3.0-only
import "@nomicfoundation/hardhat-ethers";
import { ethers } from "ethers";
import fs from "fs";
import "hardhat-deploy";
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";
import path from "path";
import { PoseidonT3, proxy } from "poseidon-solidity";

// ============================================================================
// CONFIGURATION
// ============================================================================

const CONFIG = {
  // Enclave Configuration
  enclave: {
    commitmentDuration: 60 * 60 * 24 * 30, // 30 days in seconds
    polynomialDegree: 2048n,
    plaintextModulus: 1032193n,
    moduli: [18014398492704769n],
  },

  // Tokenomics Configuration
  tokenomics: {
    licenseStake: ethers.parseEther("100"),
    ticketPrice: ethers.parseUnits("10", 6),
    minCollateralUsd: ethers.parseEther("1000"),
    operatorSetId: 1,
  },

  // Contract addresses
  addresses: {
    addressOne: "0x0000000000000000000000000000000000000001",
  },
} as const;

// ============================================================================
// UTILITIES
// ============================================================================

interface DeploymentAddresses {
  enclave: string;
  registry: string;
  filter: string;
  serviceManager?: string;
  bondingManager?: string;
  enclaveToken?: string;
  vestingEscrow?: string;
  enclToken: string;
  usdcToken: string;
  enclStrategy: string;
  usdcStrategy: string;
}

function loadEigenLayerDeployment(chainId: number) {
  const deploymentPath = path.join(
    __dirname,
    "../..",
    "deployments",
    "core",
    `${chainId}.json`,
  );

  if (!fs.existsSync(deploymentPath)) {
    throw new Error(
      `EigenLayer core deployment not found at ${deploymentPath}. Run EigenLayer core deployment first.`,
    );
  }

  return JSON.parse(fs.readFileSync(deploymentPath, "utf8")).addresses;
}

async function saveDeploymentMetadata(
  hre: HardhatRuntimeEnvironment,
  addresses: DeploymentAddresses,
  eigenLayerAddresses?: any,
) {
  const chainId = await hre.getChainId();

  // Convert BigInt values to strings for JSON serialization
  const serializableConfig = {
    enclave: {
      commitmentDuration: CONFIG.enclave.commitmentDuration,
      polynomialDegree: CONFIG.enclave.polynomialDegree.toString(),
      plaintextModulus: CONFIG.enclave.plaintextModulus.toString(),
      moduli: CONFIG.enclave.moduli.map((m) => m.toString()),
    },
    tokenomics: {
      licenseStake: CONFIG.tokenomics.licenseStake.toString(),
      ticketPrice: CONFIG.tokenomics.ticketPrice.toString(),
      minCollateralUsd: CONFIG.tokenomics.minCollateralUsd.toString(),
      operatorSetId: CONFIG.tokenomics.operatorSetId,
    },
    addresses: CONFIG.addresses,
  };

  const deploymentMetadata = {
    network: hre.network.name,
    chainId,
    deployer: addresses.enclave, // Using enclave address as deployer reference
    timestamp: new Date().toISOString(),
    contracts: {
      // Core contracts
      enclave: addresses.enclave,
      registry: addresses.registry,
      filter: addresses.filter,

      // Tokenomics contracts (if deployed)
      ...(addresses.serviceManager && {
        serviceManager: addresses.serviceManager,
        bondingManager: addresses.bondingManager,
        enclaveToken: addresses.enclaveToken,
        vestingEscrow: addresses.vestingEscrow,
        enclToken: addresses.enclToken,
        usdcToken: addresses.usdcToken,
        enclStrategy: addresses.enclStrategy,
        usdcStrategy: addresses.usdcStrategy,
      }),
    },
    ...(eigenLayerAddresses && { eigenLayer: eigenLayerAddresses }),
    config: serializableConfig,
  };

  const metadataPath = path.join(
    __dirname,
    "../..",
    "deployments",
    `deployment-${chainId}.json`,
  );
  fs.writeFileSync(metadataPath, JSON.stringify(deploymentMetadata, null, 2));
  console.log("Deployment metadata saved to:", metadataPath);
}

// ============================================================================
// DEPLOYMENT FUNCTIONS
// ============================================================================

async function deployPoseidonLibraries(hre: HardhatRuntimeEnvironment) {
  console.log("Deploying Poseidon libraries...");

  // Deploy proxy if needed
  if ((await hre.ethers.provider.getCode(proxy.address)) === "0x") {
    const [sender] = await hre.ethers.getSigners();
    await sender!.sendTransaction({
      to: proxy.from,
      value: proxy.gas,
    });
    await hre.ethers.provider.broadcastTransaction(proxy.tx);
    console.log(" Proxy deployed to:", proxy.address);
  }

  // Deploy PoseidonT3 if needed
  if ((await hre.ethers.provider.getCode(PoseidonT3.address)) === "0x") {
    const [sender] = await hre.ethers.getSigners();
    await sender!.sendTransaction({
      to: proxy.address,
      data: PoseidonT3.data,
    });
    console.log(" PoseidonT3 deployed to:", PoseidonT3.address);
  }
}

async function deployCoreContracts(
  hre: HardhatRuntimeEnvironment,
  deploy: any,
  deployer: string,
) {
  console.log("Deploying core contracts...");

  // Encode FHE parameters
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [
      CONFIG.enclave.polynomialDegree,
      CONFIG.enclave.plaintextModulus,
      CONFIG.enclave.moduli,
    ],
  );

  // Deploy Enclave
  const enclave = await deploy("Enclave", {
    from: deployer,
    args: [
      deployer,
      CONFIG.addresses.addressOne,
      CONFIG.addresses.addressOne,
      CONFIG.addresses.addressOne,
      CONFIG.enclave.commitmentDuration,
      [encoded],
    ],
    log: false,
    libraries: { PoseidonT3: PoseidonT3.address },
  });
  console.log(" Enclave:", enclave.address);

  // Deploy Registry
  const registry = await deploy("CiphernodeRegistryOwnable", {
    from: deployer,
    args: [deployer, enclave.address],
    log: false,
    libraries: { PoseidonT3: PoseidonT3.address },
  });
  console.log(" Registry:", registry.address);

  // Deploy Filter
  const filter = await deploy("NaiveRegistryFilter", {
    from: deployer,
    args: [deployer, registry.address],
    log: false,
  });
  console.log(" Filter:", filter.address);

  // Update Enclave with Registry
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclave.address,
  );
  const currentRegistry = await enclaveContract.ciphernodeRegistry();

  if (currentRegistry !== registry.address) {
    await (
      await enclaveContract.setCiphernodeRegistry(registry.address)
    ).wait();
    console.log(" Enclave registry updated");
  }

  return {
    enclave: enclave.address,
    registry: registry.address,
    filter: filter.address,
  };
}

async function deployTestTokens(deploy: any, deployer: string) {
  console.log("Deploying test tokens...");

  const enclToken = await deploy("EnclToken", {
    from: deployer,
    args: ["Enclave Token", "ENCL", 18],
    log: false,
    contract: "contracts/test/TestTokens.sol:MockERC20",
  });

  const usdcToken = await deploy("UsdcToken", {
    from: deployer,
    args: ["USD Coin", "USDC", 6],
    log: false,
    contract: "contracts/test/TestTokens.sol:MockERC20",
  });

  console.log(" ENCL Token:", enclToken.address);
  console.log(" USDC Token:", usdcToken.address);

  return { enclToken: enclToken.address, usdcToken: usdcToken.address };
}

async function deployStrategies(
  hre: HardhatRuntimeEnvironment,
  strategyFactoryAddress: string,
  enclTokenAddress: string,
  usdcTokenAddress: string,
) {
  console.log("Deploying EigenLayer strategies...");

  const strategyFactory = await hre.ethers.getContractAt(
    [
      "function deployNewStrategy(address token) external returns (address)",
      "function deployedStrategies(address token) external view returns (address)",
    ],
    strategyFactoryAddress,
  );

  // Deploy ENCL strategy
  let enclStrategyAddress =
    await strategyFactory.deployedStrategies(enclTokenAddress);
  if (enclStrategyAddress === ethers.ZeroAddress) {
    await (
      await strategyFactory.deployNewStrategy(enclTokenAddress, {
        gasLimit: 3000000,
      })
    ).wait();
    enclStrategyAddress =
      await strategyFactory.deployedStrategies(enclTokenAddress);
  }

  // Deploy USDC strategy
  let usdcStrategyAddress =
    await strategyFactory.deployedStrategies(usdcTokenAddress);
  if (usdcStrategyAddress === ethers.ZeroAddress) {
    await (
      await strategyFactory.deployNewStrategy(usdcTokenAddress, {
        gasLimit: 3000000,
      })
    ).wait();
    usdcStrategyAddress =
      await strategyFactory.deployedStrategies(usdcTokenAddress);
  }

  console.log(" ENCL Strategy:", enclStrategyAddress);
  console.log(" USDC Strategy:", usdcStrategyAddress);

  return {
    enclStrategy: enclStrategyAddress,
    usdcStrategy: usdcStrategyAddress,
  };
}

async function deployTokenomicsContracts(
  hre: HardhatRuntimeEnvironment,
  deploy: any,
  deployer: string,
  eigenLayerAddresses: any,
  coreAddresses: { enclave: string; registry: string; filter: string },
  tokenAddresses: { enclToken: string; usdcToken: string },
  strategyAddresses: { enclStrategy: string; usdcStrategy: string },
) {
  console.log("Deploying tokenomics contracts...");

  // Deploy ServiceManager implementation
  const serviceManagerImpl = await deploy("ServiceManagerImplementation", {
    from: deployer,
    args: [
      eigenLayerAddresses.avsDirectory,
      eigenLayerAddresses.rewardsCoordinator,
      eigenLayerAddresses.slashingRegistryCoordinator,
      eigenLayerAddresses.stakeRegistry,
      eigenLayerAddresses.permissionController,
      eigenLayerAddresses.allocationManager,
    ],
    log: false,
    contract: "ServiceManager",
  });

  // Deploy governance token and vesting escrow
  const enclaveToken = await deploy("EnclaveToken", {
    from: deployer,
    args: [deployer],
    log: false,
  });

  const vestingEscrow = await deploy("VestingEscrow", {
    from: deployer,
    args: [enclaveToken.address, deployer],
    log: false,
  });

  // Deploy ServiceManager proxy
  const serviceManagerInterface =
    await hre.ethers.getContractFactory("ServiceManager");
  const initData = serviceManagerInterface.interface.encodeFunctionData(
    "initialize",
    [
      deployer,
      deployer,
      eigenLayerAddresses.strategyManager,
      eigenLayerAddresses.delegationManager,
      ethers.ZeroAddress, // BondingManager - will be updated later
      strategyAddresses.enclStrategy,
      strategyAddresses.usdcStrategy,
      CONFIG.tokenomics.minCollateralUsd,
      CONFIG.tokenomics.operatorSetId,
    ],
  );

  const serviceManagerProxy = await deploy("ServiceManager_Proxy", {
    contract: "TransparentUpgradeableProxy",
    from: deployer,
    args: [
      serviceManagerImpl.address,
      eigenLayerAddresses.proxyAdmin,
      initData,
    ],
    log: false,
  });

  // Deploy BondingManager
  const bondingManager = await deploy("BondingManager", {
    from: deployer,
    args: [
      deployer,
      serviceManagerProxy.address,
      eigenLayerAddresses.delegationManager,
      eigenLayerAddresses.allocationManager,
      coreAddresses.registry,
      strategyAddresses.enclStrategy,
      strategyAddresses.usdcStrategy,
      CONFIG.tokenomics.licenseStake,
      CONFIG.tokenomics.ticketPrice,
      CONFIG.tokenomics.operatorSetId,
    ],
    log: false,
    contract: "BondingManager",
  });

  console.log(" ServiceManager:", serviceManagerProxy.address);
  console.log(" BondingManager:", bondingManager.address);
  console.log(" Enclave Token:", enclaveToken.address);
  console.log(" Vesting Escrow:", vestingEscrow.address);

  return {
    serviceManager: serviceManagerProxy.address,
    bondingManager: bondingManager.address,
    enclaveToken: enclaveToken.address,
    vestingEscrow: vestingEscrow.address,
  };
}

async function setupCrossReferences(
  hre: HardhatRuntimeEnvironment,
  addresses: DeploymentAddresses,
) {
  console.log("Setting up cross-references...");

  // Update ServiceManager with BondingManager
  const serviceManager = await hre.ethers.getContractAt(
    "ServiceManager",
    addresses.serviceManager!,
  );
  await (
    await serviceManager.setBondingManager(addresses.bondingManager!)
  ).wait();

  // Update Registry with BondingManager
  const registry = await hre.ethers.getContractAt(
    "CiphernodeRegistryOwnable",
    addresses.registry,
  );
  await (await registry.setBondingManager(addresses.bondingManager!)).wait();

  // Update Enclave with ServiceManager and token
  const enclave = await hre.ethers.getContractAt("Enclave", addresses.enclave);
  await (await enclave.setServiceManager(addresses.serviceManager!)).wait();
  await (await enclave.setEnclToken(addresses.enclToken)).wait();

  console.log("Cross-references updated");
}

async function initializeAVS(
  hre: HardhatRuntimeEnvironment,
  serviceManagerAddress: string,
  strategies: string[],
) {
  console.log("Initializing AVS...");

  const serviceManager = await hre.ethers.getContractAt(
    "ServiceManager",
    serviceManagerAddress,
  );

  // Set AVS registrar
  await (await serviceManager.setAVSRegistrar(serviceManagerAddress)).wait();
  console.log("AVS registrar set");

  // Publish AVS metadata
  await (
    await serviceManager.publishAVSMetadata(
      "https://enclave.gg/avs-metadata.json",
    )
  ).wait();
  console.log("AVS metadata published");

  // Create operator set
  try {
    await (
      await serviceManager.createOperatorSet(
        CONFIG.tokenomics.operatorSetId,
        strategies,
      )
    ).wait();
    console.log(`Operator set ${CONFIG.tokenomics.operatorSetId} created`);
  } catch (error: any) {
    console.log(`Operator set creation failed: ${error.message}`);

    // Fallback: create empty set then add strategies
    try {
      await (
        await serviceManager.createOperatorSet(
          CONFIG.tokenomics.operatorSetId,
          [],
        )
      ).wait();
      await (
        await serviceManager.addStrategies(
          CONFIG.tokenomics.operatorSetId,
          strategies,
        )
      ).wait();
      console.log(
        `Operator set ${CONFIG.tokenomics.operatorSetId} created (fallback method)`,
      );
    } catch (fallbackError: any) {
      console.log(
        `Operator set creation failed completely: ${fallbackError.message}`,
      );
    }
  }
}

// ============================================================================
// MAIN DEPLOYMENT FUNCTION
// ============================================================================

const deployFunction: DeployFunction = async function (
  hre: HardhatRuntimeEnvironment,
) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  if (!deployer) {
    throw new Error("Deployer not found from getNamedAccounts()");
  }

  console.log("Starting Enclave deployment...");
  console.log("Network:", hre.network.name);
  console.log("Deployer:", deployer);
  console.log("=".repeat(60));

  const chainId = await hre.getChainId();
  const isTokenomicsEnabled = process.env.DEPLOY_TOKENOMICS === "true";

  try {
    // Step 1: Deploy Poseidon libraries
    await deployPoseidonLibraries(hre);

    // Step 2: Deploy core contracts
    const coreAddresses = await deployCoreContracts(hre, deploy, deployer);

    let addresses: DeploymentAddresses = {
      ...coreAddresses,
      enclToken: "",
      usdcToken: "",
      enclStrategy: "",
      usdcStrategy: "",
    };

    // Step 3: Deploy tokenomics (if enabled)
    if (isTokenomicsEnabled) {
      console.log("Tokenomics deployment enabled");

      const eigenLayerAddresses = loadEigenLayerDeployment(parseInt(chainId));

      // Deploy tokens and strategies
      const tokenAddresses = await deployTestTokens(deploy, deployer);
      const strategyAddresses = await deployStrategies(
        hre,
        eigenLayerAddresses.strategyFactory,
        tokenAddresses.enclToken,
        tokenAddresses.usdcToken,
      );

      // Deploy tokenomics contracts
      const tokenomicsAddresses = await deployTokenomicsContracts(
        hre,
        deploy,
        deployer,
        eigenLayerAddresses,
        coreAddresses,
        tokenAddresses,
        strategyAddresses,
      );

      // Combine all addresses
      addresses = {
        ...addresses,
        ...tokenAddresses,
        ...strategyAddresses,
        ...tokenomicsAddresses,
      };

      // Setup cross-references
      await setupCrossReferences(hre, addresses);

      // Initialize AVS
      await initializeAVS(hre, addresses.serviceManager!, [
        addresses.enclStrategy,
        addresses.usdcStrategy,
      ]);

      // Save full deployment metadata
      saveDeploymentMetadata(hre, addresses, eigenLayerAddresses);
    } else {
      console.log(
        "Tokenomics deployment skipped (set DEPLOY_TOKENOMICS=true to enable)",
      );
      saveDeploymentMetadata(hre, addresses);
    }

    console.log("=".repeat(60));
    console.log("DEPLOYMENT COMPLETE!");
    console.log("Core contracts deployed and configured");
    if (isTokenomicsEnabled) {
      console.log("Tokenomics system deployed and initialized");
    }

    return true;
  } catch (error) {
    console.error("Deployment failed:", error);
    throw error;
  }
};

// Export configuration
deployFunction.tags = ["enclave", "tokenomics"];
deployFunction.id = "deploy_complete_system";

export default deployFunction;

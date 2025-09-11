// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { Signer } from "ethers";
import * as fs from "fs";
import { ethers } from "hardhat";
import * as path from "path";

// --- Types --------------------------------------------------------
type Address = string;

interface Deployment {
  contracts: {
    enclToken?: Address;
    enclaveToken?: Address;
    usdcToken: Address;
    serviceManager: Address;
    bondingManager: Address;
    enclStrategy: Address;
  };
  eigenLayer: {
    strategyManager: Address;
    delegationManager: Address;
    allocationManager: Address;
  };
  config: {
    tokenomics: {
      operatorSetId: number;
    };
  };
}

// --- Config -------------------------------------------------------
const OPERATORS: Address[] = [
  "0xbDA5747bFD65F08deb54cb465eB87D40e51B197E",
  "0xdD2FD4581271e230360230F9337D5c0430Bf44C0",
  "0x2546BcD3c84621e976D8185a91A922aE77ECEc30",
];
const ENCL_AMOUNT = ethers.parseEther("1000");
const USDC_AMOUNT = ethers.parseUnits("10000", 6);
const TICKET_COUNT = 10;
const AVS_METADATA_URI = "https://example.com/avs.json";
const MAG_100 = 1_000_000_000n;

// --- Minimal ABIs -------------------------------------------------
const ERC20_ABI = [
  "function approve(address,uint256) external returns (bool)",
  "function allowance(address,address) view returns (uint256)",
  "function balanceOf(address) view returns (uint256)",
  "function decimals() view returns (uint8)",
];
const MOCK_USDC_ABI = [...ERC20_ABI, "function mint(address,uint256) external"];
const STRATEGY_MANAGER_ABI = [
  "function depositIntoStrategy(address strategy,address token,uint256 amount) external returns (uint256)",
];
const DELEGATION_MANAGER_ABI = [
  "function isOperator(address) view returns (bool)",
  "function registerAsOperator(address,uint32,string) external",
];
const ALLOCATION_MANAGER_ABI = [
  "function setAllocationDelay(address operator, uint32 delay) external",
  "function modifyAllocations(address operator, tuple(tuple(address avs,uint32 id) operatorSet, address[] strategies, uint64[] newMagnitudes)[] calldata params) external",
  "function getAllocationDelay(address operator) external view returns (bool isSet, uint32 delay)",
  "function registerForOperatorSets(address operator, tuple(address avs, uint32[] operatorSetIds, bytes data) params) external",
  "function getAllocation(address operator, tuple(address avs, uint32 id) operatorSet, address strategy) external view returns (tuple(uint64 currentMagnitude, int128 pendingDiff, uint32 effectBlock))",
  "function isOperatorSlashable(address operator, tuple(address avs, uint32 id) operatorSet) external view returns (bool)",
];

// --- Helpers ------------------------------------------------------
function loadDeploymentAddresses(chainId: string): Deployment {
  const deploymentPath = path.join(
    __dirname,
    "../deployments",
    `deployment-${chainId}.json`,
  );
  const deployment = JSON.parse(fs.readFileSync(deploymentPath, "utf8"));
  return {
    contracts: deployment.contracts,
    eigenLayer: deployment.eigenLayer,
    config: deployment.config,
  } as Deployment;
}

async function mineBlocks(n: number): Promise<void> {
  const hex = "0x" + n.toString(16);
  await ethers.provider.send("hardhat_mine", [hex]);
}

async function fundOperatorWithETH(
  operatorAddress: Address,
  admin: Signer,
): Promise<void> {
  await admin.sendTransaction({
    to: operatorAddress,
    value: ethers.parseEther("10"),
  });
}

async function mintAndApproveTokens(
  admin: Signer,
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
  isLiveNetwork: boolean,
): Promise<void> {
  const enclTokenAddr =
    deployment.contracts.enclToken ?? deployment.contracts.enclaveToken!;
  const usdcTokenAddr = deployment.contracts.usdcToken;
  const strategyManagerAddr = deployment.eigenLayer.strategyManager;

  const enclToken = await ethers.getContractAt("EnclaveToken", enclTokenAddr);
  const MINTER_ROLE = await enclToken.MINTER_ROLE();
  const adminAddr = await admin.getAddress();
  const adminIsMinter = await enclToken.hasRole(MINTER_ROLE, adminAddr);
  if (!adminIsMinter)
    throw new Error("Admin does not have MINTER_ROLE on enclToken.");

  await (
    await enclToken
      .connect(admin)
      .mintAllocation(operatorAddress, ENCL_AMOUNT, "operator bootstrap")
  ).wait();

  if (!isLiveNetwork) {
    const usdc = await ethers.getContractAt(MOCK_USDC_ABI, usdcTokenAddr);
    await (await usdc.connect(admin).mint(operatorAddress, USDC_AMOUNT)).wait();
  }

  await (
    await enclToken
      .connect(operatorSigner)
      .approve(strategyManagerAddr, ENCL_AMOUNT)
  ).wait();

  console.log(
    `Tokens prepared -> ENCL minted ${ethers.formatEther(ENCL_AMOUNT)}; ` +
      `${isLiveNetwork ? "USDC: assumed pre-funded" : `USDC minted ${ethers.formatUnits(USDC_AMOUNT, 6)}`}`,
  );
}

async function depositIntoStrategies(
  operatorSigner: Signer,
  _operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const strategyManagerAddr = deployment.eigenLayer.strategyManager;
  const enclTokenAddr =
    deployment.contracts.enclToken ?? deployment.contracts.enclaveToken!;
  const enclStrategyAddr = deployment.contracts.enclStrategy;

  const strategyManager = await ethers.getContractAt(
    STRATEGY_MANAGER_ABI,
    strategyManagerAddr,
  );
  console.log("Depositing ENCL into EigenLayer strategy...");

  await (
    await strategyManager
      .connect(operatorSigner)
      .depositIntoStrategy(enclStrategyAddr, enclTokenAddr, ENCL_AMOUNT)
  ).wait();

  console.log("ENCL strategy deposit completed");
}

async function registerAsEigenLayerOperator(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<boolean> {
  const delegationManagerAddr = deployment.eigenLayer.delegationManager;
  const delegationManager = await ethers.getContractAt(
    DELEGATION_MANAGER_ABI,
    delegationManagerAddr,
  );
  const isOperator = await delegationManager.isOperator(operatorAddress);
  if (isOperator) {
    console.log("Already EigenLayer operator");
    return true;
  }
  await (
    await delegationManager
      .connect(operatorSigner)
      .registerAsOperator(operatorAddress, 0, "")
  ).wait();
  console.log("Registered as EigenLayer operator");
  return true;
}

async function setAllocationDelayAndMine(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<boolean> {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );

  try {
    await (
      await allocationManager
        .connect(operatorSigner)
        .setAllocationDelay(operatorAddress, 0)
    ).wait();
    console.log("Phase 1: Allocation delay scheduled; mining blocks...");
    await mineBlocks(2);
    await (
      await allocationManager
        .connect(operatorSigner)
        .setAllocationDelay(operatorAddress, 0)
    ).wait();
    console.log("Phase 2: Allocation delay committed");
    const [isSet, delay] =
      await allocationManager.getAllocationDelay(operatorAddress);
    console.log(`Allocation delay verified: isSet=${isSet}, delay=${delay}`);
    return true;
  } catch (e: unknown) {
    const msg = (e as Error)?.message ?? String(e);
    console.log("setAllocationDelay failed:", msg);
    return false;
  }
}

async function allocateMagnitudes(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const serviceManagerAddr = deployment.contracts.serviceManager;
  const enclStrategyAddr = deployment.contracts.enclStrategy;
  const operatorSetId = deployment.config.tokenomics.operatorSetId;

  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );

  try {
    const enclAllocation = await allocationManager.getAllocation(
      operatorAddress,
      { avs: serviceManagerAddr, id: operatorSetId },
      enclStrategyAddr,
    );
    if (enclAllocation.currentMagnitude === MAG_100) {
      console.log(
        "Already allocated ENCL magnitude (100%) to AVS operator set",
      );
      return;
    }
  } catch (_e: unknown) {
    console.log("First-time allocation (no existing allocation found)");
  }

  const params = [
    {
      operatorSet: { avs: serviceManagerAddr, id: operatorSetId },
      strategies: [enclStrategyAddr],
      newMagnitudes: [MAG_100],
    },
  ];

  console.log(`Allocating magnitude 100% to ENCL strategy ${enclStrategyAddr}`);

  await (
    await allocationManager
      .connect(operatorSigner)
      .modifyAllocations(operatorAddress, params)
  ).wait();

  const { effectBlock } = await allocationManager.getAllocation(
    operatorAddress,
    { avs: serviceManagerAddr, id: operatorSetId },
    enclStrategyAddr,
  );
  const current = await ethers.provider.getBlockNumber();
  if (effectBlock > current) {
    await mineBlocks(Number(effectBlock - current + 1));
  }
  console.log("Allocated ENCL magnitude (100%) to AVS operator set");
}

async function acquireLicense(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );
  try {
    const isActive = await bondingManager.isActive(operatorAddress);
    if (isActive) {
      console.log("License already acquired (operator is active)");
      return;
    }
  } catch (e: unknown) {
    console.log(
      "~ Checking license status failed, proceeding with acquisition",
    );
    void e;
  }
  console.log("~ Checking license stake...");
  const licenseStake = await bondingManager.getLicenseStake();
  console.log(`~ License stake: ${licenseStake}`);

  console.log("~ Acquiring license...");
  await (await bondingManager.connect(operatorSigner).acquireLicense()).wait();
  console.log("License acquired");
}

async function purchaseTickets(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const usdcTokenAddr = deployment.contracts.usdcToken;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );
  const usdcToken = await ethers.getContractAt(ERC20_ABI, usdcTokenAddr);

  const ticketPrice = await bondingManager.getTicketPrice();
  const totalCost = BigInt(TICKET_COUNT) * ticketPrice;

  try {
    const availableTickets =
      await bondingManager.getAvailableTicketCount(operatorAddress);
    if (availableTickets >= TICKET_COUNT) {
      console.log(
        `Already has sufficient tickets: ${availableTickets} available`,
      );
      return;
    }
  } catch (e: unknown) {
    const _msg = (e as Error)?.message ?? String(e);
    console.log("~ Checking ticket status failed, proceeding with purchase");
  }

  const usdcBalance = await usdcToken.balanceOf(operatorAddress);
  if (usdcBalance < totalCost) {
    throw new Error(
      `Insufficient USDC balance: ${ethers.formatUnits(usdcBalance, 6)} < ${ethers.formatUnits(totalCost, 6)}`,
    );
  }

  const allowance = await usdcToken.allowance(
    operatorAddress,
    bondingManagerAddr,
  );
  if (allowance < totalCost) {
    console.log("Approving USDC for ticket purchase...");
    await (
      await usdcToken
        .connect(operatorSigner)
        .approve(bondingManagerAddr, totalCost)
    ).wait();
  }

  console.log(
    `Purchasing ${TICKET_COUNT} tickets (${ethers.formatUnits(ticketPrice, 6)} USDC each, total: ${ethers.formatUnits(totalCost, 6)} USDC)...`,
  );
  await (
    await bondingManager.connect(operatorSigner).purchaseTickets(TICKET_COUNT)
  ).wait();
  console.log(
    `✓ Purchased ${TICKET_COUNT} tickets for ${ethers.formatUnits(totalCost, 6)} USDC`,
  );
}

async function registerToOperatorSet(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );
  const serviceManagerAddr = deployment.contracts.serviceManager;
  const operatorSetId = deployment.config.tokenomics.operatorSetId;

  try {
    console.log("Checking operator set registration status...");
    const isSlashable = await allocationManager.isOperatorSlashable(
      operatorAddress,
      { avs: serviceManagerAddr, id: operatorSetId },
    );
    if (isSlashable) {
      console.log("Already registered to AVS operator set");
      return;
    }
  } catch (e: unknown) {
    const _msg = (e as Error)?.message ?? String(e);
    console.log(
      "~ Checking operator set registration status failed, proceeding with registration",
    );
  }

  await (
    await allocationManager
      .connect(operatorSigner)
      .registerForOperatorSets(operatorAddress, {
        avs: serviceManagerAddr,
        operatorSetIds: [operatorSetId],
        data: "0x",
      })
  ).wait();
  console.log("Registered to AVS operator set");
}

async function registerCiphernode(
  operatorSigner: Signer,
  operatorAddress: Address,
  deployment: Deployment,
): Promise<void> {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );
  try {
    const isRegistered =
      await bondingManager.isRegisteredOperator(operatorAddress);
    if (isRegistered) {
      console.log("Ciphernode already registered");
      return;
    }
  } catch (e: unknown) {
    const _msg = (e as Error)?.message ?? String(e);
    console.log(
      "~ Checking ciphernode registration status failed, proceeding with registration",
    );
  }

  await (
    await bondingManager.connect(operatorSigner).registerCiphernode()
  ).wait();
  console.log("Ciphernode registered");
}

// --- Main ---------------------------------------------------------
async function main(): Promise<void> {
  const [admin] = await ethers.getSigners();
  const chainId = (await ethers.provider.getNetwork()).chainId.toString();
  const deployment = loadDeploymentAddresses(chainId);

  console.log("EIGENLAYER OPERATOR REGISTRATION");
  console.log("=".repeat(65));
  console.log("Admin:", await admin.getAddress());
  console.log("ServiceManager:", deployment.contracts.serviceManager);
  console.log("BondingManager:", deployment.contracts.bondingManager);

  const serviceManager = await ethers.getContractAt(
    "ServiceManager",
    deployment.contracts.serviceManager,
  );
  try {
    await (
      await serviceManager.connect(admin).publishAVSMetadata(AVS_METADATA_URI)
    ).wait();
    console.log("✓ AVS metadata published");
  } catch (e: unknown) {
    const msg = (e as Error)?.message ?? String(e);
    console.log("~ publishAVSMetadata skipped:", msg);
  }

  const operatorSetId = deployment.config.tokenomics.operatorSetId;
  try {
    await (
      await serviceManager
        .connect(admin)
        .createOperatorSet(operatorSetId, [deployment.contracts.enclStrategy])
    ).wait();
    console.log(`Operator set ${operatorSetId} created (runtime)`);
  } catch (e: unknown) {
    const msg = (e as Error)?.message ?? String(e);
    console.log(`~ createOperatorSet skipped (likely exists): ${msg}`);
  }
  console.log(
    `Using operator set ${operatorSetId} (configured at deploy-time)`,
  );

  const isLiveNetwork = chainId !== "31337";

  let successCount = 0;
  for (const operatorAddress of OPERATORS) {
    console.log(`\nRegistering Operator: ${operatorAddress}`);
    console.log("=".repeat(64));

    try {
      await fundOperatorWithETH(operatorAddress, admin);

      await ethers.provider.send("hardhat_impersonateAccount", [
        operatorAddress,
      ]);
      const operatorSigner = await ethers.getSigner(operatorAddress);

      await mintAndApproveTokens(
        admin,
        operatorSigner,
        operatorAddress,
        deployment,
        isLiveNetwork,
      );
      await depositIntoStrategies(operatorSigner, operatorAddress, deployment);
      await registerAsEigenLayerOperator(
        operatorSigner,
        operatorAddress,
        deployment,
      );
      await setAllocationDelayAndMine(
        operatorSigner,
        operatorAddress,
        deployment,
      );
      await allocateMagnitudes(operatorSigner, operatorAddress, deployment);
      await acquireLicense(operatorSigner, operatorAddress, deployment);
      await registerToOperatorSet(operatorSigner, operatorAddress, deployment);
      await purchaseTickets(operatorSigner, operatorAddress, deployment);
      await registerCiphernode(operatorSigner, operatorAddress, deployment);

      console.log(`Operator ${operatorAddress} registered successfully!`);
      successCount++;

      await ethers.provider.send("hardhat_stopImpersonatingAccount", [
        operatorAddress,
      ]);
    } catch (error: unknown) {
      const msg = (error as Error)?.message ?? String(error);
      console.error(`Operator ${operatorAddress} failed: ${msg}`);
      try {
        await ethers.provider.send("hardhat_stopImpersonatingAccount", [
          operatorAddress,
        ]);
      } catch (e: unknown) {
        void e;
      }
      console.log("Continuing with next operator...");
    }
  }

  console.log("\n" + "=".repeat(65));
  console.log("REGISTRATION SUMMARY");
  console.log("=".repeat(65));
  console.log(`Successful registrations: ${successCount}/${OPERATORS.length}`);
  if (successCount < OPERATORS.length) {
    console.log(
      `\\ ${OPERATORS.length - successCount} operators failed registration`,
    );
  }
}

main().catch((error: unknown) => {
  const msg = (error as Error)?.message ?? String(error);
  console.error(msg);
  process.exitCode = 1;
});

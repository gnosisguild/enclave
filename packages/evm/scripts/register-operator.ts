import * as fs from "fs";
import { ethers } from "hardhat";
import * as path from "path";

// Configuration
const OPERATORS = [
  "0xbDA5747bFD65F08deb54cb465eB87D40e51B197E",
  "0xdD2FD4581271e230360230F9337D5c0430Bf44C0",
  "0x2546BcD3c84621e976D8185a91A922aE77ECEc30",
];

// Load deployment addresses
function loadDeploymentAddresses() {
  const deploymentPath = path.join(
    __dirname,
    "../deployments/deployment-31337.json",
  );
  const deployment = JSON.parse(fs.readFileSync(deploymentPath, "utf8"));
  return {
    contracts: deployment.contracts,
    eigenLayer: deployment.eigenLayer,
    config: deployment.config,
  };
}

const ENCL_AMOUNT = ethers.parseEther("1000");
const USDC_AMOUNT = ethers.parseUnits("10000", 6);
const TICKET_COUNT = 100;
const AVS_METADATA_URI = "https://example.com/avs.json";
const MAG_100 = 1_000_000_000n; // 100% in PPB (Parts Per Billion) - EigenLayer uint64 magnitudes

// Minimal ABIs
const ERC20_ABI = [
  "function approve(address,uint256) external returns (bool)",
  "function mint(address,uint256) external",
];

const STRATEGY_MANAGER_ABI = [
  "function depositIntoStrategy(address,address,uint256) external returns (uint256)",
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

async function mineBlocks(n: number) {
  const hex = "0x" + n.toString(16);
  await ethers.provider.send("hardhat_mine", [hex]);
}

async function fundOperatorWithETH(operatorAddress: string, admin: any) {
  // Send ETH to operator for gas fees
  await admin.sendTransaction({
    to: operatorAddress,
    value: ethers.parseEther("10"), // 10 ETH for gas
  });
}

async function mintAndApproveTokens(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  // Get contract addresses from deployment
  const enclTokenAddr = deployment.contracts.enclToken;
  const usdcTokenAddr = deployment.contracts.usdcToken;
  const strategyManagerAddr = deployment.eigenLayer.strategyManager;

  // Mint and approve ENCL
  const enclToken = await ethers.getContractAt(ERC20_ABI, enclTokenAddr);
  await (await enclToken.mint(operatorAddress, ENCL_AMOUNT)).wait();
  await (
    await enclToken
      .connect(operatorSigner)
      .approve(strategyManagerAddr, ENCL_AMOUNT)
  ).wait();

  // Mint and approve USDC
  const usdcToken = await ethers.getContractAt(ERC20_ABI, usdcTokenAddr);
  await (await usdcToken.mint(operatorAddress, USDC_AMOUNT)).wait();
  await (
    await usdcToken
      .connect(operatorSigner)
      .approve(strategyManagerAddr, USDC_AMOUNT)
  ).wait();

  console.log(
    `Tokens minted and approved -> ENCL: ${ethers.formatEther(ENCL_AMOUNT)}, USDC: ${ethers.formatUnits(USDC_AMOUNT, 6)}`,
  );
}

async function depositIntoStrategies(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const strategyManagerAddr = deployment.eigenLayer.strategyManager;
  const enclTokenAddr = deployment.contracts.enclToken;
  const usdcTokenAddr = deployment.contracts.usdcToken;
  const enclStrategyAddr = deployment.contracts.enclStrategy;
  const usdcStrategyAddr = deployment.contracts.usdcStrategy;

  const strategyManager = await ethers.getContractAt(
    STRATEGY_MANAGER_ABI,
    strategyManagerAddr,
  );

  console.log("Depositing into EigenLayer strategies...");

  // Deposit ENCL
  await (
    await strategyManager
      .connect(operatorSigner)
      .depositIntoStrategy(enclStrategyAddr, enclTokenAddr, ENCL_AMOUNT)
  ).wait();

  // Deposit USDC
  await (
    await strategyManager
      .connect(operatorSigner)
      .depositIntoStrategy(usdcStrategyAddr, usdcTokenAddr, USDC_AMOUNT)
  ).wait();

  console.log("Strategy deposits completed");
}

async function registerAsEigenLayerOperator(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
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
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );

  try {
    // Phase 1: Schedule the delay (creates pending state)
    await (
      await allocationManager
        .connect(operatorSigner)
        .setAllocationDelay(operatorAddress, 0)
    ).wait();
    console.log(
      "Phase 1: Allocation delay scheduled; mining blocks for it to take effect...",
    );

    // Wait for effectBlock to pass (with config=0, +1 block is enough, but mine 2 to be safe)
    await mineBlocks(2);

    // Phase 2: Commit the pending delay (sets isSet=true)
    await (
      await allocationManager
        .connect(operatorSigner)
        .setAllocationDelay(operatorAddress, 0)
    ).wait();
    console.log("Phase 2: Allocation delay committed (isSet=true)");

    // Optional: Verify the delay is properly set
    const [isSet, delay] =
      await allocationManager.getAllocationDelay(operatorAddress);
    console.log(`Allocation delay verified: isSet=${isSet}, delay=${delay}`);
    return true;
  } catch (e: any) {
    console.log("setAllocationDelay failed:", e.message ?? e);
    return false;
  }
}

async function allocateMagnitudes(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const serviceManagerAddr = deployment.contracts.serviceManager;
  const enclStrategyAddr = deployment.contracts.enclStrategy;
  const usdcStrategyAddr = deployment.contracts.usdcStrategy;
  const operatorSetId = deployment.config.tokenomics.operatorSetId;

  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );

  // Check existing allocations first to avoid SameMagnitude error
  try {
    const enclAllocation = await allocationManager.getAllocation(
      operatorAddress,
      { avs: serviceManagerAddr, id: operatorSetId },
      enclStrategyAddr,
    );
    const usdcAllocation = await allocationManager.getAllocation(
      operatorAddress,
      { avs: serviceManagerAddr, id: operatorSetId },
      usdcStrategyAddr,
    );

    // If both strategies already have 100% allocation, skip
    if (
      enclAllocation.currentMagnitude == MAG_100 &&
      usdcAllocation.currentMagnitude == MAG_100
    ) {
      console.log(
        "Already allocated ENCL+USDC magnitude (100% each) to AVS operator set",
      );
      return;
    }
  } catch (e: any) {
    // If getAllocation fails, proceed with allocation (probably first time)
    console.log("First-time allocation (no existing allocation found)");
  }

  // Allocate 100% magnitude for BOTH strategies to operator set (make everything slashable)
  const allocParams = [
    {
      operatorSet: { avs: serviceManagerAddr, id: operatorSetId },
      strategies: [enclStrategyAddr, usdcStrategyAddr],
      newMagnitudes: [MAG_100, MAG_100],
    },
  ];

  console.log(
    `Attempting to allocate magnitudes: [${allocParams[0].newMagnitudes.join(", ")}] to strategies: [${allocParams[0].strategies.join(", ")}]`,
  );

  await (
    await allocationManager
      .connect(operatorSigner)
      .modifyAllocations(operatorAddress, allocParams)
  ).wait();
  console.log("Allocated ENCL+USDC magnitude (100% each) to AVS operator set");
}

async function acquireLicense(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );

  // Check if operator is already active (has license and tickets)
  try {
    const isActive = await bondingManager.isActive(operatorAddress);
    if (isActive) {
      console.log("License already acquired (operator is active)");
      return;
    }
  } catch (e: any) {
    // If isActive fails, try to acquire license
    console.log("Checking license status failed, proceeding with acquisition");
  }

  await (await bondingManager.connect(operatorSigner).acquireLicense()).wait();
  console.log("License acquired");
}

async function purchaseTickets(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );

  // Check if operator already has sufficient tickets
  try {
    const ticketSpent = await bondingManager.ticketBudgetSpent(operatorAddress);
    const ticketPrice = await bondingManager.getTicketPrice();
    const expectedSpent = BigInt(TICKET_COUNT) * ticketPrice;

    if (ticketSpent >= expectedSpent) {
      console.log(
        `Already purchased ${TICKET_COUNT}+ tickets (spent: ${ethers.formatUnits(ticketSpent, 6)} USDC)`,
      );
      return;
    }
  } catch (e: any) {
    console.log("~ Checking ticket status failed, proceeding with purchase");
  }

  await (
    await bondingManager.connect(operatorSigner).purchaseTickets(TICKET_COUNT)
  ).wait();
  console.log(`✓ Purchased ${TICKET_COUNT} tickets`);
}

async function registerToOperatorSet(
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const allocationManagerAddr = deployment.eigenLayer.allocationManager;
  const allocationManager = await ethers.getContractAt(
    ALLOCATION_MANAGER_ABI,
    allocationManagerAddr,
  );
  const serviceManagerAddr = deployment.contracts.serviceManager;
  const operatorSetId = deployment.config.tokenomics.operatorSetId;

  // Check if operator is already registered to this operator set
  try {
    const isSlashable = await allocationManager.isOperatorSlashable(
      operatorAddress,
      { avs: serviceManagerAddr, id: operatorSetId },
    );
    if (isSlashable) {
      console.log("Already registered to AVS operator set");
      return;
    }
  } catch (e: any) {
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
  operatorSigner: any,
  operatorAddress: string,
  deployment: any,
) {
  const bondingManagerAddr = deployment.contracts.bondingManager;
  const bondingManager = await ethers.getContractAt(
    "BondingManager",
    bondingManagerAddr,
  );

  // Check if ciphernode is already registered
  try {
    const isRegistered =
      await bondingManager.isRegisteredOperator(operatorAddress);
    if (isRegistered) {
      console.log("Ciphernode already registered");
      return;
    }
  } catch (e: any) {
    console.log(
      "~ Checking ciphernode registration status failed, proceeding with registration",
    );
  }

  await (
    await bondingManager.connect(operatorSigner).registerCiphernode()
  ).wait();
  console.log("Ciphernode registered");
}

async function main() {
  const [admin] = await ethers.getSigners();

  // Load deployment addresses
  const deployment = loadDeploymentAddresses();

  console.log("EIGENLAYER OPERATOR REGISTRATION");
  console.log("=".repeat(65));
  console.log("Admin:", admin.address);
  console.log("ServiceManager:", deployment.contracts.serviceManager);
  console.log("BondingManager:", deployment.contracts.bondingManager);

  // AVS-level setup (admin operations)
  const serviceManager = await ethers.getContractAt(
    "ServiceManager",
    deployment.contracts.serviceManager,
  );

  try {
    await (
      await serviceManager.connect(admin).publishAVSMetadata(AVS_METADATA_URI)
    ).wait();
    console.log("✓ AVS metadata published");
  } catch (e: any) {
    console.log("~ publishAVSMetadata skipped:", e.message ?? e);
  }

  // Ensure operator set exists (safety net in case deployment didn't create it)
  try {
    await (
      await serviceManager
        .connect(admin)
        .createOperatorSet(deployment.config.operatorSetId, [
          deployment.contracts.enclStrategy,
          deployment.contracts.usdcStrategy,
        ])
    ).wait();
    console.log(
      `Operator set ${deployment.config.operatorSetId} created (runtime)`,
    );
  } catch (e: any) {
    console.log(
      `~ createOperatorSet skipped (likely exists): ${e.message ?? e}`,
    );
  }

  console.log(
    `Using operator set ${deployment.config.operatorSetId} (configured at deploy-time)`,
  );

  // Register each operator
  let successCount = 0;
  for (const operatorAddress of OPERATORS) {
    console.log(`\nRegistering Operator: ${operatorAddress}`);
    console.log("=".repeat(64));

    try {
      // Fund operator with ETH for gas
      await fundOperatorWithETH(operatorAddress, admin);

      // Impersonate operator
      await ethers.provider.send("hardhat_impersonateAccount", [
        operatorAddress,
      ]);
      const operatorSigner = await ethers.getSigner(operatorAddress);

      // Complete registration flow
      await mintAndApproveTokens(operatorSigner, operatorAddress, deployment);
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
      await registerToOperatorSet(operatorSigner, operatorAddress, deployment);
      await allocateMagnitudes(operatorSigner, operatorAddress, deployment);
      await acquireLicense(operatorSigner, operatorAddress, deployment);
      await purchaseTickets(operatorSigner, operatorAddress, deployment);
      await registerCiphernode(operatorSigner, operatorAddress, deployment);

      console.log(`Operator ${operatorAddress} registered successfully!`);
      successCount++;

      // Stop impersonation
      await ethers.provider.send("hardhat_stopImpersonatingAccount", [
        operatorAddress,
      ]);
    } catch (error: any) {
      console.error(
        `Operator ${operatorAddress} failed: ${error?.message ?? error}`,
      );

      // Try to extract error signature for debugging
      if (error.receipt && error.receipt.blockNumber) {
        console.log(
          `Block: ${error.receipt.blockNumber}, Gas: ${error.receipt.gasUsed}, To: ${error.receipt.to}`,
        );
      }
      if (error.data) {
        console.log(`Error data: ${error.data}`);
      }

      try {
        await ethers.provider.send("hardhat_stopImpersonatingAccount", [
          operatorAddress,
        ]);
      } catch {}
      console.log("Continuing with next operator...");
    }
  }

  // Final summary
  console.log("\n" + "=".repeat(65));
  console.log("REGISTRATION SUMMARY");
  console.log("=".repeat(65));
  console.log(`Successful registrations: ${successCount}/${OPERATORS.length}`);
  if (successCount < OPERATORS.length) {
    console.log(
      `\ ${OPERATORS.length - successCount} operators failed registration`,
    );
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { EventLog, Log } from "ethers";
import fs from "fs";
import { task, types } from "hardhat/config";
import type { TaskArguments } from "hardhat/types";
import * as path from "path";

// --- Types --------------------------------------------------------
interface DeploymentAddresses {
  contracts: {
    enclave: string;
    enclToken?: string;
    enclaveToken?: string;
    filter: string;
    [key: string]: string | undefined;
  };
}
interface E3Data {
  id: number;
  publicKey: string;
  nodes: string[];
  inputData: string;
  seed?: string;
  createdAt: number;
}
interface PersistentState {
  latestE3Id: number;
  e3Data: { [key: number]: E3Data };
}

// --- File I/O -----------------------------------------------------
function loadDeploymentAddresses(chainId: number): DeploymentAddresses {
  const deploymentPath = path.join(
    __dirname,
    "..",
    "deployments",
    `deployment-${chainId}.json`,
  );
  if (!fs.existsSync(deploymentPath))
    throw new Error(`Deployment file not found: ${deploymentPath}`);
  return JSON.parse(fs.readFileSync(deploymentPath, "utf8"));
}
function getStateFilePath(chainId: number): string {
  const stateDir = path.join(__dirname, "..", "test-state");
  if (!fs.existsSync(stateDir)) fs.mkdirSync(stateDir, { recursive: true });
  return path.join(stateDir, `e3-state-${chainId}.json`);
}
function loadPersistentState(chainId: number): PersistentState {
  const stateFile = getStateFilePath(chainId);
  if (!fs.existsSync(stateFile)) return { latestE3Id: 0, e3Data: {} };
  try {
    return JSON.parse(fs.readFileSync(stateFile, "utf8"));
  } catch {
    return { latestE3Id: 0, e3Data: {} };
  }
}
function savePersistentState(chainId: number, state: PersistentState): void {
  const stateFile = getStateFilePath(chainId);
  fs.writeFileSync(stateFile, JSON.stringify(state, null, 2));
}

// --- Deterministic Generators ------------------------------------
const DEFAULT_OPERATORS = [
  "0xbDA5747bFD65F08deb54cb465eB87D40e51B197E",
  "0xdD2FD4581271e230360230F9337D5c0430Bf44C0",
  "0x2546BcD3c84621e976D8185a91A922aE77ECEc30",
];

function generateDeterministicBytes(seed: string, length: number): string {
  const chars = "0123456789abcdef";
  let result = "0x";
  let hash = 0;
  for (let i = 0; i < seed.length; i++)
    hash = ((hash << 5) - hash + seed.charCodeAt(i)) & 0xffffffff;
  for (let i = 0; i < length * 2; i++) {
    hash = (hash * 1103515245 + 12345) & 0xffffffff;
    result += chars.charAt(Math.abs(hash) % chars.length);
  }
  return result;
}

function getOrCreateE3Data(
  state: PersistentState,
  e3id: number,
  seed?: string,
): E3Data {
  if (state.e3Data[e3id]) return state.e3Data[e3id];
  const dataSeed = seed || `e3_${e3id}_${Date.now()}`;
  const e3Data: E3Data = {
    id: e3id,
    publicKey: generateDeterministicBytes(`${dataSeed}_pubkey`, 32),
    nodes: DEFAULT_OPERATORS,
    inputData: generateDeterministicBytes(`${dataSeed}_input`, 100),
    seed: dataSeed,
    createdAt: Date.now(),
  };
  state.e3Data[e3id] = e3Data;
  return e3Data;
}

// --- Helpers -----------------------------------------------------
function resolveE3Id(
  maybeId: number | undefined,
  state: { latestE3Id: number },
) {
  if (maybeId !== undefined && maybeId !== null) return Number(maybeId);
  const v = state.latestE3Id;
  if (v === undefined || v === null || Number.isNaN(v)) {
    throw new Error(
      "No E3 ID provided and no latest E3 found. Run e3t:new first.",
    );
  }
  return Number(v);
}

// --- Task: e3t:new -----------------------------------------------
task("e3t:new", "Request a new ciphernode committee").setAction(async function (
  _: TaskArguments,
  hre,
) {
  const { ethers } = hre;
  const [deployer] = await ethers.getSigners();
  const chainId = parseInt(await hre.getChainId());

  console.log("REQUESTING NEW COMMITTEE");
  console.log("=".repeat(50));
  console.log(`Deployer: ${deployer.address}`);
  console.log(`Network: ${hre.network.name} (${chainId})`);
  console.log("");

  try {
    const addresses = loadDeploymentAddresses(chainId);
    const state = loadPersistentState(chainId);

    const enclave = await ethers.getContractAt(
      "Enclave",
      addresses.contracts.enclave,
    );

    let e3Address: string;
    try {
      const mockE3Program = await hre.deployments.get("MockE3Program");
      e3Address = mockE3Program.address;
    } catch {
      const contracts = Object.keys(addresses.contracts);
      const e3Programs = contracts.filter((name) =>
        name.toLowerCase().includes("e3program"),
      );
      if (e3Programs.length > 0)
        e3Address = addresses.contracts[e3Programs[0]]!;
      else throw new Error("No E3 program found in deployment");
    }

    const filterAddress = addresses.contracts.filter;
    const e3Params = hre.ethers.randomBytes(32);
    const computeParams = hre.ethers.randomBytes(32);
    const thresholdQuorum = 1;
    const thresholdTotal = 2;
    const windowStart = Math.floor(Date.now() / 1000);
    const windowEnd = windowStart + 60;
    const duration = 3;

    console.log("Request Parameters:");
    console.log(`Filter: ${filterAddress}`);
    console.log(`Threshold: [${thresholdQuorum}, ${thresholdTotal}]`);
    console.log(
      `Window: [${new Date(windowStart * 1000).toISOString()}, ${new Date(windowEnd * 1000).toISOString()}]`,
    );
    console.log(`Duration: ${duration} seconds`);
    console.log(`E3 Program: ${e3Address}`);
    console.log("");

    try {
      const enableE3Tx = await enclave.enableE3Program(e3Address);
      await enableE3Tx.wait();
      console.log("E3 program enabled");
    } catch {
      console.log("E3 program already enabled or failed to enable");
    }

    const enclTokenAddr =
      addresses.contracts.enclToken ?? addresses.contracts.enclaveToken;
    if (enclTokenAddr && !hre.network.live) {
      try {
        const enclToken = await ethers.getContractAt(
          "EnclaveToken",
          enclTokenAddr,
        );
        const MINTER_ROLE = await enclToken.MINTER_ROLE();
        const adminIsMinter = await enclToken.hasRole(
          MINTER_ROLE,
          deployer.address,
        );
        if (adminIsMinter) {
          const amt = ethers.parseEther("10");
          await (
            await enclToken.mintAllocation(
              deployer.address,
              amt,
              "e3 new request",
            )
          ).wait();
          await (
            await enclToken.approve(addresses.contracts.enclave, amt)
          ).wait();
        }
      } catch {
        console.log("Local ENCL preparation skipped");
      }
    }

    console.log("Requesting committee...");
    const tx = await enclave.request({
      filter: filterAddress,
      threshold: [thresholdQuorum, thresholdTotal],
      startWindow: [windowStart, windowEnd],
      duration,
      e3Program: e3Address,
      e3ProgramParams: e3Params,
      computeProviderParams: computeParams,
    });
    const receipt = await tx.wait();
    if (!receipt) throw new Error("Transaction failed");
    console.log(`Transaction hash: ${receipt.hash}`);

    const e3RequestedEvent = receipt.logs.find((log: EventLog | Log) => {
      try {
        const parsed = enclave.interface.parseLog({
          topics: log.topics,
          data: log.data,
        });
        return parsed && parsed.name === "E3Requested";
      } catch {
        return false;
      }
    });

    if (e3RequestedEvent) {
      const parsed = enclave.interface.parseLog(e3RequestedEvent);
      if (parsed) {
        const e3id = parseInt(parsed.args[0].toString());
        console.log(`Committee requested successfully! E3 ID: ${e3id}`);
        state.latestE3Id = e3id;
        const e3 = await enclave.getE3(e3id);
        const seed = e3.seed;
        getOrCreateE3Data(state, e3id, seed.toString());
        savePersistentState(chainId, state);

        console.log("");
        console.log("E3 Details:");
        console.log(`Seed: ${e3.seed}`);
        console.log(`Threshold: [${e3.threshold[0]}, ${e3.threshold[1]}]`);
        console.log(`Request Block: ${e3.requestBlock}`);
        console.log(
          `Start Window: [${new Date(Number(e3.startWindow[0]) * 1000).toISOString()}, ${new Date(
            Number(e3.startWindow[1]) * 1000,
          ).toISOString()}]`,
        );
        console.log(`Duration: ${e3.duration} seconds`);
        console.log(
          `Expiration: ${e3.expiration === 0n ? "Not activated yet" : new Date(Number(e3.expiration) * 1000).toISOString()}`,
        );
      }
    } else {
      console.log("Committee requested but could not parse event for E3 ID");
    }

    console.log("");
    console.log("=".repeat(50));
    console.log("Committee request complete!");
  } catch (error) {
    console.error("Error requesting committee:", error);
    process.exit(1);
  }
});

// --- Task: e3t:publish -------------------------------------------
task("e3t:publish", "Publish committee public key")
  .addOptionalParam(
    "e3id",
    "E3 ID to publish for (uses latest if not provided)",
    undefined,
    types.int,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    console.log("PUBLISHING COMMITTEE PUBLIC KEY");
    console.log("=".repeat(50));

    try {
      const addresses = loadDeploymentAddresses(chainId);
      const state = loadPersistentState(chainId);

      const filter = await ethers.getContractAt(
        "NaiveRegistryFilter",
        addresses.contracts.filter,
      );

      const e3id = resolveE3Id(taskArguments.e3id, state);
      console.log(`Using E3 ID: ${e3id}`);

      const e3Data = getOrCreateE3Data(state, e3id);

      console.log("Committee Details:");
      console.log(`E3 ID: ${e3id}`);
      console.log(`Node Count: ${e3Data.nodes.length}`);
      console.log(`Nodes: ${e3Data.nodes.join(", ")}`);
      console.log(`Public Key: ${e3Data.publicKey}`);
      console.log("");

      console.log("Publishing committee...");
      const tx = await filter.publishCommittee(
        e3id,
        e3Data.nodes,
        e3Data.publicKey,
      );
      console.log(`Transaction hash: ${tx.hash}`);
      await tx.wait();

      savePersistentState(chainId, state);
      console.log("Committee public key published successfully!");
    } catch (error) {
      console.error("Error publishing committee:", error);
      process.exit(1);
    }
  });

// --- Task: e3t:activate ------------------------------------------
task("e3t:activate", "Activate E3 program")
  .addOptionalParam(
    "e3id",
    "E3 ID to activate (uses latest if not provided)",
    undefined,
    types.int,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    console.log("ACTIVATING E3 PROGRAM");
    console.log("=".repeat(50));

    try {
      const addresses = loadDeploymentAddresses(chainId);
      const state = loadPersistentState(chainId);

      const enclave = await ethers.getContractAt(
        "Enclave",
        addresses.contracts.enclave,
      );

      const e3id = resolveE3Id(taskArguments.e3id, state);
      console.log(`Using E3 ID: ${e3id}`);
      const e3Data = getOrCreateE3Data(state, e3id);

      console.log("Activation Details:");
      console.log(`E3 ID: ${e3id}`);
      console.log(`Public Key: ${e3Data.publicKey}`);
      console.log("");

      console.log("Activating E3...");
      const tx = await enclave.activate(e3id, e3Data.publicKey);
      console.log(`Transaction hash: ${tx.hash}`);
      await tx.wait();

      console.log("E3 program activated successfully!");

      try {
        const e3 = await enclave.getE3(e3id);
        console.log("");
        console.log("Updated E3 Status:");
        console.log(
          `Expiration: ${new Date(Number(e3.expiration) * 1000).toISOString()}`,
        );
      } catch {
        console.log("Could not fetch updated E3 details");
      }

      savePersistentState(chainId, state);
    } catch (error) {
      console.error("Error activating E3:", error);
      process.exit(1);
    }
  });

// --- Task: e3t:publishInput --------------------------------------
task("e3t:publishInput", "Publish input for E3 program")
  .addOptionalParam(
    "e3id",
    "E3 ID to publish input for (uses latest if not provided)",
    undefined,
    types.int,
  )
  .addOptionalParam(
    "data",
    "Custom input data (uses persistent data if not provided)",
    undefined,
    types.string,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    console.log("PUBLISHING INPUT DATA");
    console.log("=".repeat(50));

    try {
      const addresses = loadDeploymentAddresses(chainId);
      const state = loadPersistentState(chainId);

      const enclave = await ethers.getContractAt(
        "Enclave",
        addresses.contracts.enclave,
      );

      const e3id = resolveE3Id(taskArguments.e3id, state);
      console.log(`Using E3 ID: ${e3id}`);
      const e3Data = getOrCreateE3Data(state, e3id);
      const data = taskArguments.data || e3Data.inputData;

      console.log("Input Details:");
      console.log(`E3 ID: ${e3id}`);
      console.log(`Data: "${data.substring(0, 50)}..."`);
      console.log("");

      console.log("Publishing input...");
      const tx = await enclave.publishInput(e3id, data);
      console.log(`Transaction hash: ${tx.hash}`);
      await tx.wait();

      savePersistentState(chainId, state);
      console.log("Input published successfully!");
    } catch (error) {
      console.error("Error publishing input:", error);
      process.exit(1);
    }
  });

// --- Task: e3t:publishCiphertext ---------------------------------
task("e3t:publishCiphertext", "Publish ciphertext output for E3 program")
  .addOptionalParam(
    "e3id",
    "E3 ID to publish ciphertext for (uses latest if not provided)",
    undefined,
    types.int,
  )
  .addOptionalParam(
    "outputSize",
    "Size of mock ciphertext in bytes (defaults to 128)",
    128,
    types.int,
  )
  .addOptionalParam(
    "proofSize",
    "Size of mock proof in bytes (defaults to 64)",
    64,
    types.int,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    console.log("PUBLISHING CIPHERTEXT OUTPUT");
    console.log("=".repeat(50));

    try {
      const addresses = loadDeploymentAddresses(chainId);
      const state = loadPersistentState(chainId);

      const enclave = await ethers.getContractAt(
        "Enclave",
        addresses.contracts.enclave,
      );

      const e3id = resolveE3Id(taskArguments.e3id, state);
      console.log(`Using E3 ID: ${e3id}`);

      const outputSize = taskArguments.outputSize;
      const proofSize = taskArguments.proofSize;

      const e3Data = getOrCreateE3Data(state, e3id);

      const ciphertextOutput = generateDeterministicBytes(
        `${e3Data.seed}_ciphertext`,
        outputSize,
      );
      const proof = generateDeterministicBytes(
        `${e3Data.seed}_cipher_proof`,
        proofSize,
      );

      console.log("Ciphertext Details:");
      console.log(`E3 ID: ${e3id}`);
      console.log(`Output Size: ${outputSize} bytes`);
      console.log(`Proof Size: ${proofSize} bytes`);
      console.log(`Ciphertext: ${ciphertextOutput.substring(0, 50)}...`);
      console.log(`Proof: ${proof.substring(0, 50)}...`);
      console.log("");

      console.log("Publishing ciphertext...");
      const tx = await enclave.publishCiphertextOutput(
        e3id,
        ciphertextOutput,
        proof,
      );
      console.log(`Transaction hash: ${tx.hash}`);
      await tx.wait();

      savePersistentState(chainId, state);
      console.log("Ciphertext published successfully!");
    } catch (error) {
      console.error("Error publishing ciphertext:", error);
      process.exit(1);
    }
  });

// --- Task: e3t:publishPlaintext ----------------------------------
task("e3t:publishPlaintext", "Publish plaintext output for E3 program")
  .addOptionalParam(
    "e3id",
    "E3 ID to publish plaintext for (uses latest if not provided)",
    undefined,
    types.int,
  )
  .addOptionalParam(
    "outputSize",
    "Size of mock plaintext in bytes (defaults to 64)",
    64,
    types.int,
  )
  .addOptionalParam(
    "proofSize",
    "Size of mock proof in bytes (defaults to 64)",
    64,
    types.int,
  )
  .addOptionalParam(
    "textMode",
    "Generate readable text instead of hex (defaults to true)",
    true,
    types.boolean,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    console.log("PUBLISHING PLAINTEXT OUTPUT");
    console.log("=".repeat(50));

    try {
      const addresses = loadDeploymentAddresses(chainId);
      const state = loadPersistentState(chainId);

      const enclave = await ethers.getContractAt(
        "Enclave",
        addresses.contracts.enclave,
      );

      const e3id = resolveE3Id(taskArguments.e3id, state);
      console.log(`Using E3 ID: ${e3id}`);

      const outputSize = taskArguments.outputSize;
      const proofSize = taskArguments.proofSize;
      const textMode = taskArguments.textMode;

      const e3Data = getOrCreateE3Data(state, e3id);

      let plaintextOutput: string;
      if (textMode) {
        const mockText = generateDeterministicBytes(
          `${e3Data.seed}_plaintext`,
          outputSize,
        );
        plaintextOutput = ethers.hexlify(ethers.toUtf8Bytes(mockText));
        console.log(`Generated Text: "${mockText.substring(0, 50)}..."`);
      } else {
        plaintextOutput = generateDeterministicBytes(
          `${e3Data.seed}_plaintext`,
          outputSize,
        );
      }

      const proof = generateDeterministicBytes(
        `${e3Data.seed}_plain_proof`,
        proofSize,
      );

      console.log("");
      console.log("Plaintext Details:");
      console.log(`E3 ID: ${e3id}`);
      console.log(`Output Size: ${outputSize} bytes`);
      console.log(`Proof Size: ${proofSize} bytes`);
      console.log(`Text Mode: ${textMode}`);
      console.log(`Plaintext: ${plaintextOutput.substring(0, 50)}...`);
      console.log(`Proof: ${proof.substring(0, 50)}...`);
      console.log("");

      console.log("Publishing plaintext...");
      const tx = await enclave.publishPlaintextOutput(
        e3id,
        plaintextOutput,
        proof,
      );
      console.log(`Transaction hash: ${tx.hash}`);
      await tx.wait();

      savePersistentState(chainId, state);
      console.log("Plaintext published successfully!");

      if (textMode) {
        try {
          const updatedE3 = await enclave.getE3(e3id);
          if (updatedE3.plaintextOutput !== "0x") {
            const decodedText = ethers.toUtf8String(updatedE3.plaintextOutput);
            console.log(
              `Decoded Plaintext: "${decodedText.substring(0, 100)}..."`,
            );
          }
        } catch {
          console.log("Could not decode plaintext as text");
        }
      }
    } catch (error) {
      console.error("Error publishing plaintext:", error);
      process.exit(1);
    }
  });

// --- Task: e3t:complete ------------------------------------------
task("e3t:complete", "Run complete E3 workflow with persistent data")
  .addOptionalParam(
    "delayBetweenSteps",
    "Delay in seconds between workflow steps (defaults to 2)",
    2,
    types.int,
  )
  .setAction(async function (taskArguments: TaskArguments, hre) {
    const { ethers } = hre;
    const [deployer] = await ethers.getSigners();
    const chainId = parseInt(await hre.getChainId());

    console.log("COMPLETE E3 WORKFLOW TEST");
    console.log("=".repeat(60));
    console.log(`Deployer: ${deployer.address}`);
    console.log(`Network: ${hre.network.name} (${chainId})`);
    console.log(`Delay Between Steps: ${taskArguments.delayBetweenSteps}s`);
    console.log("");

    const delay = (seconds: number) =>
      new Promise((resolve) => setTimeout(resolve, seconds * 1000));

    try {
      console.log("STEP 1: Requesting new E3 committee");
      console.log("-".repeat(40));
      await hre.run("e3t:new");
      await delay(taskArguments.delayBetweenSteps);
      console.log("");

      console.log("STEP 2: Publishing committee public key");
      console.log("-".repeat(40));
      await hre.run("e3t:publish");
      await delay(taskArguments.delayBetweenSteps);
      console.log("");

      console.log("STEP 3: Activating E3");
      console.log("-".repeat(40));
      await hre.run("e3t:activate");
      await delay(taskArguments.delayBetweenSteps);
      console.log("");

      console.log("STEP 4: Publishing input data");
      console.log("-".repeat(40));
      await hre.run("e3t:publishInput");
      await delay(taskArguments.delayBetweenSteps);
      console.log("");

      console.log("STEP 5: Publishing ciphertext output");
      console.log("-".repeat(40));
      await hre.run("e3t:publishCiphertext");
      await delay(taskArguments.delayBetweenSteps);
      console.log("");

      console.log("STEP 6: Publishing plaintext output");
      console.log("-".repeat(40));
      await hre.run("e3t:publishPlaintext");
      console.log("");

      console.log("WORKFLOW COMPLETE!");
      console.log("=".repeat(60));

      const state = loadPersistentState(chainId);
      const e3id = state.latestE3Id;

      if (e3id) {
        const addresses = loadDeploymentAddresses(chainId);
        const enclave = await ethers.getContractAt(
          "Enclave",
          addresses.contracts.enclave,
        );

        try {
          const finalE3 = await enclave.getE3(e3id);
          console.log("Final E3 Status:");
          console.log(`E3 ID: ${e3id}`);
          console.log(
            `Threshold: [${finalE3.threshold[0]}, ${finalE3.threshold[1]}]`,
          );
          console.log(
            `Expiration: ${new Date(Number(finalE3.expiration) * 1000).toISOString()}`,
          );
          console.log(
            `Ciphertext Output Length: ${finalE3.ciphertextOutput.length / 2 - 1} bytes`,
          );
          console.log(
            `Plaintext Output Length: ${finalE3.plaintextOutput.length / 2 - 1} bytes`,
          );
          try {
            const decodedPlaintext = ethers.toUtf8String(
              finalE3.plaintextOutput,
            );
            console.log(
              `Decoded Plaintext: "${decodedPlaintext.substring(0, 50)}..."`,
            );
          } catch {
            console.log("Could not decode plaintext as text");
          }
        } catch {
          console.log("Could not fetch final E3 status");
        }
      }

      console.log("");
      console.log("All workflow steps completed successfully!");
    } catch (error) {
      console.error("Workflow failed:", error);
      process.exit(1);
    }
  });

task("e3t:operatorStatus", "Display comprehensive operator status")
  .addOptionalParam(
    "operator",
    "Operator address to check (shows defaults if not provided)",
  )
  .setAction(async (taskArgs, hre) => {
    const { ethers } = hre;
    const chainId = parseInt(await hre.getChainId());

    const operatorsToCheck = taskArgs.operator
      ? [taskArgs.operator]
      : DEFAULT_OPERATORS;

    const deploymentPath = path.join(
      __dirname,
      "..",
      "deployments",
      `deployment-${chainId}.json`,
    );
    if (!fs.existsSync(deploymentPath))
      throw new Error(`Deployment file not found: ${deploymentPath}`);
    const deployment = JSON.parse(fs.readFileSync(deploymentPath, "utf8"));

    const enclTokenAddr =
      deployment.contracts.enclToken ?? deployment.contracts.enclaveToken;
    const usdcTokenAddr = deployment.contracts.usdcToken;
    const enclStrategyAddr = deployment.contracts.enclStrategy;
    const usdcStrategyAddr = deployment.contracts.usdcStrategy;
    const serviceManagerAddr = deployment.contracts.serviceManager;
    const bondingManagerAddr = deployment.contracts.bondingManager;
    const registryAddr = deployment.contracts.registry;
    const operatorSetId = deployment.config.tokenomics.operatorSetId;

    const enclToken = await ethers.getContractAt("EnclaveToken", enclTokenAddr);
    const usdcToken = await ethers.getContractAt("MockERC20", usdcTokenAddr);
    const enclStrategy = await ethers.getContractAt(
      "IStrategy",
      enclStrategyAddr,
    );
    const usdcStrategy = await ethers.getContractAt(
      "IStrategy",
      usdcStrategyAddr,
    );
    const strategyManager = await ethers.getContractAt(
      "IStrategyManager",
      deployment.eigenLayer.strategyManager,
    );
    const allocationManager = await ethers.getContractAt(
      "IAllocationManager",
      deployment.eigenLayer.allocationManager,
    );
    const registry = await ethers.getContractAt(
      "CiphernodeRegistryOwnable",
      registryAddr,
    );
    const bondingManager = await ethers.getContractAt(
      "BondingManager",
      bondingManagerAddr,
    );

    console.log("OPERATOR STATUS REPORT");
    console.log("=".repeat(80));
    console.log(`Network: ${hre.network.name} (${chainId})`);
    console.log(`Checking ${operatorsToCheck.length} operator(s)`);
    console.log("");

    const MAG_100 = 1_000_000_000n;
    const operatorSet = { avs: serviceManagerAddr, id: operatorSetId };

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const rows: any[] = [];
    for (const operator of operatorsToCheck) {
      try {
        const [enclBal, usdcBal] = await Promise.all([
          enclToken.balanceOf(operator),
          usdcToken.balanceOf(operator),
        ]);

        const [enclShares, usdcShares] = await Promise.all([
          strategyManager.stakerDepositShares(operator, enclStrategyAddr),
          strategyManager.stakerDepositShares(operator, usdcStrategyAddr),
        ]);

        const [enclUnderlying, usdcUnderlying] = await Promise.all([
          enclStrategy.sharesToUnderlyingView(enclShares),
          usdcStrategy.sharesToUnderlyingView(usdcShares),
        ]);

        let enclAllocated = "0",
          usdcAllocated = "0",
          isSlashable = false;
        try {
          const [enclAlloc, usdcAlloc] = await Promise.all([
            allocationManager.getAllocation(
              operator,
              operatorSet,
              enclStrategyAddr,
            ),
            allocationManager.getAllocation(
              operator,
              operatorSet,
              usdcStrategyAddr,
            ),
          ]);
          const enclAllocShares =
            (BigInt(enclShares) * BigInt(enclAlloc.currentMagnitude)) / MAG_100;
          const usdcAllocShares =
            (BigInt(usdcShares) * BigInt(usdcAlloc.currentMagnitude)) / MAG_100;
          const [enclAllocUnderlying, usdcAllocUnderlying] = await Promise.all([
            enclStrategy.sharesToUnderlyingView(enclAllocShares),
            usdcStrategy.sharesToUnderlyingView(usdcAllocShares),
          ]);
          enclAllocated = ethers.formatEther(enclAllocUnderlying);
          usdcAllocated = ethers.formatUnits(usdcAllocUnderlying, 6);
          isSlashable = await allocationManager.isOperatorSlashable(
            operator,
            operatorSet,
          );
        } catch (e) {
          console.log("Error getting allocation:", e);
        }

        let isLicensed = false,
          isRegistered = false,
          isActive = false,
          ticketBalance = 0,
          state = "REMOVED";
        try {
          const [info, registered, cipherState] = await Promise.all([
            bondingManager.getOperatorInfo(operator),
            bondingManager.isRegisteredOperator(operator),
            bondingManager.getCiphernodeState(operator),
          ]);
          isLicensed = info.isLicensed;
          isActive = info.isActive;
          ticketBalance = Number(info.ticketBalance);
          isRegistered = registered;
          state = ["REMOVED", "REGISTERED_INACTIVE", "ACTIVE"][
            Number(cipherState)
          ];
        } catch (e) {
          console.log("Error getting bonding manager status:", e);
        }

        let inRegistry = false;
        try {
          inRegistry = await registry.isEnabled(operator);
        } catch (e) {
          console.log("Error getting registry status:", e);
        }

        rows.push({
          operator,
          enclBal: ethers.formatEther(enclBal),
          usdcBal: ethers.formatUnits(usdcBal, 6),
          enclStaked: ethers.formatEther(enclUnderlying),
          usdcStaked: ethers.formatUnits(usdcUnderlying, 6),
          enclAllocated,
          usdcAllocated,
          isSlashable,
          isLicensed,
          isRegistered,
          isActive,
          ticketBalance,
          state,
          inRegistry,
        });
      } catch (e) {
        rows.push({ operator, error: e });
      }
    }

    console.log("");
    console.log("OPERATOR STATUS");
    console.log("=".repeat(145));
    const header = `${"Operator".padEnd(10)} | ${"ENCL Bal".padEnd(10)} | ${"USDC Bal".padEnd(10)} | ${"ENCL Stake".padEnd(12)} | ${"USDC Stake".padEnd(12)} | ${"ENCL Alloc".padEnd(12)} | ${"USDC Alloc".padEnd(12)} | ${"Slash".padEnd(5)} | ${"Lic".padEnd(3)} | ${"Act".padEnd(3)} | ${"Tickets".padEnd(7)} | ${"State".padEnd(7)} | ${"Reg".padEnd(3)}`;
    console.log(header);
    console.log("-".repeat(145));
    for (const r of rows) {
      if (r.error) {
        console.log(
          `${r.operator.substring(0, 10).padEnd(10)} | ERROR: ${r.error}`,
        );
        continue;
      }
      const row = [
        r.operator.substring(0, 10).padEnd(10),
        Number(r.enclBal).toFixed(2).padEnd(10),
        Number(r.usdcBal).toFixed(2).padEnd(10),
        Number(r.enclStaked).toFixed(2).padEnd(12),
        Number(r.usdcStaked).toFixed(2).padEnd(12),
        Number(r.enclAllocated).toFixed(2).padEnd(12),
        Number(r.usdcAllocated).toFixed(2).padEnd(12),
        (r.isSlashable ? "Yes" : "No").padEnd(5),
        (r.isLicensed ? "Yes" : "No").padEnd(3),
        (r.isActive ? "Yes" : "No").padEnd(3),
        String(r.ticketBalance).padEnd(7),
        r.state.padEnd(7),
        (r.inRegistry ? "Yes" : "No").padEnd(3),
      ].join(" | ");
      console.log(row);
    }
    console.log("");
    console.log("=".repeat(145));
    console.log("Status check complete!");
  });

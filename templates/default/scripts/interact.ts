// scripts/interact.ts
import hre from "hardhat";
import { ethers } from "ethers";
import { LeanIMT } from "@zk-kit/lean-imt";
import { poseidon2 } from "poseidon-lite";

async function main() {
  const command = process.argv[2];
  const args = process.argv.slice(3);

  if (!command) {
    console.log("Available commands:");
    console.log("  add-ciphernode <address>");
    console.log("  remove-ciphernode <address> <siblings>");
    console.log("  get-siblings <address> <all-addresses>");
    console.log("  new-committee [options]");
    process.exit(1);
  }

  try {
    switch (command) {
      case "add-ciphernode":
        await addCiphernode(hre, args[0]);
        break;
      case "remove-ciphernode":
        await removeCiphernode(hre, args[0], args[1]);
        break;
      case "get-siblings":
        await getSiblings(args[0], args[1]);
        break;
      case "new-committee":
        await newCommittee(hre, args);
        break;
      default:
        console.error("Unknown command:", command);
        process.exit(1);
    }
  } catch (error) {
    console.error("❌ Command failed:", error);
    process.exit(1);
  }
}

async function addCiphernode(hre: any, ciphernodeAddress: string) {
  if (!ciphernodeAddress) {
    throw new Error("Ciphernode address is required");
  }

  console.log(`📝 Adding ciphernode: ${ciphernodeAddress}`);

  const registry = await hre.deployments.get("CiphernodeRegistryOwnable");
  const registryContract = await hre.ethers.getContractAt(
    "CiphernodeRegistryOwnable",
    registry.address,
  );

  const tx = await registryContract.addCiphernode(ciphernodeAddress);
  console.log("Transaction hash:", tx.hash);

  await tx.wait();
  console.log(`✅ Ciphernode ${ciphernodeAddress} registered successfully`);
}

async function removeCiphernode(
  hre: any,
  ciphernodeAddress: string,
  siblingsStr: string,
) {
  if (!ciphernodeAddress || !siblingsStr) {
    throw new Error("Ciphernode address and siblings are required");
  }

  console.log(`🗑️  Removing ciphernode: ${ciphernodeAddress}`);

  const registry = await hre.deployments.get("CiphernodeRegistryOwnable");
  const registryContract = await hre.ethers.getContractAt(
    "CiphernodeRegistryOwnable",
    registry.address,
  );

  const siblings = siblingsStr.split(",").map((s: string) => BigInt(s.trim()));

  const tx = await registryContract.removeCiphernode(
    ciphernodeAddress,
    siblings,
  );
  console.log("Transaction hash:", tx.hash);

  await tx.wait();
  console.log(`✅ Ciphernode ${ciphernodeAddress} removed successfully`);
}

async function getSiblings(ciphernodeAddress: string, addressesStr: string) {
  if (!ciphernodeAddress || !addressesStr) {
    throw new Error("Ciphernode address and list of addresses are required");
  }

  console.log(`🔍 Getting siblings for: ${ciphernodeAddress}`);

  const hash = (a: bigint, b: bigint) => poseidon2([a, b]);
  const tree = new LeanIMT(hash);

  const addresses = addressesStr.split(",").map((addr) => addr.trim());

  for (const address of addresses) {
    tree.insert(BigInt(address));
  }

  const index = tree.indexOf(BigInt(ciphernodeAddress));
  if (index === -1) {
    throw new Error(
      `Ciphernode ${ciphernodeAddress} not found in the provided list`,
    );
  }

  const { siblings } = tree.generateProof(index);
  console.log(`📋 Siblings: ${siblings.join(",")}`);
}

async function newCommittee(hre: any, args: string[]) {
  console.log("🏛️  Requesting new committee...");

  const enclave = await hre.deployments.get("Enclave");
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclave.address,
  );

  // Default parameters (can be made configurable)
  const thresholdQuorum = 2;
  const thresholdTotal = 2;
  const windowStart = Math.floor(Date.now() / 1000);
  const windowEnd = windowStart + 86400; // 1 day
  const duration = 86400; // 1 day

  // Get default addresses
  const naiveRegistryFilter = await hre.deployments.get("NaiveRegistryFilter");

  // For demo purposes, we'll need mock contracts
  // In production, users would provide their own E3 program
  let e3Address;
  try {
    const mockE3Program = await hre.deployments.get("MockE3Program");
    e3Address = mockE3Program.address;
  } catch {
    throw new Error(
      "MockE3Program not deployed. You may need to deploy mocks first.",
    );
  }

  // Enable E3 program
  try {
    const enableE3Tx = await enclaveContract.enableE3Program(e3Address);
    await enableE3Tx.wait();
    console.log("E3 program enabled");
  } catch (e) {
    console.log("E3 program already enabled or enabling failed");
  }

  // Request committee
  const tx = await enclaveContract.request(
    naiveRegistryFilter.address,
    [thresholdQuorum, thresholdTotal],
    [windowStart, windowEnd],
    duration,
    e3Address,
    hre.ethers.zeroPadValue("0x00", 32), // e3Params
    hre.ethers.zeroPadValue("0x00", 32), // computeParams
    { value: hre.ethers.parseEther("1.0") }, // 1 ETH
  );

  console.log("Transaction hash:", tx.hash);
  await tx.wait();
  console.log("✅ Committee requested successfully");
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}

// scripts/interact.ts
import { HardhatRuntimeEnvironment } from "hardhat/types";
import { parseArgs } from "util";

interface AddCiphernodeArgs {
  ciphernodeAddress: string;
  network: string;
}

async function main(): Promise<void> {
  const { values } = parseArgs({
    args: process.argv.slice(2),
    options: {
      "ciphernode-address": {
        type: "string",
      },
      network: {
        type: "string",
        default: "localhost",
      },
    },
  });

  if (!values["ciphernode-address"]) {
    console.error("❌ --ciphernode-address is required");
    process.exit(1);
  }

  const args: AddCiphernodeArgs = {
    ciphernodeAddress: values["ciphernode-address"],
    network: values.network!,
  };

  // Set network if provided
  if (args.network) {
    process.env.HARDHAT_NETWORK = args.network;
  }

  // Get hardhat runtime environment
  const hre = require("hardhat") as HardhatRuntimeEnvironment;

  try {
    await addCiphernode(hre, args.ciphernodeAddress);
  } catch (error) {
    console.error("❌ Command failed:", error);
    process.exit(1);
  }
}

async function addCiphernode(
  hre: any,
  ciphernodeAddress: string,
): Promise<void> {
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

if (require.main === module) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}

import hre from "hardhat";

import { verifyContracts } from "@enclave-e3/contracts/scripts";

async function main() {
  const { ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  verifyContracts(chain);
}

main().catch((error) => {
  console.error(error);
});

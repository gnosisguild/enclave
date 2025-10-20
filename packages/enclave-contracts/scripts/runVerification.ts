import { verifyContracts } from "./verify";

import hre from "hardhat";

async function main() {
    const { ethers } = await hre.network.connect();
    const [signer] = await ethers.getSigners();
    const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

    verifyContracts(chain);
}

main().catch((error => {
    console.error(error);
}));

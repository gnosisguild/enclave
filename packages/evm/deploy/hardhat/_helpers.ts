// SPDX-License-Identifier: LGPL-3.0-only
import { ethers } from "ethers";
import fs from "fs";
import type { HardhatRuntimeEnvironment } from "hardhat/types";
import path from "path";

export const CONFIG = {
  enclave: {
    maxComputeDuration: 60 * 60 * 24 * 30, // 30 days
    polynomialDegree: 2048n,
    plaintextModulus: 1032193n,
    moduli: [18014398492704769n],
  },
  tokenomics: {
    licenseStake: ethers.parseEther("100"),
    ticketPrice: ethers.parseUnits("10", 6),
    minCollateralUsd: ethers.parseEther("1000"),
    operatorSetId: 1,
  },
  addresses: {
    addressOne: "0x0000000000000000000000000000000000000001",
  },
} as const;

export function loadEigenLayerDeployment(chainId: number) {
  const p = path.join(
    __dirname,
    "../..",
    "deployments",
    "core",
    `${chainId}.json`,
  );
  if (!fs.existsSync(p)) {
    throw new Error(
      `EigenLayer core deployment not found at ${p}. Deploy it first.`,
    );
  }
  return JSON.parse(fs.readFileSync(p, "utf8")).addresses;
}

export async function saveDeploymentMetadata(
  hre: HardhatRuntimeEnvironment,
  addresses: Record<string, string | undefined>,
  eigen?: any,
) {
  const chainId = await hre.getChainId();
  const out = {
    network: hre.network.name,
    chainId,
    timestamp: new Date().toISOString(),
    contracts: addresses,
    ...(eigen && { eigenLayer: eigen }),
    config: {
      enclave: {
        maxComputeDuration: CONFIG.enclave.maxComputeDuration,
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
    },
  };
  const outPath = path.join(
    __dirname,
    "../..",
    "deployments",
    `deployment-${chainId}.json`,
  );
  fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
  console.log("Deployment metadata saved:", outPath);
}

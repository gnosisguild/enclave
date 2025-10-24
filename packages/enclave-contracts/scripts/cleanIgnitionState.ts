// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import path from "path";

/**
 * Cleans Hardhat Ignition state for a given chain ID.
 * This is useful when working with Anvil or other local nodes where chain state
 * is reset but Ignition state persists, causing reconciliation errors.
 *
 * @param chainId - The chain ID to clean state for (default: 31337 for Anvil/Hardhat)
 */
export const cleanIgnitionState = (chainId: number = 31337): void => {
  const ignitionPath = path.join(process.cwd(), "ignition", "deployments");
  const chainFolder = path.join(ignitionPath, `chain-${chainId}`);

  if (fs.existsSync(chainFolder)) {
    console.log(`Cleaning Hardhat Ignition state for chain ${chainId}...`);
    fs.rmSync(chainFolder, { recursive: true, force: true });
    console.log(`Cleaned Ignition state at: ${chainFolder}`);
  } else {
    console.log(`No Ignition state found for chain ${chainId}`);
  }
};

/**
 * Cleans deployment records for a specific network from deployed_contracts.json
 *
 * @param networkName - The network name (e.g., "localhost", "hardhat")
 */
export const cleanDeploymentRecords = (networkName: string): void => {
  const deploymentsFile = path.join(process.cwd(), "deployed_contracts.json");

  if (!fs.existsSync(deploymentsFile)) {
    return;
  }

  try {
    const deployments = JSON.parse(fs.readFileSync(deploymentsFile, "utf8"));

    if (deployments[networkName]) {
      console.log(
        `Cleaning deployment records for network '${networkName}'...`,
      );
      delete deployments[networkName];
      fs.writeFileSync(deploymentsFile, JSON.stringify(deployments, null, 2));
      console.log(`Cleaned deployment records for '${networkName}'`);
    }
  } catch (error) {
    console.warn("Failed to clean deployment records:", error);
  }
};

/**
 * Automatically clean Ignition state and deployment records for localhost/hardhat networks before deployment.
 * This prevents stale state issues when Anvil is restarted.
 */
export const autoCleanIgnitionForLocalhost = async (
  networkName: string,
  chainId: number,
): Promise<void> => {
  const localNetworks = ["localhost", "hardhat", "anvil", "ganache"];
  if (localNetworks.includes(networkName)) {
    console.log(
      `Detected local network '${networkName}', auto-cleaning stale deployment state...`,
    );
    cleanIgnitionState(chainId);
    cleanDeploymentRecords(networkName);
  }
};

/**
 *
 * Usage: pnpm hardhat run scripts/cleanIgnitionState.ts
 */
async function main() {
  console.log("Manually cleaning Ignition state for localhost (chainId 31337)");
  cleanIgnitionState(31337);
  cleanDeploymentRecords("localhost");
  console.log("Done! You can now run deployments again.");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});

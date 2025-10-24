// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import fs from "fs";
import path from "path";

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
export const autoCleanForLocalhost = async (
  networkName: string,
): Promise<void> => {
  const localNetworks = ["localhost", "hardhat", "anvil", "ganache"];
  if (localNetworks.includes(networkName)) {
    console.log(
      `Detected local network '${networkName}', auto-cleaning stale deployment state...`,
    );
    cleanDeploymentRecords(networkName);
  }
};

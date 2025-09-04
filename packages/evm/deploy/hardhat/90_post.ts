// SPDX-License-Identifier: LGPL-3.0-only
import "@nomicfoundation/hardhat-ethers";
import "hardhat-deploy";
import { DeployFunction } from "hardhat-deploy/types";

import {
  CONFIG,
  loadEigenLayerDeployment,
  saveDeploymentMetadata,
} from "./_helpers";

const AVS_METADATA_URL =
  process.env.AVS_METADATA_URL ?? "https://enclave.gg/avs-metadata.json";

const func: DeployFunction = async (hre) => {
  const addresses: Record<string, string | undefined> = {};
  const enclave = await hre.deployments.get("Enclave");
  const registry = await hre.deployments.get("CiphernodeRegistryOwnable");
  const filter = await hre.deployments.get("NaiveRegistryFilter");

  addresses.enclave = enclave.address;
  addresses.registry = registry.address;
  addresses.filter = filter.address;

  const sm = await hre.deployments.getOrNull("ServiceManager");
  const bm = await hre.deployments.getOrNull("BondingManager");
  const enclTokenDep = await hre.deployments.getOrNull("EnclaveToken");
  // Not really needed here but incase I dont forget about it.
  const vestingEscrowDep = await hre.deployments.getOrNull("VestingEscrow");

  const enclTokenAddr = enclTokenDep?.address ?? process.env.ENCL_TOKEN;
  if (enclTokenAddr) addresses.enclToken = enclTokenAddr;
  if (vestingEscrowDep?.address)
    addresses.vestingEscrow = vestingEscrowDep.address;

  if (sm && bm) {
    addresses.serviceManager = sm.address;
    addresses.bondingManager = bm.address;

    // Wire ServiceManager <-> BondingManager
    const smc = await hre.ethers.getContractAt("ServiceManager", sm.address);
    if ((await smc.bondingManager()) !== bm.address) {
      await (await smc.setBondingManager(bm.address)).wait();
    }

    // Registry.bondingManager
    const reg = await hre.ethers.getContractAt(
      "CiphernodeRegistryOwnable",
      registry.address,
    );
    if ((await reg.bondingManager()) !== bm.address) {
      await (await reg.setBondingManager(bm.address)).wait();
    }

    // Enclave.serviceManager + Enclave.enclToken
    const enc = await hre.ethers.getContractAt("Enclave", enclave.address);
    if ((await enc.serviceManager()) !== sm.address) {
      await (await enc.setServiceManager(sm.address)).wait();
    }
    if (enclTokenAddr && (await enc.enclToken()) !== enclTokenAddr) {
      await (await enc.setEnclToken(enclTokenAddr)).wait();
    }

    // EnclaveToken: whitelist BM + VestingEscrow, and relax transfers locally
    if (enclTokenAddr) {
      const token = await hre.ethers.getContractAt(
        "EnclaveToken",
        enclTokenAddr,
      );
      // WL VestingEscrow + BM
      if (
        await token.hasRole(
          await token.DEFAULT_ADMIN_ROLE(),
          (await hre.ethers.getSigners())[0].address,
        )
      ) {
        if (vestingEscrowDep?.address) {
          await (
            await token.whitelistContracts(bm.address, vestingEscrowDep.address)
          ).wait();
        } else {
          await (
            await token.whitelistContracts(bm.address, hre.ethers.ZeroAddress)
          ).wait();
        }
        // Free transfers in local
        if (!hre.network.live) {
          if (await token.transfersRestricted()) {
            await (await token.setTransferRestriction(false)).wait();
          }
        }
      }
    }

    // AVS init
    await (await smc.setAVSRegistrar(sm.address)).wait();
    await (await smc.publishAVSMetadata(AVS_METADATA_URL)).wait();

    const enclStrategy = (await hre.deployments.getOrNull("EnclStrategy"))
      ?.address;
    const usdcStrategy = (await hre.deployments.getOrNull("UsdcStrategy"))
      ?.address;
    const strategies = [enclStrategy, usdcStrategy].filter(Boolean) as string[];

    try {
      await (
        await smc.createOperatorSet(CONFIG.tokenomics.operatorSetId, strategies)
      ).wait();
    } catch {
      await (
        await smc.createOperatorSet(CONFIG.tokenomics.operatorSetId, [])
      ).wait();
      if (strategies.length) {
        await (
          await smc.addStrategies(CONFIG.tokenomics.operatorSetId, strategies)
        ).wait();
      }
    }

    addresses.enclStrategy = enclStrategy;
    addresses.usdcStrategy = usdcStrategy;
  } else {
    console.log(
      "No ServiceManager/BondingManager found; skipping wiring & AVS init.",
    );
  }

  addresses.usdcToken =
    (await hre.deployments.getOrNull("UsdcToken"))?.address ??
    process.env.USDC_TOKEN;

  let eigen;
  try {
    eigen = loadEigenLayerDeployment(parseInt(await hre.getChainId()));
  } catch {}
  await saveDeploymentMetadata(hre, addresses, eigen);
};
export default func;
func.tags = ["post"];
func.dependencies = ["enclave"];

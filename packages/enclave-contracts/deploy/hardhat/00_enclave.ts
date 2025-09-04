// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import "@nomicfoundation/hardhat-ethers";
import { ethers } from "ethers";
import "hardhat-deploy";
import { DeployFunction } from "hardhat-deploy/types";
import type { HardhatRuntimeEnvironment } from "hardhat/types";
import { PoseidonT3, proxy } from "poseidon-solidity";

import { CONFIG } from "./_helpers";

const func: DeployFunction = async (hre: HardhatRuntimeEnvironment) => {
  const { deploy } = hre.deployments;
  const { deployer } = await hre.getNamedAccounts();
  if (!deployer) throw new Error("Deployer not found");

  const codeAt = (addr: string) => hre.ethers.provider.getCode(addr);
  if (await codeAt(proxy.address).then((c) => c === "0x")) {
    const [sender] = await hre.ethers.getSigners();
    await sender!.sendTransaction({ to: proxy.from, value: proxy.gas });
    await hre.ethers.provider.broadcastTransaction(proxy.tx);
    console.log("Poseidon proxy:", proxy.address);
  }
  if (await codeAt(PoseidonT3.address).then((c) => c === "0x")) {
    const [sender] = await hre.ethers.getSigners();
    await sender!.sendTransaction({ to: proxy.address, data: PoseidonT3.data });
    console.log("PoseidonT3:", PoseidonT3.address);
  }

  // Encode FHE params
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [
      CONFIG.enclave.polynomialDegree,
      CONFIG.enclave.plaintextModulus,
      CONFIG.enclave.moduli,
    ],
  );

  // Core contracts
  const enclave = await deploy("Enclave", {
    from: deployer,
    args: [
      deployer,
      CONFIG.addresses.addressOne,
      CONFIG.addresses.addressOne,
      CONFIG.addresses.addressOne,
      CONFIG.enclave.maxComputeDuration,
      [encoded],
    ],
    log: true,
    libraries: { PoseidonT3: PoseidonT3.address },
  });

  const registry = await deploy("CiphernodeRegistryOwnable", {
    from: deployer,
    args: [deployer, enclave.address],
    log: true,
    libraries: { PoseidonT3: PoseidonT3.address },
  });

  const filter = await deploy("NaiveRegistryFilter", {
    from: deployer,
    args: [deployer, registry.address],
    log: true,
  });

  // minimal one-way update
  const enc = await hre.ethers.getContractAt("Enclave", enclave.address);
  if ((await enc.ciphernodeRegistry()) !== registry.address) {
    await (await enc.setCiphernodeRegistry(registry.address)).wait();
    console.log("Enclave registry updated");
  }

  console.log("Core deployed:", {
    enclave: enclave.address,
    registry: registry.address,
    filter: filter.address,
  });
};
export default func;
func.id = "deploy_enclave";
func.tags = ["enclave"];

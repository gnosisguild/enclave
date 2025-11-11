import type { Signer } from "ethers";
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import { getProxyAdmin, verifyProxyAdminOwner } from "../scripts/proxy";
import {
  BondingRegistry,
  BondingRegistry__factory as BondingRegistryFactory,
  CiphernodeRegistryOwnable,
  CiphernodeRegistryOwnable__factory as CiphernodeRegistryOwnableFactory,
  Enclave,
  Enclave__factory as EnclaveFactory,
} from "../types";

export const upgradeEnclaveTestUtils = async ({
  poseidonT3,
  proxyAddress,
  ownerAddress,
  signer,
  hre,
}: {
  poseidonT3: string;
  proxyAddress: string;
  ownerAddress: string;
  signer: Signer;
  hre: HardhatRuntimeEnvironment;
}): Promise<{ enclave: Enclave; implementationAddress: string }> => {
  const { ethers } = await hre.network.connect();

  const autoProxyAdminAddress = await getProxyAdmin(
    ethers.provider,
    proxyAddress,
  );
  console.log("Auto-deployed ProxyAdmin address:", autoProxyAdminAddress);

  const enclaveFactory = await ethers.getContractFactory(
    EnclaveFactory.abi,
    EnclaveFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3": poseidonT3,
    }),
    signer,
  );

  const newImplementation = await enclaveFactory.deploy();
  await newImplementation.waitForDeployment();
  const newImplementationAddress = await newImplementation.getAddress();
  console.log("New Implementation Address:", newImplementationAddress);

  const proxyAdmin = await ethers.getContractAt(
    "ProxyAdmin",
    autoProxyAdminAddress,
    signer,
  );
  await verifyProxyAdminOwner(proxyAdmin, ownerAddress);

  const initData = "0x";
  const upgradeTx = await proxyAdmin.upgradeAndCall(
    proxyAddress,
    newImplementationAddress,
    initData,
  );
  await upgradeTx.wait();

  const enclaveContract = EnclaveFactory.connect(proxyAddress, signer);
  return {
    enclave: enclaveContract,
    implementationAddress: newImplementationAddress,
  };
};

export const upgradeCiphernodeRegistryOwnableTestUtils = async ({
  poseidonT3,
  proxyAddress,
  ownerAddress,
  signer,
  hre,
}: {
  poseidonT3: string;
  proxyAddress: string;
  ownerAddress: string;
  signer: Signer;
  hre: HardhatRuntimeEnvironment;
}): Promise<{
  ciphernodeRegistry: CiphernodeRegistryOwnable;
  implementationAddress: string;
}> => {
  const { ethers } = await hre.network.connect();

  const autoProxyAdminAddress = await getProxyAdmin(
    ethers.provider,
    proxyAddress,
  );
  console.log("Auto-deployed ProxyAdmin address:", autoProxyAdminAddress);

  const ciphernodeRegistryFactory = await ethers.getContractFactory(
    CiphernodeRegistryOwnableFactory.abi,
    CiphernodeRegistryOwnableFactory.linkBytecode({
      "npm/poseidon-solidity@0.0.5/PoseidonT3.sol:PoseidonT3": poseidonT3,
    }),
    signer,
  );

  const newImplementation = await ciphernodeRegistryFactory.deploy();
  await newImplementation.waitForDeployment();
  const newImplementationAddress = await newImplementation.getAddress();
  console.log("New Implementation Address:", newImplementationAddress);

  const proxyAdmin = await ethers.getContractAt(
    "ProxyAdmin",
    autoProxyAdminAddress,
    signer,
  );

  await verifyProxyAdminOwner(proxyAdmin, ownerAddress);

  // TODO: Add init data if needed
  const initData = "0x";
  const upgradeTx = await proxyAdmin.upgradeAndCall(
    proxyAddress,
    newImplementationAddress,
    initData,
  );
  await upgradeTx.wait();

  const ciphernodeRegistryContract = CiphernodeRegistryOwnableFactory.connect(
    proxyAddress,
    signer,
  );
  return {
    ciphernodeRegistry: ciphernodeRegistryContract,
    implementationAddress: newImplementationAddress,
  };
};

export const upgradeBondingRegistryTestUtils = async ({
  proxyAddress,
  ownerAddress,
  signer,
  hre,
}: {
  proxyAddress: string;
  ownerAddress: string;
  signer: Signer;
  hre: HardhatRuntimeEnvironment;
}): Promise<{
  bondingRegistry: BondingRegistry;
  implementationAddress: string;
}> => {
  const { ethers } = await hre.network.connect();

  const autoProxyAdminAddress = await getProxyAdmin(
    ethers.provider,
    proxyAddress,
  );
  console.log("Auto-deployed ProxyAdmin address:", autoProxyAdminAddress);

  const bondingRegistryFactory = await ethers.getContractFactory(
    "BondingRegistry",
    signer,
  );

  const newImplementation = await bondingRegistryFactory.deploy();
  await newImplementation.waitForDeployment();
  const newImplementationAddress = await newImplementation.getAddress();
  console.log("New Implementation Address:", newImplementationAddress);

  const proxyAdmin = await ethers.getContractAt(
    "ProxyAdmin",
    autoProxyAdminAddress,
    signer,
  );
  await verifyProxyAdminOwner(proxyAdmin, ownerAddress);

  const initData = "0x";
  const upgradeTx = await proxyAdmin.upgradeAndCall(
    proxyAddress,
    newImplementationAddress,
    initData,
  );
  await upgradeTx.wait();

  const bondingRegistryContract = BondingRegistryFactory.connect(
    proxyAddress,
    signer,
  );

  return {
    bondingRegistry: bondingRegistryContract,
    implementationAddress: newImplementationAddress,
  };
};

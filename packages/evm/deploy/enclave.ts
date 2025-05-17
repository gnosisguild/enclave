import { ethers } from "hardhat";
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";
import { PoseidonT3, proxy } from "poseidon-solidity";

const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
const addressOne = "0x0000000000000000000000000000000000000001";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  // First check if the proxy exists
  if ((await hre.ethers.provider.getCode(proxy.address)) === "0x") {
    // probably on the hardhat network
    // fund the keyless account
    const [sender] = await hre.ethers.getSigners();
    await sender.sendTransaction({
      to: proxy.from,
      value: proxy.gas,
    });

    // then send the presigned transaction deploying the proxy
    await hre.ethers.provider.broadcastTransaction(proxy.tx);
    console.log(`Proxy deployed to: ${proxy.address}`);
  }

  // Then deploy the hasher, if needed
  if ((await hre.ethers.provider.getCode(PoseidonT3.address)) === "0x") {
    const [sender] = await hre.ethers.getSigners();
    await sender.sendTransaction({
      to: proxy.address,
      data: PoseidonT3.data,
    });

    console.log(`PoseidonT3 deployed to: ${PoseidonT3.address}`);
  }

  // Deploy Enclave contract
  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("4503599626321921")]; // 0x3FFFFFFF000001

  // Encode just the struct (NOT the function selector)
  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const enclave = await deploy("Enclave", {
    from: deployer,
    args: [deployer, addressOne, THIRTY_DAYS_IN_SECONDS, [encoded]],
    log: true,
    libraries: {
      PoseidonT3: PoseidonT3.address,
    },
  });

  console.log(`Enclave contract: `, enclave.address);

  // Deploy CiphernodeRegistryOwnable contract

  const cypherNodeRegistry = await deploy("CiphernodeRegistryOwnable", {
    from: deployer,
    args: [deployer, enclave.address],
    log: true,
    libraries: {
      PoseidonT3: PoseidonT3.address,
    },
  });

  console.log(
    `CiphernodeRegistryOwnable contract: `,
    cypherNodeRegistry.address,
  );

  // Deploy NaiveRegistryFilter contract

  const naiveRegistryFilter = await deploy("NaiveRegistryFilter", {
    from: deployer,
    args: [deployer, cypherNodeRegistry.address],
    log: true,
  });

  console.log(`NaiveRegistryFilter contract: `, naiveRegistryFilter.address);

  // set registry in enclave
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclave.address,
  );

  const registryAddress = await enclaveContract.ciphernodeRegistry();

  if (registryAddress === cypherNodeRegistry.address) {
    console.log(`Enclave contract already has registry`);
    return;
  }

  const result = await enclaveContract.setCiphernodeRegistry(
    cypherNodeRegistry.address,
  );
  await result.wait();
  console.log(`Enclave contract updated with registry`);
};
export default func;
func.tags = ["enclave"];

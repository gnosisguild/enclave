import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
const addressOne = "0x0000000000000000000000000000000000000001";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  // Deploy Enclave contract

  const enclave = await deploy("Enclave", {
    from: deployer,
    args: [deployer, addressOne, THIRTY_DAYS_IN_SECONDS],
    log: true,
  });

  console.log(`Enclave contract: `, enclave.address);

  // Deploy CyphernodeRegistryOwnable contract

  const cypherNodeRegistry = await deploy("CyphernodeRegistryOwnable", {
    from: deployer,
    args: [deployer, enclave.address],
    log: true,
  });

  console.log(
    `CyphernodeRegistryOwnable contract: `,
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

  const registryAddress = await enclaveContract.cyphernodeRegistry();

  if (registryAddress === cypherNodeRegistry.address) {
    console.log(`Enclave contract already has registry`);
    return;
  }

  const result = await enclaveContract.setCyphernodeRegistry(
    cypherNodeRegistry.address,
  );
  await result.wait();
  console.log(`Enclave contract updated with registry`);
};
export default func;
func.id = "deploy_enclave"; // id required to prevent reexecution
func.tags = ["Enclave"];

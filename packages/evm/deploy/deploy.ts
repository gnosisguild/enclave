import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  const enclave = await deploy("Enclave", {
    from: deployer,
    args: [THIRTY_DAYS_IN_SECONDS],
    log: true,
  });

  console.log(`Enclave contract: `, enclave.address);
};
export default func;
func.id = "deploy_enclave"; // id required to prevent reexecution
func.tags = ["Enclave"];

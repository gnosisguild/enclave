import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function(hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  const program = await deploy("MyProgram", {
    from: deployer,
    args: [], // verifier, imageId
    log: true,
  });
};

export default func;
func.tags = ["main"];

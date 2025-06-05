import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  const verifier = await deploy("MockRISC0Verifier", {
    from: deployer,
    args: [],
    log: true,
  });

  const imageId = await deploy("ImageID", {
    from: deployer,
    args: [],
    log: true,
  });

  const e3Program = await deploy("MyProgram", {
    from: deployer,
    args: [verifier, imageId],
    log: true,
  });
};
export default func;
func.tags = ["default"];

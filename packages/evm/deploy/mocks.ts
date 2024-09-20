import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  await deploy("MockE3Program", {
    from: deployer,
    args: [],
    log: true,
  });

  await deploy("MockComputeProvider", {
    from: deployer,
    args: [],
    log: true,
  });

  await deploy("MockDecryptionVerifier", {
    from: deployer,
    args: [],
    log: true,
  });

  await deploy("MockInputValidator", {
    from: deployer,
    args: [],
    log: true,
  });
};
export default func;
func.tags = ["mocks"];

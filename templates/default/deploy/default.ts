import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function(hre: HardhatRuntimeEnvironment) {
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
  const imageIdContract = await hre.ethers.getContractAt(
    "ImageID",
    imageId.address,
  );
  const programId = await imageIdContract.PROGRAM_ID();

  const e3Program = await deploy("MyProgram", {
    from: deployer,
    args: [verifier.address, programId],
    log: true,
  });

  const [deployerAccount] = await hre.ethers.getSigners();
  const enclave = await hre.deployments.get("Enclave");

  const contract = new hre.ethers.Contract(
    enclave.address,
    enclave.abi,
    deployerAccount,
  );
  const result = contract.interface.encodeFunctionData("enableE3Program", [
    e3Program.address,
  ]);
  const tx = await deployerAccount.sendTransaction({
    to: enclave.address,
    data: result,
  });
  await tx.wait();
};
export default func;
func.tags = ["default"];
func.dependencies = ["enclave"];

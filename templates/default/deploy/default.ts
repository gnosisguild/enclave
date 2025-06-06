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

  // Set up MockDecryptionVerifier in Enclave contract
  const enclaveDeployment = await hre.deployments.get("Enclave");
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclaveDeployment.address,
  );
  try {
    const tx = await enclaveContract.enableE3Program(e3Program.address);
    await tx.wait();
    console.log(`Successfully enabled e3Program in Enclave contract`);
  } catch (error) {
    console.error("Error enabling e3Program:", error);
    process.exit(1);
  }
};
export default func;
func.tags = ["default"];
func.dependencies = ["enclave"];

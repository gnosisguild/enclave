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

  const mockDecryptionVerifier = await deploy("MockDecryptionVerifier", {
    from: deployer,
    args: [],
    log: true,
  });

  await deploy("MockInputValidator", {
    from: deployer,
    args: [],
    log: true,
  });

  // Set up MockDecryptionVerifier in Enclave contract
  const enclaveDeployment = await hre.deployments.get("Enclave");
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclaveDeployment.address,
  );

  const encryptionSchemeId =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

  try {
    const tx = await enclaveContract.setDecryptionVerifier(
      encryptionSchemeId,
      mockDecryptionVerifier.address,
    );
    await tx.wait();
    console.log(`Successfully set MockDecryptionVerifier in Enclave contract`);
  } catch (error) {
    console.error("Error setting MockDecryptionVerifier:", error);
    process.exit(1);
  }
};
export default func;
func.tags = ["mocks"];

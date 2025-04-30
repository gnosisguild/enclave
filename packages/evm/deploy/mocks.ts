import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

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

  const mockInputValidatorChecker = await deploy("MockInputValidatorChecker", {
    from: deployer,
    args: [],
    log: true,
  });

  const inputValidatorPolicyFactory = await deploy(
    "MockInputValidatorPolicyFactory",
    {
      from: deployer,
      args: [],
      log: true,
    },
  );

  const mockInputValidatorFactory = await deploy("MockInputValidatorFactory", {
    from: deployer,
    args: [],
    log: true,
  });

  const inputValidatorFactory = await hre.ethers.getContractAt(
    "MockInputValidatorFactory",
    mockInputValidatorFactory.address,
  );

  const policyFactory = await hre.ethers.getContractAt(
    "MockInputValidatorPolicyFactory",
    inputValidatorPolicyFactory.address,
  );

  const inputLimit = 100;
  const mockE3Deployment = await deploy("MockE3Program", {
    from: deployer,
    args: [
      inputValidatorPolicyFactory.address,
      mockInputValidatorFactory.address,
      mockInputValidatorChecker.address,
      inputLimit,
    ],
    log: true,
  });

  try {
    const tx = await policyFactory.transferOwnership(mockE3Deployment.address);
    await tx.wait();
    console.log(
      `Successfully transferred ownership of policy factory to E3Program contract`,
    );
  } catch (err) {
    console.error("Error setting owner address for policyFactory");
  }

  try {
    const tx = await inputValidatorFactory.transferOwnership(
      mockE3Deployment.address,
    );
    await tx.wait();
    console.log(
      `Successfully transferred ownership of input validator factory to E3Program contract`,
    );
  } catch (err) {
    console.error("Error setting owner address for input validator factory");
  }

  // Set up MockDecryptionVerifier in Enclave contract
  const enclaveDeployment = await hre.deployments.get("Enclave");
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclaveDeployment.address,
  );

  const encryptionSchemeId = hre.ethers.keccak256(
    hre.ethers.toUtf8Bytes("fhe.rs:BFV"),
  );

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

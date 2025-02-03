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
    args: [[]],
    log: true,
  });

  const inputValidatorPolicy = await deploy("InputValidatorPolicy", {
    from: deployer,
    args: [mockInputValidatorChecker.address],
    log: true,
  });

  await deploy("MockE3Program", {
    from: deployer,
    args: [inputValidatorPolicy.address],
    log: true,
  });

  // Set up MockDecryptionVerifier in Enclave contract
  const enclaveDeployment = await hre.deployments.get("Enclave");
  const enclaveContract = await hre.ethers.getContractAt(
    "Enclave",
    enclaveDeployment.address,
  );

  const inputValidatorPolicyContract = await hre.ethers.getContractAt(
    "InputValidatorPolicy",
    inputValidatorPolicy.address,
  );

  // NOTE: We must ensure that the target has been set for the policy so that the enclave contract is allowed to call the policy
  await inputValidatorPolicyContract.setTarget(enclaveDeployment.address);

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

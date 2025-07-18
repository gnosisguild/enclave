import "@nomicfoundation/hardhat-ethers";
import "hardhat-deploy";
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;
  if (!deployer)
    throw new Error("Deployer not returned from getNamedAddresses()");

  const computeProvider = await deploy("MockComputeProvider", {
    from: deployer,
    args: [],
    log: true,
  });

  const mockDecryptionVerifier = await deploy("MockDecryptionVerifier", {
    from: deployer,
    args: [],
    log: true,
  });

  const mockInputValidator = await deploy("MockInputValidator", {
    from: deployer,
    args: [],
    log: true,
  });

  const mockE3Program = await deploy("MockE3Program", {
    from: deployer,
    args: [mockInputValidator.address],
    log: true,
  });

  console.log(`
MockDeployments:
----------------------------------------------------------------------
MockComputeProvider:${computeProvider.address}
MockDecryptionVerifier:${mockDecryptionVerifier.address}
MockInputValidator:${mockInputValidator.address}
MockE3Program:${mockE3Program.address}
`);

  // Set up MockDecryptionVerifier in Enclave contract
  const enclaveDeployment = await hre.deployments.get("Enclave");
  const enclaveArtifact = await hre.deployments.getArtifact("Enclave");
  const enclaveContract = new hre.ethers.Contract(
    enclaveDeployment.address,
    enclaveArtifact.abi,
    await hre.ethers.getSigner(deployer),
  );

  const encryptionSchemeId = hre.ethers.keccak256(
    hre.ethers.toUtf8Bytes("fhe.rs:BFV"),
  );

  try {
    const tx = await enclaveContract.setDecryptionVerifier!(
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

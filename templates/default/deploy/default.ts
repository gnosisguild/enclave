import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

async function getImageId(hre: HardhatRuntimeEnvironment): Promise<string> {
  // Compile the contract
  await hre.run("compile");

  // Get the build info which contains the compilation output
  const buildInfo = await hre.artifacts.getBuildInfo(
    "contracts/ImageID.sol:ImageID",
  );
  if (!buildInfo) {
    throw new Error("Could not get build info for ImageID contract");
  }

  // Get the Abstract Syntax Tree (AST) from compilation
  const sourceAST = buildInfo.output.sources["contracts/ImageID.sol"].ast;

  // Function to recursively search through the AST
  function findProgramId(node: any): string | null {
    // Check if this node is a library definition
    if (node.nodeType === "ContractDefinition" && node.name === "ImageID") {
      // Look through all the members of this library
      for (const member of node.nodes || []) {
        // Check if this member is a variable declaration for PROGRAM_ID
        if (
          member.nodeType === "VariableDeclaration" &&
          member.name === "PROGRAM_ID" &&
          member.constant === true
        ) {
          // The value is stored in the initialValue field
          // Structure: bytes32(0x69f2bdcf375ce3bc8c934c729c38e16ade73301bcdc6e4ae97a98910c31ab11d)
          if (
            member.initialValue &&
            member.initialValue.nodeType === "FunctionCall" &&
            member.initialValue.expression.name === "bytes32"
          ) {
            // Get the hex literal inside bytes32()
            const hexLiteral = member.initialValue.arguments[0];
            if (hexLiteral && hexLiteral.nodeType === "Literal") {
              return hexLiteral.value; // This is the actual hex string
            }
          }
        }
      }
    }

    // If not found in this node, search child nodes recursively
    if (node.nodes) {
      for (const child of node.nodes) {
        const result = findProgramId(child);
        if (result) return result;
      }
    }

    return null;
  }

  const programId = findProgramId(sourceAST);

  if (!programId) {
    throw new Error("Could not extract PROGRAM_ID from AST");
  }

  console.log("Extracted PROGRAM_ID:", programId);
  return programId;
}

const func: DeployFunction = async function(hre: HardhatRuntimeEnvironment) {
  const { deployer } = await hre.getNamedAccounts();
  const { deploy } = hre.deployments;

  const verifier = await deploy("MockRISC0Verifier", {
    from: deployer,
    args: [],
    log: true,
  });

  const imageId = await getImageId(hre);

  const e3Program = await deploy("MyProgram", {
    from: deployer,
    args: [verifier.address, imageId],
    log: true,
  });
};
export default func;
func.tags = ["default"];

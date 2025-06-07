import deployEnclave from "@gnosis-guild/enclave/deploy/enclave";
import deployMocks from "@gnosis-guild/enclave/deploy/mocks";
import { DeployFunction } from "hardhat-deploy/types";
import { HardhatRuntimeEnvironment } from "hardhat/types";

const func: DeployFunction = async function (hre: HardhatRuntimeEnvironment) {
    await deployEnclave(hre);
    // INFO: We need to deploy the mock contract due to the decryptionVerifier.
    // Once we have a real verifier, we can remove this.
    await deployMocks(hre);
};

export default func;
func.tags = ["enclave", "mocks"];

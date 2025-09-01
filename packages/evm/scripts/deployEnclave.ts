import { network } from "hardhat";

import CiphernodeRegistryModule from "../ignition/modules/ciphernodeRegistry";
import EnclaveModule from "../ignition/modules/enclave";
import NaiveRegistryFilterModule from "../ignition/modules/naiveRegistryFilter";
import { Enclave__factory as EnclaveFactory } from "../types";
import { deployMocks } from "./deployMocks";

/**
 * Deploys the Enclave contracts
 */
export const deployEnclave = async () => {
  const { ignition, ethers } = await network.connect();

  const [owner] = await ethers.getSigners();

  const ownerAddress = await owner.getAddress();

  const polynomial_degree = ethers.toBigInt(2048);
  const plaintext_modulus = ethers.toBigInt(1032193);
  const moduli = [ethers.toBigInt("18014398492704769")];

  const encoded = ethers.AbiCoder.defaultAbiCoder().encode(
    ["uint256", "uint256", "uint256[]"],
    [polynomial_degree, plaintext_modulus, moduli],
  );

  const THIRTY_DAYS_IN_SECONDS = 60 * 60 * 24 * 30;
  const addressOne = "0x0000000000000000000000000000000000000001";

  const enclave = await ignition.deploy(EnclaveModule, {
    parameters: {
      Enclave: {
        params: encoded,
        owner: ownerAddress,
        maxDuration: THIRTY_DAYS_IN_SECONDS,
        registry: addressOne,
      },
    },
  });

  const enclaveAddress = await enclave.enclave.getAddress();

  console.log("Enclave deployed to: ", enclaveAddress);

  const ciphernodeRegistry = await ignition.deploy(CiphernodeRegistryModule, {
    parameters: {
      CiphernodeRegistry: {
        enclaveAddress: enclaveAddress,
        owner: ownerAddress,
      },
    },
  });

  const ciphernodeRegistryAddress =
    await ciphernodeRegistry.cipherNodeRegistry.getAddress();

  console.log("CiphernodeRegistry deployed to: ", ciphernodeRegistryAddress);

  const naiveRegistryFilter = await ignition.deploy(NaiveRegistryFilterModule, {
    parameters: {
      NaiveRegistryFilter: {
        ciphernodeRegistryAddress,
        owner: ownerAddress,
      },
    },
  });

  const naiveRegistryFilterAddress =
    await naiveRegistryFilter.naiveRegistryFilter.getAddress();

  console.log("NaiveRegistryFilter deployed to: ", naiveRegistryFilterAddress);

  const enclaveContract = EnclaveFactory.connect(enclaveAddress, owner);

  const registryAddress = await enclaveContract.ciphernodeRegistry();

  if (registryAddress === ciphernodeRegistryAddress) {
    console.log(`Enclave contract already has registry`);
    return;
  }

  await enclaveContract.setCiphernodeRegistry(ciphernodeRegistryAddress);

  console.log(`Enclave contract updated with registry`);

  // Deploy mocks only if specified
  const shouldDeployMocks = process.env.DEPLOY_MOCKS === "true";

  if (shouldDeployMocks) {
    const { decryptionVerifierAddress } = await deployMocks();
    const encryptionSchemeId = ethers.keccak256(
      ethers.toUtf8Bytes("fhe.rs:BFV"),
    );

    const tx = await enclaveContract.setDecryptionVerifier(
      encryptionSchemeId,
      decryptionVerifierAddress,
    );
    await tx.wait();
    console.log(`Successfully set MockDecryptionVerifier in Enclave contract`);
  }
};

deployEnclave().catch((error) => console.error(error));

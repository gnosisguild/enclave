import { network } from "hardhat";

import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import {
  MockDecryptionVerifier,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export const deployAndSaveMockDecryptionVerifier = async (): Promise<{
  decryptionVerifier: MockDecryptionVerifier;
}> => {
  const { ignition, ethers } = await network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const decryptionVerifier = await ignition.deploy(
    MockDecryptionVerifierModule,
  );
  const decryptionVerifierAddress =
    await decryptionVerifier.mockDecryptionVerifier.getAddress();

  const blockNumber = await signer.provider?.getBlockNumber();

  storeDeploymentArgs(
    {
      blockNumber,
      address: decryptionVerifierAddress,
    },
    "MockDecryptionVerifier",
    chain,
  );

  const decryptionVerifierContract = MockDecryptionVerifierFactory.connect(
    decryptionVerifierAddress,
    signer,
  );

  return { decryptionVerifier: decryptionVerifierContract };
};

// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import type { HardhatRuntimeEnvironment } from "hardhat/types/hre";

import MockDecryptionVerifierModule from "../../ignition/modules/mockDecryptionVerifier";
import {
  MockDecryptionVerifier,
  MockDecryptionVerifier__factory as MockDecryptionVerifierFactory,
} from "../../types";
import { storeDeploymentArgs } from "../utils";

export const deployAndSaveMockDecryptionVerifier = async (
  hre: HardhatRuntimeEnvironment,
): Promise<{
  decryptionVerifier: MockDecryptionVerifier;
}> => {
  const { ignition, ethers } = await hre.network.connect();
  const [signer] = await ethers.getSigners();
  const chain = (await signer.provider?.getNetwork())?.name ?? "localhost";

  const decryptionVerifier = await ignition.deploy(
    MockDecryptionVerifierModule,
  );

  await decryptionVerifier.mockDecryptionVerifier.waitForDeployment();
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

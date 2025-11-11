// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { network } from "hardhat";
import { zeroAddress, zeroHash } from "viem";
import { ZKInputsGenerator } from "@crisp-e3/zk-inputs";
import {
  encryptVoteAndGenerateCRISPInputs,
  generateProof,
  VotingMode,
  encodeVote,
  MESSAGE,
  generateMerkleProof,
  hashLeaf,
} from "@crisp-e3/sdk";

import { expect } from "chai";
import type { HonkVerifier, MockEnclave } from "../types";

let zkInputsGenerator = ZKInputsGenerator.withDefaults();
let publicKey = zkInputsGenerator.generatePublicKey();
const previousCiphertext = zkInputsGenerator.encryptVote(
  publicKey,
  new BigInt64Array([0n])
);

describe("CRISP Contracts", function () {
  const nonZeroAddress = "0xc6e7DF5E7b4f2A278906862b61205850344D4e7d";

  describe("deployment", () => {
    it("should deploy the contracts", async () => {
      const { ethers } = await network.connect();
      /*
                IEnclave _enclave,
                IRiscZeroVerifier _verifier,
                CRISPInputValidatorFactory _inputValidatorFactory,
                HonkVerifier _honkVerifier,
                bytes32 _imageId
            */
      const program = await ethers.deployContract("CRISPProgram", [
        nonZeroAddress,
        nonZeroAddress,
        nonZeroAddress,
        nonZeroAddress,
        zeroHash,
      ]);

      expect(await program.getAddress()).to.not.equal(zeroAddress);
    });
  });

  describe("decode tally", () => {
    it("should decode different tallies correctly", async () => {
      const { ethers } = await network.connect();
      const mockEnclave = (await ethers.deployContract(
        "MockEnclave"
      )) as MockEnclave;

      const program = await ethers.deployContract("CRISPProgram", [
        await mockEnclave.getAddress(),
        nonZeroAddress,
        nonZeroAddress,
        nonZeroAddress,
        zeroHash,
      ]);

      // 2 * 2 + 1 * 1 = 5 Y
      // 2 * 1 + 0 * 1 = 2 N
      const tally1 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 1, 0,
      ];

      await mockEnclave.setPlaintextOutput(tally1);

      const decodedTally1 = await program.decodeTally(0);

      expect(decodedTally1[0]).to.equal(5n);
      expect(decodedTally1[1]).to.equal(2n);

      // 1 * 1 + 2 * 2 + 5 * 16 + 8 * 1024 = 8277
      // 2 * 1 + 3 * 64 + 1024 =
      const tally2 = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 5,
        0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        0, 3, 0, 0, 0, 0, 1, 0,
      ];
      await mockEnclave.setPlaintextOutput(tally2);

      const decodedTally2 = await program.decodeTally(0);

      expect(decodedTally2[0]).to.equal(8277n);
      expect(decodedTally2[1]).to.equal(1218n);
    });
  });

  describe("validate input", () => {
    it("should verify the proof correctly with the crisp verifier", async function () {
      // It needs some time to generate the proof.
      this.timeout(60000);

      const { ethers } = await network.connect();

      const signers = await ethers.getSigners();
      const signer = signers[0];
      const address = (
        await signer.getAddress()
      ).toLowerCase() as `0x${string}`;

      const honkVerifier = (await ethers.deployContract(
        "HonkVerifier"
      )) as HonkVerifier;

      const vote = { yes: 10n, no: 0n };
      const votingPower = vote.yes;

      const encodedVote = encodeVote(vote, VotingMode.GOVERNANCE, votingPower);

      const signature = (await signer.signMessage(MESSAGE)) as `0x${string}`;
      const leaf = hashLeaf(address, vote.yes.toString());
      const leaves = [...[10n, 20n], leaf];

      const threshold = 0n;
      const merkleProof = generateMerkleProof(
        threshold,
        vote.yes,
        address,
        leaves
      );

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: vote.yes,
        slotAddress: address,
        isFirstVote: true,
      });

      const proof = await generateProof(inputs);

      const isValid = await honkVerifier.verify(
        proof.proof,
        proof.publicInputs
      );

      expect(isValid).to.be.true;
    });
  });
});
